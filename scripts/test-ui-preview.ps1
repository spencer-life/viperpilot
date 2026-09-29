[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string] $Executable,

    [ValidateSet('default', 'reversed', 'unavailable')]
    [string] $Scenario = 'default'
)

$ErrorActionPreference = 'Stop'
$windowTimeoutSeconds = 10
$controlTimeoutSeconds = 5
$closeTimeoutMilliseconds = 5000

function Write-ProgressMarker {
    param([Parameter(Mandatory = $true)][string] $Message)

    Write-Host ("[{0}] {1}" -f (Get-Date -Format 'HH:mm:ss'), $Message)
    [Console]::Out.Flush()
}

$resolvedExecutable = (Resolve-Path -LiteralPath $Executable -ErrorAction Stop).ProviderPath
if (-not (Test-Path -LiteralPath $resolvedExecutable -PathType Leaf)) {
    throw "Executable is not a file: $resolvedExecutable"
}
if ([System.IO.Path]::GetFileName($resolvedExecutable) -cne 'viperpilot-ui-preview.exe') {
    throw 'Executable must be named exactly viperpilot-ui-preview.exe.'
}

$repositoryRoot = Split-Path -Parent $PSScriptRoot
$assetRoot = Join-Path $repositoryRoot 'assets'
$artworkFiles = @(
    'viper-utility-icon.ico',
    'viper-v4-black-dashboard.bmp',
    'viperpilot-mouse-silhouette.bmp'
)
foreach ($artworkFile in $artworkFiles) {
    $artworkPath = Join-Path $assetRoot $artworkFile
    if (-not (Test-Path -LiteralPath $artworkPath -PathType Leaf)) {
        throw "Required preview artwork is missing: $artworkPath"
    }
}

Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes

if (-not ([System.Management.Automation.PSTypeName]'ViperPreviewSmokeNative').Type) {
    Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class ViperPreviewSmokeNative {
    [DllImport("user32.dll", SetLastError = true)]
    public static extern IntPtr GetDlgItem(IntPtr hDlg, int nIDDlgItem);
    [DllImport("user32.dll", SetLastError = true, EntryPoint = "SendMessageTimeoutW")]
    public static extern IntPtr SendMessageTimeout(IntPtr hWnd, uint message, IntPtr wParam, IntPtr lParam, uint flags, uint timeoutMilliseconds, out IntPtr result);
}
'@
}

$arguments = @()
if ($Scenario -eq 'reversed') {
    $arguments = @('--reversed')
} elseif ($Scenario -eq 'unavailable') {
    $arguments = @('--unavailable')
}

$previewProcess = $null
$stagingRoot = Join-Path $env:TEMP ("ViperPilotPreview-{0}" -f [Guid]::NewGuid().ToString('N'))
try {
    Write-ProgressMarker "Staging the preview and three artwork files for scenario '$Scenario'."
    New-Item -ItemType Directory -Path $stagingRoot -ErrorAction Stop | Out-Null
    $stagedExecutable = Join-Path $stagingRoot 'viperpilot-ui-preview.exe'
    Copy-Item -LiteralPath $resolvedExecutable -Destination $stagedExecutable -ErrorAction Stop
    foreach ($artworkFile in $artworkFiles) {
        Copy-Item -LiteralPath (Join-Path $assetRoot $artworkFile) -Destination (Join-Path $stagingRoot $artworkFile) -ErrorAction Stop
    }

    $startProcessParameters = @{
        FilePath = $stagedExecutable
        WorkingDirectory = $stagingRoot
        PassThru = $true
    }
    if ($arguments.Count -gt 0) {
        $startProcessParameters.ArgumentList = $arguments
    }

    Write-ProgressMarker 'Starting the staged preview process.'
    $previewProcess = Start-Process @startProcessParameters
    Write-ProgressMarker "Started preview PID $($previewProcess.Id); waiting up to $windowTimeoutSeconds seconds for its main HWND."

    $mainWindowHandle = [IntPtr]::Zero
    $windowDeadline = [DateTime]::UtcNow.AddSeconds($windowTimeoutSeconds)
    do {
        $previewProcess.Refresh()
        if ($previewProcess.HasExited) {
            throw "Preview exited before creating its main window (exit code $($previewProcess.ExitCode))."
        }
        $mainWindowHandle = $previewProcess.MainWindowHandle
        if ($mainWindowHandle -eq [IntPtr]::Zero) {
            Start-Sleep -Milliseconds 100
        }
    } while ($mainWindowHandle -eq [IntPtr]::Zero -and [DateTime]::UtcNow -lt $windowDeadline)

    if ($mainWindowHandle -eq [IntPtr]::Zero) {
        throw 'Timed out waiting for the preview main window HWND.'
    }
    $windowTitle = $previewProcess.MainWindowTitle
    if ($windowTitle -notmatch 'PREVIEW') {
        throw "Main window title does not identify the preview: '$windowTitle'."
    }
    Write-ProgressMarker "Found preview HWND $mainWindowHandle with title '$windowTitle'."

    $windowElement = [System.Windows.Automation.AutomationElement]::FromHandle($mainWindowHandle)
    if ($null -eq $windowElement) {
        throw 'UI Automation could not inspect the preview main window.'
    }

    function Get-PreviewButton {
        param([Parameter(Mandatory = $true)][int] $ControlId)

        $buttonHandle = [ViperPreviewSmokeNative]::GetDlgItem($mainWindowHandle, $ControlId)
        if ($buttonHandle -eq [IntPtr]::Zero) {
            throw "GetDlgItem did not find control ID $ControlId."
        }
        $button = [System.Windows.Automation.AutomationElement]::FromHandle($buttonHandle)
        if ($null -eq $button) {
            throw "UI Automation could not inspect control ID $ControlId."
        }
        $invokeAvailable = $false
        try {
            [void]$button.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)
            $invokeAvailable = $true
        } catch {
            $invokeAvailable = $false
        }
        Write-ProgressMarker ("Control {0}: UIA type={1}; name='{2}'; enabled={3}; InvokePattern={4}" -f $ControlId, $button.Current.ControlType.ProgrammaticName, $button.Current.Name, $button.Current.IsEnabled, $invokeAvailable)
        return $button
    }

    function Click-PreviewButton {
        param([Parameter(Mandatory = $true)][int] $ControlId)

        $buttonHandle = [ViperPreviewSmokeNative]::GetDlgItem($mainWindowHandle, $ControlId)
        if ($buttonHandle -eq [IntPtr]::Zero) {
            throw "GetDlgItem did not find control ID $ControlId for BM_CLICK."
        }
        $button = [System.Windows.Automation.AutomationElement]::FromHandle($buttonHandle)
        if ($null -eq $button.Current.IsEnabled -or -not $button.Current.IsEnabled) {
            throw "Refusing BM_CLICK for disabled control ID $ControlId."
        }
        $messageResult = [IntPtr]::Zero
        # SMTO_ABORTIFHUNG bounds cross-process BM_CLICK if the preview UI thread stops responding.
        $sendStatus = [ViperPreviewSmokeNative]::SendMessageTimeout(
            $buttonHandle,
            0x00F5,
            [IntPtr]::Zero,
            [IntPtr]::Zero,
            0x0002,
            2000,
            [ref]$messageResult
        )
        if ($sendStatus -eq [IntPtr]::Zero) {
            throw "BM_CLICK timed out or failed for control ID $ControlId (Win32 error $([Runtime.InteropServices.Marshal]::GetLastWin32Error()))."
        }
    }

    function Send-PreviewCommandNotification {
        param(
            [Parameter(Mandatory = $true)][int] $ControlId,
            [Parameter(Mandatory = $true)][int] $NotificationCode
        )

        $buttonHandle = [ViperPreviewSmokeNative]::GetDlgItem($mainWindowHandle, $ControlId)
        if ($buttonHandle -eq [IntPtr]::Zero) {
            throw "GetDlgItem did not find child control ID $ControlId for WM_COMMAND notification."
        }
        $packedCommand = [long](($NotificationCode * 65536) + $ControlId)
        $messageResult = [IntPtr]::Zero
        # SMTO_ABORTIFHUNG bounds this synthetic parent notification if the preview UI thread stops responding.
        $sendStatus = [ViperPreviewSmokeNative]::SendMessageTimeout(
            $mainWindowHandle,
            0x0111,
            [IntPtr]$packedCommand,
            $buttonHandle,
            0x0002,
            2000,
            [ref]$messageResult
        )
        if ($sendStatus -eq [IntPtr]::Zero) {
            throw "WM_COMMAND notification $NotificationCode timed out or failed for control ID $ControlId (Win32 error $([Runtime.InteropServices.Marshal]::GetLastWin32Error()))."
        }
    }

    function Assert-Button {
        param(
            [Parameter(Mandatory = $true)][int] $ControlId,
            [Parameter(Mandatory = $true)][string] $ExpectedName,
            [Parameter(Mandatory = $true)][bool] $ExpectedEnabled
        )

        $button = Get-PreviewButton -ControlId $ControlId
        $actualName = $button.Current.Name
        $actualEnabled = $button.Current.IsEnabled
        if ($actualName -cne $ExpectedName) {
            throw "Control ID $ControlId name mismatch: expected '$ExpectedName', got '$actualName'."
        }
        if ($actualEnabled -ne $ExpectedEnabled) {
            throw "Control ID $ControlId enabled-state mismatch: expected $ExpectedEnabled, got $actualEnabled."
        }
        return $button
    }

    function Test-WindowHasName {
        param([Parameter(Mandatory = $true)][string] $ExpectedText)

        $allElements = $windowElement.FindAll(
            [System.Windows.Automation.TreeScope]::Descendants,
            [System.Windows.Automation.Condition]::TrueCondition
        )
        for ($index = 0; $index -lt $allElements.Count; $index++) {
            $element = $allElements.Item($index)
            if ($element.Current.Name -ceq $ExpectedText) {
                Write-ProgressMarker "UIA name '$ExpectedText' is exposed as $($element.Current.ControlType.ProgrammaticName)."
                return $true
            }
        }
        return $false
    }

    function Wait-ForWindowName {
        param(
            [Parameter(Mandatory = $true)][string] $ExpectedText,
            [Parameter(Mandatory = $true)][string] $Context
        )

        $deadline = [DateTime]::UtcNow.AddSeconds($controlTimeoutSeconds)
        do {
            if (Test-WindowHasName -ExpectedText $ExpectedText) {
                return
            }
            Start-Sleep -Milliseconds 100
        } while ([DateTime]::UtcNow -lt $deadline)
        throw "Timed out waiting for profile UIA Name '$ExpectedText' after $Context."
    }

    $emDash = [string][char]0x2014
    switch ($Scenario) {
        'default' {
            $firstProfileName = "Switch to Developer $emDash complete Developer preset"
            $secondProfileName = "Switch to Gaming $emDash complete Gaming preset"
            $profilesEnabled = $true
            $firstExpectedProfile = 'Gaming'
            $secondExpectedProfile = 'Developer'
            $firstProfileControl = 2002
            $secondProfileControl = 2001
            $firstActionLabel = $secondProfileName
            $secondActionLabel = $firstProfileName
        }
        'reversed' {
            $firstProfileName = "Switch to Gaming $emDash complete Gaming preset"
            $secondProfileName = "Switch to Developer $emDash complete Developer preset"
            $profilesEnabled = $true
            $firstExpectedProfile = 'Gaming'
            $secondExpectedProfile = 'Developer'
            $firstProfileControl = 2001
            $secondProfileControl = 2002
            $firstActionLabel = $firstProfileName
            $secondActionLabel = $secondProfileName
        }
        'unavailable' {
            $firstProfileName = 'Saved profile unavailable'
            $secondProfileName = 'Saved profile unavailable'
            $profilesEnabled = $false
        }
    }

    Write-ProgressMarker 'Checking initial UI Automation button names and enabled states.'
    [void](Assert-Button -ControlId 2001 -ExpectedName $firstProfileName -ExpectedEnabled $profilesEnabled)
    [void](Assert-Button -ControlId 2002 -ExpectedName $secondProfileName -ExpectedEnabled $profilesEnabled)
    $detailsButton = Assert-Button -ControlId 2003 -ExpectedName 'Details' -ExpectedEnabled $true
    Wait-ForWindowName -ExpectedText 'Developer' -Context 'initial presentation'
    Write-ProgressMarker 'Initial button names, enabled states, and profile presentation passed.'

    if ($Scenario -eq 'reversed') {
        Write-ProgressMarker 'Sending BN_SETFOCUS to the first-slot Gaming child; expecting the displayed profile to remain Developer.'
        Send-PreviewCommandNotification -ControlId $firstProfileControl -NotificationCode 6
        if (-not (Test-WindowHasName -ExpectedText 'Developer')) {
            throw 'Non-click BN_SETFOCUS notification changed or obscured the initial Developer profile.'
        }
        if (Test-WindowHasName -ExpectedText 'Gaming') {
            throw 'Non-click BN_SETFOCUS notification incorrectly changed the displayed profile to Gaming.'
        }
        Write-ProgressMarker 'BN_SETFOCUS did not trigger the first-slot Gaming action.'
    }

    if ($Scenario -ne 'unavailable') {
        Write-ProgressMarker "Sending BM_CLICK to profile button $firstProfileControl; expecting '$firstExpectedProfile'."
        [void](Assert-Button -ControlId $firstProfileControl -ExpectedName $firstActionLabel -ExpectedEnabled $true)
        Click-PreviewButton -ControlId $firstProfileControl
        Wait-ForWindowName -ExpectedText $firstExpectedProfile -Context "button $firstProfileControl invocation"
        Write-ProgressMarker "Observed profile UIA Name '$firstExpectedProfile'."

        Write-ProgressMarker "Sending BM_CLICK to profile button $secondProfileControl; expecting '$secondExpectedProfile'."
        [void](Assert-Button -ControlId $secondProfileControl -ExpectedName $secondActionLabel -ExpectedEnabled $true)
        Click-PreviewButton -ControlId $secondProfileControl
        Wait-ForWindowName -ExpectedText $secondExpectedProfile -Context "button $secondProfileControl invocation"
        Write-ProgressMarker "Observed profile UIA Name '$secondExpectedProfile'."

        Write-ProgressMarker 'Sending first/second/first profile BM_CLICK messages back-to-back, without an intervening UIA read.'
        Click-PreviewButton -ControlId $firstProfileControl
        Click-PreviewButton -ControlId $secondProfileControl
        Click-PreviewButton -ControlId $firstProfileControl
        Wait-ForWindowName -ExpectedText $firstExpectedProfile -Context 'back-to-back profile button invocations'
        Write-ProgressMarker "Back-to-back profile commands settled on '$firstExpectedProfile'."
    }

    Write-ProgressMarker 'Sending BM_CLICK to the Details button HWND.'
    Click-PreviewButton -ControlId 2003

    $recoveryDeadline = [DateTime]::UtcNow.AddSeconds($controlTimeoutSeconds)
    $recoveryLabelsFound = $false
    do {
        try {
            $firstRecovery = Get-PreviewButton -ControlId 2001
            $secondRecovery = Get-PreviewButton -ControlId 2002
            $recoveryLabelsFound = (
                $firstRecovery.Current.Name -ceq 'Apply Developer recovery preset' -and
                $secondRecovery.Current.Name -ceq 'Apply Gaming recovery preset'
            )
        } catch {
            $recoveryLabelsFound = $false
        }
        if (-not $recoveryLabelsFound) {
            Start-Sleep -Milliseconds 100
        }
    } while (-not $recoveryLabelsFound -and [DateTime]::UtcNow -lt $recoveryDeadline)

    if (-not $recoveryLabelsFound) {
        $firstName = (Get-PreviewButton -ControlId 2001).Current.Name
        $secondName = (Get-PreviewButton -ControlId 2002).Current.Name
        throw "Details did not expose recovery labels; got '$firstName' and '$secondName'."
    }
    [void](Assert-Button -ControlId 2001 -ExpectedName 'Apply Developer recovery preset' -ExpectedEnabled $true)
    [void](Assert-Button -ControlId 2002 -ExpectedName 'Apply Gaming recovery preset' -ExpectedEnabled $true)
    [void](Assert-Button -ControlId 2003 -ExpectedName 'Quick switch' -ExpectedEnabled $true)
    Write-ProgressMarker 'Details exposed both recovery labels.'

    Write-ProgressMarker 'Toggling Details twice back-to-back, then checking the final Details labels.'
    Click-PreviewButton -ControlId 2003
    Click-PreviewButton -ControlId 2003
    [void](Assert-Button -ControlId 2001 -ExpectedName 'Apply Developer recovery preset' -ExpectedEnabled $true)
    [void](Assert-Button -ControlId 2002 -ExpectedName 'Apply Gaming recovery preset' -ExpectedEnabled $true)
    [void](Assert-Button -ControlId 2003 -ExpectedName 'Quick switch' -ExpectedEnabled $true)

    Write-ProgressMarker 'Switching to compact mode and checking both profile labels.'
    Click-PreviewButton -ControlId 2003
    [void](Assert-Button -ControlId 2001 -ExpectedName $firstProfileName -ExpectedEnabled $profilesEnabled)
    [void](Assert-Button -ControlId 2002 -ExpectedName $secondProfileName -ExpectedEnabled $profilesEnabled)
    [void](Assert-Button -ControlId 2003 -ExpectedName 'Details' -ExpectedEnabled $true)

    Write-ProgressMarker 'Switching back to Details and checking both recovery labels.'
    Click-PreviewButton -ControlId 2003
    [void](Assert-Button -ControlId 2001 -ExpectedName 'Apply Developer recovery preset' -ExpectedEnabled $true)
    [void](Assert-Button -ControlId 2002 -ExpectedName 'Apply Gaming recovery preset' -ExpectedEnabled $true)
    [void](Assert-Button -ControlId 2003 -ExpectedName 'Quick switch' -ExpectedEnabled $true)
    Write-ProgressMarker 'Repeated Details/compact transitions passed.'

    Write-ProgressMarker 'Closing the preview window and waiting up to 5 seconds for exit.'
    if (-not $previewProcess.CloseMainWindow()) {
        throw 'CloseMainWindow could not close the preview window.'
    }
    if (-not $previewProcess.WaitForExit($closeTimeoutMilliseconds)) {
        throw 'Preview did not exit within 5 seconds after CloseMainWindow.'
    }
    if ($previewProcess.ExitCode -ne 0) {
        throw "Preview exited with code $($previewProcess.ExitCode); expected 0."
    }

    Write-Output "PASS: $Scenario scenario verified UIA names/enabled states, back-to-back profile commands, repeated Details/compact transitions, and exit code 0."
} finally {
    if ($null -ne $previewProcess) {
        try {
            $previewProcess.Refresh()
            if (-not $previewProcess.HasExited) {
                Write-ProgressMarker 'Requesting preview shutdown during cleanup.'
                [void]$previewProcess.CloseMainWindow()
                if (-not $previewProcess.WaitForExit($closeTimeoutMilliseconds)) {
                    Write-ProgressMarker "Force-stopping only spawned preview PID $($previewProcess.Id) after close timeout."
                    $previewProcess.Kill()
                    [void]$previewProcess.WaitForExit($closeTimeoutMilliseconds)
                }
            }
            $previewProcess.Refresh()
            if (-not $previewProcess.HasExited) {
                throw "Spawned preview PID $($previewProcess.Id) did not exit; retaining staging directory '$stagingRoot'."
            }
        } finally {
            $previewProcess.Dispose()
        }
    }
    if (Test-Path -LiteralPath $stagingRoot -PathType Container) {
        Remove-Item -LiteralPath $stagingRoot -Recurse -Force
        Write-ProgressMarker 'Removed the temporary preview staging directory.'
    }
}

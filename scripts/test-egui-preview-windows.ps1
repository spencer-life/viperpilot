[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string] $Executable,

    [string] $ScreenshotPath
)

$ErrorActionPreference = 'Stop'
$windowTimeoutSeconds = 15
$closeTimeoutMilliseconds = 5000
$resolvedExecutable = (Resolve-Path -LiteralPath $Executable -ErrorAction Stop).ProviderPath
if (-not (Test-Path -LiteralPath $resolvedExecutable -PathType Leaf)) {
    throw "Preview executable is not a file: $resolvedExecutable"
}
if ([System.IO.Path]::GetFileName($resolvedExecutable) -cne 'viperpilot-egui-preview.exe') {
    throw 'Executable must be named exactly viperpilot-egui-preview.exe.'
}
if (-not [string]::IsNullOrWhiteSpace($ScreenshotPath) -and (Test-Path -LiteralPath $ScreenshotPath)) {
    Remove-Item -LiteralPath $ScreenshotPath -Force
}

function Write-ProgressMarker {
    param([Parameter(Mandatory = $true)][string] $Message)

    Write-Host ("[{0}] {1}" -f (Get-Date -Format 'HH:mm:ss'), $Message)
    [Console]::Out.Flush()
}

function Get-BitmapMetrics {
    param([Parameter(Mandatory = $true)][System.Drawing.Bitmap] $Bitmap)

    $stepX = [Math]::Max(1, [int]($Bitmap.Width / 24))
    $stepY = [Math]::Max(1, [int]($Bitmap.Height / 18))
    $distinct = New-Object 'System.Collections.Generic.HashSet[string]'
    $brightnessTotal = 0.0
    $sampleCount = 0
    for ($y = 0; $y -lt $Bitmap.Height; $y += $stepY) {
        for ($x = 0; $x -lt $Bitmap.Width; $x += $stepX) {
            $pixel = $Bitmap.GetPixel($x, $y)
            [void]$distinct.Add(("{0:X2}{1:X2}{2:X2}" -f $pixel.R, $pixel.G, $pixel.B))
            $brightnessTotal += ($pixel.R + $pixel.G + $pixel.B) / 3.0
            $sampleCount++
        }
    }
    return [PSCustomObject]@{
        AverageBrightness = if ($sampleCount -gt 0) { $brightnessTotal / $sampleCount } else { 0.0 }
        UniqueColors = $distinct.Count
    }
}

Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes

if (-not ([System.Management.Automation.PSTypeName]'ViperPilotEguiPreviewNative').Type) {
    Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class ViperPilotEguiPreviewNative {
    [StructLayout(LayoutKind.Sequential)]
    public struct RECT { public int Left; public int Top; public int Right; public int Bottom; }
    private delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);
    [DllImport("user32.dll")]
    private static extern bool EnumWindows(EnumWindowsProc callback, IntPtr lParam);
    [DllImport("user32.dll", SetLastError = true)]
    public static extern bool GetWindowRect(IntPtr hWnd, out RECT rect);
    [DllImport("user32.dll")]
    public static extern bool SetForegroundWindow(IntPtr hWnd);
    [DllImport("user32.dll")]
    public static extern bool ShowWindow(IntPtr hWnd, int command);
    [DllImport("user32.dll", SetLastError = true)]
    public static extern bool SetWindowPos(IntPtr hWnd, IntPtr hWndInsertAfter, int x, int y, int cx, int cy, uint flags);
    [DllImport("user32.dll")]
    public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll", SetLastError = true)]
    public static extern bool PrintWindow(IntPtr hWnd, IntPtr hdcBlt, uint flags);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    private static extern int GetWindowText(IntPtr hWnd, System.Text.StringBuilder text, int maxCount);
    [DllImport("user32.dll")]
    private static extern bool IsWindowVisible(IntPtr hWnd);
    [DllImport("user32.dll")]
    private static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint processId);
    [DllImport("user32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    public static extern bool PostMessage(IntPtr hWnd, uint message, IntPtr wParam, IntPtr lParam);
    public static IntPtr FindProcessWindow(int wantedProcessId, out string title) {
        IntPtr found = IntPtr.Zero;
        string foundTitle = "";
        EnumWindows((hWnd, lParam) => {
            uint processId;
            GetWindowThreadProcessId(hWnd, out processId);
            if (processId == (uint)wantedProcessId && IsWindowVisible(hWnd)) {
                var buffer = new System.Text.StringBuilder(512);
                GetWindowText(hWnd, buffer, buffer.Capacity);
                var candidate = buffer.ToString();
                if (!String.IsNullOrWhiteSpace(candidate)) {
                    found = hWnd;
                    foundTitle = candidate;
                    return false;
                }
            }
            return true;
        }, IntPtr.Zero);
        title = foundTitle;
        return found;
    }
}
'@
}

$stagingRoot = Join-Path $env:TEMP ("ViperPilotEguiPreview-{0}" -f [Guid]::NewGuid().ToString('N'))
$previewProcess = $null
$uiAutomationStatus = 'not inspected'
try {
    New-Item -ItemType Directory -Path $stagingRoot -ErrorAction Stop | Out-Null
    $stagedExecutable = Join-Path $stagingRoot 'viperpilot-egui-preview.exe'
    Copy-Item -LiteralPath $resolvedExecutable -Destination $stagedExecutable -ErrorAction Stop
    if ((Get-ChildItem -LiteralPath $stagingRoot -File).Count -ne 1) {
        throw 'The runtime smoke staging directory must contain only the preview executable.'
    }

    Write-ProgressMarker 'Starting the staged egui preview (synthetic state only).'
    $previewProcess = Start-Process -FilePath $stagedExecutable -WorkingDirectory $stagingRoot -PassThru
    $windowHandle = [IntPtr]::Zero
    $windowTitle = ''
    $windowDeadline = [DateTime]::UtcNow.AddSeconds($windowTimeoutSeconds)
    do {
        $previewProcess.Refresh()
        if ($previewProcess.HasExited) {
            throw "Preview exited before creating its window (exit code $($previewProcess.ExitCode))."
        }
        $windowHandle = [ViperPilotEguiPreviewNative]::FindProcessWindow($previewProcess.Id, [ref]$windowTitle)
        if ($windowHandle -eq [IntPtr]::Zero) {
            Start-Sleep -Milliseconds 150
        }
    } while ($windowHandle -eq [IntPtr]::Zero -and [DateTime]::UtcNow -lt $windowDeadline)

    if ($windowHandle -eq [IntPtr]::Zero) {
        throw 'Timed out waiting for the preview main window.'
    }
    Write-ProgressMarker "Found visible process window HWND=$windowHandle; title='$windowTitle'."
    if ($windowTitle -notmatch 'PREVIEW') {
        throw "Main window title does not identify the preview: '$windowTitle'."
    }
    Write-ProgressMarker "Window opened: '$windowTitle' (HWND $windowHandle)."

    [void][ViperPilotEguiPreviewNative]::SetForegroundWindow($windowHandle)
    Start-Sleep -Milliseconds 500
    if (-not [string]::IsNullOrWhiteSpace($ScreenshotPath)) {
        try {
            Add-Type -AssemblyName System.Drawing
            $rect = New-Object ViperPilotEguiPreviewNative+RECT
            if (-not [ViperPilotEguiPreviewNative]::GetWindowRect($windowHandle, [ref]$rect)) {
                throw 'GetWindowRect failed.'
            }
            $width = $rect.Right - $rect.Left
            $height = $rect.Bottom - $rect.Top
            if ($width -le 0 -or $height -le 0) {
                throw "Window has invalid screenshot bounds ${width}x${height}."
            }
            $bitmap = New-Object System.Drawing.Bitmap($width, $height)
            $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
            $deviceContext = [IntPtr]::Zero
            try {
                $deviceContext = $graphics.GetHdc()
                $captured = [ViperPilotEguiPreviewNative]::PrintWindow($windowHandle, $deviceContext, 2)
                $graphics.ReleaseHdc($deviceContext)
                $deviceContext = [IntPtr]::Zero
                $metrics = Get-BitmapMetrics -Bitmap $bitmap
                if (-not $captured -or $metrics.AverageBrightness -lt 8 -or $metrics.UniqueColors -lt 8) {
                    Write-ProgressMarker ("PrintWindow content was blank or incomplete (average luminance {0:N1}, sampled colors {1}); trying a target-window screen capture." -f $metrics.AverageBrightness, $metrics.UniqueColors)
                    [void][ViperPilotEguiPreviewNative]::ShowWindow($windowHandle, 9)
                    # HWND_TOPMOST, then restore normal z-order after the bounded capture.
                    [void][ViperPilotEguiPreviewNative]::SetWindowPos($windowHandle, [IntPtr](-1), 0, 0, 0, 0, 0x0001 -bor 0x0002 -bor 0x0040)
                    [void][ViperPilotEguiPreviewNative]::SetForegroundWindow($windowHandle)
                    Start-Sleep -Milliseconds 500
                    if ([ViperPilotEguiPreviewNative]::GetForegroundWindow() -ne $windowHandle) {
                        throw 'PrintWindow did not capture the GPU surface and Windows did not foreground the preview; refusing to save a different window as the preview screenshot.'
                    }
                    if (-not [ViperPilotEguiPreviewNative]::GetWindowRect($windowHandle, [ref]$rect)) {
                        throw 'GetWindowRect failed while preparing the target-window screen capture.'
                    }
                    $graphics.CopyFromScreen($rect.Left, $rect.Top, 0, 0, $bitmap.Size)
                    $metrics = Get-BitmapMetrics -Bitmap $bitmap
                }
                if ($metrics.AverageBrightness -lt 8 -or $metrics.UniqueColors -lt 8) {
                    throw ("Refusing an unverified screenshot: average luminance {0:N1}, sampled colors {1}." -f $metrics.AverageBrightness, $metrics.UniqueColors)
                }
                Write-ProgressMarker ("Screenshot pixel check passed: average luminance {0:N1}, sampled colors {1}." -f $metrics.AverageBrightness, $metrics.UniqueColors)
                [void][ViperPilotEguiPreviewNative]::SetWindowPos($windowHandle, [IntPtr](-2), 0, 0, 0, 0, 0x0001 -bor 0x0002 -bor 0x0040)
                $screenshotDirectory = Split-Path -Parent $ScreenshotPath
                if ($screenshotDirectory) {
                    New-Item -ItemType Directory -Path $screenshotDirectory -Force | Out-Null
                }
                $bitmap.Save($ScreenshotPath, [System.Drawing.Imaging.ImageFormat]::Png)
            } finally {
                if ($deviceContext -ne [IntPtr]::Zero) {
                    $graphics.ReleaseHdc($deviceContext)
                }
                $graphics.Dispose()
                $bitmap.Dispose()
            }
            Write-ProgressMarker "Captured preview window screenshot: $ScreenshotPath"
        } catch {
            Write-Warning "VISUAL QA INCONCLUSIVE: screenshot capture unavailable: $($_.Exception.Message)"
        }
    }

    $root = [System.Windows.Automation.AutomationElement]::FromHandle($windowHandle)
        if ($null -eq $root) {
            throw 'UI Automation returned no root element for the preview window.'
        }
        $elements = $root.FindAll(
            [System.Windows.Automation.TreeScope]::Descendants,
            [System.Windows.Automation.Condition]::TrueCondition
        )
        if ($elements.Count -eq 0) {
            throw 'UI Automation returned no child elements.'
        }
        $uiAutomationStatus = 'available'
        $profileField = $null
        $applyButton = $null
        $saveButton = $null
        for ($index = 0; $index -lt $elements.Count; $index++) {
            $element = $elements.Item($index)
            $current = $element.Current
            if ($current.ControlType -eq [System.Windows.Automation.ControlType]::Edit -and $current.Name -eq 'Profile name') {
                $profileField = $element
            }
            if ($current.ControlType -eq [System.Windows.Automation.ControlType]::Button -and $current.Name -eq 'Apply changes') {
                $applyButton = $element
            }
            if ($current.ControlType -eq [System.Windows.Automation.ControlType]::Button -and $current.Name -eq 'Save draft (unavailable)') {
                $saveButton = $element
            }
        }
        if ($null -eq $profileField) {
            throw 'UI Automation is available, but no Edit named Profile name was exposed.'
        }
        if (-not $profileField.Current.IsEnabled) {
            throw 'The Profile name Edit is unexpectedly disabled.'
        }
        Write-ProgressMarker ("UIA field: role={0}; name='{1}'; enabled={2}" -f $profileField.Current.ControlType.ProgrammaticName, $profileField.Current.Name, $profileField.Current.IsEnabled)
        if ($null -eq $applyButton) {
            throw 'UI Automation is available, but the Apply changes button was not exposed by name.'
        }
        if ($applyButton.Current.IsEnabled) {
            throw 'Apply changes must remain disabled in the preview.'
        }
        Write-ProgressMarker ("UIA action: role={0}; name='{1}'; enabled={2}" -f $applyButton.Current.ControlType.ProgrammaticName, $applyButton.Current.Name, $applyButton.Current.IsEnabled)
        if ($null -eq $saveButton) {
            throw 'UI Automation is available, but the unavailable Save draft button was not exposed by name.'
        }
        if ($saveButton.Current.IsEnabled) {
            throw 'Save draft must remain disabled because this preview has no persistence.'
        }
        Write-ProgressMarker ("UIA action: role={0}; name='{1}'; enabled={2}" -f $saveButton.Current.ControlType.ProgrammaticName, $saveButton.Current.Name, $saveButton.Current.IsEnabled)
    $uiAutomationStatus = 'available; required controls verified'
} finally {
    if ($null -ne $previewProcess) {
        $previewProcess.Refresh()
        if (-not $previewProcess.HasExited) {
            Write-ProgressMarker 'Closing the preview window.'
            [void]$previewProcess.CloseMainWindow()
            [void][ViperPilotEguiPreviewNative]::PostMessage($windowHandle, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero)
            if (-not $previewProcess.WaitForExit($closeTimeoutMilliseconds)) {
                Write-Warning 'Preview did not close promptly; stopping only the process started by this smoke test.'
                $previewProcess.Kill()
                [void]$previewProcess.WaitForExit($closeTimeoutMilliseconds)
            }
        }
        $previewProcess.Refresh()
        if (-not $previewProcess.HasExited) {
            throw 'The preview process remained alive after close/cleanup.'
        }
        if ($previewProcess.ExitCode -ne 0) {
            throw "Preview closed with exit code $($previewProcess.ExitCode)."
        }
        Write-ProgressMarker "Preview closed with exit code 0. UI Automation: $uiAutomationStatus."
        $previewProcess.Dispose()
    }
    if (Test-Path -LiteralPath $stagingRoot) {
        Remove-Item -LiteralPath $stagingRoot -Recurse -Force
    }
}

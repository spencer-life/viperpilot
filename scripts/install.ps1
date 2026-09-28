$ErrorActionPreference = 'Stop'

$running = Get-Process -Name 'viper-v4-utility' -ErrorAction SilentlyContinue
if ($running) {
    throw 'Viper V4 Pro Utility is already running. Exit it from the tray before reinstalling.'
}

$packageRoot = Split-Path -Parent $PSScriptRoot
$releaseRoot = Join-Path $packageRoot 'target\x86_64-pc-windows-msvc\release'
$traySource = Join-Path $releaseRoot 'viper-v4-utility.exe'
$cliSource = Join-Path $releaseRoot 'viperctl.exe'
$iconSource = Join-Path $packageRoot 'assets\viper-utility-icon.ico'
$mouseImageSource = Join-Path $packageRoot 'assets\viper-v4-black-dashboard.bmp'
foreach ($source in @($traySource, $cliSource, $iconSource, $mouseImageSource)) {
    if (-not (Test-Path -LiteralPath $source -PathType Leaf)) {
        throw "Missing Windows release binary: $source. Run mise run build-windows first."
    }
}

$programsRoot = Join-Path $env:LOCALAPPDATA 'Programs'
$installRoot = Join-Path $programsRoot 'ViperV4Utility'
$stagingRoot = Join-Path $programsRoot "ViperV4Utility.staging-$PID"
$backupRoot = Join-Path $programsRoot "ViperV4Utility.backup-$PID"
New-Item -ItemType Directory -Path $stagingRoot -Force | Out-Null
$trayInstalled = Join-Path $installRoot 'viper-v4-utility.exe'
$cliInstalled = Join-Path $installRoot 'viperctl.exe'
$iconInstalled = Join-Path $installRoot 'viper-utility-icon.ico'
$previousInstallMoved = $false
try {
    Copy-Item -LiteralPath $traySource -Destination (Join-Path $stagingRoot 'viper-v4-utility.exe')
    Copy-Item -LiteralPath $cliSource -Destination (Join-Path $stagingRoot 'viperctl.exe')
    Copy-Item -LiteralPath $iconSource -Destination (Join-Path $stagingRoot 'viper-utility-icon.ico')
    Copy-Item -LiteralPath $mouseImageSource -Destination (Join-Path $stagingRoot 'viper-v4-black-dashboard.bmp')
    if (Test-Path -LiteralPath $installRoot) {
        Move-Item -LiteralPath $installRoot -Destination $backupRoot
        $previousInstallMoved = $true
    }
    Move-Item -LiteralPath $stagingRoot -Destination $installRoot
} catch {
    if ($previousInstallMoved) {
        if (Test-Path -LiteralPath $installRoot) {
            Remove-Item -LiteralPath $installRoot -Recurse -Force
        }
        Move-Item -LiteralPath $backupRoot -Destination $installRoot
    }
    if (Test-Path -LiteralPath $stagingRoot) {
        Remove-Item -LiteralPath $stagingRoot -Recurse -Force
    }
    throw
}

$startMenu = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs'
$shortcutPath = Join-Path $startMenu 'Viper V4 Pro Utility.lnk'
$desktop = [Environment]::GetFolderPath('Desktop')
$desktopShortcutPath = Join-Path $desktop 'Viper V4 Pro Utility.lnk'
try {
    # Both shortcuts launch without --minimized, so they open/show the window.
    $shell = New-Object -ComObject WScript.Shell
    foreach ($linkPath in @($shortcutPath, $desktopShortcutPath)) {
        $shortcut = $shell.CreateShortcut($linkPath)
        $shortcut.TargetPath = $trayInstalled
        $shortcut.WorkingDirectory = $installRoot
        $shortcut.IconLocation = "$iconInstalled,0"
        $shortcut.Description = 'Switch fixed Developer and Gaming profiles on the Viper V4 Pro'
        $shortcut.Save()
    }

    $process = Start-Process -FilePath $trayInstalled -PassThru
    Start-Sleep -Milliseconds 750
    $process.Refresh()
    if ($process.HasExited) {
        throw "The installed tray exited immediately with code $($process.ExitCode)."
    }
} catch {
    if (Test-Path -LiteralPath $installRoot) {
        Remove-Item -LiteralPath $installRoot -Recurse -Force
    }
    if ($previousInstallMoved -and (Test-Path -LiteralPath $backupRoot)) {
        Move-Item -LiteralPath $backupRoot -Destination $installRoot
    }
    throw
}
if ($previousInstallMoved -and (Test-Path -LiteralPath $backupRoot)) {
    Remove-Item -LiteralPath $backupRoot -Recurse -Force
}
Write-Output "Installed tray: $trayInstalled"
Write-Output "Installed CLI:  $cliInstalled"
Write-Output "Installed icon: $iconInstalled"
Write-Output "Start Menu:     $shortcutPath"
Write-Output "Desktop:        $desktopShortcutPath"
Write-Output 'The tray is running. Start with Windows remains controlled by its tray-menu option.'

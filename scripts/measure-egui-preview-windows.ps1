[CmdletBinding()]
param([Parameter(Mandatory = $true)][string]$Executable)
$ErrorActionPreference = 'Stop'
$stage = Join-Path $env:TEMP ('ViperPilotEguiMeasure-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $stage | Out-Null
$staged = Join-Path $stage 'viperpilot-egui-preview.exe'
Copy-Item -LiteralPath $Executable -Destination $staged
$results = @()
try {
    for ($sample = 1; $sample -le 3; $sample++) {
        $process = $null
        try {
            $clock = [System.Diagnostics.Stopwatch]::StartNew()
            $process = Start-Process -FilePath $staged -WorkingDirectory $stage -PassThru
            $deadline = [DateTime]::UtcNow.AddSeconds(15)
            do {
                Start-Sleep -Milliseconds 50
                $process.Refresh()
                if ($process.HasExited) { throw ('preview exited before window: ' + $process.ExitCode) }
            } while ($process.MainWindowHandle -eq [IntPtr]::Zero -and [DateTime]::UtcNow -lt $deadline)
            if ($process.MainWindowHandle -eq [IntPtr]::Zero) { throw 'preview window timeout' }
            $openMs = $clock.ElapsedMilliseconds
            Start-Sleep -Seconds 2
            $process.Refresh()
            $cpuBefore = $process.TotalProcessorTime.TotalMilliseconds
            $workingSet = $process.WorkingSet64
            $privateBytes = $process.PrivateMemorySize64
            Start-Sleep -Seconds 5
            $process.Refresh()
            $cpuIdleMs = $process.TotalProcessorTime.TotalMilliseconds - $cpuBefore
            $results += [pscustomobject]@{
                sample = $sample
                window_open_ms = $openMs
                working_set_after_2s_mb = [Math]::Round($workingSet / 1MB, 1)
                private_bytes_after_2s_mb = [Math]::Round($privateBytes / 1MB, 1)
                cpu_ms_over_5s_idle = [Math]::Round($cpuIdleMs, 1)
            }
            [void]$process.CloseMainWindow()
            if (-not $process.WaitForExit(5000)) { throw 'preview did not close within 5 seconds' }
            if ($process.ExitCode -ne 0) { throw ('preview exit code: ' + $process.ExitCode) }
        } finally {
            if ($null -ne $process) {
                $process.Refresh()
                if (-not $process.HasExited) { $process.Kill(); [void]$process.WaitForExit(5000) }
                $process.Dispose()
            }
        }
    }
    $results | ConvertTo-Json -Depth 3
} finally {
    Remove-Item -LiteralPath $stage -Recurse -Force
}

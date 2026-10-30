[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
$viperRepository = Split-Path $PSScriptRoot -Parent
Push-Location $viperRepository
try {
    $viperCommit = (git rev-parse HEAD).Trim()
    if ($LASTEXITCODE -ne 0) { throw 'Cannot determine candidate source commit.' }
    if (git status --porcelain) { throw 'Commit candidate source before packaging.' }
    # 2026-10-29: compile only. No executable, installer or hardware command runs.
    cargo build --release --locked --bin viper-v4-utility --bin viperctl
    if ($LASTEXITCODE -ne 0) { throw 'Native core build failed.' }
    cargo build --release --locked --features ui-preview,egui-preview --bin viperpilot-ui-preview --bin viperpilot-egui-preview
    if ($LASTEXITCODE -ne 0) { throw 'Native preview/editor build failed.' }
    $viperName = 'ViperPilot-pilot-' + $viperCommit.Substring(0,12) + '-windows-x64'
    $viperStage = Join-Path $viperRepository ('target\dist\' + $viperName)
    $viperZip = $viperStage + '.zip'
    if ((Test-Path -LiteralPath $viperStage) -or (Test-Path -LiteralPath $viperZip)) {
        throw 'This exact candidate is already staged; preserve it instead of overwriting.'
    }
    New-Item -ItemType Directory -Path $viperStage | Out-Null
    foreach ($viperBinary in @('viper-v4-utility.exe','viperctl.exe','viperpilot-ui-preview.exe','viperpilot-egui-preview.exe')) {
        Copy-Item -LiteralPath (Join-Path $viperRepository ('target\release\' + $viperBinary)) -Destination $viperStage
    }
    foreach ($viperAsset in @('viper-utility-icon.ico','viperpilot-mouse-silhouette.bmp','viper-v4-black-dashboard.bmp')) {
        Copy-Item -LiteralPath (Join-Path $viperRepository ('assets\' + $viperAsset)) -Destination $viperStage
    }
    Copy-Item -LiteralPath (Join-Path $viperRepository 'LICENSE') -Destination $viperStage
    Copy-Item -LiteralPath (Join-Path $viperRepository 'docs\pilot-candidate.md') -Destination (Join-Path $viperStage 'START-HERE.md')
    Copy-Item -LiteralPath (Join-Path $viperRepository 'scripts\start-pilot.ps1') -Destination (Join-Path $viperStage 'Start-Pilot.ps1')
    [IO.File]::WriteAllText((Join-Path $viperStage 'Preview.cmd'), "@echo off`r`n`"%~dp0viperpilot-ui-preview.exe`"`r`n")
    [IO.File]::WriteAllText((Join-Path $viperStage 'Editor.cmd'), "@echo off`r`n`"%~dp0viperpilot-egui-preview.exe`" --draft-root `"%~dp0data\editor`"`r`n")
    @{
        repository = 'https://github.com/spencer-life/viperpilot'
        source_commit = $viperCommit
        target = 'x86_64-pc-windows-msvc'
        built_at_utc = [DateTime]::UtcNow.ToString('o')
        hardware_validated = $false
        distribution = 'draft supervised pilot candidate'
        pilot_global_hotkey = $false
        pilot_startup_changes = $false
    } | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $viperStage 'build-info.json') -Encoding utf8
    $viperHashes = Get-ChildItem -LiteralPath $viperStage -File | Sort-Object Name | ForEach-Object {
        (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant() + '  ' + $_.Name
    }
    $viperHashes | Set-Content -LiteralPath (Join-Path $viperStage 'SHA256SUMS.txt') -Encoding ascii
    Compress-Archive -LiteralPath $viperStage -DestinationPath $viperZip -CompressionLevel Optimal
    $viperZipHash = (Get-FileHash -LiteralPath $viperZip -Algorithm SHA256).Hash.ToLowerInvariant()
    ($viperZipHash + '  ' + [IO.Path]::GetFileName($viperZip)) | Set-Content -LiteralPath ($viperZip + '.sha256') -Encoding ascii
    [pscustomobject]@{ Zip=$viperZip; SHA256=$viperZipHash; SourceCommit=$viperCommit }
} finally { Pop-Location }

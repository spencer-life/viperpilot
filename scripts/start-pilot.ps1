[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$FullSerial,
    [string]$Root = (Join-Path $PSScriptRoot 'data\pilot'),
    [switch]$AllowEstablishedProfiles
)
$ErrorActionPreference = 'Stop'
# 2026-10-29: opt-in only; ordinary launch remains blocked. This script never
# stages/imports a baseline, installs, alters startup or stops another process.
$viperArguments = @('--pilot', '--root', $Root, '--confirm-device', $FullSerial)
if ($AllowEstablishedProfiles) { $viperArguments += '--allow-established-profiles' }
& (Join-Path $PSScriptRoot 'viper-v4-utility.exe') @viperArguments
exit $LASTEXITCODE

[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$FullSerial,
    [string]$Root = (Join-Path $PSScriptRoot 'data\pilot'),
    [switch]$AllowEstablishedProfiles,
    [switch]$PilotHotkey
)
$ErrorActionPreference = 'Stop'
# 2026-10-29: opt-in only; ordinary launch remains blocked. This script never
# stages/imports a baseline, installs, alters startup or stops another process.
# 2026-11-02: hotkey permission is explicit and applies only to this process.
if ($PilotHotkey -and -not $AllowEstablishedProfiles) {
    throw '-PilotHotkey requires -AllowEstablishedProfiles and separate owner review.'
}
$viperArguments = @('--pilot', '--root', $Root, '--confirm-device', $FullSerial)
if ($AllowEstablishedProfiles) { $viperArguments += '--allow-established-profiles' }
if ($PilotHotkey) { $viperArguments += '--pilot-hotkey' }
& (Join-Path $PSScriptRoot 'viper-v4-utility.exe') @viperArguments
exit $LASTEXITCODE

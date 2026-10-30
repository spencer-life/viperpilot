[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$Archive,

    [Parameter(Mandatory = $true)]
    [string]$ExpectedSha256,

    [Parameter(Mandatory = $true)]
    [string]$ExpectedCommit
)

$ErrorActionPreference = 'Stop'

# 2026-10-29: verify downloaded pilot archives without extracting or launching
# content; caller-supplied digest and commit bind inspection to the intended ZIP.
function Get-PilotSha256FromStream([System.IO.Stream]$Stream) {
    $hasher = [System.Security.Cryptography.SHA256]::Create()
    try {
        return [BitConverter]::ToString($hasher.ComputeHash($Stream)).Replace('-', '').ToLowerInvariant()
    } finally {
        $hasher.Dispose()
    }
}

function Read-PilotExact([System.IO.Stream]$Stream, [byte[]]$Buffer, [int]$Count) {
    $offset = 0
    while ($offset -lt $Count) {
        $read = $Stream.Read($Buffer, $offset, $Count - $offset)
        if ($read -le 0) { throw 'A packaged executable has a truncated PE header.' }
        $offset += $read
    }
}

function Test-PilotPeX64([System.IO.Compression.ZipArchiveEntry]$Entry) {
    # Keep header work bounded: accept at most a 1 MiB DOS stub before the PE header.
    if ($Entry.Length -lt 70 -or $Entry.Length -gt 536870912) {
        throw "Executable has an unsupported size: $($Entry.Name)"
    }
    $stream = $Entry.Open()
    try {
        $dos = New-Object byte[] 64
        Read-PilotExact $stream $dos 64
        if ($dos[0] -ne 0x4d -or $dos[1] -ne 0x5a) { throw "Executable is missing the MZ signature: $($Entry.Name)" }
        $peOffset = [BitConverter]::ToUInt32($dos, 0x3c)
        if ($peOffset -lt 64 -or $peOffset -gt 1048576 -or [uint64]$peOffset + 6 -gt [uint64]$Entry.Length) {
            throw "Executable has an invalid PE header offset: $($Entry.Name)"
        }
        $remaining = [int]$peOffset - 64
        $skipBuffer = New-Object byte[] 8192
        while ($remaining -gt 0) {
            $take = [Math]::Min($remaining, $skipBuffer.Length)
            $read = $stream.Read($skipBuffer, 0, $take)
            if ($read -le 0) { throw "Executable has a truncated PE header: $($Entry.Name)" }
            $remaining -= $read
        }
        $pe = New-Object byte[] 6
        Read-PilotExact $stream $pe 6
        if ($pe[0] -ne 0x50 -or $pe[1] -ne 0x45 -or $pe[2] -ne 0 -or $pe[3] -ne 0) {
            throw "Executable is missing the PE signature: $($Entry.Name)"
        }
        $machine = [BitConverter]::ToUInt16($pe, 4)
        if ($machine -ne 0x8664) { throw "Executable is not x64 (machine 0x$('{0:x4}' -f $machine)): $($Entry.Name)" }
    } finally {
        $stream.Dispose()
    }
}

if ($ExpectedSha256 -notmatch '^[0-9a-fA-F]{64}$') { throw 'ExpectedSha256 must be exactly 64 hexadecimal characters.' }
if ($ExpectedCommit -notmatch '^[0-9a-fA-F]{40}$') { throw 'ExpectedCommit must be exactly 40 hexadecimal characters.' }
if (-not (Test-Path -LiteralPath $Archive -PathType Leaf)) { throw 'Archive must name an existing file.' }
$archivePath = (Resolve-Path -LiteralPath $Archive).ProviderPath

Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem

$archiveStream = $null
$zip = $null
try {
    $archiveStream = [System.IO.File]::OpenRead($archivePath)
    $archiveHash = Get-PilotSha256FromStream $archiveStream
    if ($archiveHash -ne $ExpectedSha256.ToLowerInvariant()) { throw 'Archive SHA-256 does not match ExpectedSha256.' }

    $archiveStream.Position = 0
    $zip = New-Object System.IO.Compression.ZipArchive($archiveStream, [System.IO.Compression.ZipArchiveMode]::Read, $true)

    $expectedNames = @(
        'viper-v4-utility.exe', 'viperctl.exe', 'viperpilot-ui-preview.exe', 'viperpilot-egui-preview.exe',
        'viper-utility-icon.ico', 'viperpilot-mouse-silhouette.bmp', 'viper-v4-black-dashboard.bmp',
        'LICENSE', 'START-HERE.md', 'Start-Pilot.ps1', 'Preview.cmd', 'Editor.cmd', 'build-info.json', 'SHA256SUMS.txt'
    )
    $binaryNames = @('viper-v4-utility.exe', 'viperctl.exe', 'viperpilot-ui-preview.exe', 'viperpilot-egui-preview.exe')
    $rootName = 'ViperPilot-pilot-' + $ExpectedCommit.Substring(0, 12).ToLowerInvariant() + '-windows-x64'

    if ($zip.Entries.Count -ne 14) { throw "Archive must contain exactly 14 file entries; found $($zip.Entries.Count)." }

    $entriesByName = @{}
    $seenPaths = @{}
    $seenBasenames = @{}
    foreach ($entry in $zip.Entries) {
        $path = $entry.FullName.Replace([char]92, [char]47)
        if ([string]::IsNullOrEmpty($path) -or $path.StartsWith('/') -or $path -match '^[A-Za-z]:' -or $path -match '(^|/)\.\.?(/|$)') {
            throw "Archive contains an unsafe path: $path"
        }
        if ($path -notmatch '^([^/]+)/([^/]+)$') { throw "Archive entry is not a single file under the expected root: $path" }
        if ($matches[1] -cne $rootName) { throw "Archive entry has an unexpected root: $path" }
        $baseName = $matches[2]
        if ($baseName -match '[:*?"<>|]' -or $baseName.EndsWith('.') -or $baseName.EndsWith(' ')) { throw "Archive contains an invalid filename: $path" }
        $pathKey = $path.ToLowerInvariant()
        $baseKey = $baseName.ToLowerInvariant()
        if ($seenPaths.ContainsKey($pathKey)) { throw "Archive contains a duplicate path: $path" }
        if ($seenBasenames.ContainsKey($baseKey)) { throw "Archive contains duplicate basenames: $baseName" }
        $seenPaths[$pathKey] = $true
        $seenBasenames[$baseKey] = $true
        if ($entry.Length -gt 536870912) { throw "Archive entry exceeds the 512 MiB verification limit: $baseName" }
        $entriesByName[$baseName] = $entry
    }

    foreach ($name in $expectedNames) {
        if (-not $entriesByName.ContainsKey($name)) { throw "Archive is missing expected file: $name" }
    }
    foreach ($name in $entriesByName.Keys) {
        if ($expectedNames -cnotcontains $name) { throw "Archive contains an unexpected file: $name" }
    }

    $manifestEntry = $entriesByName['SHA256SUMS.txt']
    $manifestStream = $manifestEntry.Open()
    try {
        $reader = New-Object System.IO.StreamReader($manifestStream, [System.Text.Encoding]::ASCII, $false)
        try { $manifestText = $reader.ReadToEnd() } finally { $reader.Dispose() }
    } finally {
        if ($manifestStream) { $manifestStream.Dispose() }
    }
    $manifestLines = @($manifestText -split "`r?`n" | Where-Object { $_.Length -gt 0 })
    if ($manifestLines.Count -ne 13) { throw "SHA256SUMS.txt must contain exactly 13 entries; found $($manifestLines.Count)." }
    $manifest = @{}
    foreach ($line in $manifestLines) {
        if ($line -notmatch '^([0-9a-fA-F]{64})  ([^/\\]+)$') { throw 'SHA256SUMS.txt contains a malformed line.' }
        $name = $matches[2]
        if ($name -ceq 'SHA256SUMS.txt' -or $manifest.ContainsKey($name)) { throw "SHA256SUMS.txt has a duplicate or forbidden entry: $name" }
        if ($expectedNames -cnotcontains $name) { throw "SHA256SUMS.txt names an unexpected file: $name" }
        $manifest[$name] = $matches[1].ToLowerInvariant()
    }
    foreach ($name in $expectedNames) {
        if ($name -ceq 'SHA256SUMS.txt') { continue }
        if (-not $manifest.ContainsKey($name)) { throw "SHA256SUMS.txt is missing an entry for $name." }
        $entryStream = $entriesByName[$name].Open()
        try { $actualHash = Get-PilotSha256FromStream $entryStream }
        finally { $entryStream.Dispose() }
        if ($actualHash -ne $manifest[$name]) { throw "SHA-256 manifest mismatch: $name" }
    }

    $infoStream = $entriesByName['build-info.json'].Open()
    try {
        $reader = New-Object System.IO.StreamReader($infoStream, [System.Text.Encoding]::UTF8, $true)
        try { $buildInfo = $reader.ReadToEnd() | ConvertFrom-Json } finally { $reader.Dispose() }
    } finally {
        if ($infoStream) { $infoStream.Dispose() }
    }
    if ($buildInfo.source_commit -cne $ExpectedCommit.ToLowerInvariant()) { throw 'build-info.json source_commit does not match ExpectedCommit.' }
    if ($buildInfo.target -cne 'x86_64-pc-windows-msvc') { throw 'build-info.json target is not x86_64-pc-windows-msvc.' }

    foreach ($name in $binaryNames) { Test-PilotPeX64 $entriesByName[$name] }
} finally {
    if ($zip) { $zip.Dispose() }
    if ($archiveStream) { $archiveStream.Dispose() }
}

[pscustomobject]@{
    Verified = $true
    Archive = [System.IO.Path]::GetFileName($archivePath)
    ArchiveSha256 = $archiveHash
    SourceCommit = $ExpectedCommit.ToLowerInvariant()
    Target = 'x86_64-pc-windows-msvc'
    FileCount = 14
    ManifestFileCount = 13
    X64Executables = 4
}

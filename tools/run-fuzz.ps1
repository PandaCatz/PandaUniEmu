[CmdletBinding()]
param(
    [ValidateRange(1, [int]::MaxValue)]
    [int] $Runs = 10000,

    [ValidateRange(0, [int]::MaxValue)]
    [int] $MaxTotalTimeSeconds = 0,

    [ValidateNotNullOrEmpty()]
    [string] $Toolchain = 'nightly'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$projectRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$toolchainArgument = "+$Toolchain"
$cargoFuzz = & cargo $toolchainArgument fuzz --help 2>&1
if ($LASTEXITCODE -ne 0) {
    throw "cargo-fuzz is unavailable. Install it with: cargo install cargo-fuzz --version 0.13.2 --locked`n$cargoFuzz"
}

if ($env:OS -eq 'Windows_NT') {
    $rustcDetails = & rustc $toolchainArgument -vV 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "Failed to inspect Rust toolchain '$Toolchain'.`n$rustcDetails"
    }
    $hostMatch = [regex]::Match(($rustcDetails -join "`n"), '(?m)^host:\s+(\S+)$')
    if (-not $hostMatch.Success -or $hostMatch.Groups[1].Value -ne 'x86_64-pc-windows-msvc') {
        throw "Windows fuzz launching currently supports only x86_64-pc-windows-msvc; detected '$($hostMatch.Groups[1].Value)'."
    }

    $runtimeName = 'clang_rt.asan_dynamic-x86_64.dll'
    $candidateDirectories = [System.Collections.Generic.List[string]]::new()

    if (-not [string]::IsNullOrWhiteSpace($env:VCToolsInstallDir)) {
        $candidateDirectories.Add(
            (Join-Path $env:VCToolsInstallDir 'bin\Hostx64\x64')
        )
    }

    $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
    if (Test-Path -LiteralPath $vswhere -PathType Leaf) {
        $installationPath = & $vswhere -latest -products '*' `
            -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 `
            -property installationPath
        if ($LASTEXITCODE -eq 0 -and -not [string]::IsNullOrWhiteSpace($installationPath)) {
            $toolRoot = Join-Path $installationPath.Trim() 'VC\Tools\MSVC'
            if (Test-Path -LiteralPath $toolRoot -PathType Container) {
                Get-ChildItem -LiteralPath $toolRoot -Directory |
                    Sort-Object Name -Descending |
                    ForEach-Object {
                        $candidateDirectories.Add(
                            (Join-Path $_.FullName 'bin\Hostx64\x64')
                        )
                    }
            }
        }
    }

    $runtimeDirectory = $candidateDirectories |
        Where-Object { Test-Path -LiteralPath (Join-Path $_ $runtimeName) -PathType Leaf } |
        Select-Object -First 1

    if ([string]::IsNullOrWhiteSpace($runtimeDirectory)) {
        throw @"
The Windows AddressSanitizer runtime '$runtimeName' was not found.
Install the Visual Studio C++ AddressSanitizer component, then rerun this script.
"@
    }

    $pathEntries = $env:Path -split [System.IO.Path]::PathSeparator
    if ($pathEntries -notcontains $runtimeDirectory) {
        $env:Path = $runtimeDirectory + [System.IO.Path]::PathSeparator + $env:Path
    }
    Write-Host "Using Windows AddressSanitizer runtime from $runtimeDirectory"
}

$corpusDirectory = Join-Path $projectRoot 'fuzz/corpus/parse_ines'
$null = New-Item -ItemType Directory -Path $corpusDirectory -Force

function Write-GeneratedInesSeed {
    param(
        [Parameter(Mandatory = $true)][string] $Name,
        [Parameter(Mandatory = $true)][byte] $PrgBanks,
        [Parameter(Mandatory = $true)][byte] $ChrBanks,
        [byte] $Flags6 = 0,
        [byte] $Flags7 = 0,
        [switch] $TruncateLastByte
    )

    $trainerLength = if (($Flags6 -band 0x04) -ne 0) { 512 } else { 0 }
    $imageLength = 16 + $trainerLength + ([int] $PrgBanks * 16KB) +
        ([int] $ChrBanks * 8KB)
    if ($TruncateLastByte) {
        $imageLength--
    }

    $image = [byte[]]::new($imageLength)
    $header = [byte[]] @(
        0x4e, 0x45, 0x53, 0x1a, $PrgBanks, $ChrBanks, $Flags6, $Flags7,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00
    )
    [Array]::Copy($header, $image, [Math]::Min($header.Length, $image.Length))
    [IO.File]::WriteAllBytes((Join-Path $corpusDirectory $Name), $image)
}

# These are original, redistribution-safe zero-filled images. They make a clean
# CI corpus exercise successful whole-image paths that random 4 KiB input cannot
# reach, plus trainer, NES 2.0, and one-byte-truncated boundaries.
Write-GeneratedInesSeed -Name 'generated-nrom128.nes' -PrgBanks 1 -ChrBanks 1
Write-GeneratedInesSeed -Name 'generated-nrom256.nes' -PrgBanks 2 -ChrBanks 1
Write-GeneratedInesSeed -Name 'generated-trainer.nes' -PrgBanks 1 -ChrBanks 0 -Flags6 0x04
Write-GeneratedInesSeed -Name 'generated-nes2.nes' -PrgBanks 1 -ChrBanks 1 -Flags7 0x08
Write-GeneratedInesSeed -Name 'generated-truncated.nes' -PrgBanks 1 -ChrBanks 1 -TruncateLastByte

$logCorpusDirectory = Join-Path $projectRoot 'fuzz/corpus/parse_nestest_log'
$null = New-Item -ItemType Directory -Path $logCorpusDirectory -Force
$generatedLog = 'C000 A9 01 LDA A:00 X:00 Y:00 P:24 SP:FD PPU: 0, 21 CYC:7'
[IO.File]::WriteAllText(
    (Join-Path $logCorpusDirectory 'generated-valid-row.log'),
    $generatedLog,
    [Text.Encoding]::ASCII
)

$fuzzerArguments = @("-runs=$Runs", '-max_len=65536')
if ($MaxTotalTimeSeconds -gt 0) {
    $fuzzerArguments += "-max_total_time=$MaxTotalTimeSeconds"
}

Push-Location $projectRoot
try {
    & cargo $toolchainArgument fuzz run parse_ines --fuzz-dir fuzz -- @fuzzerArguments
    if ($LASTEXITCODE -ne 0) {
        throw "The parse_ines fuzz target failed with exit code $LASTEXITCODE."
    }
    & cargo $toolchainArgument fuzz run parse_nestest_log --fuzz-dir fuzz -- @fuzzerArguments
    if ($LASTEXITCODE -ne 0) {
        throw "The parse_nestest_log fuzz target failed with exit code $LASTEXITCODE."
    }
}
finally {
    Pop-Location
}

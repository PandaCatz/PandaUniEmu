[CmdletBinding()]
param(
    [string] $CacheDirectory = (Join-Path $env:TEMP 'PandaUniEmu-perfect6502-09fc542'),
    [string] $ArchivePath = (Join-Path $env:TEMP 'PandaUniEmu-perfect6502-09fc542.zip'),
    [switch] $Acquire,
    [switch] $AcceptNonCommercialLicense
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$commit = '09fc542877a84318291aa42dab143a3e2c3db974'
$archiveSha256 = '594553A873D66A13E88C134495C9F55E064A36BA4670B07FBA71F5047A77BDF5'
$sourceDirectory = Join-Path $CacheDirectory "perfect6502-$commit"
$downloadUrl = "https://codeload.github.com/mist64/perfect6502/zip/$commit"
$projectRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$harness = Join-Path $PSScriptRoot 'perfect6502-oracle.c'

function Assert-OutsideProjectPath {
    param(
        [Parameter(Mandatory = $true)][string] $Path,
        [Parameter(Mandatory = $true)][string] $Label
    )

    $root = [System.IO.Path]::GetFullPath($projectRoot).TrimEnd('\')
    $candidate = [System.IO.Path]::GetFullPath($Path)
    if ($candidate -match '[\r\n"]') {
        throw "$Label contains a character that is unsafe for the build command."
    }
    if ($candidate.Equals($root, [System.StringComparison]::OrdinalIgnoreCase) -or
        $candidate.StartsWith($root + '\', [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "$Label must stay outside the PandaUniEmu repository: $candidate"
    }

    $existing = $candidate
    while (-not (Test-Path -LiteralPath $existing)) {
        $parent = Split-Path -Parent $existing
        if ([string]::IsNullOrWhiteSpace($parent) -or $parent -eq $existing) {
            break
        }
        $existing = $parent
    }
    while (Test-Path -LiteralPath $existing) {
        $item = Get-Item -LiteralPath $existing -Force
        if (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
            throw "$Label cannot use a reparse-point path: $($item.FullName)"
        }
        $parent = Split-Path -Parent $item.FullName
        if ([string]::IsNullOrWhiteSpace($parent) -or $parent -eq $item.FullName) {
            break
        }
        $existing = $parent
    }
}

Assert-OutsideProjectPath -Path $CacheDirectory -Label 'Oracle cache'
Assert-OutsideProjectPath -Path $ArchivePath -Label 'Oracle archive'

if (-not $AcceptNonCommercialLicense) {
    throw @'
perfect6502's required netlist_6502.h is licensed CC BY-NC-SA 3.0.
Pass -AcceptNonCommercialLicense only after reviewing
https://creativecommons.org/licenses/by-nc-sa/3.0/ and accepting that the
downloaded oracle is for noncommercial use. The emulator does not require it.
'@
}

function Assert-FileHash {
    param(
        [Parameter(Mandatory = $true)][string] $Path,
        [Parameter(Mandatory = $true)][string] $Expected
    )

    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw "Missing pinned oracle file: $Path"
    }
    $actual = (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash
    if ($actual -ne $Expected) {
        throw "SHA-256 mismatch for ${Path}: expected $Expected, got $actual."
    }
}

if (-not (Test-Path -LiteralPath $ArchivePath -PathType Leaf)) {
    if (-not $Acquire) {
        throw "Pinned archive is absent. Re-run with -Acquire after reviewing the license notice."
    }
    $archiveParent = Split-Path -Parent $ArchivePath
    New-Item -ItemType Directory -Force -Path $archiveParent | Out-Null
    $partial = "$ArchivePath.$([guid]::NewGuid().ToString('N')).partial"
    try {
        & curl.exe --silent --show-error --fail --location `
            --proto '=https' --tlsv1.2 --max-filesize 1048576 `
            --output $partial $downloadUrl
        if ($LASTEXITCODE -ne 0) {
            throw "curl failed with exit code $LASTEXITCODE."
        }
        Assert-FileHash -Path $partial -Expected $archiveSha256
        Move-Item -LiteralPath $partial -Destination $ArchivePath
    }
    finally {
        if (Test-Path -LiteralPath $partial) {
            Remove-Item -LiteralPath $partial
        }
    }
}

Assert-FileHash -Path $ArchivePath -Expected $archiveSha256

if (-not (Test-Path -LiteralPath $sourceDirectory -PathType Container)) {
    New-Item -ItemType Directory -Force -Path $CacheDirectory | Out-Null
    Expand-Archive -LiteralPath $ArchivePath -DestinationPath $CacheDirectory
}

$expectedFiles = [ordered]@{
    'perfect6502.c' = 'CAC56DAB1D6A08852361870191D9D5F633450939C14B7E5505E26DA78146BBBF'
    'perfect6502.h' = '15AB13035B71D5008BD14D993B34656DF088D760D18308A7FB64D7B28C53D340'
    'netlist_sim.c' = '19D1E30504FB13C27D79F8C8F01DF5D080B30B621A60A548B1B84C614D7CAED2'
    'netlist_sim.h' = 'FE483A7F43F973DFC388B02410A711CEB5B492AB08970429CC16A7CD0CAF70BB'
    'types.h' = '484747D5C63F0B4C1C8ED897EA52606BD7521B08F78A469582603E85A678F3BC'
    'netlist_6502.h' = '7A5A28F64A0D464D18FAECD3D715D96549BC5DA8F05E6F468EF3DAE97EF0F340'
    'LICENSE' = '29F44F6AF3005961E76E712A8B0F36FAF4D8C3D8E2592CA191876154ADFF2179'
}
foreach ($entry in $expectedFiles.GetEnumerator()) {
    Assert-FileHash -Path (Join-Path $sourceDirectory $entry.Key) -Expected $entry.Value
}

$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
if (-not (Test-Path -LiteralPath $vswhere -PathType Leaf)) {
    throw 'Visual Studio Installer vswhere.exe was not found; MSVC Build Tools are required.'
}
$installationPath = (& $vswhere -latest -products * `
    -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 `
    -property installationPath | Select-Object -First 1)
if ([string]::IsNullOrWhiteSpace($installationPath)) {
    throw 'Visual Studio C++ Build Tools were not found.'
}
$devCmd = Join-Path $installationPath 'Common7\Tools\VsDevCmd.bat'
if (-not (Test-Path -LiteralPath $devCmd -PathType Leaf)) {
    throw "Visual Studio developer command file was not found: $devCmd"
}

$toolRoots = Get-ChildItem -LiteralPath (Join-Path $installationPath 'VC\Tools\MSVC') -Directory |
    Sort-Object Name -Descending
$compiler = $toolRoots | ForEach-Object {
    Join-Path $_.FullName 'bin\Hostx64\x64\cl.exe'
} | Where-Object { Test-Path -LiteralPath $_ -PathType Leaf } | Select-Object -First 1
if ([string]::IsNullOrWhiteSpace($compiler)) {
    throw 'The x64 MSVC compiler executable was not found.'
}

$buildDirectory = Join-Path $env:TEMP `
    "PandaUniEmu-perfect6502-build-09fc542-$([guid]::NewGuid().ToString('N'))"
Assert-OutsideProjectPath -Path $buildDirectory -Label 'Oracle build directory'
if (Test-Path -LiteralPath $buildDirectory) {
    throw "Fresh oracle build directory unexpectedly exists: $buildDirectory"
}
New-Item -ItemType Directory -Path $buildDirectory | Out-Null
$executable = Join-Path $buildDirectory 'perfect6502-oracle.exe'
$perfect6502 = Join-Path $sourceDirectory 'perfect6502.c'
$netlistSimulator = Join-Path $sourceDirectory 'netlist_sim.c'
$compile = "call `"$devCmd`" -no_logo -arch=x64 -host_arch=x64 && " +
    "`"$compiler`" /nologo /std:c11 /O2 /W4 /WX " +
    "/I`"$sourceDirectory`" `"$harness`" `"$perfect6502`" `"$netlistSimulator`" " +
    "/Fe:`"$executable`""
Push-Location $buildDirectory
try {
    & cmd.exe /d /s /c $compile
    if ($LASTEXITCODE -ne 0) {
        throw "MSVC failed with exit code $LASTEXITCODE."
    }
}
finally {
    Pop-Location
}

function Invoke-OracleTrace {
    param(
        [Parameter(Mandatory = $true)][string] $Scenario,
        [Parameter(Mandatory = $true)][int] $AssertCycle,
        [Parameter(Mandatory = $true)][int] $Cycles
    )

    $lines = @(& $executable $Scenario $AssertCycle $Cycles)
    if ($LASTEXITCODE -ne 0) {
        throw "Oracle scenario $Scenario failed with exit code $LASTEXITCODE."
    }
    if ($lines.Count -ne $Cycles) {
        throw "Oracle scenario $Scenario returned $($lines.Count) cycles; expected $Cycles."
    }
    $records = foreach ($line in $lines) {
        if ($line -notmatch '^(\d+) SYNC=([01]) ([RW]) ([0-9A-F]{4}) ([0-9A-F]{2}) PC=([0-9A-F]{4}) SP=([0-9A-F]{2}) P=([0-9A-F]{2}) IR=([0-9A-F]{2})$') {
            throw "Malformed oracle trace line: $line"
        }
        [pscustomobject]@{
            Cycle = [int]$Matches[1]
            Sync = [int]$Matches[2]
            Kind = $Matches[3]
            Address = $Matches[4]
            Data = $Matches[5]
            Pc = $Matches[6]
        }
    }
    return @($records)
}

function Assert-BusSequence {
    param(
        [Parameter(Mandatory = $true)] $Trace,
        [Parameter(Mandatory = $true)][int] $FirstCycle,
        [Parameter(Mandatory = $true)][string[]] $Expected
    )

    $actual = @($Trace | Where-Object {
        $_.Cycle -ge $FirstCycle -and $_.Cycle -lt ($FirstCycle + $Expected.Count)
    } | ForEach-Object { "$($_.Kind):$($_.Address):$($_.Data)" })
    if (($actual -join ',') -ne ($Expected -join ',')) {
        throw "Bus mismatch at cycle ${FirstCycle}: expected $($Expected -join ' '), got $($actual -join ' ')."
    }
}

function Assert-CycleAddress {
    param(
        [Parameter(Mandatory = $true)] $Trace,
        [Parameter(Mandatory = $true)][int] $Cycle,
        [Parameter(Mandatory = $true)][string] $Kind,
        [Parameter(Mandatory = $true)][string] $Address
    )

    $record = $Trace | Where-Object Cycle -eq $Cycle | Select-Object -First 1
    if ($null -eq $record -or $record.Kind -ne $Kind -or $record.Address -ne $Address) {
        throw "Expected cycle $Cycle to be $Kind $Address."
    }
}

$irq = Invoke-OracleTrace irq 15 25
Assert-BusSequence $irq 17 @(
    'R:8005:EA', 'R:8005:EA', 'W:01FD:80', 'W:01FC:05',
    'W:01FB:A0', 'R:FFFE:00', 'R:FFFF:90'
)

$nmi = Invoke-OracleTrace nmi 15 25
Assert-BusSequence $nmi 17 @(
    'R:8005:EA', 'R:8005:EA', 'W:01FD:80', 'W:01FC:05',
    'W:01FB:A0', 'R:FFFA:00', 'R:FFFB:A0'
)

$reset = Invoke-OracleTrace reset 16 29
Assert-BusSequence $reset 21 @(
    'R:EAE9:EA', 'R:EAE9:EA', 'R:01FD:EA', 'R:01FC:EA',
    'R:01FB:EA', 'R:FFFC:00', 'R:FFFD:80'
)

$pollOnTime = Invoke-OracleTrace irq 15 26
$pollLate = Invoke-OracleTrace irq 16 27
Assert-CycleAddress $pollOnTime 20 W '01FC'
Assert-CycleAddress $pollLate 22 W '01FC'
if (($pollOnTime | Where-Object Cycle -eq 20).Data -ne '05' -or
    ($pollLate | Where-Object Cycle -eq 22).Data -ne '06') {
    throw 'Second-to-last-cycle polling did not preserve the expected return PCs.'
}

$hijackOnTime = Invoke-OracleTrace irq-nmi 20 25
$hijackLate = Invoke-OracleTrace irq-nmi 21 25
Assert-CycleAddress $hijackOnTime 22 R 'FFFA'
Assert-CycleAddress $hijackOnTime 23 R 'FFFB'
Assert-CycleAddress $hijackLate 22 R 'FFFE'
Assert-CycleAddress $hijackLate 23 R 'FFFF'

$brkHijackOnTime = Invoke-OracleTrace brk-nmi 22 27
$brkHijackLate = Invoke-OracleTrace brk-nmi 23 27
Assert-CycleAddress $brkHijackOnTime 23 W '01FB'
Assert-CycleAddress $brkHijackOnTime 24 R 'FFFA'
Assert-CycleAddress $brkHijackOnTime 25 R 'FFFB'
Assert-CycleAddress $brkHijackLate 23 W '01FB'
Assert-CycleAddress $brkHijackLate 24 R 'FFFE'
Assert-CycleAddress $brkHijackLate 25 R 'FFFF'
if (($brkHijackOnTime | Where-Object Cycle -eq 23).Data -ne 'B0' -or
    ($brkHijackLate | Where-Object Cycle -eq 23).Data -ne 'B0') {
    throw 'BRK hijack scenario did not push status with the break flag set.'
}

$branchCases = @(
    @{ Scenario = 'branch-not'; OnTime = 15; Late = 16; OnCycle = 20; LateCycle = 22; OnPc = '06'; LatePc = '07' },
    @{ Scenario = 'branch-taken'; OnTime = 15; Late = 16; OnCycle = 21; LateCycle = 23; OnPc = '06'; LatePc = '07' },
    @{ Scenario = 'branch-cross'; OnTime = 17; Late = 18; OnCycle = 22; LateCycle = 24; OnPc = 'FF'; LatePc = '00' }
)
foreach ($case in $branchCases) {
    $onTime = Invoke-OracleTrace $case.Scenario $case.OnTime 30
    $late = Invoke-OracleTrace $case.Scenario $case.Late 32
    Assert-CycleAddress $onTime $case.OnCycle W '01FC'
    Assert-CycleAddress $late $case.LateCycle W '01FC'
    if (($onTime | Where-Object Cycle -eq $case.OnCycle).Data -ne $case.OnPc -or
        ($late | Where-Object Cycle -eq $case.LateCycle).Data -ne $case.LatePc) {
        throw "Branch polling mismatch for $($case.Scenario)."
    }
}

Write-Host 'Verified pinned perfect6502 oracle: IRQ, NMI, reset, poll timing, branch paths, and NMI hijacking.'
Write-Host "External source remained in: $sourceDirectory"
Write-Host "Temporary oracle executable: $executable"

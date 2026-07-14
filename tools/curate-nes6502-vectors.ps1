[CmdletBinding()]
param(
    [string] $SourceDirectory = (Join-Path $env:TEMP 'PandaUniEmu-65x02-2f6980a'),
    [string] $OutputPath = '',
    [switch] $UseExistingDownloads
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

if ([string]::IsNullOrWhiteSpace($OutputPath)) {
    $OutputPath = Join-Path $PSScriptRoot '..\crates\cpu-6502\src\singlestep_vectors.rs'
}

$upstreamCommit = '2f6980a2d95757486c7bee24355c360e40e2a224'
$expectedOpcodes = @(
    0x00, 0x01, 0x05, 0x06, 0x08, 0x09, 0x0a, 0x0d, 0x0e,
    0x10, 0x11, 0x15, 0x16, 0x18, 0x19, 0x1d, 0x1e,
    0x20, 0x21, 0x24, 0x25, 0x26, 0x28, 0x29, 0x2a, 0x2c, 0x2d, 0x2e,
    0x30, 0x31, 0x35, 0x36, 0x38, 0x39, 0x3d, 0x3e,
    0x40, 0x41, 0x45, 0x46, 0x48, 0x49, 0x4a, 0x4c, 0x4d, 0x4e,
    0x50, 0x51, 0x55, 0x56, 0x58, 0x59, 0x5d, 0x5e,
    0x60, 0x61, 0x65, 0x66, 0x68, 0x69, 0x6a, 0x6c, 0x6d, 0x6e,
    0x70, 0x71, 0x75, 0x76, 0x78, 0x79, 0x7d, 0x7e,
    0x81, 0x84, 0x85, 0x86, 0x88, 0x8a, 0x8c, 0x8d, 0x8e,
    0x90, 0x91, 0x94, 0x95, 0x96, 0x98, 0x99, 0x9a, 0x9d,
    0xa0, 0xa1, 0xa2, 0xa4, 0xa5, 0xa6, 0xa8, 0xa9, 0xaa, 0xac, 0xad, 0xae,
    0xb0, 0xb1, 0xb4, 0xb5, 0xb6, 0xb8, 0xb9, 0xba, 0xbc, 0xbd, 0xbe,
    0xc0, 0xc1, 0xc4, 0xc5, 0xc6, 0xc8, 0xc9, 0xca, 0xcc, 0xcd, 0xce,
    0xd0, 0xd1, 0xd5, 0xd6, 0xd8, 0xd9, 0xdd, 0xde,
    0xe0, 0xe1, 0xe4, 0xe5, 0xe6, 0xe8, 0xe9, 0xea, 0xec, 0xed, 0xee,
    0xf0, 0xf1, 0xf5, 0xf6, 0xf8, 0xf9, 0xfd, 0xfe
)
$branchOpcodes = @(0x10, 0x30, 0x50, 0x70, 0x90, 0xb0, 0xd0, 0xf0)
$pagePenaltyBaseFive = @(0x11, 0x31, 0x51, 0x71, 0xb1, 0xd1, 0xf1)
$pagePenaltyBaseFour = @(
    0x19, 0x1d, 0x39, 0x3d, 0x59, 0x5d, 0x79, 0x7d,
    0xb9, 0xbc, 0xbd, 0xbe, 0xd9, 0xdd, 0xf9, 0xfd
)

if ($expectedOpcodes.Count -ne 151) {
    throw "Curator opcode table has $($expectedOpcodes.Count) entries; expected 151."
}
if (($expectedOpcodes | Sort-Object -Unique).Count -ne 151) {
    throw 'Curator opcode table contains duplicates.'
}

New-Item -ItemType Directory -Force -Path $SourceDirectory | Out-Null
if (-not $UseExistingDownloads) {
    $opcodeList = ($expectedOpcodes | ForEach-Object { '{0:x2}' -f $_ }) -join ','
    $url = "https://raw.githubusercontent.com/SingleStepTests/65x02/$upstreamCommit/nes6502/v1/{$opcodeList}.json"
    & curl.exe --silent --show-error --fail --location --parallel --parallel-max 16 `
        --proto '=https' --tlsv1.2 --range 0-65535 --max-filesize 65536 `
        --output (Join-Path $SourceDirectory '#1.json') $url
    if ($LASTEXITCODE -ne 0) {
        throw "curl failed with exit code $LASTEXITCODE."
    }
}

function Read-CompleteVectors {
    param([Parameter(Mandatory = $true)][string] $Path)

    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw "Missing source vector chunk: $Path"
    }
    $length = (Get-Item -LiteralPath $Path).Length
    if ($length -ne 65536) {
        throw "Source vector chunk must be exactly 65,536 bytes; got $length bytes: $Path"
    }
    $vectors = [System.Collections.Generic.List[object]]::new()
    foreach ($line in [System.IO.File]::ReadLines($Path)) {
        $trimmed = $line.Trim()
        if (-not $trimmed.StartsWith('{')) {
            continue
        }
        if (-not $trimmed.EndsWith('},')) {
            continue
        }
        $openBraces = $trimmed.Length - $trimmed.Replace('{', '').Length
        $closeBraces = $trimmed.Length - $trimmed.Replace('}', '').Length
        if ($openBraces -ne $closeBraces) {
            continue
        }
        $json = $trimmed.TrimEnd(',')
        $vectors.Add(($json | ConvertFrom-Json))
    }
    if ($vectors.Count -eq 0) {
        throw "No complete vectors were found in source chunk: $Path"
    }
    return $vectors
}

function Assert-IntegerRange {
    param(
        [Parameter(Mandatory = $true)] $Value,
        [Parameter(Mandatory = $true)][int64] $Minimum,
        [Parameter(Mandatory = $true)][int64] $Maximum,
        [Parameter(Mandatory = $true)][string] $Field,
        [Parameter(Mandatory = $true)][string] $VectorName
    )

    $integerTypes = @(
        'System.Byte', 'System.SByte', 'System.Int16', 'System.UInt16',
        'System.Int32', 'System.UInt32', 'System.Int64', 'System.UInt64'
    )
    if ($null -eq $Value -or $Value.GetType().FullName -notin $integerTypes) {
        throw "Vector $VectorName field $Field is not an integer."
    }
    $number = [int64]$Value
    if ($number -lt $Minimum -or $number -gt $Maximum) {
        throw "Vector $VectorName field $Field is outside $Minimum..$Maximum."
    }
}

function Assert-Snapshot {
    param(
        [Parameter(Mandatory = $true)] $Snapshot,
        [Parameter(Mandatory = $true)][string] $Field,
        [Parameter(Mandatory = $true)][string] $VectorName
    )

    Assert-IntegerRange $Snapshot.pc 0 65535 "$Field.pc" $VectorName
    foreach ($register in 's', 'a', 'x', 'y', 'p') {
        Assert-IntegerRange $Snapshot.$register 0 255 "$Field.$register" $VectorName
    }
}

function Assert-Ram {
    param(
        [Parameter(Mandatory = $true)] $Entries,
        [Parameter(Mandatory = $true)][string] $Field,
        [Parameter(Mandatory = $true)][string] $VectorName
    )

    if ($Entries.Count -lt 1 -or $Entries.Count -gt 32) {
        throw "Vector $VectorName field $Field has an invalid entry count."
    }
    foreach ($entry in $Entries) {
        if ($entry.Count -ne 2) {
            throw "Vector $VectorName field $Field contains a non-pair entry."
        }
        Assert-IntegerRange $entry[0] 0 65535 "$Field.address" $VectorName
        Assert-IntegerRange $entry[1] 0 255 "$Field.value" $VectorName
    }
}

function Assert-Vector {
    param([Parameter(Mandatory = $true)] $Vector)

    $name = [string]$Vector.name
    if ($name -notmatch '^[0-9a-f]{2}(?: [0-9a-f]{2}){2}$') {
        throw "Vector name has an invalid form: $name"
    }
    Assert-Snapshot $Vector.initial 'initial' $name
    Assert-Snapshot $Vector.final 'final' $name
    Assert-Ram $Vector.initial.ram 'initial.ram' $name
    Assert-Ram $Vector.final.ram 'final.ram' $name
    if ($Vector.cycles.Count -lt 2 -or $Vector.cycles.Count -gt 7) {
        throw "Vector $name has an invalid cycle count."
    }
    foreach ($cycle in $Vector.cycles) {
        if ($cycle.Count -ne 3) {
            throw "Vector $name has a malformed cycle entry."
        }
        Assert-IntegerRange $cycle[0] 0 65535 'cycles.address' $name
        Assert-IntegerRange $cycle[1] 0 255 'cycles.value' $name
        if ($cycle[2] -notin 'read', 'write') {
            throw "Vector $name has an invalid cycle operation."
        }
    }
}

function Select-FirstCycleProfile {
    param(
        [Parameter(Mandatory = $true)] $Candidates,
        [Parameter(Mandatory = $true)][int] $Opcode,
        [Parameter(Mandatory = $true)][int] $Cycles
    )

    $selected = $Candidates | Where-Object { $_.cycles.Count -eq $Cycles } | Select-Object -First 1
    if ($null -eq $selected) {
        throw ('No {0}-cycle profile found for opcode ${1:x2}.' -f $Cycles, $Opcode)
    }
    return $selected
}

function Format-Snapshot {
    param([Parameter(Mandatory = $true)] $Snapshot)

    return ('Snapshot {{ pc: 0x{0:x4}, sp: 0x{1:x2}, a: 0x{2:x2}, x: 0x{3:x2}, y: 0x{4:x2}, status: 0x{5:x2} }}' -f `
        [int]$Snapshot.pc, [int]$Snapshot.s, [int]$Snapshot.a, [int]$Snapshot.x,
        [int]$Snapshot.y, [int]$Snapshot.p)
}

function Format-Ram {
    param([Parameter(Mandatory = $true)] $Entries)

    $pairs = foreach ($entry in $Entries) {
        '(0x{0:x4}, 0x{1:x2})' -f [int]$entry[0], [int]$entry[1]
    }
    return '&[' + ($pairs -join ', ') + ']'
}

function Format-BusCycles {
    param([Parameter(Mandatory = $true)] $Cycles)

    $entries = foreach ($cycle in $Cycles) {
        $kind = if ($cycle[2] -eq 'write') { 'CycleKind::Write' } else { 'CycleKind::Read' }
        '(0x{0:x4}, 0x{1:x2}, {2})' -f [int]$cycle[0], [int]$cycle[1], $kind
    }
    return '&[' + ($entries -join ', ') + ']'
}

$selectedVectors = [System.Collections.Generic.List[object]]::new()
foreach ($opcode in $expectedOpcodes) {
    $opcodeName = '{0:x2}' -f $opcode
    $candidates = Read-CompleteVectors -Path (Join-Path $SourceDirectory "$opcodeName.json")

    $cycleProfiles = if ($opcode -in $branchOpcodes) {
        @(2, 3, 4)
    }
    elseif ($opcode -in $pagePenaltyBaseFive) {
        @(5, 6)
    }
    elseif ($opcode -in $pagePenaltyBaseFour) {
        @(4, 5)
    }
    else {
        @([int]$candidates[0].cycles.Count)
    }

    foreach ($cycles in $cycleProfiles) {
        $vector = Select-FirstCycleProfile -Candidates $candidates -Opcode $opcode -Cycles $cycles
        Assert-Vector $vector
        $opcodeAtPc = $vector.initial.ram | Where-Object {
            [int]$_[0] -eq [int]$vector.initial.pc
        } | Select-Object -First 1
        if ($null -eq $opcodeAtPc -or [int]$opcodeAtPc[1] -ne $opcode) {
            throw "Vector $($vector.name) does not contain its opcode at the initial PC."
        }
        $selectedVectors.Add([pscustomobject]@{
            Opcode = $opcode
            Vector = $vector
        })
    }
}

if ($selectedVectors.Count -ne 190) {
    throw "Curator selected $($selectedVectors.Count) vectors; expected 190."
}
if (($selectedVectors.Vector.name | Sort-Object -Unique).Count -ne 190) {
    throw 'Curator selected duplicate vector names.'
}

$builder = [System.Text.StringBuilder]::new()
[void]$builder.AppendLine('// @generated by tools/curate-nes6502-vectors.ps1; do not edit by hand.')
[void]$builder.AppendLine('// Source: https://github.com/SingleStepTests/65x02')
[void]$builder.AppendLine("// Commit: $upstreamCommit")
[void]$builder.AppendLine('// License: MIT, Copyright (c) 2024 Thomas Harte et al; see NOTICE.')
[void]$builder.AppendLine()
[void]$builder.AppendLine('#[derive(Clone, Copy, Debug, Eq, PartialEq)]')
[void]$builder.AppendLine('pub(crate) enum CycleKind {')
[void]$builder.AppendLine('    Read,')
[void]$builder.AppendLine('    Write,')
[void]$builder.AppendLine('}')
[void]$builder.AppendLine()
[void]$builder.AppendLine('#[derive(Clone, Copy, Debug)]')
[void]$builder.AppendLine('pub(crate) struct Snapshot {')
[void]$builder.AppendLine('    pub(crate) pc: u16,')
[void]$builder.AppendLine('    pub(crate) sp: u8,')
[void]$builder.AppendLine('    pub(crate) a: u8,')
[void]$builder.AppendLine('    pub(crate) x: u8,')
[void]$builder.AppendLine('    pub(crate) y: u8,')
[void]$builder.AppendLine('    pub(crate) status: u8,')
[void]$builder.AppendLine('}')
[void]$builder.AppendLine()
[void]$builder.AppendLine('#[derive(Clone, Copy, Debug)]')
[void]$builder.AppendLine('pub(crate) struct Vector {')
[void]$builder.AppendLine("    pub(crate) name: &'static str,")
[void]$builder.AppendLine('    pub(crate) opcode: u8,')
[void]$builder.AppendLine('    pub(crate) initial: Snapshot,')
[void]$builder.AppendLine('    pub(crate) final_state: Snapshot,')
[void]$builder.AppendLine("    pub(crate) initial_ram: &'static [(u16, u8)],")
[void]$builder.AppendLine("    pub(crate) final_ram: &'static [(u16, u8)],")
[void]$builder.AppendLine('    pub(crate) cycles: u8,')
[void]$builder.AppendLine("    pub(crate) bus_cycles: &'static [(u16, u8, CycleKind)],")
[void]$builder.AppendLine('}')
[void]$builder.AppendLine()
[void]$builder.AppendLine(('pub(crate) const UPSTREAM_COMMIT: &str = "{0}";' -f $upstreamCommit))
[void]$builder.AppendLine('#[rustfmt::skip]')
[void]$builder.AppendLine('pub(crate) const VECTORS: &[Vector] = &[')
foreach ($selection in $selectedVectors) {
    $vector = $selection.Vector
    $escapedName = ([string]$vector.name).Replace('\', '\\').Replace('"', '\"')
    [void]$builder.AppendLine('    Vector {')
    [void]$builder.AppendLine(('        name: "{0}",' -f $escapedName))
    [void]$builder.AppendLine(('        opcode: 0x{0:x2},' -f [int]$selection.Opcode))
    [void]$builder.AppendLine(('        initial: ' + (Format-Snapshot $vector.initial) + ','))
    [void]$builder.AppendLine(('        final_state: ' + (Format-Snapshot $vector.final) + ','))
    [void]$builder.AppendLine(('        initial_ram: ' + (Format-Ram $vector.initial.ram) + ','))
    [void]$builder.AppendLine(('        final_ram: ' + (Format-Ram $vector.final.ram) + ','))
    [void]$builder.AppendLine(('        cycles: {0},' -f [int]$vector.cycles.Count))
    [void]$builder.AppendLine(('        bus_cycles: ' + (Format-BusCycles $vector.cycles) + ','))
    [void]$builder.AppendLine('    },')
}
[void]$builder.AppendLine('];')

$resolvedOutput = [System.IO.Path]::GetFullPath($OutputPath)
$outputDirectory = [System.IO.Path]::GetDirectoryName($resolvedOutput)
New-Item -ItemType Directory -Force -Path $outputDirectory | Out-Null
[System.IO.File]::WriteAllText($resolvedOutput, $builder.ToString(), [System.Text.UTF8Encoding]::new($false))
Write-Host "Wrote $($selectedVectors.Count) pinned vectors to $resolvedOutput"

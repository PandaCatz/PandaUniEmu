[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateNotNullOrEmpty()]
    [string] $Message,

    [string] $Repository = 'PandaCatz/PandaUniEmu',

    [string] $Branch = 'main',

    [switch] $WhatIf,

    [switch] $DeleteMissingManagedFiles
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$projectRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path

if ($null -eq ('PandaUniEmu.NativePath' -as [type])) {
    Add-Type -TypeDefinition @'
using System;
using System.ComponentModel;
using System.Runtime.InteropServices;
using System.Text;
using Microsoft.Win32.SafeHandles;

namespace PandaUniEmu {
    public static class NativePath {
        [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
        private static extern uint GetFinalPathNameByHandle(
            SafeFileHandle file,
            StringBuilder path,
            uint pathLength,
            uint flags);

        public static string FromHandle(SafeFileHandle file) {
            var path = new StringBuilder(32768);
            uint length = GetFinalPathNameByHandle(file, path, (uint)path.Capacity, 0);
            if (length == 0) {
                throw new Win32Exception(Marshal.GetLastWin32Error());
            }
            if (length >= path.Capacity) {
                throw new InvalidOperationException("Final publish path exceeded the Windows path limit.");
            }
            return path.ToString();
        }
    }
}
'@
}

$exactFiles = @(
    '.github/workflows/ci.yml',
    '.gitattributes',
    '.gitignore',
    'BUILD_PATH.md',
    'CLAUDE.md',
    'Cargo.lock',
    'Cargo.toml',
    'COPYING',
    'crates/core-nes/Cargo.toml',
    'crates/core-nes/src/lib.rs',
    'crates/core-nes/src/machine.rs',
    'crates/core-nes/src/nrom_bus.rs',
    'crates/core-nes/src/ppu_timing.rs',
    'crates/cpu-6502/Cargo.toml',
    'crates/cpu-6502/src/lib.rs',
    'crates/cpu-6502/src/singlestep_vectors.rs',
    'crates/format-ines/Cargo.toml',
    'crates/format-ines/src/lib.rs',
    'crates/format-nestest-log/Cargo.toml',
    'crates/format-nestest-log/src/lib.rs',
    'crates/retro-cli/Cargo.toml',
    'crates/retro-cli/src/lib.rs',
    'crates/retro-cli/src/main.rs',
    'crates/retro-cli/src/nestest_identity.rs',
    'crates/retro-cli/tests/cleanroom_process.rs',
    'crates/retro-core/Cargo.toml',
    'crates/retro-core/src/lib.rs',
    'crates/retro-testkit/Cargo.toml',
    'crates/retro-testkit/src/cleanroom_nrom.rs',
    'crates/retro-testkit/src/lib.rs',
    'crates/retro-testkit/src/nes_trace.rs',
    'docs/ARCHITECTURE.md',
    'docs/compatibility/NES_ACCEPTANCE.md',
    'docs/compatibility/CLEANROOM_NROM_PROVENANCE.md',
    'docs/compatibility/NESTEST_PROCEDURE.md',
    'docs/compatibility/NESTEST_PROVENANCE.md',
    'docs/compatibility/PERFECT6502_PROVENANCE.md',
    'docs/CPU_6502.md',
    'docs/NES_REFERENCE_INTAKE.md',
    'docs/PROJECT_STATE.md',
    'docs/PROPOSAL_REVIEW.md',
    'docs/TEST_PROVENANCE.md',
    'docs/UNIVERSAL_RETRO_EMULATOR_PROPOSAL.md',
    'fuzz/.gitignore',
    'fuzz/Cargo.lock',
    'fuzz/Cargo.toml',
    'fuzz/fuzz_targets/parse_ines.rs',
    'fuzz/fuzz_targets/parse_nestest_log.rs',
    'LICENSE',
    'NOTICE',
    'README.md',
    'ROADMAP.md',
    'SECURITY.md',
    'rust-toolchain.toml',
    'tools/check-cleanroom-nrom.py',
    'tools/curate-nes6502-vectors.ps1',
    'tools/generate-cleanroom-nrom.py',
    'tools/perfect6502-oracle.c',
    'tools/publish-github.ps1',
    'tools/run-fuzz.ps1',
    'tools/test_generate_cleanroom_nrom.py',
    'tools/verify-perfect6502.ps1'
)

function Test-PublishablePath {
    param([Parameter(Mandatory = $true)][string] $Path)

    if ($exactFiles -contains $Path) {
        return $true
    }
    return $false
}

function Assert-SafePublishFile {
    param([Parameter(Mandatory = $true)][string] $FullName)

    $rootFullName = [System.IO.Path]::GetFullPath($projectRoot).TrimEnd('\')
    $rootPrefix = $rootFullName + '\'
    $candidate = [System.IO.Path]::GetFullPath($FullName)
    if (-not $candidate.StartsWith($rootPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Publish candidate resolves outside the project root: $candidate"
    }

    $item = Get-Item -LiteralPath $candidate -Force
    if (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
        throw "Publish candidate is a reparse point: $candidate"
    }
    $directory = $item.Directory
    while ($null -ne $directory) {
        if (($directory.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
            throw "Publish candidate has a reparse-point ancestor: $($directory.FullName)"
        }
        if ($directory.FullName.Equals($rootFullName, [System.StringComparison]::OrdinalIgnoreCase)) {
            return $candidate
        }
        $directory = $directory.Parent
    }
    throw "Publish candidate has no project-root ancestor: $candidate"
}

function Read-SafePublishText {
    param([Parameter(Mandatory = $true)][string] $FullName)

    $safeFullName = Assert-SafePublishFile -FullName $FullName
    $stream = [System.IO.FileStream]::new(
        $safeFullName,
        [System.IO.FileMode]::Open,
        [System.IO.FileAccess]::Read,
        [System.IO.FileShare]::None
    )
    try {
        $finalPath = [PandaUniEmu.NativePath]::FromHandle($stream.SafeFileHandle)
        if ($finalPath.StartsWith('\\?\UNC\', [System.StringComparison]::OrdinalIgnoreCase)) {
            $finalPath = '\\' + $finalPath.Substring(8)
        }
        elseif ($finalPath.StartsWith('\\?\', [System.StringComparison]::OrdinalIgnoreCase)) {
            $finalPath = $finalPath.Substring(4)
        }

        $rootFullName = [System.IO.Path]::GetFullPath($projectRoot).TrimEnd('\')
        $rootPrefix = $rootFullName + '\'
        $finalFullName = [System.IO.Path]::GetFullPath($finalPath)
        if (-not $finalFullName.StartsWith($rootPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
            throw "Opened publish file resolves outside the project root: $finalFullName"
        }

        $encoding = [System.Text.UTF8Encoding]::new($false, $true)
        $reader = [System.IO.StreamReader]::new($stream, $encoding, $true, 1024, $true)
        try {
            return $reader.ReadToEnd()
        }
        finally {
            $reader.Dispose()
        }
    }
    finally {
        $stream.Dispose()
    }
}

$files = Get-ChildItem -LiteralPath $projectRoot -Recurse -File | ForEach-Object {
    $relative = $_.FullName.Substring($projectRoot.Length + 1).Replace('\', '/')
    if (Test-PublishablePath -Path $relative) {
        $safeFullName = Assert-SafePublishFile -FullName $_.FullName
        [pscustomobject]@{ Path = $relative; FullName = $safeFullName }
    }
} | Sort-Object Path

if ($files.Count -eq 0) {
    throw 'No allowlisted project files were found.'
}

Write-Host "Snapshot contains $($files.Count) allowlisted files:"
$files.Path | ForEach-Object { Write-Host "  $_" }

$gh = Get-Command gh -ErrorAction Stop

function Invoke-GhApi {
    param(
        [Parameter(Mandatory = $true)][string] $Endpoint,
        [Parameter(Mandatory = $true)][ValidateSet('GET', 'POST', 'PUT', 'PATCH')][string] $Method,
        [object] $Body,
        [switch] $AllowFailure
    )

    $startInfo = New-Object System.Diagnostics.ProcessStartInfo
    $startInfo.FileName = $gh.Source
    $startInfo.Arguments = "api `"$Endpoint`" --method $Method" + $(
        if ($null -ne $Body) { ' --input -' } else { '' }
    )
    $startInfo.UseShellExecute = $false
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true
    $startInfo.RedirectStandardInput = $null -ne $Body
    $startInfo.CreateNoWindow = $true

    $process = New-Object System.Diagnostics.Process
    $process.StartInfo = $startInfo
    if (-not $process.Start()) {
        throw "Failed to start GitHub CLI for $Endpoint."
    }
    if ($null -ne $Body) {
        $json = ConvertTo-Json -InputObject $Body -Depth 20 -Compress
        $process.StandardInput.Write($json)
        $process.StandardInput.Close()
    }
    $stdout = $process.StandardOutput.ReadToEnd()
    $stderr = $process.StandardError.ReadToEnd()
    $process.WaitForExit()
    if ($process.ExitCode -ne 0) {
        if ($AllowFailure) {
            return $null
        }
        throw "GitHub API $Method $Endpoint failed: $stderr"
    }
    if ([string]::IsNullOrWhiteSpace($stdout)) {
        return $null
    }
    return ConvertFrom-Json -InputObject $stdout
}

$repositoryInfo = Invoke-GhApi -Endpoint "repos/$Repository" -Method GET
if ($repositoryInfo.full_name -ne $Repository) {
    throw "GitHub resolved an unexpected repository: $($repositoryInfo.full_name)"
}

$reference = Invoke-GhApi -Endpoint "repos/$Repository/git/ref/heads/$Branch" -Method GET -AllowFailure
$createBranch = $null -eq $reference
$parentSha = $null
$baseTreeSha = $null
$remoteEntries = @()

if ($createBranch) {
    $defaultBranch = [string] $repositoryInfo.default_branch
    $reference = Invoke-GhApi `
        -Endpoint "repos/$Repository/git/ref/heads/$defaultBranch" `
        -Method GET `
        -AllowFailure
    if ($null -eq $reference) {
        Write-Host "Repository is empty; the complete snapshot will atomically create branch '$Branch'."
    }
    else {
        Write-Host "Branch '$Branch' is missing; it will be created from '$defaultBranch' plus this snapshot."
    }
}

if ($null -ne $reference) {
    $parentSha = [string] $reference.object.sha
    $parentCommit = Invoke-GhApi -Endpoint "repos/$Repository/git/commits/$parentSha" -Method GET
    $baseTreeSha = [string] $parentCommit.tree.sha
    $remoteTree = Invoke-GhApi -Endpoint "repos/$Repository/git/trees/$baseTreeSha`?recursive=1" -Method GET
    if ($remoteTree.truncated) {
        throw 'The remote tree listing was truncated; refusing to publish an incomplete diff.'
    }
    $remoteEntries = @($remoteTree.tree)
}

$localPaths = @($files.Path)
$missingRemotePaths = @(
    $remoteEntries |
        Where-Object {
            $_.type -eq 'blob' -and
            (Test-PublishablePath -Path ([string] $_.path)) -and
            $localPaths -notcontains ([string] $_.path)
        } |
        ForEach-Object { [string] $_.path } |
        Sort-Object -Unique
)
$deletedPaths = @()
if ($DeleteMissingManagedFiles) {
    $deletedPaths = @($missingRemotePaths)
}

if ($deletedPaths.Count -gt 0) {
    Write-Host 'Explicitly scheduled managed-file deletions:'
    $deletedPaths | ForEach-Object { Write-Host "  $_" }
}
elseif ($missingRemotePaths.Count -gt 0) {
    Write-Host 'Managed remote files absent locally and preserved by default:'
    $missingRemotePaths | ForEach-Object { Write-Host "  $_" }
    Write-Host 'Use -DeleteMissingManagedFiles only after reviewing this list.'
}
else {
    Write-Host 'No managed remote files are absent locally.'
}

if ($WhatIf) {
    foreach ($file in $files) {
        [void](Read-SafePublishText -FullName $file.FullName)
    }
    Write-Host 'WhatIf: every allowlisted file passed handle-based in-root and UTF-8 validation.'
    Write-Host 'WhatIf: the parent tree and all non-allowlisted remote files will be preserved.'
    Write-Host 'WhatIf: no GitHub write calls were made.'
    exit 0
}

$treeEntries = @(
    foreach ($file in $files) {
    $content = Read-SafePublishText -FullName $file.FullName
    $contentBytes = [System.Text.Encoding]::UTF8.GetBytes($content)
    $blob = Invoke-GhApi -Endpoint "repos/$Repository/git/blobs" -Method POST -Body @{
        content = [System.Convert]::ToBase64String($contentBytes)
        encoding = 'base64'
    }
    @{
        path = $file.Path
        mode = '100644'
        type = 'blob'
        sha = [string] $blob.sha
    }
    }
    foreach ($path in $deletedPaths) {
        @{
            path = $path
            mode = '100644'
            type = 'blob'
            sha = $null
        }
    }
)

$treeBody = @{
    tree = @($treeEntries)
}
if ($null -ne $baseTreeSha) {
    $treeBody.base_tree = $baseTreeSha
}
$tree = Invoke-GhApi -Endpoint "repos/$Repository/git/trees" -Method POST -Body $treeBody

$commitBody = @{
    message = $Message
    tree = [string] $tree.sha
}
if ($null -ne $parentSha) {
    $commitBody.parents = @($parentSha)
}
$commit = Invoke-GhApi -Endpoint "repos/$Repository/git/commits" -Method POST -Body $commitBody

if ($createBranch) {
    $null = Invoke-GhApi -Endpoint "repos/$Repository/git/refs" -Method POST -Body @{
        ref = "refs/heads/$Branch"
        sha = [string] $commit.sha
    }
}
else {
    $null = Invoke-GhApi -Endpoint "repos/$Repository/git/refs/heads/$Branch" -Method PATCH -Body @{
        sha = [string] $commit.sha
        force = $false
    }
}

Write-Host "Published $($commit.sha) to $Repository@$Branch"

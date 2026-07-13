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
$exactFiles = @(
    '.gitignore',
    'BUILD_PATH.md',
    'CLAUDE.md',
    'Cargo.lock',
    'Cargo.toml',
    'COPYING',
    'LICENSE',
    'NOTICE',
    'README.md',
    'ROADMAP.md',
    'SECURITY.md',
    'rust-toolchain.toml'
)

function Test-PublishablePath {
    param([Parameter(Mandatory = $true)][string] $Path)

    if ($exactFiles -contains $Path) {
        return $true
    }
    return $Path -match '^\.github/workflows/[^/]+\.ya?ml$' -or
        $Path -match '^crates/[^/]+/Cargo\.toml$' -or
        $Path -match '^crates/[^/]+/src/.+\.rs$' -or
        $Path -match '^docs/(?:.+/)?[^/]+\.md$' -or
        $Path -match '^fuzz/(?:Cargo\.(?:toml|lock)|\.gitignore|fuzz_targets/.+\.rs)$' -or
        $Path -match '^tools/[^/]+\.ps1$'
}

$files = Get-ChildItem -LiteralPath $projectRoot -Recurse -File | ForEach-Object {
    $relative = $_.FullName.Substring($projectRoot.Length + 1).Replace('\', '/')
    if (Test-PublishablePath -Path $relative) {
        [pscustomobject]@{ Path = $relative; FullName = $_.FullName }
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
    Write-Host 'WhatIf: the parent tree and all non-allowlisted remote files will be preserved.'
    Write-Host 'WhatIf: no GitHub write calls were made.'
    exit 0
}

$treeEntries = @(
    foreach ($file in $files) {
    $content = [System.IO.File]::ReadAllText($file.FullName)
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

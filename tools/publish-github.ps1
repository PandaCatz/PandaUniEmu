[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateNotNullOrEmpty()]
    [string] $Message,

    [string] $Repository = 'PandaCatz/PandaUniEmu',

    [string] $Branch = 'main',

    [switch] $WhatIf
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
    'README.md',
    'ROADMAP.md',
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

if ($WhatIf) {
    Write-Host 'WhatIf: no GitHub API calls were made.'
    exit 0
}

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
if ($null -eq $reference) {
    $readme = $files | Where-Object Path -eq 'README.md' | Select-Object -First 1
    if ($null -eq $readme) {
        throw 'README.md is required to initialize an empty repository safely.'
    }
    $readmeBytes = [System.Text.Encoding]::UTF8.GetBytes(
        [System.IO.File]::ReadAllText($readme.FullName)
    )
    $null = Invoke-GhApi -Endpoint "repos/$Repository/contents/README.md" -Method PUT -Body @{
        message = 'chore: initialize repository'
        content = [System.Convert]::ToBase64String($readmeBytes)
    }
}
$reference = Invoke-GhApi -Endpoint "repos/$Repository/git/ref/heads/$Branch" -Method GET
$parentSha = [string] $reference.object.sha

$treeEntries = foreach ($file in $files) {
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

$tree = Invoke-GhApi -Endpoint "repos/$Repository/git/trees" -Method POST -Body @{
    tree = @($treeEntries)
}

$commitBody = @{
    message = $Message
    tree = [string] $tree.sha
}
if ($null -ne $parentSha) {
    $commitBody.parents = @($parentSha)
}
$commit = Invoke-GhApi -Endpoint "repos/$Repository/git/commits" -Method POST -Body $commitBody

if ($null -eq $parentSha) {
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

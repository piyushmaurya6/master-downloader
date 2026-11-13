<#
.SYNOPSIS
    Automated 1-command uploader to publish Master Downloader Suite (Source Code + Docs + Commercial Releases) to GitHub.

.EXAMPLE
    .\PUSH_TO_GITHUB.ps1 -RepoUrl "https://github.com/YOUR_USERNAME/master-downloader.git"

.EXAMPLE
    .\PUSH_TO_GITHUB.ps1 -RepoUrl "https://github.com/YOUR_USERNAME/master-downloader.git" -UseGitLfs
#>
param(
    [Parameter(Mandatory = $false)]
    [string]$RepoUrl = "",

    [Parameter(Mandatory = $false)]
    [switch]$UseGitLfs,

    [Parameter(Mandatory = $false)]
    [string]$ReleaseTag = "v1.0.0"
)

$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host "  Master Downloader Suite v1.0.0 — GitHub Publisher" -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

if (-not (Get-Command git -ErrorAction SilentlyContinue)) {
    Write-Error "Git is not installed or not in PATH. Install Git from https://git-scm.com/download/win first."
}

if ([string]::IsNullOrWhiteSpace($RepoUrl)) {
    $RepoUrl = Read-Host "Enter your empty GitHub Repository URL (e.g. https://github.com/username/master-downloader.git)"
}
if ([string]::IsNullOrWhiteSpace($RepoUrl)) {
    Write-Error "A GitHub Repository URL is required."
}

if (-not (Test-Path ".git")) {
    git init -b main
}

$existingOrigin = git remote | Where-Object { $_ -eq "origin" }
if ($existingOrigin) {
    git remote set-url origin $RepoUrl
} else {
    git remote add origin $RepoUrl
}

if ($UseGitLfs) {
    Write-Host "`n[1/3] Enabling Git LFS for *.exe and *.dll binaries..." -ForegroundColor Yellow
    git lfs install
    git add .
} else {
    Write-Host "`n[1/3] Staging source code, instructions, scripts, and CI/CD workflows (skipping >95 MB binaries from git blob tree so standard GitHub push succeeds without LFS)..." -ForegroundColor Yellow
    git add .
    # Unstage files > 95 MB and releases/*.exe so standard GitHub push never hits the 100 MB limit;
    # those installers are published directly to GitHub Releases in Step 3!
    git reset -- releases/*.exe 2>$null | Out-Null
    Get-ChildItem -Path . -Recurse -File | Where-Object {
        $_.FullName -notmatch '\\\.git\\' -and $_.Length -gt 95MB
    } | ForEach-Object {
        $rel = $_.FullName.Substring($PSScriptRoot.Length + 1).Replace('\', '/')
        Write-Host "  -> Excluding >95MB binary from Git commit (handled via setup-sidecars.ps1 / Releases): $rel" -ForegroundColor DarkGray
        git reset -- "$rel" 2>$null | Out-Null
    }
}

$status = git status --porcelain
if ($status) {
    git commit -m "feat: Master Downloader Suite v1.0.0 (Unified YouTube/Instagram/Facebook + Standalone Apps & Docs)"
}

Write-Host "`n[2/3] Pushing repository to $RepoUrl ..." -ForegroundColor Yellow
git push -u origin main

Write-Host "`n[3/3] Publishing Commercial Release Installers ($ReleaseTag)..." -ForegroundColor Yellow
$releaseFiles = Get-ChildItem -Path ".\releases" -File | Select-Object -ExpandProperty FullName

if (Get-Command gh -ErrorAction SilentlyContinue) {
    Write-Host "GitHub CLI (gh) detected! Creating GitHub Release $ReleaseTag and uploading installers..." -ForegroundColor Green
    gh release create $ReleaseTag @releaseFiles `
        --title "Master Downloader Suite $ReleaseTag — Commercial Windows x64 Release" `
        --notes-file ".\releases\RELEASE_NOTES_v1.0.0.md"
    Write-Host "`nDONE! Your code AND commercial .exe installers are live on GitHub!" -ForegroundColor Green
} else {
    $webUrl = $RepoUrl -replace '\.git$', ''
    $newReleaseUrl = "$webUrl/releases/new?tag=$ReleaseTag&title=Master+Downloader+Suite+$ReleaseTag"
    Write-Host "`nRepository code pushed successfully!" -ForegroundColor Green
    Write-Host "GitHub CLI ('gh') is not installed, so opening your GitHub New Release page and local 'releases' folder now:" -ForegroundColor Cyan
    Write-Host "  1. Drag & drop the .exe installers + SHA256SUMS.txt from the opened 'releases' Explorer window into your browser."
    Write-Host "  2. Paste 'releases\RELEASE_NOTES_v1.0.0.md' as the description and click 'Publish release'!"
    Start-Process $newReleaseUrl
    Start-Process explorer.exe ".\releases"
}

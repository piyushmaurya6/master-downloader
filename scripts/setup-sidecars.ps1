<#
.SYNOPSIS
    Ensures yt-dlp.exe, gallery-dl.exe, deno.exe, ffmpeg.exe, ffprobe.exe, and FFmpeg DLLs
    are present in master-downloader/src-tauri/resources/bin and synced to standalone-apps/*.
#>
$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

$repoRoot = Split-Path -Parent $PSScriptRoot
$primaryBin = Join-Path $repoRoot "master-downloader\src-tauri\resources\bin"
New-Item -ItemType Directory -Force -Path $primaryBin | Out-Null

function Invoke-FileDownload {
    param(
        [Parameter(Mandatory = $true)][string]$OutFile,
        [Parameter(Mandatory = $true)][string[]]$Urls
    )
    $lastErr = $null
    foreach ($u in $Urls) {
        try {
            Write-Host "  -> Fetching $u" -ForegroundColor DarkCyan
            & curl.exe -fL --retry 3 --retry-delay 2 -A "MasterDownloader-CI/1.0" -o $OutFile $u
            if ($LASTEXITCODE -eq 0 -and (Test-Path $OutFile) -and ((Get-Item $OutFile).Length -gt 10240)) {
                return
            }
            throw "curl.exe exited with code $LASTEXITCODE or produced an empty file for $u"
        } catch {
            $lastErr = $_
            Remove-Item $OutFile -Force -ErrorAction SilentlyContinue
            Write-Warning "Download attempt failed from ${u}: $lastErr"
        }
    }
    throw "All download mirrors failed for $(Split-Path -Leaf $OutFile). Last error: $lastErr"
}

function Resolve-GalleryDlUrls {
    $urls = New-Object System.Collections.Generic.List[string]
    try {
        # mikf/gallery-dl migrated official binary release assets from GitHub to Codeberg (gh#9374)
        $cbRelease = Invoke-RestMethod -Uri "https://codeberg.org/api/v1/repos/mikf/gallery-dl/releases/latest" -Headers @{ "User-Agent" = "MasterDownloader-CI/1.0" } -TimeoutSec 20
        $asset = $cbRelease.assets | Where-Object { $_.name -eq "gallery-dl.exe" } | Select-Object -First 1
        if ($asset -and $asset.browser_download_url) {
            $urls.Add([string]$asset.browser_download_url)
        }
    } catch {
        Write-Warning "Could not query Codeberg API for latest gallery-dl release: $_"
    }
    # Official gdl-org standalone x64 Windows build on GitHub as automatic fallback
    $urls.Add("https://github.com/gdl-org/builds/releases/latest/download/gallery-dl_windows.exe")
    return $urls.ToArray()
}

$ytdlpPath = Join-Path $primaryBin "yt-dlp.exe"
if (-not (Test-Path $ytdlpPath)) {
    Write-Host "Downloading yt-dlp.exe ..." -ForegroundColor Cyan
    Invoke-FileDownload -OutFile $ytdlpPath -Urls @(
        "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe"
    )
}

$galleryDlPath = Join-Path $primaryBin "gallery-dl.exe"
if (-not (Test-Path $galleryDlPath)) {
    Write-Host "Downloading gallery-dl.exe ..." -ForegroundColor Cyan
    $gdlUrls = Resolve-GalleryDlUrls
    Invoke-FileDownload -OutFile $galleryDlPath -Urls $gdlUrls
}

$denoPath = Join-Path $primaryBin "deno.exe"
if (-not (Test-Path $denoPath)) {
    $denoZip = Join-Path $env:TEMP "deno-x86_64-pc-windows-msvc.zip"
    Write-Host "Downloading deno.exe ..." -ForegroundColor Cyan
    Invoke-FileDownload -OutFile $denoZip -Urls @(
        "https://github.com/denoland/deno/releases/latest/download/deno-x86_64-pc-windows-msvc.zip"
    )
    Expand-Archive -Path $denoZip -DestinationPath $primaryBin -Force
    Remove-Item $denoZip -Force -ErrorAction SilentlyContinue
}

$ffmpegPath = Join-Path $primaryBin "ffmpeg.exe"
$ffprobePath = Join-Path $primaryBin "ffprobe.exe"
if ((-not (Test-Path $ffmpegPath)) -or (-not (Test-Path $ffprobePath))) {
    $ffZip = Join-Path $env:TEMP "ffmpeg-master-latest-win64-gpl-shared.zip"
    Write-Host "Downloading FFmpeg & FFprobe (win64 GPL shared) ..." -ForegroundColor Cyan
    Invoke-FileDownload -OutFile $ffZip -Urls @(
        "https://github.com/FFmpeg-Builds/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-gpl-shared.zip",
        "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-gpl-shared.zip",
        "https://github.com/yt-dlp/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-gpl-shared.zip"
    )
    $ffExtract = Join-Path $env:TEMP "ffmpeg-extract-md"
    Remove-Item $ffExtract -Recurse -Force -ErrorAction SilentlyContinue
    Expand-Archive -Path $ffZip -DestinationPath $ffExtract -Force
    Get-ChildItem -Path $ffExtract -Recurse -File | Where-Object {
        $_.Name -in @("ffmpeg.exe", "ffprobe.exe") -or $_.Extension -eq ".dll"
    } | ForEach-Object {
        Copy-Item $_.FullName -Destination (Join-Path $primaryBin $_.Name) -Force
    }
    Remove-Item $ffZip -Force -ErrorAction SilentlyContinue
    Remove-Item $ffExtract -Recurse -Force -ErrorAction SilentlyContinue
}

Write-Host "`nVerifying provisioned sidecar binaries..." -ForegroundColor Cyan
foreach ($check in @(
    @{ Name = "yt-dlp.exe"; Args = @("--version") },
    @{ Name = "gallery-dl.exe"; Args = @("--version") },
    @{ Name = "deno.exe"; Args = @("--version") },
    @{ Name = "ffmpeg.exe"; Args = @("-version") },
    @{ Name = "ffprobe.exe"; Args = @("-version") }
)) {
    $exe = Join-Path $primaryBin $check.Name
    if (-not (Test-Path $exe)) {
        throw "Required sidecar binary missing after provisioning: $exe"
    }
    $prevEap = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    $rawOut = & $exe @($check.Args) 2>&1
    $exitCode = $LASTEXITCODE
    $ErrorActionPreference = $prevEap
    if ($exitCode -ne 0) {
        throw "Sidecar binary $($check.Name) failed verification with exit code $exitCode"
    }
    $verOut = ($rawOut | Select-Object -First 1)
    Write-Host "  [OK] $($check.Name): $verOut" -ForegroundColor Green
}

# Sync sidecars into standalone-apps/{flowdl,instaflow,flowfb}
foreach ($app in @("flowdl", "instaflow", "flowfb")) {
    $appRoot = Join-Path $repoRoot "standalone-apps\$app"
    if (Test-Path $appRoot) {
        $targetBin = Join-Path $appRoot "src-tauri\resources\bin"
        New-Item -ItemType Directory -Force -Path $targetBin | Out-Null
        Get-ChildItem -Path $primaryBin -File | ForEach-Object {
            if ($app -eq "flowdl" -and $_.Name -eq "gallery-dl.exe") { return }
            $dst = Join-Path $targetBin $_.Name
            if (-not (Test-Path $dst)) {
                try {
                    New-Item -ItemType HardLink -Path $dst -Target $_.FullName -ErrorAction Stop | Out-Null
                } catch {
                    Copy-Item $_.FullName -Destination $dst -Force
                }
            }
        }
        Write-Host "Synced sidecar binaries -> standalone-apps\$app\src-tauri\resources\bin" -ForegroundColor Green
    }
}

Write-Host "`nAll sidecar engines verified and ready!" -ForegroundColor Green

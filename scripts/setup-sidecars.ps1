<#
.SYNOPSIS
    Ensures yt-dlp.exe, gallery-dl.exe, deno.exe, ffmpeg.exe, ffprobe.exe, and FFmpeg DLLs
    are present in master-downloader/src-tauri/resources/bin and synced to standalone-apps/*.
#>
$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot
$primaryBin = Join-Path $repoRoot "master-downloader\src-tauri\resources\bin"
New-Item -ItemType Directory -Force -Path $primaryBin | Out-Null

function Ensure-Download($FileName, $Url) {
    $dest = Join-Path $primaryBin $FileName
    if (-not (Test-Path $dest)) {
        Write-Host "Downloading $FileName from $Url ..." -ForegroundColor Cyan
        Invoke-WebRequest -Uri $Url -OutFile $dest -UseBasicParsing
    }
}

Ensure-Download "yt-dlp.exe" "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe"
Ensure-Download "gallery-dl.exe" "https://github.com/mikf/gallery-dl/releases/latest/download/gallery-dl.exe"

$denoPath = Join-Path $primaryBin "deno.exe"
if (-not (Test-Path $denoPath)) {
    $denoZip = Join-Path $env:TEMP "deno-x86_64-pc-windows-msvc.zip"
    Write-Host "Downloading deno.exe ..." -ForegroundColor Cyan
    Invoke-WebRequest -Uri "https://github.com/denoland/deno/releases/latest/download/deno-x86_64-pc-windows-msvc.zip" -OutFile $denoZip -UseBasicParsing
    Expand-Archive -Path $denoZip -DestinationPath $primaryBin -Force
    Remove-Item $denoZip -Force -ErrorAction SilentlyContinue
}

$ffmpegPath = Join-Path $primaryBin "ffmpeg.exe"
if (-not (Test-Path $ffmpegPath)) {
    $ffZip = Join-Path $env:TEMP "ffmpeg-shared-win64.zip"
    Write-Host "Downloading FFmpeg & FFprobe (win64 GPL shared) ..." -ForegroundColor Cyan
    Invoke-WebRequest -Uri "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-gpl-shared.zip" -OutFile $ffZip -UseBasicParsing
    $ffExtract = Join-Path $env:TEMP "ffmpeg-extract-md"
    Remove-Item $ffExtract -Recurse -Force -ErrorAction SilentlyContinue
    Expand-Archive -Path $ffZip -DestinationPath $ffExtract -Force
    Get-ChildItem -Path $ffExtract -Recurse -Include "ffmpeg.exe","ffprobe.exe","*.dll" | ForEach-Object {
        Copy-Item $_.FullName -Destination (Join-Path $primaryBin $_.Name) -Force
    }
    Remove-Item $ffZip -Force -ErrorAction SilentlyContinue
    Remove-Item $ffExtract -Recurse -Force -ErrorAction SilentlyContinue
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

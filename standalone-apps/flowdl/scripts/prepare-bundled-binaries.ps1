# Prepares and verifies the private bundled binaries (yt-dlp, FFmpeg, ffprobe, and Deno runtime)
# in src-tauri/resources/bin/ before building the self-contained FlowDL Windows installer.

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot
$binDir = Join-Path $repoRoot "src-tauri\resources\bin"
$licDir = Join-Path $repoRoot "src-tauri\resources\licenses"

New-Item -ItemType Directory -Force -Path $binDir | Out-Null
New-Item -ItemType Directory -Force -Path $licDir | Out-Null

# 1. yt-dlp.exe
$ytdlpDest = Join-Path $binDir "yt-dlp.exe"
if (!(Test-Path $ytdlpDest)) {
    Write-Host "Downloading official yt-dlp.exe..."
    curl.exe -L -s -o $ytdlpDest "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe"
}

# 2. FFmpeg & ffprobe (shared GPL win64 build)
$ffmpegDest = Join-Path $binDir "ffmpeg.exe"
if (!(Test-Path $ffmpegDest)) {
    Write-Host "Downloading official FFmpeg shared win64 package..."
    $tmpZip = Join-Path $env:TEMP "ffmpeg-shared-flowdl.zip"
    $tmpExtract = Join-Path $env:TEMP "ffmpeg-shared-extract"
    curl.exe -L -s -o $tmpZip "https://github.com/yt-dlp/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-gpl-shared.zip"
    Remove-Item -Recurse -Force $tmpExtract -ErrorAction SilentlyContinue
    Expand-Archive -Path $tmpZip -DestinationPath $tmpExtract -Force
    Get-ChildItem -Recurse $tmpExtract -Include "ffmpeg.exe","ffprobe.exe","*.dll" | ForEach-Object {
        Copy-Item $_.FullName (Join-Path $binDir $_.Name) -Force
    }
    Get-ChildItem -Recurse $tmpExtract -Filter "LICENSE.txt" | Select-Object -First 1 | ForEach-Object {
        Copy-Item $_.FullName (Join-Path $licDir "LICENSE-FFmpeg-GPLv3.txt") -Force
    }
    Remove-Item -Recurse -Force $tmpExtract -ErrorAction SilentlyContinue
    Remove-Item -Force $tmpZip -ErrorAction SilentlyContinue
}

# 3. Deno JS runtime for YouTube challenge solving
$denoDest = Join-Path $binDir "deno.exe"
if (!(Test-Path $denoDest)) {
    Write-Host "Downloading official Deno x64 Windows runtime..."
    $denoZip = Join-Path $env:TEMP "deno-win64-flowdl.zip"
    curl.exe -L -s -o $denoZip "https://github.com/denoland/deno/releases/latest/download/deno-x86_64-pc-windows-msvc.zip"
    Expand-Archive -Path $denoZip -DestinationPath $binDir -Force
    Remove-Item -Force $denoZip -ErrorAction SilentlyContinue
}

Write-Host "Verifying bundled binaries..."
& $ytdlpDest --version
& (Join-Path $binDir "ffmpeg.exe") -version | Select-Object -First 1
& (Join-Path $binDir "ffprobe.exe") -version | Select-Object -First 1
& $denoDest --version | Select-Object -First 1
Write-Host "All bundled binaries verified in $binDir"

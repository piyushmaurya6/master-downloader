# FlowDL Troubleshooting Guide

## 1. Missing or Outdated Download Engines (`yt-dlp` / `FFmpeg`)
- **Symptom**: URL analysis fails with `yt-dlp is not installed`, or high-resolution (1080p/4K) downloads fail to merge video and audio streams.
- **Resolution**:
  1. Open **Settings → Download Engine** (or click the **Engines Ready / Setup Required** badge at the bottom of the left sidebar).
  2. Click **Repair Engines** or **Reinstall / Update**.
  3. FlowDL will download verified copies of `yt-dlp.exe`, `ffmpeg.exe`, and `ffprobe.exe` into `%LOCALAPPDATA%\FlowDL\bin` and verify their SHA-256 checksums.

## 2. "This video isn't available in 4K" or Disabled Quality Options
- **Cause**: FlowDL queries the actual stream manifest of each URL via `yt-dlp --dump-single-json`. If a creator uploaded a video at a maximum resolution of 1080p or 720p, higher options (such as 1440p and 4K) are disabled automatically so FlowDL never pretends an unavailable stream exists.
- **Resolution**: Select **Best available** or the highest enabled resolution in the Video Quality dropdown.

## 3. "This video requires authentication"
- **Cause**: Age-restricted, private, members-only, or DRM-protected videos cannot be accessed without authorization. FlowDL never attempts to bypass DRM, paywalls, or authentication controls.
- **Resolution**: Only publicly accessible videos or links you have permission to download are supported.

## 4. "Not enough storage space in your selected download folder"
- **Cause**: High-resolution 4K videos and lossless audio streams require free disk space for both the temporary stream files and the merged output file.
- **Resolution**: Open **Settings → General → Default Download Folder** (or click **Save to** on the video preview card) and select a drive with sufficient free space (e.g., `D:\Downloads`).

## 5. Inspecting Diagnostic Logs
- Open **Settings → Download Engine → View Diagnostic Logs** to inspect sanitized execution logs.
- Logs are also saved locally at `%LOCALAPPDATA%\FlowDL\logs\flowdl.log` with sensitive URL tokens automatically redacted.

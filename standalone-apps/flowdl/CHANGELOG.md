# Changelog

All notable changes to **FlowDL** will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Planned
- Batch URL file import (`.txt` list)
- Custom user format preset profiles

---

## [1.0.0] — 2026-11-02

### Added
- **Self-Contained Windows Desktop Application**:
  - Built native Windows 10/11 (x64) desktop app with **Tauri 2**, **Rust**, **React 18**, **TypeScript**, and **Tailwind CSS**.
  - Bundled verified private copies of `yt-dlp.exe` (`2026.10.26`), `ffmpeg.exe` & `ffprobe.exe` (`N-128280-ga88838af48-20261101` GPLv3 shared build), and `deno.exe` (`2.9.8` for YouTube `yt-dlp-ejs` JS challenge solving) inside the LZMA-compressed Windows NSIS installer (`FlowDL_1.0.0_x64-setup.exe`).
- **Future-Proof Download Engine Architecture**:
  - Introduced the `DownloadEngine` trait (`YtDlpEngine`) and `EngineManager` service for clean separation between UI, orchestration, and command-line execution.
  - Real-time URL metadata analysis (`--dump-single-json --flat-playlist`) extracting genuine available video resolutions, codecs (`H.264`, `H.265`, `VP9`, `AV1`), frame rates, subtitles, and playlist previews.
- **Video & Audio Download Controls**:
  - Video quality presets (`Best`, `4K 2160p`, `1440p`, `1080p`, `720p`, `480p`, `360p`), exact vs. maximum resolution mode, containers (`MP4`, `MKV`, `WebM`, `Auto`), and subtitle/chapter/thumbnail embedding.
  - Audio extraction (`MP3`, `M4A`, `Original Best`, `FLAC`, `WAV`, `OPUS`), target bitrates (`320 / 256 / 192 / 128 kbps`), and optional EBU R128 loudness normalization (`loudnorm`).
- **Dual Independent Update Systems**:
  - **System A (FlowDL App Updater)**: Checks GitHub Releases for newer FlowDL versions, displays release notes, and coordinates updates with active downloads.
  - **System B (Engine Updater & Self-Repair)**: Independently updates `yt-dlp` and `FFmpeg` in `%LOCALAPPDATA%\FlowDL\bin` with SHA-256 verification and automatic `.bak` rollback on failure.
- **Versioned SQLite Persistence & Queue Recovery**:
  - Transactional SQLite schema migration runner (`V1__initial_schema.sql`, `V2__indices_and_metadata.sql`) storing download history and queue state in `%LOCALAPPDATA%\FlowDL\flowdl.db`.
  - Interrupted downloads are safely restored to `queued` state on application restart and resumed via `--continue`.
- **Structured Error System & Sanitized Diagnostics**:
  - 13 canonical error codes (`NETWORK_ERROR`, `INVALID_URL`, `UNSUPPORTED_PROVIDER`, `VIDEO_UNAVAILABLE`, `AUTH_REQUIRED`, `FORMAT_UNAVAILABLE`, `DEPENDENCY_MISSING`, `ENGINE_INCOMPATIBLE`, `DISK_FULL`, `PERMISSION_DENIED`, `POSTPROCESSING_FAILED`, `DOWNLOAD_CANCELLED`, `UNKNOWN_ERROR`) with recovery guidance and one-click engine self-repair.
  - Privacy-preserving diagnostic report generator that redacts tokens, cookies, and personal Windows usernames.

### Security
- Enforced `CREATE_NO_WINDOW` (`0x08000000`) on all child processes and replaced external shell folder dialogs with native Win32 `SHBrowseForFolderW` (`shell32.dll`).
- Added Windows filename and output directory sanitization against path traversal (`..`) and reserved DOS device names.

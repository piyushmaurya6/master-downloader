# Changelog

All notable changes to **Master Downloader** are documented in this file.

## [1.1.0] — 2026-11-15

### Added & Improved
- **Codeberg + Multi-Mirror Engine Provisioning (`gallery-dl` & `FFmpeg 8`)**: Updated both CI sidecar provisioning (`scripts/setup-sidecars.ps1`) and the in-app Rust `BinaryManager` (`binary_manager.rs`) to dynamically resolve `gallery-dl.exe` from Codeberg's official release API (`gh#9374`) with automatic `gdl-org/builds` fallback, plus multi-mirror `FFmpeg-Builds` downloads.
- **Automated Windows x64 CI/CD Release Pipeline**: Verified end-to-end automated GitHub Actions NSIS installer compilation, frontend + Rust unit test gates, and MSVC Common Controls v6 test-harness manifest embedding (`build.rs`).
- **Complete Visual Product Showcase & Documentation**: Added 2x Retina UI screenshots of both **Master Downloader** (Workspace, Instagram Carousel Picker, Media Library) and the **FlowDL Booth** (YouTube 4K/8K Studio & Parallel Queue), plus expanded real-world use-case and FAQ documentation.

## [1.0.0] — 2026-11-13

### Added
- **Unified Multi-Platform Architecture**: Converged **FlowDL** (YouTube 4K/8K & Audio), **InstaFlow** (Instagram Reels, Carousels, Stories & Highlights), and **FlowFB** (Facebook Watch, Reels, Videos & Multi-Photo Albums) into one commercial Tauri v2 + Rust + React 19 Windows desktop suite.
- **Universal Smart Omnibar & Auto-Detection**: Instant platform classification, URL canonicalization, and zero-click clipboard/paste **Auto-Download (`Auto-DL: ON`)** mode alongside granular **Inspect** mode.
- **Instagram Public 1080p GraphQL Polaris Extractor**: Native Rust extractor supporting `xdt_shortcode_media` and `xig_polaris_media` for zero-login public Reels and multi-slide Carousels (`?img_index=N`), with automatic `yt-dlp` and `gallery-dl` fallback.
- **Facebook Pre-Flight Redirect & Share Resolver**: Resolves `fb.watch/*` and `facebook.com/share/{v|r|p}/*` tokens before routing through the dual-engine (`yt-dlp` -> `gallery-dl`) pipeline.
- **YouTube Radio/Mix Guard & Deno Signature Solver**: Automatically strips auto-generated `&list=RD...` Infinite Mix parameters on single watch links while preserving real playlists, paired with `--force-ipv4` and bundled `deno.exe` JS challenge runtime.
- **Multi-Platform AES-256-GCM Session Vault**: Isolated WebView2 login windows for Instagram (`sessionid` + `ds_user_id`) and Facebook (`c_user` + `xs`), encrypted via AES-256-GCM + Windows Credential Manager (`keyring`), plus `cookies.txt` import and browser cookie delegation (`Firefox`, `Edge`, `Chrome`, `Brave`), passed through zeroized-on-drop `EphemeralCookieGuard` temp files.
- **EBU R128 Broadcast Loudness Normalization**: Optional `loudnorm=I=-16:TP=-1.5:LRA=11` FFmpeg audio filter alongside `MP3`, `M4A`, `FLAC`, `Opus`, and `WAV` extraction.

### Commercial Hardening & Stability Fixes
- **Win32 Single-Instance Mutex**: Prevents duplicate app instances from competing over SQLite and automatically restores/focuses the existing window.
- **SQLite WAL + Power-Cut Corruption Recovery**: Enabled `WAL` journaling, `8000ms` busy timeout, orphaned temp cookie cleanup on boot, and automatic `.corrupt.<timestamp>.db` backup & recovery if sudden power loss corrupts SQLite.
- **Zero-Copy NTFS Hardlink Sidecar Provisioning**: Provisions bundled `yt-dlp`, `gallery-dl`, `ffmpeg`, `ffprobe`, and `deno` in `< 2ms` using NTFS hardlinks (saving 315 MB of disk space per device) with safe unlink-before-rename on self-update.
- **Per-Job Subprocess Tree Termination & Byte-Offset Resume**: Tracks subprocess PIDs per `job_id` so Pause/Cancel immediately halts `taskkill /F /T` trees without triggering fallback engines, and resumes `.part` files from existing byte offsets via `--continue`.
- **Win32 `ShellExecuteW` & RFC 3986 Media Protocol**: Safe file opening and Explorer `/select` handling for paths containing spaces, `&`, parentheses, and Unicode characters, plus a 128 MB inline memory guard on `master-media://`.
- **Commercial NSIS Installer**: Configured `installMode: "both"` (per-user zero-UAC or per-machine admin), LZMA compression, and silent WebView2 bootstrapper support.

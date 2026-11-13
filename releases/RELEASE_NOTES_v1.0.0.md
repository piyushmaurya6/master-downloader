# Master Downloader Suite `v1.0.0` — Commercial Windows x64 Release

**Master Downloader `v1.0.0`** unifies **FlowDL** (YouTube 4K/8K & Audio), **InstaFlow** (Instagram Reels, Carousels, Stories & Highlights), and **FlowFB** (Facebook Watch, Reels, Videos & Photo Albums) into a single, 100% self-contained Windows 10/11 desktop application.

---

## Which File Should I Download?

- **`Master_Downloader_1.0.0_x64-setup.exe`** *(Recommended for everyone)* — All-in-One installer supporting **YouTube + Instagram + Facebook** with zero-click clipboard Auto-Download (`Auto-DL: ON`), granular stream/carousel inspector, EBU R128 `-16 LUFS` loudness normalization, encrypted AES-256-GCM Session Vault, and bundled `yt-dlp`, `gallery-dl`, `FFmpeg`, and `Deno` engines.
- **Individual Standalone Installers** *(Optional)*:
  - `FlowDL_1.0.0_x64-setup.exe` — Dedicated YouTube 4K/8K & Audio app
  - `InstaFlow_1.0.0_x64-setup.exe` — Dedicated Instagram Reels, Carousels & Stories app
  - `FlowFB_1.0.0_x64-setup.exe` — Dedicated Facebook Watch, Reels & Videos app

---

## Highlights in `v1.0.0`
- **Universal URL Auto-Detection & Normalization**: Supports single URLs and mixed multi-platform batch pastes; strips YouTube `&list=RD...` Infinite Mixes on single watch links while preserving full playlists; resolves `fb.watch/*` and `facebook.com/share/*` tokens automatically.
- **Zero-Login Public Extraction + Encrypted Vault**: Native Rust Instagram GraphQL Polaris (`xdt_shortcode_media`) extractor + `yt-dlp` + `gallery-dl` fallback cascade, plus isolated WebView2 login & AES-256-GCM + Windows Credential Manager session storage for private/story content.
- **Commercial Desktop Reliability**:
  - Win32 single-instance mutex with automatic window focus/restore
  - SQLite `WAL` mode + automatic power-cut corruption backup & recovery
  - Zero-copy NTFS hardlink sidecar provisioning (`< 2ms` cold launch, saving 315 MB disk space)
  - Instant per-job `taskkill /F /T` pause/cancel and `--continue` byte-offset resume
  - Per-user (zero-UAC) or per-machine NSIS installation with silent WebView2 bootstrapper

Verify installer integrity against `SHA256SUMS.txt`.

# Master Downloader — Unified Auto-Detecting & Auto-Downloading Media Suite

**Master Downloader** (`v1.0.0`) converges every capability of **FlowDL** (YouTube 4K/8K & Audio Downloader), **InstaFlow** (Instagram Reels, Carousels, Stories & Highlights Downloader), and **FlowFB** (Facebook Watch, Reels, Videos & Albums Downloader) into a single commercial-grade, self-contained Windows desktop application built with **Tauri v2**, **Rust**, **React 19**, **TypeScript**, **Tailwind CSS v4**, and **SQLite WAL**.

---

## Key Capabilities

### 1. Universal Platform & URL Auto-Detection
Automatically classifies, canonicalizes, and routes URLs across **YouTube**, **Instagram**, **Facebook**, and any `yt-dlp` / `gallery-dl` supported link:
- **YouTube (`FlowDL` Engine)**:
  - Standard Watch (`youtube.com/watch?v=*`, `youtu.be/*`) with automatic `&list=RD...` Radio/Infinite-Mix stripping on single videos
  - YouTube Shorts (`youtube.com/shorts/*`), Playlists (`youtube.com/playlist?list=*`), YouTube Music (`music.youtube.com/*`), Live Streams & Embeds
  - Up to **8K / 4K60 HDR** (`AV1`, `VP9`, `H.264`, `HEVC`) with bundled `deno.exe` signature solver and `--force-ipv4` throttling resilience
- **Instagram (`InstaFlow` Engine)**:
  - Public **1080p GraphQL Polaris Extractor** (`xdt_shortcode_media` / `xig_polaris_media`) for zero-login public Reels & multi-slide Carousels (`?img_index=N`)
  - Automatic fallback cascade: `Instagram GraphQL -> yt-dlp -> gallery-dl`
  - Stories, Highlights (`/stories/{user}/{id}`, `/stories/highlights/{id}`, `/s/{code}`), IGTV, and Creator Profiles
- **Facebook (`FlowFB` Engine)**:
  - Pre-flight redirect & share-token resolver for `fb.watch/{token}` and `facebook.com/share/{v|r|p}/{token}`
  - Watch streams, Reels, Page Videos, Permalinks, Group Posts, Stories, and Multi-Photo Albums (`yt-dlp -> gallery-dl` fallback)

### 2. Zero-Click Auto-Download (`Auto-DL: ON`) & Granular `Inspect` Mode
- **Smart Clipboard Monitor**: Primed on startup to ignore stale clipboard text, deduplicates against active queue items, and immediately enqueues newly copied YouTube/Instagram/Facebook links when `Auto-DL: ON` is active.
- **Inspect Drawer**: Full video/audio stream matrix, selective Instagram carousel slide picker, Single Video vs. Full Playlist toggle, container selection (`MP4`, `MKV`, `WebM`), subtitle/chapter embedding, and **EBU R128 Broadcast Loudness Normalization** (`loudnorm=I=-16:TP=-1.5:LRA=11`).

### 3. Multi-Platform AES-256-GCM Session Vault
- Isolated WebView2 login windows for **Instagram** (`sessionid` + `ds_user_id`) and **Facebook** (`c_user` + `xs`).
- Encrypts sessions at rest in `%LOCALAPPDATA%\com.masterdownloader.desktop\vault.bin` using **AES-256-GCM** with master keys stored in **Windows Credential Manager**.
- Supports Netscape `cookies.txt` import and browser cookie delegation (`Firefox`, `Edge`, `Chrome`, `Brave`) via zeroized-on-drop `EphemeralCookieGuard` temp files.

### 4. Commercial Desktop Hardening
- **Win32 Single-Instance Mutex**: Restores and focuses the running window if launched a second time.
- **SQLite WAL + Power-Cut Recovery**: `WAL` journaling, `8000ms` busy timeout, startup temp-file scrubbing, and automatic `.corrupt.<timestamp>.db` backup & recovery.
- **Zero-Copy NTFS Hardlink Sidecar Provisioning**: Provisions bundled `yt-dlp.exe`, `gallery-dl.exe`, `ffmpeg.exe`, `ffprobe.exe`, and `deno.exe` in `< 2ms` while saving 315 MB of disk space per device.
- **Instant Per-Job Subprocess Tree Termination & Byte-Offset Resume**: Tracks subprocess trees per `job_id` (`taskkill /F /T`) and resumes paused `.part` downloads from existing byte offsets (`--continue`).
- **Light & Dark Theme**: Full persistent theme support in the custom frameless titlebar.

---

## Quick Build Commands

```powershell
# 1. Install frontend dependencies
npm install

# 2. Typecheck, run Vitest suite, and build frontend
npm run typecheck
npm run test
npm run build

# 3. Run Rust backend unit tests
cd src-tauri
cargo test --lib -- --skip test_live_multi_platform_inspect_and_download
cd ..

# 4. Build commercial NSIS installer (.exe)
npx tauri build --bundles nsis
```

See [CHANGELOG.md](./CHANGELOG.md), [SECURITY.md](./SECURITY.md), [CONTRIBUTING.md](./CONTRIBUTING.md), and [THIRD_PARTY_NOTICES.md](./THIRD_PARTY_NOTICES.md) for full details.

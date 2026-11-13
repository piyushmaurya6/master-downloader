# FlowFB — Premium Windows Desktop Facebook Media Downloader

<div align="center">
  <strong>A self-contained, zero-dependency Windows 10/11 desktop application for downloading Facebook Videos, Reels, Watch streams, and Multi-Media Posts in up to 4K / 1080p60 DASH quality.</strong>
  <br />
  Built with <strong>Tauri v2 · Rust · React 19 · TypeScript · Tailwind CSS v4 · SQLite WAL</strong>
</div>

---

## ✨ Key Features

- **Universal Facebook Link Support:**
  - **Facebook Reels:** `facebook.com/reel/{id}` & `facebook.com/reels/{id}` (9:16 vertical HD/1080p)
  - **Facebook Watch & Videos:** `facebook.com/watch/?v={id}`, `facebook.com/{page}/videos/{id}`, `video.php?v={id}`
  - **Shortlinks & Modern Share Tokens:** Automatic HTTP redirect resolution for `fb.watch/{token}` and `facebook.com/share/{v|r|p}/{token}`
  - **Story & Permalink Posts:** `story.php?story_fbid={pfbid}&id={page_id}`, `permalink.php`, `/groups/{group}/posts/{id}`
  - **Photos & Multi-Item Albums:** `facebook.com/photo/?fbid={id}` and multi-item posts via automatic `gallery-dl` fallback
- **True 4K / 1440p / 1080p60 DASH Stream Matrix:**
  - Facebook serves resolutions above 720p as split DASH video (`avc1`/`vp09`/`av01`) and audio (`mp4a`) streams.
  - FlowFB inspects all available DASH representations, displays resolution/codec/bitrate cards, and losslessly muxes video + audio into a clean `.mp4` container via bundled `FFmpeg`.
- **Dedicated Audio Extraction:**
  - Extract 320kbps `.mp3` or native `.m4a` (AAC) audio tracks with embedded cover thumbnails and ID3 metadata.
- **100% Self-Contained Windows `.exe`:**
  - Bundles standalone `yt-dlp.exe`, `gallery-dl.exe`, `ffmpeg.exe`, and `ffprobe.exe` in `%LOCALAPPDATA%\FlowFB\bin`.
  - Users never need to install Python, open PowerShell, or run terminal commands.
  - Every background process executes with `CREATE_NO_WINDOW (0x08000000)`—zero flashing CMD windows.
- **3-Tier Encrypted Authentication Vault:**
  - **Tier 1:** Built-in isolated WebView2 Facebook login (`c_user` & `xs` cookies captured and encrypted locally).
  - **Tier 2:** Netscape `cookies.txt` file import or drag-and-drop.
  - **Tier 3:** Browser cookie delegation (Firefox, Edge, Chrome, Brave) with Chromium v127+ App-Bound Encryption (`v20`) advisory.
  - All stored sessions are encrypted at rest with **AES-256-GCM** using a 256-bit key stored in **Windows Credential Manager**.
- **Linear & Raycast-Inspired Craft UI:**
  - Custom frameless titlebar, `Ctrl+K` Command Palette, Smart Clipboard Monitor, Batch Multi-URL Downloader (up to 100 links), persistent SQLite download queue, and searchable Media Library.

---

## 🏗️ Architecture Overview

| Layer | Technology | Role |
| :--- | :--- | :--- |
| **Desktop Shell** | Tauri v2 (`WebView2`) | Native frameless window, low RAM footprint (~45 MB idle), OS integration |
| **Core Backend** | Rust (2021 Edition, Tokio) | URL canonicalization, pre-flight redirect resolver, subprocess supervisor, AES-256-GCM vault |
| **Database** | SQLite (`rusqlite` bundled WAL) | Crash-resilient queue persistence, download history, and user settings |
| **Primary Video Engine** | `yt-dlp.exe` (PyInstaller x64) | Facebook GraphQL/Tahoe metadata & DASH split-stream extraction |
| **Fallback Post Engine** | `gallery-dl.exe` (Standalone x64) | Multi-photo Facebook albums, text posts, and static image sets |
| **Media Remuxer** | `ffmpeg.exe` + `ffprobe.exe` | Lossless DASH video+audio stream merging & MP3/M4A audio extraction |
| **Frontend UI** | React 19 + TypeScript 5.7 + Tailwind v4 | Responsive UI, Framer Motion animations, Zustand stores |

See [`docs/ARCHITECTURE.md`](./docs/ARCHITECTURE.md) for the complete system design and data-flow diagrams.

---

## 🚀 Development & Building from Source

### Prerequisites (For Developers Only)
- **Node.js 20+** and `npm`
- **Rust 1.80+** (`cargo`)
- Standalone Windows x64 binaries placed in `src-tauri/resources/bin/`:
  - `yt-dlp.exe`
  - `gallery-dl.exe`
  - `ffmpeg.exe` & `ffprobe.exe`

### Commands

```bash
# Install dependencies
npm install

# Run TypeScript typecheck & Vitest unit tests
npm run typecheck
npm run test

# Run Rust backend unit tests
cd src-tauri && cargo test

# Start development mode with hot-reload
npm run tauri dev

# Build production Windows NSIS installer (.exe)
npm run tauri build -- --bundles nsis
```

---

## ⚖️ Legal & Authorized Use Disclaimer

FlowFB is an independent, open-source media utility designed strictly for personal archiving, offline viewing of public content, and downloading content that you own or are explicitly authorized to access under your own Facebook account session. FlowFB does **not** bypass DRM, crack private profiles, or circumvent platform access controls.

---

## 📄 License

Licensed under the **MIT License**. See [`LICENSE`](./LICENSE) and [`THIRD_PARTY_NOTICES.md`](./THIRD_PARTY_NOTICES.md) for bundled component licenses.

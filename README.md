# Master Downloader Suite — Complete Unified & Standalone Media Downloader Ecosystem (`v1.0.0`)

**Master Downloader** converges three specialized commercial desktop downloaders — **FlowDL** (YouTube 4K/8K & Audio), **InstaFlow** (Instagram Reels, Carousels, Stories & Highlights), and **FlowFB** (Facebook Watch, Reels, Videos & Multi-Photo Albums) — into **one unified auto-detecting, auto-downloading Windows desktop application**, built on **Tauri v2**, **Rust**, **React 19**, **TypeScript**, **Tailwind CSS v4**, and **SQLite WAL**.

This repository contains **everything** in one place:
1. **Flagship Unified Application (`master-downloader/`)** — All-in-one YouTube + Instagram + Facebook + Generic `yt-dlp`/`gallery-dl` desktop suite.
2. **All 3 Original Standalone Applications (`standalone-apps/`)** — Full source code for **FlowDL**, **InstaFlow**, and **FlowFB**.
3. **Ready-to-Install Commercial Windows Releases (`releases/`)** — Pre-built NSIS Setup Installers (`*-setup.exe`), standalone executables, release notes, and cryptographic `SHA256SUMS.txt`.
4. **Complete Documentation, Automation & CI/CD (`docs/`, `scripts/`, `.github/`)** — User manual, technical convergence architecture, source build guide, sidecar bootstrap script, automated GitHub upload script (`PUSH_TO_GITHUB.ps1`), and GitHub Actions release pipeline.

---

## Repository Directory Map

```text
.
├── README.md                             # Project overview, feature matrix & quick start
├── GITHUB_UPLOAD_INSTRUCTIONS.md         # Step-by-step guide to upload code + releases to GitHub
├── PUSH_TO_GITHUB.ps1                    # 1-command automated GitHub repository + Release uploader
├── CHANGELOG.md                          # v1.0.0 release & commercial hardening changelog
├── LICENSE                               # MIT License
├── SECURITY.md                           # AES-256-GCM vault, log redaction & subprocess security model
├── CONTRIBUTING.md                       # Developer workflow & verification checklist
├── THIRD_PARTY_NOTICES.md                # Open-source attributions (yt-dlp, gallery-dl, FFmpeg, Deno, Tauri)
├── .gitignore                            # Clean Git ignore rules
├── .gitattributes                        # Git LFS rules for large binaries (*.exe, *.dll)
├── .github/
│   ├── PULL_REQUEST_TEMPLATE.md
│   └── workflows/
│       └── build-and-release.yml         # Automated Windows x64 CI/CD build & GitHub Release workflow
├── docs/
│   ├── USER_MANUAL.md                    # Complete end-user guide & troubleshooting
│   ├── ARCHITECTURE_AND_CONVERGENCE.md   # Technical deep-dive into the multi-engine Rust architecture
│   └── BUILD_FROM_SOURCE.md              # Building & signing all 4 apps from source on Windows
├── scripts/
│   └── setup-sidecars.ps1                # Downloads or syncs yt-dlp, gallery-dl, FFmpeg & Deno sidecars
├── releases/                             # Ready-to-share Windows x64 installers & checksums
│   ├── Master_Downloader_1.0.0_x64-setup.exe
│   ├── MasterDownloader.exe
│   ├── FlowDL_1.0.0_x64-setup.exe
│   ├── FlowDL.exe
│   ├── InstaFlow_1.0.0_x64-setup.exe
│   ├── InstaFlow.exe
│   ├── FlowFB_1.0.0_x64-setup.exe
│   ├── FlowFB.exe
│   ├── RELEASE_NOTES_v1.0.0.md
│   └── SHA256SUMS.txt
├── master-downloader/                    # Flagship Unified Desktop App (YouTube + Instagram + Facebook)
└── standalone-apps/
    ├── flowdl/                           # Standalone YouTube 4K/8K & Audio Downloader
    ├── instaflow/                        # Standalone Instagram Reels/Carousels/Stories Downloader
    └── flowfb/                           # Standalone Facebook Watch/Reels/Videos Downloader
```

---

## Pre-Built Windows x64 Installers (`releases/`)

| Application | Scope | NSIS Installer (`releases/`) | Standalone Binary |
| :--- | :--- | :--- | :--- |
| **Master Downloader v1.0.0** *(Recommended)* | **Unified All-in-One** (YouTube + Instagram + Facebook + Generic URLs) | `Master_Downloader_1.0.0_x64-setup.exe` | `MasterDownloader.exe` |
| **FlowDL v1.0.0** | YouTube 4K/8K, Shorts, Music & Playlists | `FlowDL_1.0.0_x64-setup.exe` | `FlowDL.exe` |
| **InstaFlow v1.0.0** | Instagram Reels, Carousels, Stories & Highlights | `InstaFlow_1.0.0_x64-setup.exe` | `InstaFlow.exe` |
| **FlowFB v1.0.0** | Facebook Watch, Reels, Videos & Photo Albums | `FlowFB_1.0.0_x64-setup.exe` | `FlowFB.exe` |

> **Note for End Users**: Always distribute the **`*_x64-setup.exe`** NSIS installer — it is 100% self-contained and bundles `yt-dlp.exe`, `gallery-dl.exe`, `ffmpeg.exe`, `ffprobe.exe`, `deno.exe`, and silent Microsoft Edge WebView2 bootstrapper support for clean Windows 10/11 machines without requiring admin rights (supports both Per-User and Per-Machine installation).

---

## Converged Feature Matrix (`Master Downloader v1.0.0`)

| Capability | YouTube (`FlowDL`) | Instagram (`InstaFlow`) | Facebook (`FlowFB`) |
| :--- | :---: | :---: | :---: |
| **Zero-Click Clipboard & Paste Auto-Download (`Auto-DL: ON`)** | Yes | Yes | Yes |
| **Granular Stream Matrix & Quality/Codec Inspector (`Inspect`)** | Up to 8K / 4K60 HDR (`AV1`/`VP9`/`H.264`/`HEVC`) | 1080p MP4 + Original JPEG/WebP Slides | Up to 4K/1080p HD Dash Muxing |
| **Zero-Login Public Extraction** | `yt-dlp` + `deno.exe` + `--force-ipv4` | Native Rust GraphQL Polaris (`xdt_shortcode_media`) + `yt-dlp` + `gallery-dl` | Pre-flight Share/Redirect Resolver + `yt-dlp` + `gallery-dl` |
| **Multi-Item Batching & Selective Picker** | Playlists + Radio-Mix (`&list=RD...`) Guard | Multi-Slide Carousels (`?img_index=N`), Stories & Highlights | Multi-Photo Albums & Story Posts |
| **Audio Extraction & EBU R128 Normalization (`-16 LUFS`)** | `MP3` / `M4A` / `FLAC` / `Opus` / `WAV` | `MP3` / `M4A` / `FLAC` / `Opus` / `WAV` | `MP3` / `M4A` / `FLAC` / `Opus` / `WAV` |
| **Encrypted Session Vault (AES-256-GCM + Windows Credential Manager)** | Browser Cookie Delegation & `cookies.txt` | Built-in WebView2 Login (`sessionid` + `ds_user_id`) | Built-in WebView2 Login (`c_user` + `xs`) |

---

## Commercial-Grade Engineering Highlights

- **Win32 Single-Instance Mutex**: Launching a second copy automatically restores and focuses the running window instead of opening duplicate instances over SQLite.
- **SQLite WAL + Power-Cut Self-Healing**: Runs with `journal_mode = WAL` and `busy_timeout = 8000ms`. If sudden power loss corrupts `master_downloader.db`, startup automatically backs up the damaged file (`master_downloader.corrupt.<timestamp>.db`) and initializes cleanly.
- **Zero-Copy NTFS Hardlink Sidecar Seeding (`< 2ms` Cold Start)**: Seeds 315 MB of bundled media engines into `%LOCALAPPDATA%\com.masterdownloader.desktop\bin` via NTFS hardlinks (with cross-volume copy fallback), saving 315 MB of disk space per device.
- **Per-Job Subprocess Tree Control & Byte-Offset Resume**: Tracks child PIDs per `job_id` so pausing or cancelling immediately terminates only that job's process tree (`taskkill /F /T`) and resumes `.part` files from their exact byte offset (`--continue`).
- **Hardened Win32 Shell & Custom Media Protocol**: Uses direct Win32 `ShellExecuteW` (`open`), raw Explorer `/select,"<path>"` arguments, and RFC 3986 percent-decoded `master-media://` streaming with a 128 MB inline memory guard.

---

## Documentation Index

1. **[Uploading Everything to GitHub (`GITHUB_UPLOAD_INSTRUCTIONS.md`)](./GITHUB_UPLOAD_INSTRUCTIONS.md)** — Start here to publish this bundle to your GitHub account in minutes.
2. **[End-User Manual (`docs/USER_MANUAL.md`)](./docs/USER_MANUAL.md)** — Complete walkthrough of Auto-DL, Inspect mode, Cookie Vault, Audio Normalization, and Troubleshooting.
3. **[Architecture & Convergence Spec (`docs/ARCHITECTURE_AND_CONVERGENCE.md`)](./docs/ARCHITECTURE_AND_CONVERGENCE.md)** — Deep technical overview of the Rust backend, URL router, GraphQL extractor, and state machine.
4. **[Build & Code-Signing Guide (`docs/BUILD_FROM_SOURCE.md`)](./docs/BUILD_FROM_SOURCE.md)** — How to compile, test, package, and Authenticode-sign all 4 applications from scratch.

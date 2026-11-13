# Architecture & Multi-App Convergence Specification

## 1. How the Three Standalone Apps Converged into `Master Downloader`

Originally built as three specialized Tauri v2 + Rust applications in `standalone-apps/`:
1. **`FlowDL` (`standalone-apps/flowdl`)**: Specialized YouTube 4K/8K, HDR, Shorts, Music & Playlist engine (`yt-dlp` + `deno` JS challenge solver + FFmpeg stream muxing + EBU R128 `loudnorm`).
2. **`InstaFlow` (`standalone-apps/instaflow`)**: Specialized Instagram Reels, multi-slide Carousels (`?img_index=N`), Stories & Highlights engine with AES-256-GCM WebView2 session capture and dual `yt-dlp` / `gallery-dl` extraction.
3. **`FlowFB` (`standalone-apps/flowfb`)**: Specialized Facebook Watch, Reels, Page Videos, Permalinks & Photo Albums engine with pre-flight HTTP redirect/share-token resolution (`fb.watch/*`, `facebook.com/share/*`).

**`Master Downloader` (`master-downloader/`)** unifies all three codebases into a single coherent architecture without feature loss:

```text
User Clipboard / Smart Omnibar (Mixed Batch URLs Supported)
                        │
                        ▼
       [ core::url_normalizer::normalize_url ]
  ┌─────────────────────┼─────────────────────┐
  ▼                     ▼                     ▼
YouTube             Instagram              Facebook / Generic
(&list=RD guard,    (Shortcode, Reel,      (Pre-flight HTTP HEAD/GET
Playlist detect,     Carousel ?img_index,   resolver for fb.watch/* &
Shorts/Music/Live)   Story/Highlight)       /share/{v|r|p}/* tokens)
  │                     │                     │
  └─────────────────────┼─────────────────────┘
                        ▼
        [ core::engine_router::EngineRouter ]
  • Instagram Public Posts/Reels:
      1) Native Rust GraphQL Polaris (`xdt_shortcode_media` / `xig_polaris_media`)
      2) Fallback -> `yt-dlp.exe`
      3) Fallback -> `gallery-dl.exe`
  • Facebook Videos / Reels / Albums:
      1) `yt-dlp.exe` (HD + Audio DASH merge via FFmpeg)
      2) Fallback -> `gallery-dl.exe` (Multi-photo sets & stories)
  • YouTube Watch / Shorts / Music / Playlists:
      1) `yt-dlp.exe --force-ipv4 --js-runtimes deno:<deno.exe>` + `ffmpeg.exe`
```

---

## 2. Core Rust Modules (`master-downloader/src-tauri/src/`)

| File | Responsibility |
| :--- | :--- |
| `lib.rs` | Win32 single-instance mutex (`CreateMutexW` + window restore), `%LOCALAPPDATA%\com.masterdownloader.desktop` migration, orphaned temp file scrubbing, RFC 3986 `master-media://` custom URI protocol (with 128 MB memory guard), Tauri state wiring. |
| `core/url_normalizer.rs` | Deterministic classification across YouTube, Instagram, Facebook, and Generic URLs; strips tracking params; guards YouTube `&list=RD...` Radio mixes; enforces post-truncation Windows reserved filename sanitization (`CON`, `NUL`, `COM1`..`LPT9`). |
| `core/redirect_resolver.rs` | Resolves opaque Facebook `fb.watch/{token}` and `facebook.com/share/{v\|r\|p}/{token}` URLs to canonical post/reel URLs prior to engine invocation. |
| `core/engine_router.rs` | Multi-engine inspection & download pipeline (`Instagram GraphQL Polaris -> yt-dlp -> gallery-dl`), `--continue` byte-offset resume, 180ms IPC progress throttling, per-job PID registration, and EBU R128 `loudnorm` post-processing. |
| `core/binary_manager.rs` | Zero-copy NTFS hardlink sidecar provisioning (`< 2ms` startup), per-`job_id` `ProcessRegistry` (`abort_and_kill_job` via `taskkill /F /T`), and atomic self-updater with unlink-before-rename backup/rollback. |
| `core/auth_vault.rs` | Multi-platform AES-256-GCM cookie vault (`Instagram` + `Facebook`) backed by Windows Credential Manager (`keyring`), plus zeroized-on-drop `EphemeralCookieGuard` temp files. |
| `core/queue_worker.rs` | Bounded async worker pool with configurable concurrency, randomized anti-429 jitter (`250–600ms`), background Auto-DL metadata enrichment, and pause/cancel race protection. |
| `db/repository.rs` | Embedded SQLite (`WAL` mode + `8000ms` busy timeout) storing jobs, media outputs, history, and settings, with automatic `.corrupt.<timestamp>.db` backup & self-healing on power loss. |
| `error.rs` | Context-aware error taxonomy (`RateLimited`, `AuthRequired`, `GeoOrAgeBlocked`, `MediaUnavailable`, `NetworkTimeout`) avoiding false-positive `"429"` matches inside media IDs, plus regex cookie/token redaction. |

# Dependency Licenses & Legal Compliance

This document summarizes the licenses and redistribution obligations for **FlowDL** and its bundled runtime components.

---

## 1. Summary of Bundled Binary Licenses

| Component | Bundled Files | Upstream Source | License | Redistribution Obligation |
| :--- | :--- | :--- | :--- | :--- |
| **FlowDL Application** | `FlowDL.exe` | [`piyushmaurya6/flowdl`](https://github.com/piyushmaurya6/flowdl) | **GPL-3.0-or-later** (with original code dual-licensed under MIT) | Provide license text and source code repository link. |
| **yt-dlp** | `bin/yt-dlp.exe` | [`yt-dlp/yt-dlp`](https://github.com/yt-dlp/yt-dlp) | **The Unlicense** (standalone Windows x64 binary bundles Python & libraries) | Include `UNLICENSE-yt-dlp.txt` notice. |
| **FFmpeg & ffprobe** | `bin/ffmpeg.exe`, `bin/ffprobe.exe`, `bin/av*.dll`, `bin/sw*.dll` | [`yt-dlp/FFmpeg-Builds`](https://github.com/yt-dlp/FFmpeg-Builds) (`win64-gpl-shared`) | **GNU GPLv3** | Include `LICENSE-FFmpeg-GPLv3.txt`, state GPLv3 licensing clearly, and provide access to corresponding source code (`yt-dlp/FFmpeg-Builds` and `FFmpeg/FFmpeg`). |
| **Deno Runtime** | `bin/deno.exe` | [`denoland/deno`](https://github.com/denoland/deno) | **MIT License** | Include `LICENSE-Deno-MIT.txt` copyright and permission notice. |

---

## 2. Summary of Framework & Library Licenses

| Ecosystem | Key Packages | Licenses |
| :--- | :--- | :--- |
| **Rust Crates (`src-tauri/Cargo.toml`)** | `tauri`, `tokio`, `reqwest`, `serde`, `serde_json`, `rusqlite`, `sha2`, `zip`, `uuid`, `chrono`, `regex`, `thiserror` | `MIT` or `Apache-2.0` |
| **Frontend NPM (`package.json`)** | `react`, `react-dom`, `lucide-react`, `framer-motion`, `tailwindcss`, `clsx`, `tailwind-merge` | `MIT` or `ISC` |

---

## 3. How FlowDL Complies in Distribution

1. **Installer License Packaging**:
   - The NSIS installer (`FlowDL_1.0.0_x64-setup.exe`) packages `LICENSE-FFmpeg-GPLv3.txt`, `UNLICENSE-yt-dlp.txt`, `LICENSE-Deno-MIT.txt`, and `THIRD_PARTY_NOTICES.txt` directly into `%LOCALAPPDATA%\FlowDL\licenses\`.
2. **Repository Notices**:
   - Root [`LICENSE`](../LICENSE) and [`THIRD_PARTY_NOTICES.md`](../THIRD_PARTY_NOTICES.md) provide full text and upstream source links.
3. **No Trademark Misrepresentation**:
   - FlowDL is an independent open-source desktop interface and is not affiliated with, sponsored by, or endorsed by YouTube, Google, or the upstream `yt-dlp` / `FFmpeg` maintainers.
4. **Responsible Use Notice**:
   - Users are advised to download media only when they have permission from the rights holder or where permitted by applicable law.

# Third-Party Notices & Distribution License Compliance

FlowDL is distributed under the **MIT License** and bundles unmodified, standalone open-source executables in its `bin/` directory (`%LOCALAPPDATA%\FlowDL\bin`) that are invoked as separate child processes at runtime.

Full license texts are installed alongside the application in the `licenses/` directory (`%LOCALAPPDATA%\FlowDL\licenses`).

---

## 1. Bundled Media & Runtime Executables

### yt-dlp (`bin/yt-dlp.exe`)
- **License**: The Unlicense (Public Domain)
- **Source Code**: https://github.com/yt-dlp/yt-dlp
- **Bundled Runtime Components inside `yt-dlp.exe`**:
  - Python 3 Runtime — PSF License Version 2 (`https://docs.python.org/3/license.html`)
  - `yt-dlp-ejs` — The Unlicense (`https://github.com/yt-dlp/yt-dlp-ejs`)
  - `certifi` — MPL 2.0 (`https://github.com/certifi/python-certifi`)
  - `brotli` — MIT (`https://github.com/google/brotli`)
  - `mutagen` — GPLv2+ (`https://github.com/quodlibet/mutagen`)
  - `pycryptodomex` — BSD / Public Domain (`https://github.com/Legrandin/pycryptodome`)
  - `urllib3` — MIT (`https://github.com/urllib3/urllib3`)
  - `websockets` — BSD-3-Clause (`https://github.com/python-websockets/websockets`)
  - `curl_cffi` — MIT (`https://github.com/lexiforest/curl_cffi`)

### FFmpeg & ffprobe (`bin/ffmpeg.exe`, `bin/ffprobe.exe`, and shared libraries)
- **License**: GNU General Public License version 3 (GPLv3)
- **License Text**: Installed at `licenses/LICENSE-FFmpeg-GPLv3.txt`
- **Source Code & Build Scripts**:
  - Official FFmpeg project: https://ffmpeg.org/download.html
  - Exact build configuration and source scripts used to produce the bundled Windows x64 binaries: https://github.com/yt-dlp/FFmpeg-Builds
- **Aggregate Distribution Notice**: FlowDL invokes `ffmpeg.exe` and `ffprobe.exe` strictly as separate command-line processes over standard pipes and does not link against FFmpeg libraries.

### Deno JavaScript Runtime (`bin/deno.exe`)
- **Purpose**: Used by `yt-dlp` (`--js-runtimes deno`) for local YouTube signature and challenge solving (`yt-dlp-ejs`).
- **License**: MIT License (`licenses/LICENSE-Deno-MIT.txt`)
- **Source Code**: https://github.com/denoland/deno

---

## 2. Desktop & Backend Libraries (Rust / Tauri)
- **Tauri 2** (`https://tauri.app`) — Apache-2.0 OR MIT
- **Tokio** (`https://tokio.rs`) — MIT
- **Serde & serde_json** (`https://serde.rs`) — Apache-2.0 OR MIT
- **Reqwest** (`https://github.com/seanmonstar/reqwest`) — Apache-2.0 OR MIT
- **Rusqlite / SQLite** (`https://github.com/rusqlite/rusqlite`) — MIT (SQLite is Public Domain)
- **SHA2 / RustCrypto** (`https://github.com/RustCrypto/hashes`) — Apache-2.0 OR MIT
- **Zip-rs** (`https://github.com/zip-rs/zip2`) — MIT

---

## 3. Frontend Libraries (React / TypeScript)
- **React & React DOM** (`https://react.dev`) — MIT
- **Tailwind CSS** (`https://tailwindcss.com`) — MIT
- **Framer Motion** (`https://www.framer.com/motion/`) — MIT
- **Lucide Icons** (`https://lucide.dev`) — ISC

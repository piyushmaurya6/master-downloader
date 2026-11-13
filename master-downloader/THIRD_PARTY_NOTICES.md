# Third-Party Notices

**Master Downloader** bundles or interoperates with the following open-source tools and libraries. Their respective license texts are included under `src-tauri/resources/licenses/`.

| Component | Purpose | License | Upstream |
| :--- | :--- | :--- | :--- |
| **yt-dlp** | Primary media inspection & stream extraction engine | Unlicense | https://github.com/yt-dlp/yt-dlp |
| **gallery-dl** | Secondary carousel, photo album & story extraction engine | GPL-2.0 | https://github.com/mikf/gallery-dl |
| **FFmpeg / FFprobe** | Container muxing, audio transcoding & EBU R128 `loudnorm` | GPL-3.0 / LGPL | https://ffmpeg.org / https://github.com/BtbN/FFmpeg-Builds |
| **Deno** | JavaScript challenge runtime for YouTube extraction | MIT | https://github.com/denoland/deno |
| **Tauri v2** | Desktop application framework & IPC runtime | Apache-2.0 / MIT | https://github.com/tauri-apps/tauri |
| **React 19** | Frontend user interface library | MIT | https://github.com/facebook/react |
| **SQLite (`rusqlite`)** | Embedded WAL persistence engine | Public Domain / MIT | https://sqlite.org |

> Note: Sidecar binaries (`yt-dlp.exe`, `gallery-dl.exe`, `ffmpeg.exe`, `ffprobe.exe`, `deno.exe`) are invoked exclusively as standalone out-of-process executables over standard I/O streams and are not statically or dynamically linked into the `master-downloader.exe` binary.

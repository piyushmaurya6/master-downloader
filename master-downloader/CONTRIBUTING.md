# Contributing to Master Downloader

Thank you for contributing to **Master Downloader**!

## Development Prerequisites (Windows 10 / 11 x64)

1. **Node.js 20+ LTS** & **npm 10+**
2. **Rust 1.77.2+** (`rustup` with `stable-x86_64-pc-windows-msvc` or `gnu`)
3. **Microsoft Edge WebView2 Runtime** (pre-installed on Windows 11 and modern Windows 10)
4. **Sidecar Binaries** placed in `src-tauri/resources/bin/` (run `..\scripts\fetch-sidecars.ps1` from the repository root if cloning without Git LFS binaries):
   - `yt-dlp.exe`
   - `gallery-dl.exe`
   - `deno.exe`
   - `ffmpeg.exe`, `ffprobe.exe`, and shared FFmpeg DLLs (`avcodec-*.dll`, `avformat-*.dll`, `avfilter-*.dll`, `avdevice-*.dll`, `avutil-*.dll`, `swresample-*.dll`, `swscale-*.dll`)

## Verification Checklist Before Submitting a Pull Request

```powershell
# 1. Frontend TypeScript & unit tests
npm install
npm run typecheck
npm run test
npm run build

# 2. Rust backend unit tests (excluding live network test)
cd src-tauri
cargo test --lib -- --skip test_live_multi_platform_inspect_and_download
```

## Engineering Rules
- Every Windows `std::process::Command` or `tokio::process::Command` spawn **must** set `.creation_flags(0x08000000)` (`CREATE_NO_WINDOW`).
- Never log raw cookie values or session headers; route stderr/stdout diagnostics through `crate::error::redact_sensitive_text`.
- Never use `cmd.exe /C` with untrusted media filenames.

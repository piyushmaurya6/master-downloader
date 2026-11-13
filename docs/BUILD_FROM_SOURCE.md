# Building & Signing From Source (Windows 10 / 11 x64)

## 1. Prerequisites
1. **Node.js 20+ LTS**: https://nodejs.org/
2. **Rust Toolchain (`1.77.2+`)**: https://rustup.rs/
3. **Visual Studio C++ Build Tools** (Desktop development with C++) or **MinGW-w64**

---

## 2. Provision Sidecar Binaries (`yt-dlp`, `gallery-dl`, `FFmpeg`, `Deno`)

If you extracted the full ZIP bundle, `master-downloader/src-tauri/resources/bin/` **already contains all 12 sidecar binaries**.

To sync those sidecars into `standalone-apps/*` (or to download fresh sidecars from upstream GitHub Releases on a clean clone where binaries were excluded from Git):

```powershell
Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass
.\scripts\setup-sidecars.ps1
```

---

## 3. Build & Test `Master Downloader` (Flagship Unified App)

```powershell
cd master-downloader

# Install npm dependencies
npm install

# Run TypeScript check, Vitest unit tests (8 tests), and Vite production build
npm run typecheck
npm run test
npm run build

# Run Rust backend unit tests (9 tests)
cd src-tauri
cargo test --lib -- --skip test_live_multi_platform_inspect_and_download
cd ..

# Build commercial NSIS Setup Installer (.exe)
npx tauri build --bundles nsis
```

The resulting NSIS installer is emitted at:
`master-downloader/src-tauri/target/release/bundle/nsis/Master Downloader_1.0.0_x64-setup.exe`

---

## 4. Build Any Standalone App (`FlowDL`, `InstaFlow`, `FlowFB`)

```powershell
# First ensure sidecars are synced into standalone-apps/*
.\scripts\setup-sidecars.ps1

# Example: Build FlowDL, InstaFlow, or FlowFB
cd standalone-apps\flowdl   # or standalone-apps\instaflow or standalone-apps\flowfb
npm install
npx tauri build --bundles nsis
```

---

## 5. Optional Commercial Code Signing (Authenticode EV / OV / Azure Trusted Signing)

To remove the Windows SmartScreen *"Unknown Publisher"* prompt when distributing publicly on the web to millions of users, sign both the inner executable (via Tauri hook) or the generated setup `.exe` with `signtool.exe`:

```powershell
signtool sign /fd SHA256 /tr http://timestamp.digicert.com /td SHA256 /a "releases\Master_Downloader_1.0.0_x64-setup.exe"
```

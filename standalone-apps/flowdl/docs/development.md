# FlowDL Development & Maintenance Guide

This guide covers setting up a local development environment, running tests, debugging, and performing long-term maintenance tasks.

---

## 1. Required SDKs and Toolchains

- **Operating System**: Windows 10 or Windows 11 (64-bit)
- **Node.js**: v18+ (v20 LTS recommended) and `npm`
- **Rust**: Stable toolchain (`1.77.2+`) via `rustup`
- **C/C++ Linker**:
  - Either **Visual Studio Build Tools** (`Desktop development with C++` for `x86_64-pc-windows-msvc`), or
  - **MinGW-w64 / w64devkit** with `rust-lld` (`x86_64-pc-windows-gnu`)
- **WebView2 Runtime**: Pre-installed on Windows 11 and modern Windows 10 (installer includes bootstrapper fallback).

---

## 2. Setup & Startup Commands

```powershell
# 1. Clone the repository
git clone https://github.com/piyushmaurya6/flowdl.git
cd flowdl

# 2. Install frontend dependencies
npm install

# 3. Fetch & verify bundled engines (yt-dlp.exe, ffmpeg.exe, ffprobe.exe, deno.exe)
powershell -ExecutionPolicy Bypass -File scripts/prepare-bundled-binaries.ps1

# 4. Start development mode with hot reload
npm run tauri dev
```

---

## 3. Automated Testing & Quality Checks

```powershell
# Frontend TypeScript check
npm run typecheck

# Frontend ESLint check
npm run lint

# Frontend unit & UI integration tests (Vitest + Testing Library)
npm run test

# Backend Rust formatting, lint, and unit/integration/migration tests
cd src-tauri
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
```

---

## 4. Production Build Commands

```powershell
# Build optimized release binary and self-contained Windows NSIS installer
npm run tauri build -- --bundles nsis
```

Outputs:
- Standalone executable: `src-tauri/target/release/flowdl.exe`
- Self-contained NSIS installer: `src-tauri/target/release/bundle/nsis/FlowDL_1.0.0_x64-setup.exe`

---

## 5. Long-Term Maintenance Playbooks

### Upgrading Tauri
1. Update `@tauri-apps/api` and `@tauri-apps/cli` in `package.json`.
2. Update `tauri` and `tauri-build` in `src-tauri/Cargo.toml`.
3. Run `npm install` and `cargo update -p tauri -p tauri-build`.
4. Verify permissions in `src-tauri/capabilities/default.json` and run `cargo test` + `npm run test`.

### Upgrading Rust or React Dependencies
- Dependabot (`.github/dependabot.yml`) opens weekly PRs for minor/patch updates.
- Always verify `cargo test` and `npm run test` pass before merging.

### Modifying Download Engine Behavior
- All `yt-dlp` command-line flags are isolated in `src-tauri/src/downloader/download_manager.rs` (`build_ytdlp_arguments`) and `src-tauri/src/services/metadata_service.rs`.
- All stdout/stderr progress parsing is isolated in `src-tauri/src/downloader/progress_parser.rs`.
- When adding or modifying flags, add corresponding unit tests in `download_manager.rs` and fixture tests in `progress_parser.rs`.

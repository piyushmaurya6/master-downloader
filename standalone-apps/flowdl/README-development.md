# FlowDL — Developer Quickstart Guide

For comprehensive documentation, see the [`docs/`](./docs/) folder:

- [`docs/architecture.md`](./docs/architecture.md) — 5-layer architecture, `DownloadEngine` trait, IPC events, and SQLite migrations
- [`docs/development.md`](./docs/development.md) — Toolchain setup, testing, debugging, and maintenance playbooks
- [`docs/release-process.md`](./docs/release-process.md) — Semantic versioning, Tauri signing keys, and GitHub Actions release pipeline
- [`docs/auto-updates.md`](./docs/auto-updates.md) — Dual update systems (System A App Updater & System B Engine Updater)
- [`docs/security-model.md`](./docs/security-model.md) — Threat model, process isolation, and SHA-256 verification
- [`docs/dependency-licenses.md`](./docs/dependency-licenses.md) — GPLv3 / Unlicense / MIT compliance obligations
- [`docs/troubleshooting.md`](./docs/troubleshooting.md) — Diagnostic guide and recovery procedures

---

## Quick Commands

```powershell
# 1. Install frontend dependencies
npm install

# 2. Download & verify bundled engines (yt-dlp, FFmpeg, ffprobe, Deno)
powershell -ExecutionPolicy Bypass -File scripts/prepare-bundled-binaries.ps1

# 3. Run in development mode
npm run tauri dev

# 4. Run frontend & backend tests
npm run typecheck
npm run lint
npm run test
cd src-tauri && cargo test

# 5. Build self-contained Windows NSIS installer
npm run tauri build -- --bundles nsis
```

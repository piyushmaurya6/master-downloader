# Contributing to FlowDL

Thank you for your interest in contributing to **FlowDL**! FlowDL is built as a clean, maintainable, open-source Windows desktop application powered by **Tauri 2**, **Rust**, **React**, **TypeScript**, **`yt-dlp`**, and **`FFmpeg`**.

---

## 1. Architecture Principles

Before opening a pull request, please review [`docs/architecture.md`](./docs/architecture.md):

- **Presentation Layer (`src/`)**: React components, hooks, and stores. Must never execute raw shell commands directly.
- **Application Service Layer (`src-tauri/src/queue/`, `src-tauri/src/services/`)**: URL validation, download orchestration, queue concurrency, error classification, and application updater.
- **Download Engine Layer (`src-tauri/src/downloader/`, `src-tauri/src/dependencies/`)**: `DownloadEngine` trait (`YtDlpEngine`), `EngineManager`, machine-readable progress parsing, and SHA-256 binary verification.
- **Persistence Layer (`src-tauri/src/storage/`, `src-tauri/migrations/`)**: Versioned SQLite migrations (`flowdl.db`) and atomic JSON settings (`settings.json`).
- **Platform Integration Layer (`src-tauri/src/platform/`)**: Native Win32 folder dialogs (`shell32.dll`), Explorer integration, and Windows filename/path security sanitization.

---

## 2. Branch & Commit Conventions

### Branch Naming
- `feat/<short-description>` — New user-facing features
- `fix/<short-description>` — Bug fixes
- `refactor/<short-description>` — Internal code structure improvements
- `docs/<short-description>` — Documentation updates
- `ci/<short-description>` — GitHub Actions or build script changes

### Commit Messages (Conventional Commits)
- `feat(downloader): add opus bitrate selector`
- `fix(parser): handle N/A total_bytes in live stream progress`
- `docs(release): document Tauri updater key rotation`
- `test(storage): add SQLite V1 to V2 migration test`

---

## 3. Local Development & Verification Checklist

Before submitting a Pull Request, ensure all checks pass locally:

```powershell
# 1. Install frontend dependencies
npm install

# 2. Prepare bundled binaries (yt-dlp, FFmpeg, ffprobe, Deno)
powershell -ExecutionPolicy Bypass -File scripts/prepare-bundled-binaries.ps1

# 3. Frontend checks (TypeScript, ESLint, Vitest)
npm run typecheck
npm run lint
npm run test

# 4. Backend checks (rustfmt, clippy, cargo test)
cd src-tauri
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
```

---

## 4. Database Schema Changes

If your contribution modifies the SQLite schema in `flowdl.db`:
1. Never modify an existing `V1__*.sql` or `V2__*.sql` migration file.
2. Add a new numbered migration file in `src-tauri/migrations/` (e.g., `V3__description.sql`).
3. Register the migration in `src-tauri/src/storage/history_store.rs` and increment `CURRENT_SCHEMA_VERSION`.
4. Add a unit test verifying that upgrading from previous versions preserves all `download_history` and `download_queue` records.

---

## 5. Pull Request Review Checklist

- [ ] No console windows (`CREATE_NO_WINDOW`) flash during any operation on Windows.
- [ ] All child processes pass arguments as structured arrays (no shell string interpolation).
- [ ] Sensitive tokens, cookies, and personal user paths are redacted from logs.
- [ ] Automated unit and integration tests cover the new or changed behavior.
- [ ] `CHANGELOG.md` is updated under the `[Unreleased]` section.

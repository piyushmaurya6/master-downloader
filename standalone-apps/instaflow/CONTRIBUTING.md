# Contributing to InstaFlow

Thank you for your interest in contributing to **InstaFlow**! This document outlines our development environment setup, architectural conventions, and pull request requirements.

---

## 1. Prerequisites

- **OS:** Windows 10 or Windows 11 (64-bit)
- **Node.js:** v20 LTS+ with `pnpm` or `npm`
- **Rust:** Stable toolchain (`1.77.2+`) via `rustup`
- **WebView2:** Pre-installed on Windows 11 / modern Windows 10

---

## 2. Local Development Workflow

```powershell
# 1. Install frontend dependencies
pnpm install

# 2. Run Frontend Typecheck & Unit Tests
pnpm typecheck
pnpm test

# 3. Run Rust Backend Checks & Unit Tests
cd src-tauri
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test

# 4. Launch Tauri Development Mode
cd ..
pnpm tauri dev
```

---

## 3. Architectural & Security Rules

1. **Zero Console Window Flashing (`CREATE_NO_WINDOW`):**
   Every `tokio::process::Command` or `std::process::Command` spawning `yt-dlp.exe`, `gallery-dl.exe`, or `ffmpeg.exe` on Windows **must** use `new_hidden_tokio_command` (which sets `.creation_flags(0x08000000)` and `.kill_on_drop(true)`).
2. **No Shell Interpolation:**
   Never invoke `cmd.exe /C` with user-supplied strings. Always pass structured argument vectors (`Vec<String>`) with `"--"` before the target URL.
3. **Credential & PII Redaction:**
   Never log raw cookies (`sessionid`, `csrftoken`, `ds_user_id`) or full user home directory paths. All diagnostic log entries must pass through `error::redact_sensitive`.
4. **Access Control Respect:**
   InstaFlow only archives public posts or private posts that the authenticated user is explicitly authorized to view. Do not submit PRs attempting to bypass platform privacy controls.

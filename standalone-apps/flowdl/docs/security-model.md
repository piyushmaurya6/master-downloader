# FlowDL Security Model & Threat Analysis

FlowDL is designed with least-privilege principles to protect users against command injection, supply-chain tampering, path traversal, and credential leakage.

---

## 1. Constrained IPC Boundary

- The React WebView2 frontend (`src/`) has **zero** access to unrestricted shell execution or arbitrary filesystem APIs.
- `src-tauri/capabilities/default.json` grants only `core:default`, `core:event:default`, and explicit window sizing/minimize/maximize/close permissions.
- Every backend action passes through typed Rust command handlers in `src-tauri/src/commands/mod.rs`.

---

## 2. Child Process Execution Safety

- **No Shell Interpolation**: FlowDL never invokes `cmd.exe /c` or `powershell.exe -Command` to run `yt-dlp` or `FFmpeg`. Arguments are passed directly to the OS process creation API (`CreateProcessW`) via `std::process::Command::args`.
- **Argument Injection Prevention**: In `src-tauri/src/downloader/download_manager.rs` and `src-tauri/src/services/metadata_service.rs`, FlowDL validates that every URL starts with `http://` or `https://`, contains no whitespace or control characters, and always places `"--"` immediately before the URL argument so a URL can never be interpreted as a command-line flag.
- **Hidden Window Execution**: Every child process is spawned with `CREATE_NO_WINDOW` (`0x08000000`) and has FlowDL's private `%LOCALAPPDATA%\FlowDL\bin` prepended to its process-local `PATH` without modifying the user's system `PATH`.

---

## 3. Dependency Supply-Chain Verification

| Threat | Mitigation in FlowDL |
| :--- | :--- |
| Compromised or truncated binary download | `ytdlp_service.rs` and `ffmpeg_service.rs` fetch official `SHA2-256SUMS` / `checksums.sha256` over HTTPS and compute the local SHA-256 digest (`compute_file_sha256`) before executing or activating any binary. |
| Broken update leaves app unusable | Updates download to `.tmp`, back up the working binary to `.bak`, run a `--version` health check, and automatically roll back to `.bak` if verification fails. |
| Binary replaced while in use | `EngineManager` and `setup_or_repair_dependencies` check `active_downloads_count > 0` and refuse to overwrite binaries during active downloads. |

---

## 4. Filesystem & Filename Hardening

- `validate_safe_output_dir` rejects relative path traversal (`..`).
- `sanitize_windows_filename` strips control characters, replaces Windows-invalid characters (`<>:"/\|?*`), neutralizes `..` segments, strips trailing dots, and prefixes reserved DOS device names (`CON`, `PRN`, `AUX`, `NUL`, `COM1..9`, `LPT1..9`).

---

## 5. Privacy & Log Sanitization

- FlowDL collects **zero telemetry or analytics**.
- `sanitize_text_for_log` automatically redacts:
  - URL query secrets (`token`, `access_token`, `auth`, `key`, `signature`, `sig`, `session`, `password`, `cookie`)
  - HTTP headers (`Authorization`, `Cookie`, `Set-Cookie`, `X-Auth-Token`)
  - Personal Windows profile paths (`C:\Users\<Username>\` → `C:\Users\[USER]\`)

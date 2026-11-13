# Security Policy

FlowDL takes desktop application security seriously. Because FlowDL executes external media tools (`yt-dlp`, `FFmpeg`, `ffprobe`, and `Deno`) and downloads media from remote URLs, we enforce strict security boundaries across the entire stack.

---

## Supported Versions

| Version | Supported |
| :--- | :--- |
| `1.0.x` | :white_check_mark: Supported |
| `< 1.0.0` | :x: Not supported |

---

## Reporting a Vulnerability

**Please do not report security vulnerabilities through public GitHub issues.**

Instead, please report sensitive security issues privately using **[GitHub Private Vulnerability Reporting](https://github.com/piyushmaurya6/flowdl/security/advisories/new)** on this repository.

When reporting a vulnerability, please include:
1. FlowDL version and Windows version.
2. Component affected (e.g., IPC command handler, URL parser, dependency updater, filename sanitizer).
3. Step-by-step reproduction instructions or proof-of-concept.
4. Impact assessment.

We aim to acknowledge vulnerability reports within **48 hours** and publish a coordinated fix and advisory as quickly as possible.

---

## Security Architecture Highlights

1. **No Shell Interpolation / No Command Injection**:
   - The React frontend has zero direct shell execution permissions.
   - All child processes are spawned via Rust's `std::process::Command` / `tokio::process::Command` with explicit argument arrays and `--` separating options from the target URL.
2. **Verified Dependency Updates**:
   - Engine updates (`yt-dlp` and `FFmpeg`) are downloaded exclusively over HTTPS from official GitHub releases and verified against publisher `SHA2-256SUMS` / `checksums.sha256` before activation.
   - Failed or unverified binaries are immediately discarded and rolled back to the preserved `.bak` binary.
3. **Path Traversal & Reserved Filename Protection**:
   - Output directories and filenames are sanitized against relative path traversal (`..`), control characters, and Windows reserved device names (`CON`, `PRN`, `AUX`, `NUL`, `COM1..9`, `LPT1..9`).
4. **Diagnostic Redaction**:
   - Diagnostic logs and issue reports automatically redact authentication tokens, cookies, session query parameters, and personal Windows usernames (`C:\Users\[USER]\`).

For the complete threat model, see [`docs/security-model.md`](./docs/security-model.md).

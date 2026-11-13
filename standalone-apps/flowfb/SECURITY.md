# Security Policy

## Credential & Session Protection

FlowFB handles optional user-authenticated Facebook sessions with strict defense-in-depth:

1. **Zero Plaintext Persistence:** Session cookies captured via WebView2 or imported via `cookies.txt` are encrypted in memory using **AES-256-GCM** before being written to `%APPDATA%\FlowFB\vault\session.enc`.
2. **OS Keyring Storage:** The 256-bit master encryption key is stored exclusively inside the **Windows Credential Manager** (`keyring` crate) under service `com.flowfb.desktop`.
3. **Ephemeral Subprocess Temp Files:** When `yt-dlp` or `gallery-dl` requires a cookie file, FlowFB writes a short-lived temporary file with a random UUID filename and immediately overwrites it with zeros before unlinking via a Rust `Drop` guard (`EphemeralCookieGuard`).
4. **Log Sanitization:** All diagnostic logs strip `c_user=`, `xs=`, `datr=`, `fr=`, and cookie header strings prior to UI display.

## Reporting a Vulnerability

If you discover a security vulnerability, please open a private security advisory on GitHub rather than a public issue.

# Security Policy & Architecture

## Supported Versions

| Version | Supported |
| :--- | :--- |
| `1.0.x` | Yes |

## Security Architecture

1. **AES-256-GCM Encrypted Session Vault**:
   - Optional Instagram (`sessionid`, `ds_user_id`) and Facebook (`c_user`, `xs`) cookies are encrypted at rest in `%LOCALAPPDATA%\com.masterdownloader.desktop\vault.bin` using 256-bit AES-GCM with a random 96-bit nonce per write.
   - The 256-bit master key is stored in **Windows Credential Manager** via the native Win32 `CredWriteW` / `CredReadW` API (`keyring` crate).
2. **Ephemeral Cookie Guards & Startup Scrubbing**:
   - Decrypted cookies are written to random per-job files only for the lifetime of a single `yt-dlp` or `gallery-dl` invocation, overwritten with zeros on drop (`EphemeralCookieGuard`), and scrubbed automatically at startup (`cleanup_stale_temp_files`) in the event of power loss.
3. **Log Redaction**:
   - All subprocess `stderr`/`stdout` diagnostics pass through regex redaction (`redact_sensitive_text`) before logging or UI display, masking `sessionid`, `csrftoken`, `ds_user_id`, `c_user`, `xs`, and `Bearer` tokens.
4. **Strict Content Security Policy (CSP) & Subprocess Isolation**:
   - All child sidecars (`yt-dlp.exe`, `gallery-dl.exe`, `ffmpeg.exe`, `ffprobe.exe`, `deno.exe`) spawn without shell interpolation (`Command::new` + `CREATE_NO_WINDOW = 0x08000000`).
   - Opening downloaded files uses direct Win32 `ShellExecuteW` (`open`) rather than `cmd.exe /C start`, eliminating command-injection vectors from untrusted media titles.

## Reporting a Vulnerability

Please report security issues privately via **GitHub Security Advisories** (`Security -> Report a vulnerability`) on your repository rather than opening a public issue.

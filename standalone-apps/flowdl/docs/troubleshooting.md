# FlowDL Troubleshooting & Error Reference

This guide explains how to diagnose and resolve common issues in FlowDL.

---

## 1. Canonical Error Codes & Recovery Actions

| Error Code | Meaning | Recommended Recovery Action |
| :--- | :--- | :--- |
| `INVALID_URL` | Input is not a valid `http://` or `https://` URL | Copy the full URL from your browser address bar and try again. |
| `UNSUPPORTED_PROVIDER` | No `yt-dlp` extractor matches the domain | Check that the link points directly to a public video or audio page. |
| `VIDEO_UNAVAILABLE` | Video was deleted, made private, or geo-restricted | Confirm the video plays in an incognito browser window in your region. |
| `AUTH_REQUIRED` | Video is private, members-only, age-gated, or DRM-protected | FlowDL only downloads publicly accessible, non-DRM streams. |
| `FORMAT_UNAVAILABLE` | Selected resolution or codec is not offered by the source | Re-analyze the link or choose **Best available** quality and **Auto** container. |
| `ENGINE_INCOMPATIBLE` | Provider changed its player script or signature cipher | Click **Update Engine & Retry** on the download card, or go to **Settings → Check for Updates**. |
| `DEPENDENCY_MISSING` | `yt-dlp.exe` or `ffmpeg.exe` was removed from `%LOCALAPPDATA%\FlowDL\bin` | Open **Settings → Download Engine** and click **Repair Engines**. |
| `DISK_FULL` | Insufficient storage space on the target drive | Free up disk space or choose a folder on another drive in Settings. |
| `PERMISSION_DENIED` | Selected output folder is read-only or protected | Select a user-writable folder such as `C:\Users\<You>\Downloads`. |
| `POSTPROCESSING_FAILED` | FFmpeg failed while merging streams or converting audio | Click **Repair Engines** in Settings or select **Auto** container. |
| `NETWORK_ERROR` | DNS lookup failure, timeout, or connection reset | FlowDL automatically retries up to 3 times; check your internet connection or VPN and click **Retry**. |

---

## 2. Self-Repairing Broken Download Engines

If an antivirus false positive or manual deletion removes a file from `%LOCALAPPDATA%\FlowDL\bin`:
1. Open **FlowDL → Settings → Download Engine**.
2. Click **Repair Engines**.
3. FlowDL first restores the verified bundled binaries from its installation package (instant, offline), and if needed downloads fresh SHA-256 verified copies from official GitHub releases.

---

## 3. Generating a Sanitized Diagnostic Report

When filing a bug report on GitHub:
1. Go to **Settings → Download Engine**.
2. Click **Copy Sanitized Diagnostic Report**.
3. FlowDL generates a Markdown summary of your OS, engine versions, database schema version, and the last 50 log lines with all tokens, cookies, auth headers, and personal Windows usernames (`C:\Users\[USER]\`) automatically redacted.

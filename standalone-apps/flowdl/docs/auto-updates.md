# FlowDL Dual Auto-Update Architecture

FlowDL separates **Application Updates (System A)** from **Download Engine Updates (System B)** so users get immediate compatibility fixes for video providers without needing to reinstall the desktop application.

---

## System A — FlowDL Application Updates

- **Implementation**: `src-tauri/src/services/app_updater.rs` + `src/components/AppUpdateBanner.tsx`
- **Behavior**:
  1. On startup (when `checkAppUpdatesOnStartup` is enabled) or when the user clicks **Check for App Updates** in Settings, FlowDL queries `https://api.github.com/repos/piyushmaurya6/flowdl/releases/latest` over HTTPS.
  2. Compares the published release tag against `CARGO_PKG_VERSION` using strict semantic versioning (`parse_semver`).
  3. Verifies whether the release includes a signed installer/checksum artifact (`signature_verified`).
  4. Emits `app:update-available` and displays an unobtrusive top banner:
     - **"FlowDL X.Y.Z is available."**
     - **Update Now**: Checks that no downloads are currently active before launching the verified release installer download.
     - **View What's New**: Expands the release date, version diff, file size, and Markdown release notes.
     - **Remind Me Later**: Dismisses the banner for the current session.
- **Data Preservation**:
  - User settings (`%LOCALAPPDATA%\FlowDL\settings.json`), SQLite history & queue (`%LOCALAPPDATA%\FlowDL\flowdl.db`), and user media downloads (stored in the user's `Downloads` folder) are never deleted during an application upgrade.

---

## System B — Independent Engine Updates (`yt-dlp` & `FFmpeg`)

- **Implementation**: `src-tauri/src/dependencies/engine_manager.rs`, `ytdlp_service.rs`, and `ffmpeg_service.rs`
- **Why Independent Updates Matter**:
  - Video sites frequently change player scripts or signature algorithms. FlowDL 1.0.0 can adopt `yt-dlp 2026.11.x+` within seconds without waiting for FlowDL 1.1.0.
- **10-Step Safe Update & Rollback Procedure**:
  1. Fetch official release metadata over HTTPS (`api.github.com/repos/yt-dlp/yt-dlp/releases/latest` and `yt-dlp/FFmpeg-Builds`).
  2. Select the Windows x64 standalone binary (`yt-dlp.exe` / `ffmpeg-master-latest-win64-gpl.zip`).
  3. Download to a temporary staging file (`yt-dlp.exe.tmp`) in `%LOCALAPPDATA%\FlowDL\tmp`.
  4. Download the publisher's `SHA2-256SUMS` / `checksums.sha256` file and verify the exact SHA-256 digest of the downloaded file.
  5. Execute `<staged_binary> --version` (`CREATE_NO_WINDOW`) to confirm the binary starts cleanly on the user's system.
  6. Verify that `active_downloads_count == 0` so a binary is never replaced while in use.
  7. Back up the current working binary in `%LOCALAPPDATA%\FlowDL\bin\yt-dlp.exe` to `yt-dlp.exe.bak`.
  8. Atomically move the verified staged binary into `%LOCALAPPDATA%\FlowDL\bin\yt-dlp.exe`.
  9. Run a post-activation health check (`EngineManager::health_check`).
  10. If any step fails, `EngineManager::rollback` automatically restores `yt-dlp.exe.bak` so the user's existing working setup is never broken.

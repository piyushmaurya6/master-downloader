# FlowDL Release Process & Signing Guide

FlowDL uses **Semantic Versioning (`MAJOR.MINOR.PATCH`)** and automated **GitHub Actions** (`.github/workflows/release.yml`) to build, verify, sign, and publish Windows releases.

---

## 1. Semantic Versioning Policy

- **`1.0.0`**: Initial production release
- **`1.0.x` (PATCH)**: Bug fixes, extractor compatibility tweaks, and security patches
- **`1.x.0` (MINOR)**: New backward-compatible features (e.g., batch URL import, presets)
- **`2.0.0` (MAJOR)**: Breaking architectural or configuration changes (must include SQLite schema migration)

---

## 2. Configuring Update Signing Keys (Maintainer Setup)

To enable cryptographic signature verification for Tauri updater packages:

1. Generate a Tauri Ed25519 keypair locally (never commit the private key):
   ```powershell
   npx tauri signer generate -w ~/.tauri/flowdl.key
   ```
2. Copy the **public key** string into `src-tauri/tauri.conf.json` under `plugins.updater.pubkey` (or set it before tagging a release).
3. In your GitHub repository (`Settings -> Secrets and variables -> Actions`), add:
   - `TAURI_SIGNING_PRIVATE_KEY`: Contents of `~/.tauri/flowdl.key`
   - `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: Password chosen during key generation
4. **Optional Windows Authenticode Signing**:
   - If you have an OV/EV Windows Code Signing Certificate (`.pfx`), add `WINDOWS_CERTIFICATE` (base64 PFX) and `WINDOWS_CERTIFICATE_PASSWORD` to GitHub Actions secrets.
   - Without an Authenticode certificate, Windows SmartScreen will display an "Unknown Publisher" prompt on first launch until the installer builds reputation.

### Key Backup & Rotation
- Store a backup of `~/.tauri/flowdl.key` in an offline password manager.
- If rotating keys, publish one transitional release signed with the old key that contains the new public key in `tauri.conf.json`.

---

## 3. Publishing a Release

1. Update the version in:
   - `package.json`
   - `src-tauri/Cargo.toml`
   - `src-tauri/tauri.conf.json`
2. Update `CHANGELOG.md` with the new version header and release notes.
3. Commit and push to `main`:
   ```powershell
   git add package.json src-tauri/Cargo.toml src-tauri/tauri.conf.json CHANGELOG.md
   git commit -m "chore(release): v1.0.0"
   git tag v1.0.0
   git push origin main --tags
   ```
4. The `.github/workflows/release.yml` workflow automatically:
   - Validates version consistency across `package.json`, `Cargo.toml`, and `tauri.conf.json`
   - Runs `npm run typecheck`, `npm run lint`, `npm run test`, and `cargo test`
   - Downloads and verifies official `yt-dlp`, `FFmpeg`/`ffprobe`, and `Deno` binaries
   - Builds the self-contained Windows NSIS installer (`FlowDL_<version>_x64-setup.exe`)
   - Generates `.sha256` checksum files, `latest.json` update metadata, and SPDX SBOM (`FlowDL_<version>_sbom.spdx.json`)
   - Creates the GitHub Release and attaches all artifacts.

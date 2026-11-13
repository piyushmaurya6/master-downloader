# How to Upload Everything to GitHub (Complete Step-by-Step Guide)

This ZIP bundle embeds **everything**:
- Full source code for **Master Downloader** (`master-downloader/`) + **FlowDL**, **InstaFlow**, and **FlowFB** (`standalone-apps/`)
- Bundled sidecar engines (`master-downloader/src-tauri/resources/bin/`)
- All 4 commercial Windows installers (`releases/*_x64-setup.exe`), standalone `.exe` apps, and `SHA256SUMS.txt`
- Complete user/developer documentation (`docs/`), helper scripts (`scripts/`), and GitHub Actions CI/CD (`.github/workflows/build-and-release.yml`)

---

## Important GitHub File-Size Rules (Read First!)

GitHub enforces strict per-file limits depending on **where** you upload files:
1. **GitHub Web Browser File Upload (`Add file -> Upload files`)**: Maximum **25 MB per file** (cannot upload `.exe` installers via the code tab in a browser).
2. **Standard `git push` (without Git LFS)**: Maximum **100 MB per file** (`Master_Downloader_1.0.0_x64-setup.exe` is `129.53 MB` and `avcodec-63.dll` is `115 MB`, so a plain `git push` without LFS or exclusion will be rejected by GitHub).
3. **GitHub Releases (`Releases -> Draft a new release`)**: Supports up to **2,000 MB (2 GB) per file** for **FREE** with unlimited public downloads! This is the industry-standard place on GitHub for `.exe` installers and full `.zip` bundles.
4. **Git LFS (`git lfs`)**: Allows storing `>100 MB` `.exe`/`.dll` files directly inside your Git repository commits (GitHub's free tier includes 1 GB LFS storage/bandwidth).

Below are **3 ready-to-use methods** — pick whichever you prefer!

---

## Method 1 (Recommended & Fastest): 1-Command Automated Script (`PUSH_TO_GITHUB.ps1`)

We included an automated PowerShell uploader right in the root of this unzipped folder.

### Steps:
1. Extract `Master_Downloader_GitHub_Complete_v1.0.0.zip` to any folder (for example, `D:\Master-Downloader-GitHub`).
2. Create a **new empty repository** on GitHub: https://github.com/new (for example, `master-downloader` — leave *"Add a README"* unchecked).
3. Open **PowerShell** inside the extracted folder and run:

```powershell
Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass
.\PUSH_TO_GITHUB.ps1 -RepoUrl "https://github.com/YOUR_USERNAME/master-downloader.git"
```

### What `PUSH_TO_GITHUB.ps1` does automatically:
- Initializes Git (`main` branch), commits all source code, documentation, scripts, and GitHub Actions workflows, and pushes them cleanly to GitHub without triggering GitHub's 100 MB blob error.
- If **GitHub CLI (`gh`)** is installed and signed in (`gh auth login`), it **automatically creates GitHub Release `v1.0.0`** and uploads all installers in `releases/` (`Master_Downloader_1.0.0_x64-setup.exe`, `FlowDL_1.0.0_x64-setup.exe`, `InstaFlow_1.0.0_x64-setup.exe`, `FlowFB_1.0.0_x64-setup.exe`, standalone `.exe`s, and `SHA256SUMS.txt`) directly to your repository's **Releases** page!
- If `gh` is not installed, it automatically opens your new repository's **`/releases/new`** page in your browser and opens your local `releases\` folder in Explorer so you can drag-and-drop the `.exe` installers in 5 seconds!

> **Want every `.exe` and `.dll` stored directly inside the Git branch via Git LFS too?**
> Run:
> ```powershell
> .\PUSH_TO_GITHUB.ps1 -RepoUrl "https://github.com/YOUR_USERNAME/master-downloader.git" -UseGitLfs
> ```

---

## Method 2 (Zero Command Line): GitHub Desktop + Browser Releases Drag & Drop

If you prefer visual tools without typing Git commands:

1. **Extract** `Master_Downloader_GitHub_Complete_v1.0.0.zip` to a folder on your PC.
2. Temporarily move the `releases` folder and `master-downloader\src-tauri\resources\bin` folder to your Desktop (so every file in the code folder is under 1 MB!).
3. Open **GitHub Desktop** -> `File` -> `Add local repository...` -> select the extracted folder -> click **create a repository** -> click **Publish repository** to push all source code, instructions, and workflows to GitHub.
4. On your GitHub repository page in your browser, click **Releases** (right sidebar) -> **Create a new release**:
   - **Tag**: `v1.0.0`
   - **Release title**: `Master Downloader Suite v1.0.0 — Commercial Windows x64 Release`
   - **Description**: Copy & paste the contents of `releases\RELEASE_NOTES_v1.0.0.md`.
   - **Attach binaries**: Drag & drop all files from `releases\` (`Master_Downloader_1.0.0_x64-setup.exe`, `FlowDL_1.0.0_x64-setup.exe`, `InstaFlow_1.0.0_x64-setup.exe`, `FlowFB_1.0.0_x64-setup.exe`, `SHA256SUMS.txt`) — and you can even drag & drop `Master_Downloader_GitHub_Complete_v1.0.0.zip` itself (< 2 GB)!
   - Click **Publish release**.

*(Note: Anyone cloning your repo later can restore `resources\bin` in one command by running `.\scripts\setup-sidecars.ps1`, and your `.github\workflows\build-and-release.yml` automatically downloads fresh sidecars on GitHub's cloud servers whenever you create future tags!)*

---

## Method 3 (Manual Git CLI + Git LFS for Full Binary Tracking)

Because `.gitattributes` is already pre-configured in this folder for `*.exe` and `*.dll`:

```powershell
cd path\to\extracted\Master_Downloader_GitHub_Complete_v1.0.0
git init -b main
git lfs install
git add .
git commit -m "feat: initial release of Master Downloader Suite v1.0.0 (YouTube, Instagram, Facebook)"
git remote add origin https://github.com/YOUR_USERNAME/master-downloader.git
git push -u origin main
```

Then publish the downloadable installers on your GitHub **Releases** tab (`gh release create v1.0.0 .\releases\*.exe .\releases\SHA256SUMS.txt --notes-file .\releases\RELEASE_NOTES_v1.0.0.md` or via the browser UI).

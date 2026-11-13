# Master Downloader v1.0.0 — Complete End-User Manual

## 1. Installation & Uninstallation
1. Double-click **`Master_Downloader_1.0.0_x64-setup.exe`** (inside `releases/`).
2. Choose **Install for me only** (no administrator/UAC prompt required) or **Install for anyone using this computer**.
3. If your Windows 10 PC is missing the Microsoft Edge WebView2 Runtime, the installer silently provisions it automatically.
4. **First Launch (`< 2ms` Setup)**: All media engines (`yt-dlp`, `gallery-dl`, `ffmpeg`, `ffprobe`, and `deno`) are pre-bundled and provisioned via zero-copy NTFS hardlinks into `%LOCALAPPDATA%\com.masterdownloader.desktop\bin` — no separate Python, FFmpeg, or command-line setup is ever needed.
5. **Clean Uninstall**: Uninstall anytime via **Windows Settings -> Apps -> Installed Apps -> Master Downloader**.

---

## 2. Downloading Media: Two Workflows

### Workflow A: Zero-Click Auto-Download (`Auto-DL: ON`)
- Toggle **`Auto-DL: ON`** in the top Smart Omnibar (enabled by default) and keep **Clipboard Monitor** active in Settings.
- Simply **copy (`Ctrl+C`)** or **paste** any YouTube, Instagram, or Facebook link (or a multi-line batch of mixed links!).
- Master Downloader automatically:
  1. Classifies the platform and strips tracking parameters (`?si=`, `?igsh=`, `?fbclid=`, YouTube `&list=RD...` Infinite Radio Mixes).
  2. Suppresses duplicate active jobs so copying the same link twice never downloads twice.
  3. Enqueues the download using your preferred default resolution (`Best Available`, `2160p 4K`, `1080p`, `720p`, or `Audio Only`), container (`MP4`, `MKV`, `WebM`), and audio normalization settings.
  4. Saves files into clean platform/creator subfolders (`Downloads\MasterDownloader\{Platform}\{Creator}\...`).

### Workflow B: Granular Stream & Carousel Picker (`Inspect`)
- Click **`Inspect`** on any pasted URL (or switch `Auto-DL: OFF`):
  - **YouTube**: Choose exact video resolution/FPS/HDR (`AV1`, `VP9`, `H.264`, `HEVC`), toggle between **Single Video** and **Full Playlist**, embed subtitles/chapters, or extract pure audio (`MP3`, `M4A`, `FLAC`, `Opus`, `WAV`).
  - **Instagram Carousels**: Preview every photo/video slide in a multi-slide post and select all slides or specific slides (`1, 3, 5`).
  - **Facebook Watch / Reels / Albums**: Inspect HD vs. SD streams or multi-photo post attachments before queuing.
  - **EBU R128 Audio Loudness**: Toggle broadcast-standard `-16 LUFS` loudness normalization (`loudnorm=I=-16:TP=-1.5:LRA=11`) so quiet Reels and loud music videos play at uniform volume.

---

## 3. Private Content, Stories & Age-Gated Posts (Session Vault)
Public YouTube videos, public Instagram Reels/Carousels, and public Facebook Watch/Reel links work **out of the box without logging in**.

For **Instagram Stories/Highlights**, **private followed accounts**, or **login-gated Facebook posts**:
1. Open the **Accounts / Session Vault** tab in the left sidebar.
2. Choose one of three authentication methods:
   - **Built-in Isolated Login Window**: Sign in directly to Instagram or Facebook; session cookies (`sessionid`/`ds_user_id` or `c_user`/`xs`) are captured and encrypted on disk with **AES-256-GCM** + **Windows Credential Manager**.
   - **Browser Cookie Delegation**: Delegate live cookies from **Firefox**, **Edge**, **Chrome**, or **Brave**.
   - **Import `cookies.txt`**: Load a standard Netscape `cookies.txt` file.
3. Click **Clear Vault** at any time to wipe stored credentials from both disk and Windows Credential Manager.

---

## 4. Queue Controls, History & Self-Updating Engines
- **Pause / Resume / Cancel**: Pausing kills only that download's subprocess tree immediately and preserves `.part` data so clicking **Resume** continues from the exact downloaded byte offset.
- **History & Media Preview**: Preview completed videos/images directly inside the app, click **Open File** to launch in your default media player, or click **Show in Folder** to highlight the file in Windows Explorer.
- **Keep Engines Updated**: In **Settings -> Engine Binaries**, click **Check & Update Engines** whenever YouTube, Instagram, or Facebook changes site layouts — Master Downloader downloads the latest official `yt-dlp.exe` and `gallery-dl.exe` in place with automatic rollback protection.

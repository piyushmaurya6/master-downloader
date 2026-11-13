# FlowFB System Architecture

## 1. Dual-Engine Facebook Media Pipeline

FlowFB orchestrates two complementary standalone media engines managed in `%LOCALAPPDATA%\FlowFB\bin`:

- **Primary Engine (`yt-dlp.exe`):**
  - Handles all Facebook Videos, Reels (`/reel/*`), Watch streams (`/watch/?v=*`), Page videos (`/{page}/videos/*`), `fb.watch/*` shortlinks, and `/share/{v|r}/*` links.
  - Queries Facebook's Tahoe/GraphQL video endpoints and extracts split DASH video (`avc1`/`vp09`/`av01`) and audio (`mp4a`) streams up to 4K (2160p) and 1080p60.
  - Invokes bundled `ffmpeg.exe` to losslessly remux video and audio streams into `.mp4` or extract `.mp3`/`.m4a` audio.
- **Secondary Engine (`gallery-dl.exe`):**
  - Automatically invoked when a Facebook URL points to a photo post (`/photo/?fbid=*`), media set (`/media/set/?set=*`), or when `yt-dlp` returns `No video formats found` on a photo/multi-image post.

## 2. Pre-Flight Redirect Resolver

Modern Facebook mobile/desktop "Copy Link" actions generate opaque share URLs (`https://www.facebook.com/share/r/{token}/`, `/share/v/{token}/`, or `https://fb.watch/{token}/`). Before invoking the extraction engine, `url_normalizer::resolve_facebook_share_redirect` performs a fast HTTP `HEAD`/`GET` pre-flight check with desktop headers to resolve the `301`/`302` redirect location or `og:url` meta tag into its canonical destination (`/reel/{id}` or `/watch/?v={id}`).

## 3. Process Supervision & Zero Console Flashing

Every child process spawned by FlowFB (`yt-dlp.exe`, `gallery-dl.exe`, `ffmpeg.exe`, `ffprobe.exe`) is configured with the Windows Win32 creation flag `CREATE_NO_WINDOW (0x08000000)` and registered in `ProcessRegistry` so active downloads can be paused, cancelled, or cleaned up on application exit without orphaned processes.

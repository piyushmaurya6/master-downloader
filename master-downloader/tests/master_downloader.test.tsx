import { describe, expect, it } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import {
  classifyMediaUrlClient,
  formatBytes,
  formatCount,
  formatDuration,
  formatSpeed,
} from "../src/lib/utils";
import { useQueueStore } from "../src/stores/useQueueStore";
import { useSettingsStore } from "../src/stores/useSettingsStore";
import { useUIStore } from "../src/stores/useUIStore";
import { SmartOmnibar } from "../src/components/omnibar/SmartOmnibar";
import { MediaInspectorCard } from "../src/components/inspector/MediaInspectorCard";
import { ActiveQueueView } from "../src/components/queue/ActiveQueueView";
import { AuthVaultModal } from "../src/components/auth/AuthVaultModal";

describe("Master Downloader Universal Client URL Classifier", () => {
  it("auto-detects YouTube Videos, Shorts, Playlists, and Music", () => {
    const watch = classifyMediaUrlClient(
      "https://www.youtube.com/watch?v=dQw4w9WgXcQ"
    );
    expect(watch.valid).toBe(true);
    expect(watch.platform).toBe("youtube");
    expect(watch.type).toBe("video");
    expect(watch.badgeLabel).toBe("YOUTUBE");

    const shorts = classifyMediaUrlClient(
      "https://youtube.com/shorts/abc123XYZ"
    );
    expect(shorts.valid).toBe(true);
    expect(shorts.platform).toBe("youtube");
    expect(shorts.type).toBe("shorts");
    expect(shorts.badgeLabel).toBe("YT SHORTS");

    const playlist = classifyMediaUrlClient(
      "https://www.youtube.com/playlist?list=PL123456789"
    );
    expect(playlist.valid).toBe(true);
    expect(playlist.platform).toBe("youtube");
    expect(playlist.type).toBe("playlist");
    expect(playlist.badgeLabel).toBe("YT PLAYLIST");

    const music = classifyMediaUrlClient(
      "https://music.youtube.com/watch?v=dQw4w9WgXcQ"
    );
    expect(music.valid).toBe(true);
    expect(music.platform).toBe("youtube");
    expect(music.type).toBe("music");
    expect(music.badgeLabel).toBe("YT MUSIC");
  });

  it("auto-detects Instagram Reels, Posts, Carousels, Stories, Highlights, and Profiles", () => {
    const reel = classifyMediaUrlClient(
      "https://www.instagram.com/reel/C8abc123/"
    );
    expect(reel.valid).toBe(true);
    expect(reel.platform).toBe("instagram");
    expect(reel.type).toBe("reel");
    expect(reel.badgeLabel).toBe("IG REEL");

    const post = classifyMediaUrlClient(
      "https://www.instagram.com/p/D9xyz789/?img_index=2"
    );
    expect(post.valid).toBe(true);
    expect(post.platform).toBe("instagram");
    expect(post.type).toBe("post");
    expect(post.badgeLabel).toBe("IG POST");

    const story = classifyMediaUrlClient(
      "https://www.instagram.com/stories/natgeo/123456789/"
    );
    expect(story.valid).toBe(true);
    expect(story.platform).toBe("instagram");
    expect(story.type).toBe("stories");
    expect(story.badgeLabel).toBe("IG STORY");

    const highlight = classifyMediaUrlClient(
      "https://www.instagram.com/stories/highlights/179000000/"
    );
    expect(highlight.valid).toBe(true);
    expect(highlight.platform).toBe("instagram");
    expect(highlight.type).toBe("highlight");
    expect(highlight.badgeLabel).toBe("IG HIGHLIGHT");
  });

  it("auto-detects Facebook Watch, Reels, Shortlinks, Share tokens, and Photos", () => {
    const watch = classifyMediaUrlClient(
      "https://www.facebook.com/watch/?v=9876543210"
    );
    expect(watch.valid).toBe(true);
    expect(watch.platform).toBe("facebook");
    expect(watch.type).toBe("watch");
    expect(watch.badgeLabel).toBe("FB WATCH");

    const shortlink = classifyMediaUrlClient("https://fb.watch/xYz123AbC/");
    expect(shortlink.valid).toBe(true);
    expect(shortlink.platform).toBe("facebook");
    expect(shortlink.type).toBe("shortlink");
    expect(shortlink.badgeLabel).toBe("FB.WATCH");

    const share = classifyMediaUrlClient(
      "https://www.facebook.com/share/r/1AbCdEfGh/"
    );
    expect(share.valid).toBe(true);
    expect(share.platform).toBe("facebook");
    expect(share.type).toBe("share");
    expect(share.badgeLabel).toBe("FB SHARE");
  });
});

describe("Formatting Utilities & Stores", () => {
  it("formats bytes, speeds, durations, and counts accurately", () => {
    expect(formatBytes(1048576)).toBe("1.0 MB");
    expect(formatSpeed(2097152)).toBe("2.0 MB/s");
    expect(formatDuration(125)).toBe("2:05");
    expect(formatCount(1500000)).toBe("1.5M");
  });

  it("manages carousel slide selection and queue progress updates", () => {
    useUIStore.getState().setInspectedMedia(
      {
        source_url: "https://www.instagram.com/p/C8abc123/",
        platform: "instagram",
        shortcode: "C8abc123",
        url_type: "carousel",
        author_username: "natgeo",
        author_full_name: "National Geographic",
        caption: "Sample Carousel",
        timestamp: null,
        like_count: 1000,
        view_count: null,
        comment_count: null,
        slide_count: 3,
        slides: [
          {
            index: 1,
            media_type: "image",
            thumbnail_url: null,
            direct_url: null,
            width: 1080,
            height: 1080,
            duration_seconds: null,
            filesize_approx: null,
          },
          {
            index: 2,
            media_type: "video",
            thumbnail_url: null,
            direct_url: null,
            width: 1080,
            height: 1920,
            duration_seconds: 15,
            filesize_approx: null,
          },
          {
            index: 3,
            media_type: "image",
            thumbnail_url: null,
            direct_url: null,
            width: 1080,
            height: 1080,
            duration_seconds: null,
            filesize_approx: null,
          },
        ],
        available_qualities: ["best", "1080p"],
        formats: [],
        subtitles: [],
        chapters: [],
        is_playlist: false,
        playlist_title: null,
        playlist_count: null,
        playlist_entries: [],
        engine_used: "instagram-graphql",
      },
      2
    );

    expect(useUIStore.getState().selectedSlides).toEqual([2]);
    useUIStore.getState().selectAllSlides();
    expect(useUIStore.getState().selectedSlides).toEqual([1, 2, 3]);
    useUIStore.getState().selectSlidesByType("image");
    expect(useUIStore.getState().selectedSlides).toEqual([1, 3]);

    useQueueStore.getState().setJobs([
      {
        id: "job-1",
        source_url: "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
        platform: "youtube",
        shortcode: "dQw4w9WgXcQ",
        url_type: "video",
        author_username: "RickAstley",
        caption: "Never Gonna Give You Up",
        thumbnail_url: null,
        status: "running",
        selected_indices: [1],
        quality_preset: "4k",
        output_dir: "C:\\Downloads",
        error_message: null,
        save_caption: false,
        save_metadata: false,
        embed_thumbnail: true,
        naming_template: null,
        options: {},
        progress_percent: 10,
        downloaded_bytes: 100,
        total_bytes: 1000,
        created_at: 1,
        completed_at: null,
        media_files: [],
      },
    ]);

    useQueueStore.getState().applyProgressTelemetry({
      job_id: "job-1",
      slide_index: 1,
      status: "downloading",
      downloaded_bytes: 500,
      total_bytes: 1000,
      speed_bps: 250000,
      eta_seconds: 2,
      percentage: 50,
    });

    expect(useQueueStore.getState().jobs[0].progress_percent).toBe(50);
    expect(useQueueStore.getState().getAggregateSpeedBps()).toBe(250000);
  });
});

describe("Master Downloader Converged UI Components", () => {
  it("renders SmartOmnibar with live platform badge & Auto-DL toggle", () => {
    let autoDownloadedUrl = "";
    useUIStore.getState().setOmnibarUrl("https://www.youtube.com/watch?v=dQw4w9WgXcQ");
    useSettingsStore.getState().setSettings({
      output_dir: "C:\\Downloads\\MasterDownloader",
      default_quality: "best",
      default_container: "mp4",
      default_audio_format: "mp3",
      default_audio_bitrate: "320",
      audio_normalization: false,
      concurrency: 2,
      jitter_min_ms: 1200,
      jitter_max_ms: 3000,
      speed_limit_kbps: null,
      clipboard_monitor: true,
      auto_download_on_detect: true,
      organize_by_platform: true,
      organize_by_uploader: true,
      embed_thumbnail: true,
      save_caption_sidecar: false,
      save_metadata_json: false,
      write_subtitles: false,
      embed_chapters: true,
      naming_template: "{username}_{date}_{shortcode}_{index}",
      browser_cookie_source: null,
    });

    render(
      <SmartOmnibar
        onAnalyze={() => {}}
        onAutoDownload={(u) => {
          autoDownloadedUrl = u;
        }}
      />
    );

    expect(screen.getByText("YOUTUBE")).toBeDefined();
    expect(screen.getByText("Auto-DL: ON")).toBeDefined();

    fireEvent.click(screen.getByText("Auto-Download"));
    expect(autoDownloadedUrl).toBe("https://www.youtube.com/watch?v=dQw4w9WgXcQ");
  });

  it("renders MediaInspectorCard with Playlist Mode & Advanced Codec Controls", () => {
    useUIStore.getState().setInspectedMedia({
      source_url: "https://www.youtube.com/playlist?list=PL12345",
      platform: "youtube",
      shortcode: "PL12345",
      url_type: "playlist",
      author_username: "TechChannel",
      author_full_name: "Tech Channel",
      caption: "Rust & Tauri Masterclass",
      timestamp: null,
      like_count: 5000,
      view_count: 120000,
      comment_count: null,
      slide_count: 1,
      slides: [
        {
          index: 1,
          media_type: "video",
          thumbnail_url: null,
          direct_url: null,
          width: 3840,
          height: 2160,
          duration_seconds: 600,
          filesize_approx: null,
        },
      ],
      available_qualities: ["best", "4k", "1080p60", "audio_mp3"],
      formats: [],
      subtitles: [],
      chapters: [],
      is_playlist: true,
      playlist_title: "Rust & Tauri Masterclass",
      playlist_count: 12,
      playlist_entries: [
        {
          index: 1,
          id: "vid1",
          title: "Episode 1: Architecture",
          duration_seconds: 600,
          uploader: "TechChannel",
        },
      ],
      engine_used: "yt-dlp",
    });

    render(<MediaInspectorCard />);
    expect(screen.getByText(/Playlist Detected:/)).toBeDefined();
    expect(screen.getByText("Full Playlist")).toBeDefined();
    expect(screen.getByText("4K UHD (2160p)")).toBeDefined();
  });

  it("renders ActiveQueueView and Multi-Platform AuthVaultModal", () => {
    useUIStore.getState().setAuthModalOpen(true);
    render(
      <>
        <ActiveQueueView />
        <AuthVaultModal />
      </>
    );

    expect(screen.getByText("Login to Instagram")).toBeDefined();
    expect(screen.getByText("Login to Facebook")).toBeDefined();
  });
});

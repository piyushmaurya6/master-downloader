import { describe, expect, it, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { classifyInstagramUrlClient, formatBytes, formatEta } from "../src/lib/utils";
import { useUIStore } from "../src/stores/useUIStore";
import { useQueueStore } from "../src/stores/useQueueStore";
import { SmartOmnibar } from "../src/components/omnibar/SmartOmnibar";
import { MediaInspectorCard } from "../src/components/inspector/MediaInspectorCard";
import type { MediaInspectionResult } from "../src/types";

describe("InstaFlow URL Classifier & Formatting Utilities", () => {
  it("classifies Instagram Reels, Posts, Stories, Highlights, and Profiles accurately", () => {
    const reel = classifyInstagramUrlClient(
      "https://www.instagram.com/reel/C8abc_123/?igsh=MzRlODBiNWFlZA=="
    );
    expect(reel.valid).toBe(true);
    expect(reel.badge).toBe("REEL");
    expect(reel.canonicalUrl).toBe("https://www.instagram.com/reel/C8abc_123/");

    const postWithHint = classifyInstagramUrlClient(
      "https://www.instagram.com/natgeo/p/C9xyz987/?img_index=3"
    );
    expect(postWithHint.valid).toBe(true);
    expect(postWithHint.badge).toBe("POST");
    expect(postWithHint.imgIndex).toBe(3);

    const story = classifyInstagramUrlClient(
      "https://www.instagram.com/stories/design.daily/33918273618273/"
    );
    expect(story.valid).toBe(true);
    expect(story.badge).toBe("STORY");

    const highlight = classifyInstagramUrlClient(
      "https://www.instagram.com/stories/highlights/17945829102938/"
    );
    expect(highlight.valid).toBe(true);
    expect(highlight.badge).toBe("HIGHLIGHT");

    const invalid = classifyInstagramUrlClient("https://example.com/reel/123");
    expect(invalid.valid).toBe(false);
  });

  it("formats byte sizes and ETAs cleanly", () => {
    expect(formatBytes(0)).toBe("0 B");
    expect(formatBytes(1048576)).toBe("1.0 MB");
    expect(formatEta(65)).toBe("01:05");
  });
});

describe("Carousel Slide Selection & Media Inspector Logic", () => {
  const mockCarousel: MediaInspectionResult = {
    canonical_url: "https://www.instagram.com/p/C9Mixed123/",
    shortcode: "C9Mixed123",
    url_type: "carousel",
    type_badge: "CAROUSEL",
    owner_username: "studio.design",
    owner_full_name: "Studio Design",
    caption: "Mixed photo and video carousel post",
    timestamp: 1730000000,
    thumbnail_url: "data:image/jpeg;base64,AA==",
    total_items: 3,
    slides: [
      {
        index: 1,
        media_type: "image",
        width: 1080,
        height: 1350,
        display_url: "data:image/jpeg;base64,AA==",
        video_url: null,
        duration_seconds: null,
        ext: "jpg",
      },
      {
        index: 2,
        media_type: "video",
        width: 1080,
        height: 1920,
        display_url: "data:image/jpeg;base64,AA==",
        video_url: "https://cdn.instagram.com/v2.mp4",
        duration_seconds: 12.5,
        ext: "mp4",
      },
      {
        index: 3,
        media_type: "image",
        width: 1080,
        height: 1350,
        display_url: "data:image/jpeg;base64,AA==",
        video_url: null,
        duration_seconds: null,
        ext: "jpg",
      },
    ],
    available_qualities: ["best", "1080p", "720p", "audio_only"],
  };

  beforeEach(() => {
    useUIStore.getState().setInspectedMedia(null);
  });

  it("filters Photos Only, Videos Only, and individual slide toggles", () => {
    const store = useUIStore.getState();
    store.setInspectedMedia(mockCarousel);
    expect(useUIStore.getState().selectedSlideIndices).toEqual([1, 2, 3]);

    useUIStore.getState().selectPhotosOnly();
    expect(useUIStore.getState().selectedSlideIndices).toEqual([1, 3]);

    useUIStore.getState().selectVideosOnly();
    expect(useUIStore.getState().selectedSlideIndices).toEqual([2]);

    useUIStore.getState().selectAllSlides();
    expect(useUIStore.getState().selectedSlideIndices).toEqual([1, 2, 3]);

    useUIStore.getState().toggleSlideIndex(2);
    expect(useUIStore.getState().selectedSlideIndices).toEqual([1, 3]);
  });

  it("renders SmartOmnibar and MediaInspectorCard with interactive slide controls", () => {
    useUIStore.getState().setOmnibarUrl("https://www.instagram.com/reel/C8abc_123/");
    useUIStore.getState().setInspectedMedia(mockCarousel);

    render(
      <div>
        <SmartOmnibar onAnalyzeUrl={async () => {}} />
        <MediaInspectorCard />
      </div>
    );

    expect(screen.getByText("REEL")).toBeInTheDocument();
    expect(screen.getByText("@studio.design")).toBeInTheDocument();
    expect(screen.getByText("Photos Only")).toBeInTheDocument();

    fireEvent.click(screen.getByText("Videos Only"));
    expect(screen.getByText(/Download Selected \(1 Item\)/i)).toBeInTheDocument();
  });
});

describe("Queue State Transitions & Real-Time Telemetry Aggregation", () => {
  beforeEach(() => {
    useQueueStore.getState().setJobs([]);
    useQueueStore.getState().setActiveFilter("all");
  });

  it("transitions job states and computes aggregate download speed", () => {
    useQueueStore.getState().upsertJob({
      id: "job-alpha",
      source_url: "https://www.instagram.com/reel/Alpha1/",
      shortcode: "Alpha1",
      url_type: "reel",
      author_username: "creator1",
      caption: "First reel",
      thumbnail_url: null,
      status: "queued",
      selected_indices: [1],
      quality_preset: "best",
      output_dir: "C:\\Downloads",
      naming_template: "{username}/{shortcode}.{ext}",
      save_caption: false,
      save_metadata: false,
      embed_thumbnail: true,
      error_message: null,
      progress_percent: 0,
      downloaded_bytes: 0,
      total_bytes: null,
      created_at: 1730000000,
      completed_at: null,
      media_files: [],
    });

    useQueueStore.getState().applyProgressTelemetry({
      job_id: "job-alpha",
      item_index: 1,
      total_items: 1,
      status: "downloading_video",
      stage_label: "Downloading video stream...",
      percentage: 55.0,
      downloaded_bytes: 5_500_000,
      total_bytes: 10_000_000,
      speed_bps: 4_200_000,
      eta_seconds: 1,
    });

    const runningJob = useQueueStore.getState().jobs[0];
    expect(runningJob.status).toBe("running");
    expect(runningJob.progress_percent).toBe(55.0);
    expect(useQueueStore.getState().getAggregateSpeedBps()).toBe(4_200_000);

    useQueueStore.getState().applyProgressTelemetry({
      job_id: "job-alpha",
      item_index: 1,
      total_items: 1,
      status: "completed",
      stage_label: "Completed",
      percentage: 100.0,
      downloaded_bytes: 10_000_000,
      total_bytes: 10_000_000,
      speed_bps: null,
      eta_seconds: 0,
    });

    expect(useQueueStore.getState().jobs[0].status).toBe("completed");
  });
});

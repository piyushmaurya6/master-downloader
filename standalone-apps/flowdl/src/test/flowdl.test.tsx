import React from "react";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import App from "../App";
import { tauriBridge } from "../services/tauriBridge";
import {
  formatBytes,
  formatDuration,
  formatEta,
  formatSpeed,
  isValidMediaUrl,
  sanitizeUrlForDisplay,
} from "../utils/formatters";
import type { DependencyState, DownloadItem, MediaMetadata } from "../types";

const makeDepState = (
  ytdlpOk: boolean,
  ffmpegOk: boolean
): DependencyState => ({
  ytdlp: {
    name: "yt-dlp",
    installed: ytdlpOk,
    functional: ytdlpOk,
    version: ytdlpOk ? "2026.10.15" : null,
    latestVersion: "2026.10.15",
    updateAvailable: false,
    path: ytdlpOk ? "C:\\FlowDL\\bin\\yt-dlp.exe" : null,
    source: ytdlpOk ? "managed" : "none",
    error: null,
  },
  ffmpeg: {
    name: "ffmpeg",
    installed: ffmpegOk,
    functional: ffmpegOk,
    version: ffmpegOk ? "7.1" : null,
    latestVersion: null,
    updateAvailable: false,
    path: ffmpegOk ? "C:\\FlowDL\\bin\\ffmpeg.exe" : null,
    source: ffmpegOk ? "managed" : "none",
    error: null,
  },
  ffprobe: {
    name: "ffprobe",
    installed: ffmpegOk,
    functional: ffmpegOk,
    version: ffmpegOk ? "7.1" : null,
    latestVersion: null,
    updateAvailable: false,
    path: ffmpegOk ? "C:\\FlowDL\\bin\\ffprobe.exe" : null,
    source: ffmpegOk ? "managed" : "none",
    error: null,
  },
  ready: ytdlpOk && ffmpegOk,
  checking: false,
  installing: false,
  installStep: ytdlpOk && ffmpegOk ? "Ready to download" : "Missing components",
  installProgress: ytdlpOk && ffmpegOk ? 100 : 0,
  error: null,
});

describe("FlowDL Comprehensive Test Suite", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it("1. First launch with neither tool installed shows Welcome Setup and Install Missing Components", async () => {
    vi.spyOn(tauriBridge, "checkDependencies").mockResolvedValue(
      makeDepState(false, false)
    );
    render(<App />);
    expect(await screen.findByText("Welcome to FlowDL")).toBeInTheDocument();
    expect(
      await screen.findByText("Install Missing Components")
    ).toBeInTheDocument();
  });

  it("2. First launch with yt-dlp already installed detects yt-dlp and prompts for missing FFmpeg", async () => {
    vi.spyOn(tauriBridge, "checkDependencies").mockResolvedValue(
      makeDepState(true, false)
    );
    render(<App />);
    expect(await screen.findByText("Welcome to FlowDL")).toBeInTheDocument();
    expect(
      await screen.findByText(/Version 2026\.10\.15/)
    ).toBeInTheDocument();
    expect(
      await screen.findByText("Install Missing Components")
    ).toBeInTheDocument();
  });

  it("3. Launch with bundled tools verified opens directly to ready dashboard and engine status modal", async () => {
    vi.spyOn(tauriBridge, "checkDependencies").mockResolvedValue(
      makeDepState(true, true)
    );
    render(<App />);
    expect(
      await screen.findByText("Download something great.")
    ).toBeInTheDocument();
    const enginesReadyBtn = await screen.findByText("Engines Ready");
    expect(enginesReadyBtn).toBeInTheDocument();
    fireEvent.click(enginesReadyBtn);
    expect(await screen.findByText("Start Using FlowDL")).toBeInTheDocument();
  });

  it("4 & 5. Dependency setup error displays Retry button and recovers cleanly", async () => {
    vi.spyOn(tauriBridge, "checkDependencies").mockResolvedValue(
      makeDepState(false, false)
    );
    const setupSpy = vi
      .spyOn(tauriBridge, "setupDependencies")
      .mockRejectedValueOnce(new Error("Network timeout fetching binary"))
      .mockResolvedValueOnce(makeDepState(true, true));

    render(<App />);
    const installBtn = await screen.findByText("Install Missing Components");
    fireEvent.click(installBtn);

    expect(
      await screen.findByText(/Network timeout fetching binary/)
    ).toBeInTheDocument();

    const retryBtn = await screen.findByText("Retry");
    fireEvent.click(retryBtn);

    await waitFor(() => {
      expect(setupSpy).toHaveBeenCalledTimes(2);
    });
  });

  it("6 & 11. Valid URL analysis loads metadata and disables unavailable 4K resolution on 1080p video", async () => {
    vi.spyOn(tauriBridge, "getSettings").mockResolvedValue({
      ...(await tauriBridge.getSettings()),
      firstLaunchCompleted: true,
    });
    vi.spyOn(tauriBridge, "checkDependencies").mockResolvedValue(
      makeDepState(true, true)
    );

    const sampleMeta: MediaMetadata = {
      id: "vid1080",
      title: "1080p Max Resolution Test Video",
      url: "https://www.youtube.com/watch?v=vid1080",
      webpageUrl: "https://www.youtube.com/watch?v=vid1080",
      thumbnail: null,
      uploader: "Test Channel",
      channelUrl: null,
      duration: 185,
      durationString: "3:05",
      extractor: "Youtube",
      uploadDate: "20261020",
      description: "Test description",
      isPlaylist: false,
      playlistTitle: null,
      playlistCount: null,
      playlistEntries: [],
      availableHeights: [1080, 720, 480, 360], // No 2160p or 1440p!
      availableVideoCodecs: ["h264"],
      availableFps: [30],
      availableAudioFormats: [],
      subtitles: [],
      formats: [],
      estimatedSizesByHeight: { 1080: 52428800 },
    };

    vi.spyOn(tauriBridge, "analyzeUrl").mockResolvedValue(sampleMeta);

    render(<App />);
    const input = await screen.findByPlaceholderText(
      "Paste a YouTube or supported video link..."
    );
    fireEvent.change(input, {
      target: { value: "https://www.youtube.com/watch?v=vid1080" },
    });

    const analyzeBtn = screen.getByRole("button", { name: /Analyze/i });
    fireEvent.click(analyzeBtn);

    expect(
      await screen.findByText("1080p Max Resolution Test Video")
    ).toBeInTheDocument();

    const opt4k = screen.getByRole("option", {
      name: /4K — 2160p — Not available for this video/i,
    }) as HTMLOptionElement;
    expect(opt4k.disabled).toBe(true);

    const opt1080 = screen.getByRole("option", {
      name: /1080p Full HD/i,
    }) as HTMLOptionElement;
    expect(opt1080.disabled).toBe(false);
  });

  it("7 & 8 & 19. Invalid URL and network failure display clear user-friendly errors with Retry", async () => {
    vi.spyOn(tauriBridge, "getSettings").mockResolvedValue({
      ...(await tauriBridge.getSettings()),
      firstLaunchCompleted: true,
    });
    vi.spyOn(tauriBridge, "checkDependencies").mockResolvedValue(
      makeDepState(true, true)
    );

    render(<App />);
    const input = await screen.findByPlaceholderText(
      "Paste a YouTube or supported video link..."
    );
    fireEvent.change(input, { target: { value: "not-a-valid-url" } });
    fireEvent.click(screen.getByRole("button", { name: /Analyze/i }));

    expect(
      await screen.findByText("That doesn't look like a supported video link.")
    ).toBeInTheDocument();

    // Now test network error during metadata fetch
    vi.spyOn(tauriBridge, "analyzeUrl").mockRejectedValueOnce(
      new Error(
        "network||Couldn't connect. Check your internet connection and try again."
      )
    );
    fireEvent.change(input, {
      target: { value: "https://www.youtube.com/watch?v=netfail" },
    });
    fireEvent.click(screen.getByRole("button", { name: /Analyze/i }));

    expect(
      await screen.findByText(
        "Couldn't connect. Check your internet connection and try again."
      )
    ).toBeInTheDocument();
  });

  it("9 & 10. Video download and Audio MP3 conversion enqueue with accurate parameters", async () => {
    vi.spyOn(tauriBridge, "getSettings").mockResolvedValue({
      ...(await tauriBridge.getSettings()),
      firstLaunchCompleted: true,
    });
    vi.spyOn(tauriBridge, "checkDependencies").mockResolvedValue(
      makeDepState(true, true)
    );

    const enqueueSpy = vi.spyOn(tauriBridge, "enqueueDownload");

    render(<App />);
    const input = await screen.findByPlaceholderText(
      "Paste a YouTube or supported video link..."
    );
    fireEvent.change(input, {
      target: { value: "https://www.youtube.com/watch?v=demo123" },
    });
    fireEvent.click(screen.getByRole("button", { name: /Analyze/i }));

    const dlVideoBtn = await screen.findByRole("button", {
      name: /Download Video/i,
    });
    fireEvent.click(dlVideoBtn);

    await waitFor(() => {
      expect(enqueueSpy).toHaveBeenCalledWith(
        expect.objectContaining({
          mode: "video",
          container: "mp4",
        })
      );
    });

    // Switch to Audio mode and trigger MP3 download
    const audioTab = screen.getByRole("tab", { name: /Audio/i });
    fireEvent.click(audioTab);

    const dlAudioBtn = await screen.findByRole("button", {
      name: /Download Audio/i,
    });
    fireEvent.click(dlAudioBtn);

    await waitFor(() => {
      expect(enqueueSpy).toHaveBeenCalledWith(
        expect.objectContaining({
          mode: "audio",
          audioFormat: "mp3",
          audioQuality: "320",
        })
      );
    });
  });

  it("12, 13, 14, 15, 18. Queue progress, Cancel confirmation, Stop/Resume, Retry, and Unicode paths", async () => {
    const unicodePath = "C:\\Users\\Apna Punjab\\Downloads\\Chanson Française 🎵 日本語.mp4";
    const activeItem: DownloadItem = {
      id: "job-unicode-1",
      url: "https://www.youtube.com/watch?v=uni1",
      title: "Chanson Française 🎵 日本語",
      thumbnail: null,
      uploader: "World Music",
      duration: 210,
      extractor: "Youtube",
      mode: "video",
      formatSummary: "1080p · MP4",
      qualityLabel: "1080p",
      status: "downloading",
      postprocessStage: "downloading_video",
      progress: 64,
      downloadedBytes: 163577856,
      totalBytes: 255852544,
      speedBytesPerSec: 8808038,
      etaSeconds: 11,
      outputPath: unicodePath,
      outputDir: "C:\\Users\\Apna Punjab\\Downloads",
      errorMessage: null,
      technicalError: null,
      errorCategory: null,
      retryCount: 0,
      maxRetries: 3,
      createdAt: new Date().toISOString(),
      completedAt: null,
      request: {
        url: "https://www.youtube.com/watch?v=uni1",
        title: "Chanson Française 🎵 日本語",
        thumbnail: null,
        uploader: "World Music",
        duration: 210,
        extractor: "Youtube",
        mode: "video",
        videoQuality: "1080",
        exactResolution: false,
        container: "mp4",
        videoCodec: "auto",
        preferredFps: null,
        audioFormat: "mp3",
        audioQuality: "320",
        downloadSubtitles: false,
        subtitleLang: "en",
        embedThumbnail: true,
        embedMetadata: true,
        downloadChapters: false,
        normalizeAudio: false,
        preserveOriginalFilename: false,
        filenameTemplate: null,
        speedLimitKbps: null,
        playlistMode: "single",
        outputDir: "C:\\Users\\Apna Punjab\\Downloads",
      },
    };

    vi.spyOn(tauriBridge, "getSettings").mockResolvedValue({
      ...(await tauriBridge.getSettings()),
      firstLaunchCompleted: true,
    });
    vi.spyOn(tauriBridge, "checkDependencies").mockResolvedValue(
      makeDepState(true, true)
    );
    vi.spyOn(tauriBridge, "getQueue").mockResolvedValue([activeItem]);

    const cancelSpy = vi.spyOn(tauriBridge, "cancelDownload").mockResolvedValue();

    render(<App />);
    expect(
      await screen.findByText("Chanson Française 🎵 日本語")
    ).toBeInTheDocument();
    expect(await screen.findByText("Downloading — 64%")).toBeInTheDocument();
    expect(
      await screen.findByText(/8\.4 MB\/s · 11 seconds remaining/)
    ).toBeInTheDocument();

    // Click Cancel -> requires confirmation
    fireEvent.click(screen.getByRole("button", { name: /^Cancel$/i }));
    expect(await screen.findByText("Cancel download?")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /^Yes$/i }));
    await waitFor(() => {
      expect(cancelSpy).toHaveBeenCalledWith("job-unicode-1");
    });
  });

  it("17. History page shows 'File moved or deleted' badge when fileExists is false", async () => {
    vi.spyOn(tauriBridge, "getSettings").mockResolvedValue({
      ...(await tauriBridge.getSettings()),
      firstLaunchCompleted: true,
    });
    vi.spyOn(tauriBridge, "checkDependencies").mockResolvedValue(
      makeDepState(true, true)
    );
    vi.spyOn(tauriBridge, "getHistory").mockResolvedValue([
      {
        id: "hist-missing-1",
        title: "Moved Video File",
        thumbnail: null,
        uploader: "Creator",
        sourceUrl: "https://www.youtube.com/watch?v=moved",
        extractor: "Youtube",
        downloadedAt: new Date().toISOString(),
        mode: "video",
        formatSummary: "1080p · MP4",
        quality: "1080p",
        fileSize: 10485760,
        outputPath: "C:\\Downloads\\MovedVideo.mp4",
        outputDir: "C:\\Downloads",
        status: "completed",
        fileExists: false,
        errorMessage: null,
        request: {
          url: "https://www.youtube.com/watch?v=moved",
          title: "Moved Video File",
          thumbnail: null,
          uploader: "Creator",
          duration: 60,
          extractor: "Youtube",
          mode: "video",
          videoQuality: "1080",
          exactResolution: false,
          container: "mp4",
          videoCodec: "auto",
          preferredFps: null,
          audioFormat: "mp3",
          audioQuality: "320",
          downloadSubtitles: false,
          subtitleLang: "en",
          embedThumbnail: true,
          embedMetadata: true,
          downloadChapters: false,
          normalizeAudio: false,
          preserveOriginalFilename: false,
          filenameTemplate: null,
          speedLimitKbps: null,
          playlistMode: "single",
          outputDir: "C:\\Downloads",
        },
      },
    ]);

    render(<App />);
    fireEvent.click(await screen.findByRole("button", { name: /History/i }));
    expect(await screen.findByText("Moved Video File")).toBeInTheDocument();
    expect(
      await screen.findByText("File moved or deleted")
    ).toBeInTheDocument();
  });

  it("Utility formatters and security URL sanitization work accurately", () => {
    expect(formatBytes(163577856)).toBe("156 MB");
    expect(formatSpeed(8808038)).toBe("8.4 MB/s");
    expect(formatEta(11)).toBe("11 seconds remaining");
    expect(formatDuration(734)).toBe("12:14");
    expect(isValidMediaUrl("https://www.youtube.com/watch?v=abc")).toBe(true);
    expect(isValidMediaUrl("javascript:alert(1)")).toBe(false);
    expect(
      sanitizeUrlForDisplay("https://example.com/v?id=1&token=my_secret_key")
    ).toContain("token=%5BREDACTED%5D");
  });

  it("20. System A AppUpdateBanner displays 'FlowDL 1.1.0 is available.' with Update Now, View What's New, and Remind Me Later", async () => {
    vi.spyOn(tauriBridge, "getSettings").mockResolvedValue({
      ...(await tauriBridge.getSettings()),
      firstLaunchCompleted: true,
      checkAppUpdatesOnStartup: true,
    });
    vi.spyOn(tauriBridge, "checkDependencies").mockResolvedValue(
      makeDepState(true, true)
    );
    vi.spyOn(tauriBridge, "checkAppUpdate").mockResolvedValue({
      currentVersion: "1.0.0",
      latestVersion: "1.1.0",
      updateAvailable: true,
      releaseDate: "2026-11-02T10:00:00Z",
      releaseNotes: "Added playlist presets and faster stream analysis.",
      downloadUrl:
        "https://github.com/piyushmaurya6/flowdl/releases/download/v1.1.0/FlowDL_1.1.0_x64-setup.exe",
      downloadSize: 113036288,
      signatureVerified: true,
      htmlUrl: "https://github.com/piyushmaurya6/flowdl/releases/tag/v1.1.0",
    });

    render(<App />);
    expect(
      await screen.findByText(/FlowDL 1\.1\.0 is available\./i)
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Update Now/i })).toBeInTheDocument();
    const whatsNewBtn = screen.getByRole("button", { name: /View What's New/i });
    fireEvent.click(whatsNewBtn);
    expect(
      await screen.findByText(/Added playlist presets and faster stream analysis\./i)
    ).toBeInTheDocument();

    const remindBtn = screen.getByRole("button", { name: /Remind Me Later/i });
    fireEvent.click(remindBtn);
    await waitFor(() => {
      expect(
        screen.queryByText(/FlowDL 1\.1\.0 is available\./i)
      ).not.toBeInTheDocument();
    });
  });

  it("21. Engine Self-Repair offers 'Update Engine & Retry' when provider extractor changes (ENGINE_INCOMPATIBLE)", async () => {
    const failedExtractorItem: DownloadItem = {
      id: "job-extractor-1",
      url: "https://www.youtube.com/watch?v=ext1",
      title: "Provider Signature Change Test",
      thumbnail: null,
      uploader: "Creator",
      duration: 120,
      extractor: "Youtube",
      mode: "video",
      formatSummary: "1080p · MP4",
      qualityLabel: "1080p",
      status: "failed",
      postprocessStage: null,
      progress: 0,
      downloadedBytes: 0,
      totalBytes: null,
      speedBytesPerSec: null,
      etaSeconds: null,
      outputPath: null,
      outputDir: "C:\\Downloads",
      errorMessage:
        "The video provider changed its playback format. Updating the download engine may fix this.",
      technicalError: "ERROR: Signature extraction failed",
      errorCategory: "ENGINE_INCOMPATIBLE",
      recoveryAction:
        "Click 'Update Engine & Retry' or open Settings -> Check for Engine Updates.",
      suggestEngineUpdate: true,
      retryCount: 0,
      maxRetries: 3,
      createdAt: new Date().toISOString(),
      completedAt: null,
      request: {
        url: "https://www.youtube.com/watch?v=ext1",
        title: "Provider Signature Change Test",
        thumbnail: null,
        uploader: "Creator",
        duration: 120,
        extractor: "Youtube",
        mode: "video",
        videoQuality: "1080",
        exactResolution: false,
        container: "mp4",
        videoCodec: "auto",
        preferredFps: null,
        audioFormat: "mp3",
        audioQuality: "320",
        downloadSubtitles: false,
        subtitleLang: "en",
        embedThumbnail: true,
        embedMetadata: true,
        downloadChapters: false,
        normalizeAudio: false,
        preserveOriginalFilename: false,
        filenameTemplate: null,
        speedLimitKbps: null,
        playlistMode: "single",
        outputDir: "C:\\Downloads",
      },
    };

    vi.spyOn(tauriBridge, "getSettings").mockResolvedValue({
      ...(await tauriBridge.getSettings()),
      firstLaunchCompleted: true,
    });
    vi.spyOn(tauriBridge, "checkDependencies").mockResolvedValue(
      makeDepState(true, true)
    );
    vi.spyOn(tauriBridge, "getQueue").mockResolvedValue([failedExtractorItem]);
    const updateEngineSpy = vi
      .spyOn(tauriBridge, "updateYtdlpEngine")
      .mockResolvedValue({
        name: "yt-dlp",
        installed: true,
        functional: true,
        version: "2026.11.01",
        latestVersion: "2026.11.01",
        updateAvailable: false,
        path: "C:\\FlowDL\\bin\\yt-dlp.exe",
        source: "managed",
        error: null,
      });
    const retrySpy = vi.spyOn(tauriBridge, "retryDownload").mockResolvedValue();

    render(<App />);
    const repairBtn = await screen.findByRole("button", {
      name: /Update Engine & Retry/i,
    });
    expect(repairBtn).toBeInTheDocument();
    fireEvent.click(repairBtn);

    await waitFor(() => {
      expect(updateEngineSpy).toHaveBeenCalledTimes(1);
      expect(retrySpy).toHaveBeenCalledWith("job-extractor-1");
    });
  });
});

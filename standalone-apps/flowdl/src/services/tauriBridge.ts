import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type {
  AppSettings,
  AppUpdateInfo,
  DependencyState,
  DiagnosticLogEntry,
  DiagnosticReport,
  DownloadItem,
  DownloadRequest,
  EngineUpdateInfo,
  HistoryItem,
  MediaMetadata,
  ToolStatus,
} from "../types";

export function isTauriEnvironment(): boolean {
  return (
    typeof window !== "undefined" &&
    ("__TAURI_INTERNALS__" in window || "__TAURI__" in window)
  );
}

const DEFAULT_SETTINGS: AppSettings = {
  firstLaunchCompleted: false,
  preferManagedBinaries: true,
  ytdlpPath: null,
  ffmpegPath: null,
  ffprobePath: null,
  downloadDir: "C:\\Users\\Downloads",
  defaultVideoContainer: "mp4",
  defaultVideoQuality: "1080",
  defaultAudioFormat: "mp3",
  defaultAudioQuality: "320",
  theme: "dark",
  launchOnStartup: false,
  minimizeToTray: false,
  maxConcurrentDownloads: 2,
  filenameTemplate: "%(title)s [%(id)s].%(ext)s",
  createPlaylistFolders: true,
  askBeforeOverwrite: false,
  downloadSubtitlesByDefault: false,
  defaultSubtitleLang: "en",
  preserveIntermediateFiles: false,
  speedLimitKbps: 0,
  retryCount: 3,
  autoUpdateEngines: true,
  clipboardDetectionEnabled: false,
};

let fallbackSettings: AppSettings = { ...DEFAULT_SETTINGS };
let fallbackQueue: DownloadItem[] = [];
let fallbackHistory: HistoryItem[] = [];
let fallbackLogs: DiagnosticLogEntry[] = [
  {
    timestamp: new Date().toISOString(),
    level: "INFO",
    component: "System",
    message: "FlowDL initialized.",
  },
];

export const tauriBridge = {
  async checkDependencies(checkUpdates = false): Promise<DependencyState> {
    if (isTauriEnvironment()) {
      return await invoke<DependencyState>("check_dependencies", { checkUpdates });
    }
    return {
      ytdlp: {
        name: "yt-dlp",
        installed: true,
        functional: true,
        version: "2026.10.15",
        latestVersion: "2026.10.15",
        updateAvailable: false,
        path: "C:\\Users\\AppData\\Local\\FlowDL\\bin\\yt-dlp.exe",
        source: "managed",
        error: null,
      },
      ffmpeg: {
        name: "ffmpeg",
        installed: true,
        functional: true,
        version: "7.1-full_build",
        latestVersion: null,
        updateAvailable: false,
        path: "C:\\Users\\AppData\\Local\\FlowDL\\bin\\ffmpeg.exe",
        source: "managed",
        error: null,
      },
      ffprobe: {
        name: "ffprobe",
        installed: true,
        functional: true,
        version: "7.1-full_build",
        latestVersion: null,
        updateAvailable: false,
        path: "C:\\Users\\AppData\\Local\\FlowDL\\bin\\ffprobe.exe",
        source: "managed",
        error: null,
      },
      ready: true,
      checking: false,
      installing: false,
      installStep: "Ready to download",
      installProgress: 100,
      error: null,
    };
  },

  async setupDependencies(forceDownload = false): Promise<DependencyState> {
    if (isTauriEnvironment()) {
      return await invoke<DependencyState>("setup_dependencies", { forceDownload });
    }
    fallbackSettings.firstLaunchCompleted = true;
    return await this.checkDependencies(false);
  },

  async updateYtdlpEngine(): Promise<ToolStatus> {
    if (isTauriEnvironment()) {
      return await invoke<ToolStatus>("update_ytdlp_engine");
    }
    const state = await this.checkDependencies(false);
    return state.ytdlp;
  },

  async updateFfmpegEngine(): Promise<DependencyState> {
    if (isTauriEnvironment()) {
      return await invoke<DependencyState>("update_ffmpeg_engine");
    }
    return await this.checkDependencies(false);
  },

  async analyzeUrl(url: string): Promise<MediaMetadata> {
    if (isTauriEnvironment()) {
      return await invoke<MediaMetadata>("analyze_url", { url });
    }
    if (!url.startsWith("http://") && !url.startsWith("https://")) {
      throw new Error("invalid_url||That doesn't look like a supported video link.");
    }
    return {
      id: "demo_id",
      title: "Blender Open Movie — Tears of Steel (4K Remaster)",
      url,
      webpageUrl: url,
      thumbnail: "https://images.unsplash.com/photo-1618005182384-a83a8bd57fbe?w=640&q=80",
      uploader: "Blender Foundation",
      channelUrl: "https://www.youtube.com/@BlenderFoundation",
      duration: 734,
      durationString: "12:14",
      extractor: "Youtube",
      uploadDate: "20261012",
      description:
        "Open-source science fiction short film created by the Blender Institute in Amsterdam.",
      isPlaylist: url.includes("list="),
      playlistTitle: url.includes("list=") ? "Blender Open Movies Collection" : null,
      playlistCount: url.includes("list=") ? 5 : null,
      playlistEntries: [],
      availableHeights: [2160, 1440, 1080, 720, 480, 360],
      availableVideoCodecs: ["h264", "vp9", "av1"],
      availableFps: [60, 30],
      availableAudioFormats: [],
      subtitles: [
        { lang: "en", name: "English", isAutoGenerated: false },
        { lang: "es", name: "Spanish", isAutoGenerated: false },
      ],
      formats: [],
      estimatedSizesByHeight: {
        2160: 645922816,
        1440: 312475648,
        1080: 158334976,
        720: 84934656,
        480: 45088768,
        360: 26214400,
      },
    };
  },

  async enqueueDownload(request: DownloadRequest): Promise<DownloadItem> {
    if (isTauriEnvironment()) {
      return await invoke<DownloadItem>("enqueue_download", { request });
    }
    const item: DownloadItem = {
      id: `dl-${Date.now()}`,
      url: request.url,
      title: request.title,
      thumbnail: request.thumbnail,
      uploader: request.uploader,
      duration: request.duration,
      extractor: request.extractor,
      mode: request.mode,
      formatSummary:
        request.mode === "audio"
          ? `${request.audioFormat.toUpperCase()} · ${request.audioQuality === "best" ? "Best" : request.audioQuality + " kbps"}`
          : `${request.videoQuality === "best" ? "Best" : request.videoQuality + "p"} · ${request.container.toUpperCase()}`,
      qualityLabel:
        request.mode === "audio"
          ? request.audioQuality
          : request.videoQuality === "best"
            ? "Best"
            : `${request.videoQuality}p`,
      status: "downloading",
      postprocessStage:
        request.mode === "audio" ? "downloading_audio" : "downloading_video",
      progress: 42,
      downloadedBytes: 66400000,
      totalBytes: 158334976,
      speedBytesPerSec: 8808038,
      etaSeconds: 11,
      outputPath: null,
      outputDir: request.outputDir || fallbackSettings.downloadDir,
      errorMessage: null,
      technicalError: null,
      errorCategory: null,
      retryCount: 0,
      maxRetries: fallbackSettings.retryCount,
      createdAt: new Date().toISOString(),
      completedAt: null,
      request,
    };
    fallbackQueue = [item, ...fallbackQueue];
    return item;
  },

  async getQueue(): Promise<DownloadItem[]> {
    if (isTauriEnvironment()) {
      return await invoke<DownloadItem[]>("get_queue");
    }
    return fallbackQueue;
  },

  async cancelDownload(id: string): Promise<void> {
    if (isTauriEnvironment()) {
      await invoke("cancel_download", { id });
      return;
    }
    fallbackQueue = fallbackQueue.map((i) =>
      i.id === id
        ? { ...i, status: "cancelled", speedBytesPerSec: null, etaSeconds: null }
        : i
    );
  },

  async stopDownload(id: string): Promise<void> {
    if (isTauriEnvironment()) {
      await invoke("stop_download", { id });
      return;
    }
    fallbackQueue = fallbackQueue.map((i) =>
      i.id === id
        ? { ...i, status: "stopped", speedBytesPerSec: null, etaSeconds: null }
        : i
    );
  },

  async resumeDownload(id: string): Promise<void> {
    if (isTauriEnvironment()) {
      await invoke("resume_download", { id });
      return;
    }
    fallbackQueue = fallbackQueue.map((i) =>
      i.id === id ? { ...i, status: "downloading" } : i
    );
  },

  async retryDownload(id: string): Promise<void> {
    if (isTauriEnvironment()) {
      await invoke("retry_download", { id });
      return;
    }
    fallbackQueue = fallbackQueue.map((i) =>
      i.id === id
        ? { ...i, status: "downloading", errorMessage: null, errorCategory: null }
        : i
    );
  },

  async removeDownload(id: string): Promise<void> {
    if (isTauriEnvironment()) {
      await invoke("remove_download", { id });
      return;
    }
    fallbackQueue = fallbackQueue.filter((i) => i.id !== id);
  },

  async clearCompletedDownloads(): Promise<void> {
    if (isTauriEnvironment()) {
      await invoke("clear_completed_downloads");
      return;
    }
    fallbackQueue = fallbackQueue.filter(
      (i) => i.status !== "completed" && i.status !== "cancelled"
    );
  },

  async reorderQueue(id: string, direction: "up" | "down"): Promise<void> {
    if (isTauriEnvironment()) {
      await invoke("reorder_queue", { id, direction });
      return;
    }
  },

  async getHistory(): Promise<HistoryItem[]> {
    if (isTauriEnvironment()) {
      return await invoke<HistoryItem[]>("get_history");
    }
    return fallbackHistory;
  },

  async removeHistoryItem(id: string): Promise<void> {
    if (isTauriEnvironment()) {
      await invoke("remove_history_item", { id });
      return;
    }
    fallbackHistory = fallbackHistory.filter((h) => h.id !== id);
  },

  async clearHistory(): Promise<void> {
    if (isTauriEnvironment()) {
      await invoke("clear_history");
      return;
    }
    fallbackHistory = [];
  },

  async getSettings(): Promise<AppSettings> {
    if (isTauriEnvironment()) {
      return await invoke<AppSettings>("get_settings");
    }
    return fallbackSettings;
  },

  async saveSettings(settings: AppSettings): Promise<AppSettings> {
    if (isTauriEnvironment()) {
      return await invoke<AppSettings>("save_settings", { settings });
    }
    fallbackSettings = { ...settings };
    return fallbackSettings;
  },

  async pickFolder(initialDir?: string): Promise<string | null> {
    if (isTauriEnvironment()) {
      return await invoke<string | null>("pick_folder", { initialDir });
    }
    return initialDir || fallbackSettings.downloadDir;
  },

  async openFile(path: string): Promise<void> {
    if (isTauriEnvironment()) {
      await invoke("open_file", { path });
    }
  },

  async showInFolder(path: string): Promise<void> {
    if (isTauriEnvironment()) {
      await invoke("show_in_folder", { path });
    }
  },

  async getDiagnosticLogs(): Promise<DiagnosticLogEntry[]> {
    if (isTauriEnvironment()) {
      return await invoke<DiagnosticLogEntry[]>("get_diagnostic_logs");
    }
    return fallbackLogs;
  },

  async generateDiagnosticReport(): Promise<DiagnosticReport> {
    if (isTauriEnvironment()) {
      return await invoke<DiagnosticReport>("generate_diagnostic_report");
    }
    return {
      generatedAt: new Date().toISOString(),
      appVersion: "1.0.0",
      osVersion: "windows (x86_64)",
      arch: "x86_64",
      schemaVersion: 2,
      ytdlpVersion: "2026.10.26",
      ffmpegVersion: "7.1-full_build",
      ffprobeVersion: "7.1-full_build",
      denoVersion: "deno 2.9.8",
      recentLogs: fallbackLogs,
      markdownReport:
        "### FlowDL Sanitized Diagnostic Report\n- **FlowDL Version**: `v1.0.0`\n",
    };
  },

  async checkAppUpdate(): Promise<AppUpdateInfo> {
    if (isTauriEnvironment()) {
      return await invoke<AppUpdateInfo>("check_app_update");
    }
    return {
      currentVersion: "1.0.0",
      latestVersion: "1.0.0",
      updateAvailable: false,
      releaseDate: null,
      releaseNotes: null,
      downloadUrl: null,
      downloadSize: null,
      signatureVerified: true,
      htmlUrl: "https://github.com/piyushmaurya6/flowdl/releases",
    };
  },

  async checkEngineUpdates(): Promise<EngineUpdateInfo> {
    if (isTauriEnvironment()) {
      return await invoke<EngineUpdateInfo>("check_engine_updates");
    }
    return {
      engine: "yt-dlp",
      currentVersion: "2026.10.26",
      latestVersion: "2026.10.26",
      channel: fallbackSettings.ytdlpChannel || "stable",
      updateAvailable: false,
      releaseDate: null,
    };
  },

  async openExternalUrl(url: string): Promise<void> {
    if (isTauriEnvironment()) {
      await invoke("open_external_url", { url });
    }
  },

  async clearTemporaryFiles(): Promise<void> {
    if (isTauriEnvironment()) {
      await invoke("clear_temporary_files");
    }
  },

  async onQueueUpdated(
    callback: (items: DownloadItem[]) => void
  ): Promise<UnlistenFn> {
    if (isTauriEnvironment()) {
      return await listen<DownloadItem[]>("queue-updated", (event) => {
        callback(event.payload);
      });
    }
    return () => {};
  },

  async onAppUpdateAvailable(
    callback: (info: AppUpdateInfo) => void
  ): Promise<UnlistenFn> {
    if (isTauriEnvironment()) {
      return await listen<AppUpdateInfo>("app:update-available", (event) => {
        callback(event.payload);
      });
    }
    return () => {};
  },

  async onSetupProgress(
    callback: (payload: { progress: number; step: string }) => void
  ): Promise<UnlistenFn> {
    if (isTauriEnvironment()) {
      return await listen<{ progress: number; step: string }>(
        "dependency-setup-progress",
        (event) => {
          callback(event.payload);
        }
      );
    }
    return () => {};
  },

  windowMinimize() {
    if (isTauriEnvironment()) {
      getCurrentWindow().minimize();
    }
  },

  windowToggleMaximize() {
    if (isTauriEnvironment()) {
      getCurrentWindow().toggleMaximize();
    }
  },

  windowClose() {
    if (isTauriEnvironment()) {
      getCurrentWindow().close();
    }
  },
};

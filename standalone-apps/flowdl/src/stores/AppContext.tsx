import React, {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useRef,
  useState,
} from "react";
import { tauriBridge } from "../services/tauriBridge";
import type {
  ActivePage,
  AppSettings,
  AppUpdateInfo,
  DependencyState,
  DownloadItem,
  DownloadRequest,
  HistoryItem,
  MediaMetadata,
} from "../types";

export interface ToastNotification {
  id: string;
  type: "success" | "error" | "info" | "warning";
  title: string;
  description?: string;
  actionLabel?: string;
  onAction?: () => void;
}

interface AppContextValue {
  activePage: ActivePage;
  setActivePage: (page: ActivePage) => void;
  dependencies: DependencyState;
  showWelcomeSetup: boolean;
  setShowWelcomeSetup: (show: boolean) => void;
  dependenciesDeclined: boolean;
  setDependenciesDeclined: (declined: boolean) => void;
  settings: AppSettings;
  updateSettings: (partial: Partial<AppSettings>) => Promise<void>;
  queue: DownloadItem[];
  history: HistoryItem[];
  refreshHistory: () => Promise<void>;
  urlInput: string;
  setUrlInput: (url: string) => void;
  analyzing: boolean;
  analysisError: { category: string; message: string } | null;
  clearAnalysisError: () => void;
  analyzedMedia: MediaMetadata | null;
  clearAnalyzedMedia: () => void;
  analyzeUrl: (targetUrl?: string) => Promise<void>;
  startDownload: (request: DownloadRequest) => Promise<void>;
  cancelDownload: (id: string) => Promise<void>;
  stopDownload: (id: string) => Promise<void>;
  resumeDownload: (id: string) => Promise<void>;
  retryDownload: (id: string) => Promise<void>;
  removeDownload: (id: string) => Promise<void>;
  clearCompletedDownloads: () => Promise<void>;
  reorderQueue: (id: string, direction: "up" | "down") => Promise<void>;
  removeHistoryItem: (id: string) => Promise<void>;
  clearAllHistory: () => Promise<void>;
  checkDependenciesNow: (checkUpdates?: boolean) => Promise<DependencyState>;
  runDependencySetup: (forceDownload?: boolean) => Promise<boolean>;
  appUpdateInfo: AppUpdateInfo | null;
  dismissAppUpdate: () => void;
  checkAppUpdateNow: () => Promise<AppUpdateInfo>;
  toasts: ToastNotification[];
  addToast: (toast: Omit<ToastNotification, "id">) => void;
  dismissToast: (id: string) => void;
}

const initialDependencyState: DependencyState = {
  ytdlp: {
    name: "yt-dlp",
    installed: false,
    functional: false,
    version: null,
    latestVersion: null,
    updateAvailable: false,
    path: null,
    source: "none",
    error: null,
  },
  ffmpeg: {
    name: "ffmpeg",
    installed: false,
    functional: false,
    version: null,
    latestVersion: null,
    updateAvailable: false,
    path: null,
    source: "none",
    error: null,
  },
  ffprobe: {
    name: "ffprobe",
    installed: false,
    functional: false,
    version: null,
    latestVersion: null,
    updateAvailable: false,
    path: null,
    source: "none",
    error: null,
  },
  ready: false,
  checking: true,
  installing: false,
  installStep: "Checking system...",
  installProgress: 10,
  error: null,
};

const defaultSettingsState: AppSettings = {
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

const AppContext = createContext<AppContextValue | null>(null);

export const AppProvider: React.FC<{ children: React.ReactNode }> = ({
  children,
}) => {
  const [activePage, setActivePage] = useState<ActivePage>("home");
  const [dependencies, setDependencies] = useState<DependencyState>(
    initialDependencyState
  );
  const [showWelcomeSetup, setShowWelcomeSetup] = useState<boolean>(false);
  const [dependenciesDeclined, setDependenciesDeclined] =
    useState<boolean>(false);
  const [settings, setSettings] = useState<AppSettings>(defaultSettingsState);
  const [queue, setQueue] = useState<DownloadItem[]>([]);
  const [history, setHistory] = useState<HistoryItem[]>([]);
  const [urlInput, setUrlInput] = useState<string>("");
  const [analyzing, setAnalyzing] = useState<boolean>(false);
  const [analysisError, setAnalysisError] = useState<{
    category: string;
    message: string;
  } | null>(null);
  const [analyzedMedia, setAnalyzedMedia] = useState<MediaMetadata | null>(null);
  const [appUpdateInfo, setAppUpdateInfo] = useState<AppUpdateInfo | null>(
    null
  );
  const [toasts, setToasts] = useState<ToastNotification[]>([]);

  const prevStatusesRef = useRef<Record<string, string>>({});

  const addToast = useCallback((toast: Omit<ToastNotification, "id">) => {
    const id = `toast-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`;
    setToasts((prev) => [...prev, { ...toast, id }]);
    setTimeout(() => {
      setToasts((prev) => prev.filter((t) => t.id !== id));
    }, 5500);
  }, []);

  const dismissToast = useCallback((id: string) => {
    setToasts((prev) => prev.filter((t) => t.id !== id));
  }, []);

  // Theme synchronization
  useEffect(() => {
    const root = document.documentElement;
    const applyTheme = (themeMode: "dark" | "light" | "system") => {
      if (themeMode === "system") {
        const prefersDark =
          typeof window.matchMedia === "function" &&
          window.matchMedia("(prefers-color-scheme: dark)").matches;
        root.classList.toggle("dark", prefersDark);
      } else {
        root.classList.toggle("dark", themeMode === "dark");
      }
    };
    applyTheme(settings.theme);
  }, [settings.theme]);

  const refreshHistory = useCallback(async () => {
    try {
      const h = await tauriBridge.getHistory();
      setHistory(h);
    } catch {
      // ignore
    }
  }, []);

  const handleQueueUpdate = useCallback(
    (items: DownloadItem[]) => {
      let shouldRefreshHistory = false;
      for (const item of items) {
        const prev = prevStatusesRef.current[item.id];
        if (prev && prev !== item.status) {
          if (item.status === "completed") {
            shouldRefreshHistory = true;
            addToast({
              type: "success",
              title: "Your download is ready.",
              description: item.title,
              actionLabel: item.outputPath ? "Show in Folder" : undefined,
              onAction: item.outputPath
                ? () => tauriBridge.showInFolder(item.outputPath!)
                : undefined,
            });
          } else if (item.status === "failed") {
            shouldRefreshHistory = true;
            addToast({
              type: "error",
              title: "Download failed",
              description:
                item.errorMessage || "Could not complete the download.",
            });
          }
        }
        prevStatusesRef.current[item.id] = item.status;
      }
      setQueue(items);
      if (shouldRefreshHistory) {
        refreshHistory();
      }
    },
    [addToast, refreshHistory]
  );

  // Initial load of settings, queue, history, and dependencies
  useEffect(() => {
    let mounted = true;
    let unlistenQueue: (() => void) | null = null;
    let unlistenSetup: (() => void) | null = null;

    async function init() {
      try {
        const loadedSettings = await tauriBridge.getSettings();
        if (!mounted) return;
        setSettings(loadedSettings);

        const [loadedQueue, loadedHistory] = await Promise.all([
          tauriBridge.getQueue(),
          tauriBridge.getHistory(),
        ]);
        if (!mounted) return;
        for (const q of loadedQueue) {
          prevStatusesRef.current[q.id] = q.status;
        }
        setQueue(loadedQueue);
        setHistory(loadedHistory);

        const depState = await tauriBridge.checkDependencies(false);
        if (!mounted) return;
        setDependencies(depState);

        if (!depState.ready) {
          setShowWelcomeSetup(true);
        } else {
          if (!loadedSettings.firstLaunchCompleted) {
            const updated = { ...loadedSettings, firstLaunchCompleted: true };
            setSettings(updated);
            tauriBridge.saveSettings(updated).catch(() => {});
          }
          // Nonblocking background update check on startup
          tauriBridge
            .checkDependencies(true)
            .then((remoteState) => {
              if (!mounted) return;
              setDependencies(remoteState);
              if (remoteState.ytdlp.updateAvailable) {
                if (loadedSettings.autoUpdateEngines) {
                  tauriBridge
                    .updateYtdlpEngine()
                    .then((updatedTool) => {
                      if (!mounted) return;
                      setDependencies((prev) => ({
                        ...prev,
                        ytdlp: updatedTool,
                      }));
                      addToast({
                        type: "info",
                        title: "Download engine updated",
                        description: `yt-dlp updated to ${updatedTool.version}`,
                      });
                    })
                    .catch(() => {
                      addToast({
                        type: "warning",
                        title: "Engine update skipped",
                        description:
                          "Couldn't update the download engine. Your existing working version is still available.",
                      });
                    });
                } else {
                  addToast({
                    type: "info",
                    title: "Download engine update available",
                    description: `yt-dlp ${remoteState.ytdlp.latestVersion} is available in Settings.`,
                  });
                }
              }
            })
            .catch(() => {});

          if (loadedSettings.checkAppUpdatesOnStartup !== false) {
            tauriBridge
              .checkAppUpdate()
              .then((info) => {
                if (mounted && info.updateAvailable) {
                  setAppUpdateInfo(info);
                }
              })
              .catch(() => {});
          }
        }
      } catch {
        // ignore init error
      }

      unlistenQueue = await tauriBridge.onQueueUpdated((items) => {
        if (mounted) handleQueueUpdate(items);
      });
      unlistenSetup = await tauriBridge.onSetupProgress((payload) => {
        if (mounted) {
          setDependencies((prev) => ({
            ...prev,
            installing: true,
            installProgress: payload.progress,
            installStep: payload.step,
          }));
        }
      });
    }

    init();

    return () => {
      mounted = false;
      if (unlistenQueue) unlistenQueue();
      if (unlistenSetup) unlistenSetup();
    };
  }, [addToast, handleQueueUpdate]);

  const updateSettings = useCallback(
    async (partial: Partial<AppSettings>) => {
      const merged = { ...settings, ...partial };
      setSettings(merged);
      try {
        const saved = await tauriBridge.saveSettings(merged);
        setSettings(saved);
      } catch (e: unknown) {
        addToast({
          type: "error",
          title: "Couldn't save settings",
          description: e instanceof Error ? e.message : String(e),
        });
      }
    },
    [settings, addToast]
  );

  const checkDependenciesNow = useCallback(
    async (checkUpdates = true): Promise<DependencyState> => {
      setDependencies((prev) => ({ ...prev, checking: true, error: null }));
      try {
        const res = await tauriBridge.checkDependencies(checkUpdates);
        setDependencies(res);
        return res;
      } catch (e: unknown) {
        const msg = e instanceof Error ? e.message : String(e);
        setDependencies((prev) => ({
          ...prev,
          checking: false,
          error: msg,
        }));
        throw e;
      }
    },
    []
  );

  const runDependencySetup = useCallback(
    async (forceDownload = false): Promise<boolean> => {
      setDependencies((prev) => ({
        ...prev,
        installing: true,
        error: null,
        installProgress: 5,
        installStep: "Checking system environment...",
      }));
      try {
        const res = await tauriBridge.setupDependencies(forceDownload);
        setDependencies(res);
        setDependenciesDeclined(false);
        const updatedSettings = await tauriBridge.getSettings();
        setSettings(updatedSettings);
        return res.ready;
      } catch (e: unknown) {
        const msg = e instanceof Error ? e.message : String(e);
        setDependencies((prev) => ({
          ...prev,
          installing: false,
          error: msg,
        }));
        return false;
      }
    },
    []
  );

  const analyzeUrl = useCallback(
    async (targetUrl?: string) => {
      const raw = (targetUrl ?? urlInput).trim();
      if (!raw) {
        setAnalysisError({
          category: "invalid_url",
          message: "That doesn't look like a supported video link.",
        });
        return;
      }
      if (!raw.startsWith("http://") && !raw.startsWith("https://")) {
        setAnalysisError({
          category: "invalid_url",
          message: "That doesn't look like a supported video link.",
        });
        return;
      }

      setAnalyzing(true);
      setAnalysisError(null);
      setAnalyzedMedia(null);

      try {
        const meta = await tauriBridge.analyzeUrl(raw);
        setAnalyzedMedia(meta);
      } catch (e: unknown) {
        const rawErr = e instanceof Error ? e.message : String(e);
        if (rawErr.includes("||")) {
          const [category, message] = rawErr.split("||");
          setAnalysisError({ category, message });
        } else {
          setAnalysisError({
            category: "unknown",
            message:
              rawErr || "Couldn't analyze this link. Check the URL and try again.",
          });
        }
      } finally {
        setAnalyzing(false);
      }
    },
    [urlInput]
  );

  const startDownload = useCallback(
    async (request: DownloadRequest) => {
      try {
        const item = await tauriBridge.enqueueDownload(request);
        prevStatusesRef.current[item.id] = item.status;
        const updatedQueue = await tauriBridge.getQueue();
        setQueue(updatedQueue);
        addToast({
          type: "info",
          title: "Added to download queue",
          description: `${request.title} (${item.formatSummary})`,
          actionLabel: "View Queue",
          onAction: () => setActivePage("downloads"),
        });
      } catch (e: unknown) {
        addToast({
          type: "error",
          title: "Couldn't start download",
          description: e instanceof Error ? e.message : String(e),
        });
      }
    },
    [addToast]
  );

  const cancelDownload = useCallback(async (id: string) => {
    await tauriBridge.cancelDownload(id);
    setQueue(await tauriBridge.getQueue());
  }, []);

  const stopDownload = useCallback(async (id: string) => {
    await tauriBridge.stopDownload(id);
    setQueue(await tauriBridge.getQueue());
  }, []);

  const resumeDownload = useCallback(async (id: string) => {
    await tauriBridge.resumeDownload(id);
    setQueue(await tauriBridge.getQueue());
  }, []);

  const retryDownload = useCallback(async (id: string) => {
    await tauriBridge.retryDownload(id);
    setQueue(await tauriBridge.getQueue());
  }, []);

  const removeDownload = useCallback(async (id: string) => {
    await tauriBridge.removeDownload(id);
    setQueue(await tauriBridge.getQueue());
  }, []);

  const clearCompletedDownloads = useCallback(async () => {
    await tauriBridge.clearCompletedDownloads();
    setQueue(await tauriBridge.getQueue());
  }, []);

  const reorderQueue = useCallback(
    async (id: string, direction: "up" | "down") => {
      await tauriBridge.reorderQueue(id, direction);
      setQueue(await tauriBridge.getQueue());
    },
    []
  );

  const removeHistoryItem = useCallback(
    async (id: string) => {
      await tauriBridge.removeHistoryItem(id);
      await refreshHistory();
    },
    [refreshHistory]
  );

  const clearAllHistory = useCallback(async () => {
    await tauriBridge.clearHistory();
    await refreshHistory();
    addToast({
      type: "info",
      title: "Download history cleared",
      description: "Your downloaded files on disk were preserved.",
    });
  }, [refreshHistory, addToast]);

  const checkAppUpdateNow = useCallback(async (): Promise<AppUpdateInfo> => {
    const info = await tauriBridge.checkAppUpdate();
    if (info.updateAvailable) {
      setAppUpdateInfo(info);
    }
    return info;
  }, []);

  return (
    <AppContext.Provider
      value={{
        activePage,
        setActivePage,
        dependencies,
        showWelcomeSetup,
        setShowWelcomeSetup,
        dependenciesDeclined,
        setDependenciesDeclined,
        settings,
        updateSettings,
        queue,
        history,
        refreshHistory,
        urlInput,
        setUrlInput,
        analyzing,
        analysisError,
        clearAnalysisError: () => setAnalysisError(null),
        analyzedMedia,
        clearAnalyzedMedia: () => setAnalyzedMedia(null),
        analyzeUrl,
        startDownload,
        cancelDownload,
        stopDownload,
        resumeDownload,
        retryDownload,
        removeDownload,
        clearCompletedDownloads,
        reorderQueue,
        removeHistoryItem,
        clearAllHistory,
        checkDependenciesNow,
        runDependencySetup,
        appUpdateInfo,
        dismissAppUpdate: () => setAppUpdateInfo(null),
        checkAppUpdateNow,
        toasts,
        addToast,
        dismissToast,
      }}
    >
      {children}
    </AppContext.Provider>
  );
};

export function useApp() {
  const ctx = useContext(AppContext);
  if (!ctx) {
    throw new Error("useApp must be used within an AppProvider");
  }
  return ctx;
}

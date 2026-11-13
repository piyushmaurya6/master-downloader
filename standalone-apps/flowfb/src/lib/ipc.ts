import { invoke } from "@tauri-apps/api/core";
import type {
  AppSettings,
  AuthSessionStatus,
  BatchUrlParseResult,
  BinaryStatusReport,
  DiagnosticLogEntry,
  HistoryFilter,
  JobRecord,
  MediaInspectionResult,
  NewJobRequest,
  NormalizedFacebookUrl,
  SerializedAppError,
} from "../types";

export function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export function parseIpcError(err: unknown): SerializedAppError {
  if (typeof err === "object" && err !== null && "code" in err && "user_message" in err) {
    return err as SerializedAppError;
  }
  if (typeof err === "string") {
    try {
      const parsed = JSON.parse(err);
      if (parsed && typeof parsed === "object" && "user_message" in parsed) {
        return parsed as SerializedAppError;
      }
    } catch {
      // Not JSON
    }
    return {
      code: "IPC_ERROR",
      user_message: err,
      technical_details: err,
      remediation_action: "VIEW_LOGS",
    };
  }
  const msg = err instanceof Error ? err.message : String(err);
  return {
    code: "UNKNOWN_ERROR",
    user_message: msg,
    technical_details: msg,
    remediation_action: "VIEW_LOGS",
  };
}

export const parseSerializedError = parseIpcError;

export const ipc = {
  // URL & Metadata Inspection
  validateAndNormalizeUrl: (url: string) =>
    invoke<NormalizedFacebookUrl>("validate_and_normalize_url", { url }),

  parseBatchLinks: (rawText: string) =>
    invoke<BatchUrlParseResult>("parse_batch_links", { rawText }),

  parseBatchUrls: (rawText: string) =>
    invoke<BatchUrlParseResult>("parse_batch_links", { rawText }),

  analyzeFacebookUrl: (url: string) =>
    invoke<MediaInspectionResult>("analyze_facebook_url", { url }),

  proxyThumbnailUrl: (url: string) =>
    invoke<string>("proxy_thumbnail_url", { url }),

  // Queue Operations
  enqueueJob: (request: NewJobRequest) =>
    invoke<JobRecord>("enqueue_job", { request }),

  enqueueBatchJobs: (
    params:
      | {
          urls: string[];
          qualityPreset: string;
          subdirectory?: string | null;
          namingTemplate?: string | null;
        }
      | NewJobRequest[]
  ) => {
    if (Array.isArray(params)) {
      const urls = params.map((p) => p.source_url);
      const qualityPreset = params[0]?.quality_preset || "best";
      const namingTemplate = params[0]?.naming_template ?? null;
      return invoke<JobRecord[]>("enqueue_batch_jobs", {
        urls,
        qualityPreset,
        subdirectory: null,
        namingTemplate,
      });
    }
    return invoke<JobRecord[]>("enqueue_batch_jobs", params);
  },

  listQueue: () => invoke<JobRecord[]>("list_queue"),

  getActiveQueue: () => invoke<JobRecord[]>("list_queue"),

  pauseJob: (jobId: string) => invoke<void>("pause_job", { jobId }),

  resumeJob: (jobId: string) => invoke<void>("resume_job", { jobId }),

  retryJob: (jobId: string) => invoke<void>("resume_job", { jobId }),

  cancelJob: (jobId: string) => invoke<void>("cancel_job", { jobId }),

  pauseAllJobs: () => invoke<number>("pause_all_jobs"),

  resumeAllJobs: () => invoke<number>("resume_all_jobs"),

  cancelAllJobs: () => invoke<void>("cancel_all_jobs"),

  clearCompletedJobs: () => invoke<void>("cancel_all_jobs"),

  retryFailedJobs: () => invoke<number>("retry_failed_jobs"),

  reorderQueueJob: (jobId: string, newCreatedAt: number) =>
    invoke<void>("reorder_queue_job", { jobId, newCreatedAt }),

  // Authentication & Session Vault
  getAuthStatus: () => invoke<AuthSessionStatus>("get_auth_status"),

  openFacebookLoginWebview: () => invoke<void>("open_facebook_login_webview"),

  importCookiesTxt: (cookieContent: string, usernameHint?: string | null) =>
    invoke<AuthSessionStatus>("import_cookies_txt", {
      cookieContent,
      usernameHint: usernameHint ?? null,
    }),

  setBrowserCookieMode: (browser: string | null) =>
    invoke<AuthSessionStatus>("set_browser_cookie_mode", { browser }),

  logoutAndWipeVault: () => invoke<AuthSessionStatus>("logout_and_wipe_vault"),

  logoutAuthSession: () => invoke<AuthSessionStatus>("logout_and_wipe_vault"),

  // Binary Manager
  checkBinariesStatus: () => invoke<BinaryStatusReport>("check_binaries_status"),

  getBinariesStatus: () => invoke<BinaryStatusReport>("check_binaries_status"),

  bootstrapOrUpdateBinaries: (forceUpdate: boolean) =>
    invoke<BinaryStatusReport>("bootstrap_or_update_binaries", { forceUpdate }),

  bootstrapBinaries: () =>
    invoke<BinaryStatusReport>("bootstrap_or_update_binaries", { forceUpdate: false }),

  updateBinary: (_binaryName?: string) =>
    invoke<BinaryStatusReport>("bootstrap_or_update_binaries", { forceUpdate: true }),

  verifyBinarySha256: (binaryName: string) =>
    invoke<string>("verify_binary_sha256", { binaryName }),

  // History & Media Library
  queryDownloadHistory: (filter: HistoryFilter) =>
    invoke<JobRecord[]>("query_download_history", { filter }),

  getHistory: (filter: HistoryFilter) =>
    invoke<JobRecord[]>("query_download_history", { filter }),

  deleteHistoryItem: (jobId: string, deleteFiles: boolean) =>
    invoke<void>("delete_history_item", { jobId, deleteFiles }),

  clearAllHistory: async () => {
    const items = await invoke<JobRecord[]>("query_download_history", {
      filter: { limit: 500, offset: 0 },
    });
    for (const item of items) {
      await invoke<void>("delete_history_item", { jobId: item.id, deleteFiles: false });
    }
  },

  revealInExplorer: (filePath: string) =>
    invoke<void>("reveal_in_explorer", { filePath }),

  openInExplorer: (filePath: string) =>
    invoke<void>("reveal_in_explorer", { filePath }),

  openMediaFile: (filePath: string) =>
    invoke<void>("open_media_file", { filePath }),

  exportHistoryJson: (exportPath: string) =>
    invoke<number>("export_history_json", { exportPath }),

  // Settings & Diagnostics
  getAppSettings: () => invoke<AppSettings>("get_app_settings"),

  getSettings: () => invoke<AppSettings>("get_app_settings"),

  updateAppSettings: (settings: AppSettings) =>
    invoke<AppSettings>("update_app_settings", { settings }),

  updateSettings: (settings: AppSettings) =>
    invoke<AppSettings>("update_app_settings", { settings }),

  pickDirectoryDialog: (defaultDir?: string | null) =>
    invoke<string | null>("pick_directory_dialog", { defaultDir }),

  getDiagnosticLogs: () => invoke<DiagnosticLogEntry[]>("get_diagnostic_logs"),

  exportDiagnosticBundle: () => invoke<string>("export_diagnostic_bundle"),
};

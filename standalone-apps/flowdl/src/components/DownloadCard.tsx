import React, { useState } from "react";
import { motion } from "framer-motion";
import {
  AlertCircle,
  ArrowDown,
  ArrowUp,
  CheckCircle2,
  ChevronDown,
  ChevronUp,
  ExternalLink,
  Film,
  FolderOpen,
  Loader2,
  Music,
  Pause,
  Play,
  RotateCcw,
  Trash2,
  XCircle,
} from "lucide-react";
import { tauriBridge } from "../services/tauriBridge";
import { useApp } from "../stores/AppContext";
import type { DownloadItem } from "../types";
import {
  formatBytes,
  formatEta,
  formatSpeed,
  getPostprocessLabel,
} from "../utils/formatters";

interface Props {
  item: DownloadItem;
  showReorder?: boolean;
}

export const DownloadCard: React.FC<Props> = ({ item, showReorder = false }) => {
  const {
    cancelDownload,
    stopDownload,
    resumeDownload,
    retryDownload,
    removeDownload,
    reorderQueue,
    setShowWelcomeSetup,
    checkDependenciesNow,
    addToast,
  } = useApp();

  const [confirmCancel, setConfirmCancel] = useState(false);
  const [showDiagnostics, setShowDiagnostics] = useState(false);
  const [repairingEngine, setRepairingEngine] = useState(false);

  const handleRepairEngineAndRetry = async () => {
    setRepairingEngine(true);
    try {
      const updated = await tauriBridge.updateYtdlpEngine();
      await checkDependenciesNow(false);
      addToast({
        type: "success",
        title: "Download engine updated",
        description: `yt-dlp ${updated.version} activated. Retrying download...`,
      });
      await retryDownload(item.id);
    } catch {
      addToast({
        type: "error",
        title: "Engine update failed",
        description:
          "Could not update yt-dlp right now. Check your connection or try Repair Engines in Settings.",
      });
    } finally {
      setRepairingEngine(false);
    }
  };

  const isRunning =
    item.status === "downloading" ||
    item.status === "analyzing" ||
    item.status === "postprocessing" ||
    item.status === "retrying";

  const statusLabel = () => {
    switch (item.status) {
      case "queued":
        return "Queued";
      case "analyzing":
        return "Preparing streams...";
      case "downloading":
        return `Downloading — ${Math.round(item.progress)}%`;
      case "postprocessing":
        return getPostprocessLabel(item.postprocessStage);
      case "completed":
        return "Completed";
      case "stopped":
        return "Stopped (Resumable)";
      case "cancelled":
        return "Cancelled";
      case "retrying":
        return `Retrying (${item.retryCount}/${item.maxRetries})...`;
      case "failed":
        return "Failed";
      default:
        return item.status;
    }
  };

  return (
    <motion.div
      layout
      initial={{ opacity: 0, y: 12 }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0, scale: 0.98 }}
      transition={{ duration: 0.22, ease: [0.2, 0, 0, 1] }}
      className="p-4 rounded-2xl bg-surface-card border border-border hover:border-border-strong transition-colors shadow-sm"
    >
      <div className="flex items-start gap-4">
        {/* Thumbnail */}
        <div className="relative w-28 aspect-video rounded-xl overflow-hidden bg-zinc-900 shrink-0 border border-border">
          {item.thumbnail ? (
            <img
              src={item.thumbnail}
              alt={item.title}
              className="w-full h-full object-cover"
            />
          ) : (
            <div className="w-full h-full flex items-center justify-center text-zinc-600">
              {item.mode === "audio" ? (
                <Music className="w-6 h-6" />
              ) : (
                <Film className="w-6 h-6" />
              )}
            </div>
          )}
          <div className="absolute bottom-1 right-1 px-1.5 py-0.5 rounded bg-black/80 text-[10px] font-mono text-white uppercase">
            {item.mode}
          </div>
        </div>

        {/* Center details & progress */}
        <div className="flex-1 min-w-0 space-y-2">
          <div className="flex items-start justify-between gap-3">
            <div className="min-w-0">
              <h4 className="text-sm font-semibold text-zinc-900 dark:text-zinc-100 truncate">
                {item.title}
              </h4>
              <div className="flex flex-wrap items-center gap-2 mt-0.5 text-xs text-zinc-500 dark:text-zinc-400">
                <span className="font-mono font-medium text-indigo-600 dark:text-indigo-400">
                  {item.formatSummary}
                </span>
                {item.uploader && <span>· {item.uploader}</span>}
              </div>
            </div>

            {/* Status badge */}
            <div className="shrink-0 flex items-center gap-1.5">
              {item.status === "completed" && (
                <span className="inline-flex items-center gap-1 px-2.5 py-1 rounded-full text-xs font-semibold bg-emerald-500/15 text-emerald-600 dark:text-emerald-400">
                  <CheckCircle2 className="w-3.5 h-3.5" />
                  Ready
                </span>
              )}
              {item.status === "failed" && (
                <span className="inline-flex items-center gap-1 px-2.5 py-1 rounded-full text-xs font-semibold bg-rose-500/15 text-rose-600 dark:text-rose-400">
                  <AlertCircle className="w-3.5 h-3.5" />
                  Failed
                </span>
              )}
              {isRunning && (
                <span className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-semibold bg-indigo-500/15 text-indigo-600 dark:text-indigo-300">
                  <Loader2 className="w-3.5 h-3.5 animate-spin" />
                  {statusLabel()}
                </span>
              )}
              {(item.status === "queued" ||
                item.status === "stopped" ||
                item.status === "cancelled") && (
                <span className="inline-flex items-center px-2.5 py-1 rounded-full text-xs font-medium bg-surface-subtle text-zinc-600 dark:text-zinc-400 border border-border">
                  {statusLabel()}
                </span>
              )}
            </div>
          </div>

          {/* Live progress bar and metrics */}
          {(isRunning || item.status === "stopped") && (
            <div className="space-y-1.5 pt-0.5">
              <div className="h-2 w-full rounded-full bg-surface-subtle overflow-hidden">
                <motion.div
                  className={`h-full rounded-full ${
                    item.status === "postprocessing"
                      ? "bg-violet-500 animate-pulse"
                      : item.status === "stopped"
                        ? "bg-amber-500"
                        : "bg-indigo-600"
                  }`}
                  initial={{ width: 0 }}
                  animate={{ width: `${Math.max(2, item.progress)}%` }}
                  transition={{ duration: 0.2 }}
                />
              </div>

              <div className="flex flex-wrap items-center justify-between text-xs text-zinc-500 dark:text-zinc-400 font-mono">
                <span>
                  {formatBytes(item.downloadedBytes)}
                  {item.totalBytes ? ` / ${formatBytes(item.totalBytes)}` : ""}
                </span>
                {item.status === "downloading" && (
                  <span>
                    {formatSpeed(item.speedBytesPerSec)} ·{" "}
                    {formatEta(item.etaSeconds)}
                  </span>
                )}
                {item.status === "postprocessing" && (
                  <span className="text-violet-500 dark:text-violet-400">
                    {getPostprocessLabel(item.postprocessStage)}
                  </span>
                )}
              </div>
            </div>
          )}

          {/* Error Banner with Retry, Engine Self-Repair & Diagnostics */}
          {item.status === "failed" && item.errorMessage && (
            <div className="p-3 rounded-xl bg-rose-500/10 border border-rose-500/20 space-y-2">
              <div className="flex items-start justify-between gap-3">
                <div className="space-y-0.5">
                  <div className="text-xs text-rose-600 dark:text-rose-300 font-medium">
                    {item.errorMessage}
                  </div>
                  {item.recoveryAction && (
                    <div className="text-[11px] text-zinc-600 dark:text-zinc-400">
                      {item.recoveryAction}
                    </div>
                  )}
                </div>
                <div className="flex items-center gap-2 shrink-0">
                  {(item.errorCategory === "missing_ffmpeg" ||
                    item.errorCategory === "DEPENDENCY_MISSING") && (
                    <button
                      type="button"
                      onClick={() => setShowWelcomeSetup(true)}
                      className="px-2.5 py-1 rounded-lg text-xs font-semibold bg-indigo-600 text-white hover:bg-indigo-500"
                    >
                      Install FFmpeg
                    </button>
                  )}
                  {(item.suggestEngineUpdate ||
                    item.errorCategory === "ENGINE_INCOMPATIBLE") && (
                    <button
                      type="button"
                      disabled={repairingEngine}
                      onClick={handleRepairEngineAndRetry}
                      className="px-2.5 py-1 rounded-lg text-xs font-semibold bg-indigo-600 text-white hover:bg-indigo-500 disabled:opacity-50"
                    >
                      {repairingEngine
                        ? "Updating Engine..."
                        : "Update Engine & Retry"}
                    </button>
                  )}
                  {item.technicalError && (
                    <button
                      type="button"
                      onClick={() => setShowDiagnostics((s) => !s)}
                      className="inline-flex items-center gap-1 text-[11px] text-rose-500 hover:underline"
                    >
                      Diagnostics
                      {showDiagnostics ? (
                        <ChevronUp className="w-3 h-3" />
                      ) : (
                        <ChevronDown className="w-3 h-3" />
                      )}
                    </button>
                  )}
                </div>
              </div>
              {showDiagnostics && item.technicalError && (
                <pre className="p-2 rounded-lg bg-black/40 text-zinc-300 text-[11px] font-mono overflow-x-auto max-h-24 whitespace-pre-wrap">
                  {item.technicalError}
                </pre>
              )}
            </div>
          )}

          {/* Controls row */}
          <div className="flex flex-wrap items-center justify-between gap-2 pt-1">
            <div className="flex flex-wrap items-center gap-2">
              {item.status === "completed" && (
                <>
                  {item.outputPath && (
                    <button
                      type="button"
                      onClick={() => tauriBridge.openFile(item.outputPath!)}
                      className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-semibold bg-indigo-600 hover:bg-indigo-500 text-white transition-colors"
                    >
                      <ExternalLink className="w-3.5 h-3.5" />
                      Open File
                    </button>
                  )}
                  <button
                    type="button"
                    onClick={() =>
                      tauriBridge.showInFolder(
                        item.outputPath || item.outputDir
                      )
                    }
                    className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium bg-surface-subtle hover:bg-surface-hover border border-border text-zinc-700 dark:text-zinc-300 transition-colors"
                  >
                    <FolderOpen className="w-3.5 h-3.5" />
                    Show in Folder
                  </button>
                </>
              )}

              {isRunning && (
                <>
                  <button
                    type="button"
                    onClick={() => stopDownload(item.id)}
                    className="inline-flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg text-xs font-medium bg-surface-subtle hover:bg-surface-hover border border-border text-zinc-700 dark:text-zinc-300 transition-colors"
                    title="Stop and save partial progress for later resumption"
                  >
                    <Pause className="w-3.5 h-3.5" />
                    Stop
                  </button>

                  {!confirmCancel ? (
                    <button
                      type="button"
                      onClick={() => setConfirmCancel(true)}
                      className="inline-flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg text-xs font-medium text-rose-600 dark:text-rose-400 hover:bg-rose-500/10 transition-colors"
                    >
                      <XCircle className="w-3.5 h-3.5" />
                      Cancel
                    </button>
                  ) : (
                    <div className="inline-flex items-center gap-1.5 bg-rose-500/10 px-2.5 py-1 rounded-lg border border-rose-500/20">
                      <span className="text-xs text-rose-600 dark:text-rose-300 font-medium">
                        Cancel download?
                      </span>
                      <button
                        type="button"
                        onClick={() => {
                          cancelDownload(item.id);
                          setConfirmCancel(false);
                        }}
                        className="px-2 py-0.5 rounded bg-rose-600 text-white text-xs font-semibold"
                      >
                        Yes
                      </button>
                      <button
                        type="button"
                        onClick={() => setConfirmCancel(false)}
                        className="px-2 py-0.5 rounded bg-surface-elevated text-xs text-zinc-600 dark:text-zinc-300"
                      >
                        No
                      </button>
                    </div>
                  )}
                </>
              )}

              {item.status === "stopped" && (
                <button
                  type="button"
                  onClick={() => resumeDownload(item.id)}
                  className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-semibold bg-indigo-600 hover:bg-indigo-500 text-white transition-colors"
                >
                  <Play className="w-3.5 h-3.5" />
                  Resume
                </button>
              )}

              {(item.status === "failed" || item.status === "cancelled") && (
                <button
                  type="button"
                  onClick={() => retryDownload(item.id)}
                  className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-semibold bg-indigo-600 hover:bg-indigo-500 text-white transition-colors"
                >
                  <RotateCcw className="w-3.5 h-3.5" />
                  Retry
                </button>
              )}
            </div>

            {/* Right side queue reorder & remove from list */}
            <div className="flex items-center gap-1">
              {showReorder && item.status === "queued" && (
                <>
                  <button
                    type="button"
                    onClick={() => reorderQueue(item.id, "up")}
                    aria-label="Move up in queue"
                    className="p-1.5 rounded-lg text-zinc-400 hover:text-zinc-700 dark:hover:text-zinc-200 hover:bg-surface-hover"
                  >
                    <ArrowUp className="w-3.5 h-3.5" />
                  </button>
                  <button
                    type="button"
                    onClick={() => reorderQueue(item.id, "down")}
                    aria-label="Move down in queue"
                    className="p-1.5 rounded-lg text-zinc-400 hover:text-zinc-700 dark:hover:text-zinc-200 hover:bg-surface-hover"
                  >
                    <ArrowDown className="w-3.5 h-3.5" />
                  </button>
                </>
              )}

              <button
                type="button"
                onClick={() => removeDownload(item.id)}
                aria-label="Remove from list"
                title="Remove from list (keeps any downloaded file on disk)"
                className="p-1.5 rounded-lg text-zinc-400 hover:text-rose-500 hover:bg-rose-500/10 transition-colors"
              >
                <Trash2 className="w-3.5 h-3.5" />
              </button>
            </div>
          </div>
        </div>
      </div>
    </motion.div>
  );
};

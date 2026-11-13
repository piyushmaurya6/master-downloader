import React from "react";
import {
  Play,
  Pause,
  RotateCcw,
  X,
  FolderOpen,
  Trash2,
  CheckCircle2,
  AlertTriangle,
  Loader2,
  Film,
  Sparkles,
  ExternalLink,
} from "lucide-react";
import { useQueueStore } from "../../stores/useQueueStore";
import { useUIStore } from "../../stores/useUIStore";
import { ipc, parseSerializedError } from "../../lib/ipc";
import { formatBytes, formatSpeed, formatEta } from "../../lib/utils";
import { ProxiedThumbnail } from "../ui/ProxiedThumbnail";
import type { JobRecord } from "../../types";

export const ActiveQueueView: React.FC = () => {
  const { jobs, telemetryByJobId, upsertJob, removeJob } = useQueueStore();
  const { addToast, setAuthModalOpen } = useUIStore();

  const handlePause = async (id: string) => {
    try {
      await ipc.pauseJob(id);
      const target = jobs.find((j) => j.id === id);
      if (target) upsertJob({ ...target, status: "paused" });
    } catch (err) {
      addToast({
        type: "error",
        title: "Pause Failed",
        message: parseSerializedError(err).user_message,
      });
    }
  };

  const handleResume = async (id: string) => {
    try {
      await ipc.resumeJob(id);
      const target = jobs.find((j) => j.id === id);
      if (target) upsertJob({ ...target, status: "queued" });
    } catch (err) {
      addToast({
        type: "error",
        title: "Resume Failed",
        message: parseSerializedError(err).user_message,
      });
    }
  };

  const handleRetry = async (id: string) => {
    try {
      await ipc.retryJob(id);
      const target = jobs.find((j) => j.id === id);
      if (target)
        upsertJob({ ...target, status: "queued", error_message: null });
    } catch (err) {
      addToast({
        type: "error",
        title: "Retry Failed",
        message: parseSerializedError(err).user_message,
      });
    }
  };

  const handleCancel = async (id: string) => {
    try {
      await ipc.cancelJob(id);
      removeJob(id);
    } catch (err) {
      addToast({
        type: "error",
        title: "Cancel Failed",
        message: parseSerializedError(err).user_message,
      });
    }
  };

  const handleClearFinished = async () => {
    try {
      await ipc.clearCompletedJobs();
      const updated = await ipc.getActiveQueue();
      useQueueStore.getState().setJobs(updated);
    } catch {
      // ignore
    }
  };

  const handleOpenFileLocation = async (job: JobRecord) => {
    const firstFile = job.media_files?.[0]?.file_path || job.output_dir;
    try {
      await ipc.openInExplorer(firstFile);
    } catch (err) {
      addToast({
        type: "error",
        title: "Could Not Open Explorer",
        message: parseSerializedError(err).user_message,
      });
    }
  };

  const handleOpenFile = async (job: JobRecord) => {
    const firstFile = job.media_files?.[0]?.file_path;
    if (!firstFile) return;
    try {
      await ipc.openMediaFile(firstFile);
    } catch (err) {
      addToast({
        type: "error",
        title: "Could Not Open File",
        message: parseSerializedError(err).user_message,
      });
    }
  };

  if (jobs.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-16 px-4 text-center rounded-2xl bg-bg-surface/50 border border-border-subtle">
        <div className="w-12 h-12 rounded-2xl fb-gradient-bg/15 border border-blue-500/30 flex items-center justify-center mb-3">
          <Sparkles className="w-5 h-5 text-accent-primary" />
        </div>
        <h3 className="text-sm font-semibold text-text-primary">
          Your Facebook Download Queue is Ready
        </h3>
        <p className="text-xs text-text-secondary max-w-md mt-1 leading-relaxed">
          Paste any Facebook Video, Reel, Watch stream, <code className="font-mono text-sky-400">fb.watch</code> shortlink, or multi-media post above to inspect 4K/1080p DASH streams or queue instant downloads.
        </p>
      </div>
    );
  }

  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between">
        <h2 className="text-xs font-semibold uppercase tracking-wider text-text-secondary">
          Download Queue ({jobs.length})
        </h2>
        <button
          onClick={handleClearFinished}
          className="flex items-center gap-1.5 text-xs text-text-secondary hover:text-text-primary transition-colors"
        >
          <Trash2 className="w-3.5 h-3.5" />
          <span>Clear Finished</span>
        </button>
      </div>

      <div className="space-y-2.5">
        {jobs.map((job) => {
          const telemetry = telemetryByJobId[job.id];
          const percent = telemetry?.percentage ?? job.progress_percent ?? 0;
          const downloadedBytes =
            telemetry?.downloaded_bytes ?? job.downloaded_bytes ?? 0;
          const totalBytes = telemetry?.total_bytes ?? job.total_bytes ?? null;
          const stageLabel =
            telemetry?.stage_label ||
            (job.status === "running"
              ? "Downloading stream..."
              : job.status.toUpperCase());

          return (
            <div
              key={job.id}
              className="rounded-xl bg-bg-surface border border-border-subtle p-3.5 transition-all hover:border-border-focus/60"
            >
              <div className="flex items-center gap-3.5">
                {/* Thumbnail */}
                <div className="w-12 h-12 rounded-lg bg-bg-elevated border border-border-subtle overflow-hidden shrink-0 flex items-center justify-center">
                  <ProxiedThumbnail
                    src={
                      job.thumbnail_url || job.media_files?.[0]?.file_path
                        ? job.thumbnail_url
                        : null
                    }
                    alt={job.author_username || "Media"}
                    className="w-full h-full object-cover"
                    fallbackIcon={<Film className="w-4 h-4 text-text-muted" />}
                  />
                </div>

                {/* Info */}
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2">
                    <span className="px-1.5 py-0.5 text-[10px] font-mono font-bold uppercase rounded bg-blue-500/15 text-blue-400">
                      {job.url_type}
                    </span>
                    <span className="px-1.5 py-0.5 text-[10px] font-mono uppercase rounded bg-bg-elevated text-text-secondary border border-border-subtle">
                      {job.quality_preset.toUpperCase()}
                    </span>
                    <span className="text-xs font-semibold text-text-primary truncate">
                      {job.author_username || job.shortcode || "Facebook Media"}
                    </span>
                    {job.caption && (
                      <span className="text-xs text-text-muted truncate hidden md:inline">
                        — {job.caption}
                      </span>
                    )}
                  </div>

                  {/* Progress Bar */}
                  <div className="mt-2 w-full h-1.5 bg-bg-canvas rounded-full overflow-hidden">
                    <div
                      className={`h-full transition-all duration-200 ${
                        job.status === "completed"
                          ? "bg-status-success"
                          : job.status === "failed"
                          ? "bg-status-error"
                          : "fb-gradient-bg"
                      }`}
                      style={{
                        width: `${Math.min(100, Math.max(0, percent))}%`,
                      }}
                    />
                  </div>

                  {/* Telemetry Row */}
                  <div className="flex items-center justify-between mt-1.5 text-[11px] font-mono text-text-secondary">
                    <div className="flex items-center gap-2">
                      {job.status === "running" && (
                        <Loader2 className="w-3 h-3 text-accent-primary animate-spin" />
                      )}
                      {job.status === "completed" && (
                        <CheckCircle2 className="w-3 h-3 text-status-success" />
                      )}
                      {job.status === "failed" && (
                        <AlertTriangle className="w-3 h-3 text-status-error" />
                      )}
                      <span
                        className={
                          job.status === "failed" ? "text-status-error" : ""
                        }
                      >
                        {job.status === "failed"
                          ? job.error_message || "Download failed"
                          : stageLabel}
                      </span>
                    </div>

                    <div className="flex items-center gap-3">
                      {downloadedBytes > 0 && (
                        <span>
                          {formatBytes(downloadedBytes)}
                          {totalBytes ? ` / ${formatBytes(totalBytes)}` : ""}
                        </span>
                      )}
                      {job.status === "running" && telemetry?.speed_bps && (
                        <span className="text-accent-primary">
                          {formatSpeed(telemetry.speed_bps)}
                        </span>
                      )}
                      {job.status === "running" &&
                        telemetry?.eta_seconds !== null &&
                        telemetry?.eta_seconds !== undefined && (
                          <span>ETA {formatEta(telemetry.eta_seconds)}</span>
                        )}
                      <span>{percent.toFixed(0)}%</span>
                    </div>
                  </div>
                </div>

                {/* Actions */}
                <div className="flex items-center gap-1 shrink-0">
                  {job.status === "running" && (
                    <button
                      onClick={() => handlePause(job.id)}
                      title="Pause Job"
                      className="p-1.5 rounded-lg text-text-secondary hover:text-text-primary hover:bg-bg-elevated transition-colors"
                    >
                      <Pause className="w-4 h-4" />
                    </button>
                  )}

                  {job.status === "paused" && (
                    <button
                      onClick={() => handleResume(job.id)}
                      title="Resume Job"
                      className="p-1.5 rounded-lg text-status-success hover:bg-bg-elevated transition-colors"
                    >
                      <Play className="w-4 h-4" />
                    </button>
                  )}

                  {job.status === "failed" && (
                    <>
                      {job.error_message?.toLowerCase().includes("login") ||
                      job.error_message?.toLowerCase().includes("private") ? (
                        <button
                          onClick={() => setAuthModalOpen(true)}
                          className="px-2 py-1 rounded text-[11px] font-medium bg-status-warning/15 text-status-warning hover:bg-status-warning/25 transition-colors"
                        >
                          Connect Auth
                        </button>
                      ) : null}
                      <button
                        onClick={() => handleRetry(job.id)}
                        title="Retry Job"
                        className="p-1.5 rounded-lg text-accent-primary hover:bg-bg-elevated transition-colors"
                      >
                        <RotateCcw className="w-4 h-4" />
                      </button>
                    </>
                  )}

                  {job.status === "completed" && (
                    <>
                      <button
                        onClick={() => handleOpenFile(job)}
                        title="Play Media"
                        className="p-1.5 rounded-lg text-accent-primary hover:bg-bg-elevated transition-colors"
                      >
                        <ExternalLink className="w-4 h-4" />
                      </button>
                      <button
                        onClick={() => handleOpenFileLocation(job)}
                        title="Reveal in Windows Explorer"
                        className="p-1.5 rounded-lg text-status-success hover:bg-bg-elevated transition-colors"
                      >
                        <FolderOpen className="w-4 h-4" />
                      </button>
                    </>
                  )}

                  <button
                    onClick={() => handleCancel(job.id)}
                    title="Remove / Cancel"
                    className="p-1.5 rounded-lg text-text-muted hover:text-status-error hover:bg-bg-elevated transition-colors"
                  >
                    <X className="w-4 h-4" />
                  </button>
                </div>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
};

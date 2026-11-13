import React from "react";
import { motion } from "framer-motion";
import {
  AlertTriangle,
  CheckCircle2,
  FolderOpen,
  Pause,
  Play,
  RotateCcw,
  Trash2,
  XCircle,
  Zap,
} from "lucide-react";
import { ipc } from "../../lib/ipc";
import { formatBytes, formatEta, formatSpeed } from "../../lib/utils";
import { useQueueStore, type QueueFilterTab } from "../../stores/useQueueStore";
import { useUIStore } from "../../stores/useUIStore";
import { ProxiedThumbnail } from "../ui/ProxiedThumbnail";

export const ActiveQueueView: React.FC = () => {
  const jobs = useQueueStore((s) => s.jobs);
  const telemetryMap = useQueueStore((s) => s.telemetryMap);
  const activeFilter = useQueueStore((s) => s.activeFilter);
  const setActiveFilter = useQueueStore((s) => s.setActiveFilter);
  const rateLimitAlert = useQueueStore((s) => s.rateLimitAlert);
  const setRateLimitAlert = useQueueStore((s) => s.setRateLimitAlert);
  const filteredJobs = useQueueStore((s) => s.getFilteredJobs());
  const aggregateSpeedBps = useQueueStore((s) => s.getAggregateSpeedBps());

  const setAuthModalOpen = useUIStore((s) => s.setAuthModalOpen);
  const setActiveView = useUIStore((s) => s.setActiveView);

  const filterTabs: Array<{ id: QueueFilterTab; label: string; count: number }> = [
    { id: "all", label: "All", count: jobs.length },
    {
      id: "active",
      label: "Active",
      count: jobs.filter((j) => j.status === "running").length,
    },
    {
      id: "queued",
      label: "Queued",
      count: jobs.filter((j) => j.status === "queued").length,
    },
    {
      id: "paused",
      label: "Paused",
      count: jobs.filter((j) => j.status === "paused").length,
    },
    {
      id: "failed",
      label: "Failed",
      count: jobs.filter((j) => j.status === "failed" || j.status === "cancelled")
        .length,
    },
  ];

  return (
    <div className="flex-1 flex flex-col min-h-0 space-y-3">
      {/* Circuit Breaker Banner (if tripped) */}
      {rateLimitAlert && (
        <div className="p-3.5 rounded-xl bg-amber-500/10 border border-amber-500/30 flex items-center justify-between gap-4">
          <div className="flex items-center gap-3">
            <AlertTriangle className="w-5 h-5 text-amber-400 shrink-0" />
            <div>
              <div className="text-xs font-semibold text-amber-200">
                Global Circuit Breaker Paused Queue
              </div>
              <div className="text-[11px] text-amber-300/80">
                {rateLimitAlert.reason}
              </div>
            </div>
          </div>

          <div className="flex items-center gap-2 shrink-0">
            {rateLimitAlert.requires_reauth && (
              <button
                onClick={() => setAuthModalOpen(true)}
                className="px-3 py-1.5 rounded-lg text-xs font-semibold bg-indigo-500 text-white hover:bg-indigo-400"
              >
                Connect Account
              </button>
            )}
            <button
              onClick={() => {
                ipc.resumeAllJobs();
                setRateLimitAlert(null);
              }}
              className="px-3 py-1.5 rounded-lg text-xs font-medium bg-amber-500/20 text-amber-200 hover:bg-amber-500/30 border border-amber-500/30"
            >
              Resume Queue Now
            </button>
          </div>
        </div>
      )}

      {/* Filter Tabs & Global Action Bar */}
      <div className="flex flex-wrap items-center justify-between gap-2 pb-1 border-b border-white/[0.07]">
        <div className="flex items-center gap-1">
          {filterTabs.map((tab) => {
            const active = activeFilter === tab.id;
            return (
              <button
                key={tab.id}
                onClick={() => setActiveFilter(tab.id)}
                className={`px-3 py-1.5 rounded-lg text-xs font-medium transition-colors flex items-center gap-1.5 ${
                  active
                    ? "bg-white/[0.09] text-zinc-100 border border-white/10"
                    : "text-zinc-400 hover:text-zinc-200 hover:bg-white/[0.04]"
                }`}
              >
                <span>{tab.label}</span>
                <span className="px-1.5 py-0.2 rounded-full text-[10px] font-mono bg-white/[0.06] text-zinc-300">
                  {tab.count}
                </span>
              </button>
            );
          })}
        </div>

        {/* Global Queue Action Controls */}
        <div className="flex items-center gap-1.5">
          {aggregateSpeedBps > 0 && (
            <span className="inline-flex items-center gap-1 px-2.5 py-1 rounded-lg text-[11px] font-mono font-semibold bg-indigo-500/15 text-indigo-300 border border-indigo-500/25 mr-1">
              <Zap className="w-3 h-3 text-indigo-400" />
              {formatSpeed(aggregateSpeedBps)}
            </span>
          )}

          <button
            onClick={() => ipc.resumeAllJobs()}
            className="inline-flex items-center gap-1 px-2.5 py-1.5 rounded-lg text-xs font-medium text-zinc-300 hover:text-white bg-white/[0.05] hover:bg-white/[0.1] border border-white/[0.07] transition-colors"
            title="Start or Resume All Paused Jobs"
          >
            <Play className="w-3 h-3 text-emerald-400" />
            <span>Start/Resume All</span>
          </button>

          <button
            onClick={() => ipc.pauseAllJobs()}
            className="inline-flex items-center gap-1 px-2.5 py-1.5 rounded-lg text-xs font-medium text-zinc-300 hover:text-white bg-white/[0.05] hover:bg-white/[0.1] border border-white/[0.07] transition-colors"
            title="Pause All Running & Queued Jobs"
          >
            <Pause className="w-3 h-3 text-amber-400" />
            <span>Pause All</span>
          </button>

          <button
            onClick={() => ipc.retryFailedJobs()}
            className="inline-flex items-center gap-1 px-2.5 py-1.5 rounded-lg text-xs font-medium text-zinc-300 hover:text-white bg-white/[0.05] hover:bg-white/[0.1] border border-white/[0.07] transition-colors"
            title="Retry Failed Jobs"
          >
            <RotateCcw className="w-3 h-3 text-indigo-400" />
            <span>Retry Failed</span>
          </button>

          <button
            onClick={() => ipc.cancelAllJobs()}
            className="inline-flex items-center gap-1 px-2.5 py-1.5 rounded-lg text-xs font-medium text-rose-300 hover:text-rose-200 bg-rose-500/10 hover:bg-rose-500/20 border border-rose-500/20 transition-colors"
            title="Cancel All Active & Queued Jobs"
          >
            <XCircle className="w-3 h-3" />
            <span>Cancel All</span>
          </button>
        </div>
      </div>

      {/* Queue List */}
      <div className="flex-1 overflow-y-auto space-y-2 pr-1">
        {filteredJobs.length === 0 ? (
          <div className="h-56 rounded-2xl border border-dashed border-white/[0.08] flex flex-col items-center justify-center text-center p-6">
            <p className="text-xs font-medium text-zinc-400">
              No downloads in this view
            </p>
            <p className="text-[11px] text-zinc-500 mt-1">
              Paste any Instagram Reel, Carousel, Post, or Story link above (or press{" "}
              <kbd className="px-1.5 py-0.5 font-mono bg-white/[0.06] rounded text-zinc-300">
                Ctrl+V
              </kbd>{" "}
              anywhere) to start archiving in original quality.
            </p>
          </div>
        ) : (
          filteredJobs.map((job) => {
            const telemetry = telemetryMap[job.id];
            const progress = telemetry?.percentage ?? job.progress_percent ?? 0;
            const downloaded =
              telemetry?.downloaded_bytes ?? job.downloaded_bytes ?? 0;
            const total = telemetry?.total_bytes ?? job.total_bytes;
            const speed = telemetry?.speed_bps ?? null;
            const eta = telemetry?.eta_seconds ?? null;

            const stageLabel =
              job.status === "completed"
                ? "Completed"
                : job.status === "paused"
                ? job.error_message || "Paused"
                : job.status === "failed"
                ? job.error_message || "Download failed"
                : job.status === "cancelled"
                ? "Cancelled"
                : telemetry?.stage_label ||
                  (job.status === "running"
                    ? "Downloading media stream..."
                    : "Queued with anti-ban jitter pacing...");

            return (
              <motion.div
                key={job.id}
                layout
                initial={{ opacity: 0, y: 8 }}
                animate={{ opacity: 1, y: 0 }}
                className="group rounded-xl bg-[#121215] hover:bg-[#16161a] border border-white/[0.07] p-3 flex items-center gap-3.5 transition-colors"
              >
                {/* Left: Aspect-Ratio Thumbnail */}
                <div className="relative w-16 h-16 rounded-lg overflow-hidden shrink-0 border border-white/10">
                  <ProxiedThumbnail
                    src={job.thumbnail_url}
                    mediaType={
                      job.url_type === "reel" || job.url_type === "tv"
                        ? "video"
                        : "image"
                    }
                    className="w-full h-full"
                  />
                  <span className="absolute bottom-1 left-1 px-1.5 py-0.2 rounded bg-black/80 text-[9px] font-mono font-bold uppercase text-indigo-300">
                    {job.url_type}
                  </span>
                </div>

                {/* Center: Title, Progress Bar, and Telemetry */}
                <div className="flex-1 min-w-0 space-y-1.5">
                  <div className="flex items-center justify-between gap-2">
                    <div className="flex items-center gap-2 min-w-0">
                      <span className="text-xs font-semibold text-zinc-100 truncate">
                        {job.author_username
                          ? `@${job.author_username}`
                          : job.shortcode ?? job.source_url}
                      </span>
                      {job.shortcode && (
                        <span className="text-[11px] font-mono text-zinc-500">
                          #{job.shortcode}
                        </span>
                      )}
                      <span className="px-1.5 py-0.2 rounded text-[10px] font-mono bg-white/[0.05] text-zinc-400">
                        {job.quality_preset.toUpperCase()}
                      </span>
                    </div>

                    <span className="text-[11px] font-mono text-zinc-400 shrink-0">
                      {job.status === "completed" ? (
                        <span className="inline-flex items-center gap-1 text-emerald-400">
                          <CheckCircle2 className="w-3.5 h-3.5" />
                          100%
                        </span>
                      ) : (
                        `${progress.toFixed(1)}%`
                      )}
                    </span>
                  </div>

                  {/* Smooth Spring Progress Bar */}
                  <div className="h-1.5 w-full rounded-full bg-zinc-800/90 overflow-hidden">
                    <motion.div
                      className={`h-full ${
                        job.status === "completed"
                          ? "bg-emerald-500"
                          : job.status === "failed" || job.status === "cancelled"
                          ? "bg-rose-500"
                          : job.status === "paused"
                          ? "bg-amber-500"
                          : "bg-gradient-to-r from-indigo-500 to-violet-500"
                      }`}
                      initial={{ width: 0 }}
                      animate={{ width: `${Math.max(progress, job.status === "running" ? 4 : 0)}%` }}
                      transition={{ type: "spring", stiffness: 200, damping: 28 }}
                    />
                  </div>

                  {/* Stage Label & Speed/Size/ETA Telemetry */}
                  <div className="flex items-center justify-between text-[11px] text-zinc-400">
                    <span
                      className={`truncate max-w-[360px] ${
                        job.status === "failed" ? "text-rose-400" : "text-zinc-400"
                      }`}
                    >
                      {stageLabel}
                    </span>

                    <div className="flex items-center gap-3 font-mono text-[10px] text-zinc-500 shrink-0">
                      {downloaded > 0 && (
                        <span>
                          {formatBytes(downloaded)}
                          {total ? ` / ${formatBytes(total)}` : ""}
                        </span>
                      )}
                      {job.status === "running" && (
                        <>
                          <span className="text-indigo-300">
                            {formatSpeed(speed)}
                          </span>
                          <span>ETA {formatEta(eta)}</span>
                        </>
                      )}
                    </div>
                  </div>
                </div>

                {/* Right: Hover Action Buttons */}
                <div className="flex items-center gap-1 shrink-0">
                  {job.status === "running" || job.status === "queued" ? (
                    <button
                      onClick={() => ipc.pauseJob(job.id)}
                      className="p-2 rounded-lg text-zinc-400 hover:text-amber-300 hover:bg-white/[0.06] transition-colors"
                      title="Pause Job"
                    >
                      <Pause className="w-4 h-4" />
                    </button>
                  ) : job.status === "paused" ||
                    job.status === "failed" ||
                    job.status === "cancelled" ? (
                    <button
                      onClick={() => ipc.resumeJob(job.id)}
                      className="p-2 rounded-lg text-zinc-400 hover:text-emerald-300 hover:bg-white/[0.06] transition-colors"
                      title="Resume / Retry Job"
                    >
                      <Play className="w-4 h-4" />
                    </button>
                  ) : null}

                  <button
                    onClick={() => {
                      const target =
                        job.media_files[0]?.file_path ?? job.output_dir;
                      ipc.revealInExplorer(target);
                    }}
                    className="p-2 rounded-lg text-zinc-400 hover:text-zinc-100 hover:bg-white/[0.06] transition-colors"
                    title="Open Folder in Windows Explorer"
                  >
                    <FolderOpen className="w-4 h-4" />
                  </button>

                  {job.status === "failed" && (
                    <button
                      onClick={() => setActiveView("settings")}
                      className="px-2 py-1 rounded text-[10px] font-medium bg-rose-500/15 text-rose-300 hover:bg-rose-500/25 border border-rose-500/30"
                      title="View Diagnostic Logs"
                    >
                      Logs
                    </button>
                  )}

                  {job.status === "running" || job.status === "queued" ? (
                    <button
                      onClick={() => ipc.cancelJob(job.id)}
                      className="p-2 rounded-lg text-zinc-500 hover:text-rose-400 hover:bg-white/[0.06] transition-colors"
                      title="Cancel Job"
                    >
                      <XCircle className="w-4 h-4" />
                    </button>
                  ) : (
                    <button
                      onClick={() => ipc.deleteHistoryItem(job.id, false)}
                      className="p-2 rounded-lg text-zinc-500 hover:text-rose-400 hover:bg-white/[0.06] transition-colors"
                      title="Remove from Queue"
                    >
                      <Trash2 className="w-4 h-4" />
                    </button>
                  )}
                </div>
              </motion.div>
            );
          })
        )}
      </div>
    </div>
  );
};

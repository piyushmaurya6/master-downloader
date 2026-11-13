import React from "react";
import {
  AlertTriangle,
  FolderOpen,
  Pause,
  Play,
  RefreshCw,
  Trash2,
  XCircle,
} from "lucide-react";
import { ipc } from "../../lib/ipc";
import { formatBytes, formatEta, formatSpeed } from "../../lib/utils";
import { useQueueStore, type QueueFilterTab } from "../../stores/useQueueStore";
import { useUIStore } from "../../stores/useUIStore";

export const ActiveQueueView: React.FC = () => {
  const jobs = useQueueStore((s) => s.jobs);
  const telemetryMap = useQueueStore((s) => s.telemetryMap);
  const activeFilter = useQueueStore((s) => s.activeFilter);
  const setActiveFilter = useQueueStore((s) => s.setActiveFilter);
  const getFilteredJobs = useQueueStore((s) => s.getFilteredJobs);
  const rateLimitAlert = useQueueStore((s) => s.rateLimitAlert);
  const setRateLimitAlert = useQueueStore((s) => s.setRateLimitAlert);
  const setJobs = useQueueStore((s) => s.setJobs);

  const setAuthModalOpen = useUIStore((s) => s.setAuthModalOpen);
  const filteredJobs = getFilteredJobs();

  const refreshQueue = async () => {
    try {
      const updated = await ipc.listQueue();
      setJobs(updated);
    } catch {
      // ignore
    }
  };

  const tabs: Array<{ id: QueueFilterTab; label: string; count: number }> = [
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
      count: jobs.filter(
        (j) => j.status === "failed" || j.status === "cancelled"
      ).length,
    },
  ];

  const getPlatformBadge = (platform: string) => {
    switch (platform) {
      case "youtube":
        return "bg-rose-500/20 text-rose-300 border-rose-500/35";
      case "instagram":
        return "bg-fuchsia-500/20 text-fuchsia-300 border-fuchsia-500/35";
      case "facebook":
        return "bg-blue-500/20 text-blue-300 border-blue-500/35";
      default:
        return "bg-violet-500/20 text-violet-300 border-violet-500/35";
    }
  };

  return (
    <div className="flex-1 flex flex-col min-h-0 space-y-3">
      {/* Circuit-Breaker / Rate Limit Banner */}
      {rateLimitAlert && (
        <div className="flex items-center justify-between px-4 py-3 rounded-xl bg-amber-500/10 border border-amber-500/35 text-xs text-amber-200">
          <div className="flex items-center gap-2.5">
            <AlertTriangle className="w-4 h-4 text-amber-400 shrink-0" />
            <div>
              <span className="font-semibold">
                {rateLimitAlert.requires_reauth
                  ? "Platform Authentication Required"
                  : "Rate-Limit Protection Triggered"}
                :{" "}
              </span>
              <span className="text-amber-200/90">{rateLimitAlert.reason}</span>
            </div>
          </div>
          <div className="flex items-center gap-2 shrink-0">
            {rateLimitAlert.requires_reauth ? (
              <button
                onClick={() => setAuthModalOpen(true)}
                className="px-3 py-1 rounded-lg bg-amber-500/25 hover:bg-amber-500/35 text-amber-100 font-semibold"
              >
                Open Session Vault
              </button>
            ) : (
              <button
                onClick={async () => {
                  await ipc.resumeAllJobs();
                  setRateLimitAlert(null);
                  refreshQueue();
                }}
                className="px-3 py-1 rounded-lg bg-amber-500/25 hover:bg-amber-500/35 text-amber-100 font-semibold"
              >
                Resume Queue
              </button>
            )}
          </div>
        </div>
      )}

      {/* Queue Controls Bar */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-1 p-1 rounded-xl bg-[#0F131D] border border-white/[0.08]">
          {tabs.map((tab) => (
            <button
              key={tab.id}
              onClick={() => setActiveFilter(tab.id)}
              className={`flex items-center gap-1.5 px-2.5 py-1 rounded-lg text-xs font-medium transition-all ${
                activeFilter === tab.id
                  ? "bg-violet-500/20 text-violet-200 border border-violet-500/35"
                  : "text-slate-400 hover:text-slate-200"
              }`}
            >
              <span>{tab.label}</span>
              <span className="text-[10px] font-mono px-1.5 py-0.2 rounded bg-white/[0.06]">
                {tab.count}
              </span>
            </button>
          ))}
        </div>

        <div className="flex items-center gap-1.5">
          <button
            onClick={async () => {
              await ipc.pauseAllJobs();
              refreshQueue();
            }}
            className="flex items-center gap-1 px-2.5 py-1.5 rounded-lg bg-[#0F131D] hover:bg-white/[0.06] border border-white/[0.08] text-xs text-slate-300"
            title="Pause All Jobs"
          >
            <Pause className="w-3.5 h-3.5 text-amber-400" />
            <span>Pause All</span>
          </button>

          <button
            onClick={async () => {
              await ipc.resumeAllJobs();
              setRateLimitAlert(null);
              refreshQueue();
            }}
            className="flex items-center gap-1 px-2.5 py-1.5 rounded-lg bg-[#0F131D] hover:bg-white/[0.06] border border-white/[0.08] text-xs text-slate-300"
            title="Resume Paused Jobs"
          >
            <Play className="w-3.5 h-3.5 text-emerald-400" />
            <span>Resume All</span>
          </button>

          <button
            onClick={async () => {
              await ipc.retryFailedJobs();
              refreshQueue();
            }}
            className="flex items-center gap-1 px-2.5 py-1.5 rounded-lg bg-[#0F131D] hover:bg-white/[0.06] border border-white/[0.08] text-xs text-slate-300"
            title="Retry Failed Jobs"
          >
            <RefreshCw className="w-3.5 h-3.5 text-sky-400" />
            <span>Retry Failed</span>
          </button>

          <button
            onClick={async () => {
              await ipc.cancelAllJobs();
              refreshQueue();
            }}
            className="flex items-center gap-1 px-2.5 py-1.5 rounded-lg bg-[#0F131D] hover:bg-rose-500/15 border border-white/[0.08] text-xs text-slate-400 hover:text-rose-300"
            title="Cancel Active Queue"
          >
            <XCircle className="w-3.5 h-3.5" />
            <span>Cancel All</span>
          </button>
        </div>
      </div>

      {/* Queue List */}
      <div className="flex-1 overflow-y-auto space-y-2 pr-1">
        {filteredJobs.length === 0 ? (
          <div className="h-56 rounded-2xl border border-dashed border-white/[0.08] bg-[#0F131D]/40 flex flex-col items-center justify-center text-center p-6">
            <p className="text-xs font-medium text-slate-300">
              No active downloads in this view
            </p>
            <p className="text-[11px] text-slate-500 mt-1 max-w-md">
              Copy or paste any YouTube, Instagram, or Facebook link — with{" "}
              <strong className="text-emerald-300">Auto-DL: ON</strong>, Master
              Downloader detects the platform and starts downloading automatically.
            </p>
          </div>
        ) : (
          filteredJobs.map((job) => {
            const telemetry = telemetryMap[job.id];
            const pct = telemetry?.percentage ?? job.progress_percent ?? 0;
            const isMuxing = telemetry?.status === "muxing";
            const isExtractingAudio = telemetry?.status === "extracting_audio";
            const speed = formatSpeed(telemetry?.speed_bps);
            const eta = formatEta(telemetry?.eta_seconds);

            return (
              <div
                key={job.id}
                className="p-3.5 rounded-xl bg-[#0F131D] border border-white/[0.08] hover:border-white/[0.14] transition-all flex items-center justify-between gap-4"
              >
                {/* Left Info */}
                <div className="flex items-center gap-3 min-w-0 flex-1">
                  <div className="w-12 h-12 rounded-lg bg-[#07090E] border border-white/[0.08] overflow-hidden shrink-0 flex items-center justify-center">
                    {job.thumbnail_url ? (
                      <img
                        src={job.thumbnail_url}
                        alt=""
                        className="w-full h-full object-cover"
                      />
                    ) : (
                      <span className="text-[10px] font-mono uppercase text-violet-400 font-bold">
                        {job.url_type.slice(0, 5)}
                      </span>
                    )}
                  </div>

                  <div className="min-w-0 flex-1 space-y-1">
                    <div className="flex items-center gap-2">
                      <span
                        className={`text-[10px] font-mono uppercase px-1.5 py-0.5 rounded border font-bold ${getPlatformBadge(
                          job.platform
                        )}`}
                      >
                        {job.platform || "media"}
                      </span>
                      <span className="text-xs font-semibold text-slate-100 truncate">
                        {job.author_username
                          ? `@${job.author_username}`
                          : job.shortcode ?? "Media Item"}
                      </span>
                      <span className="text-[10px] font-mono uppercase px-1.5 py-0.5 rounded bg-white/[0.06] text-slate-300">
                        {job.quality_preset}
                      </span>
                      <span
                        className={`text-[10px] font-mono uppercase px-1.5 py-0.5 rounded ${
                          job.status === "running"
                            ? "bg-violet-500/20 text-violet-300"
                            : job.status === "completed"
                            ? "bg-emerald-500/20 text-emerald-300"
                            : job.status === "paused"
                            ? "bg-amber-500/20 text-amber-300"
                            : job.status === "failed"
                            ? "bg-rose-500/20 text-rose-300"
                            : "bg-slate-800 text-slate-400"
                        }`}
                      >
                        {isMuxing
                          ? "FFMPEG MUXING"
                          : isExtractingAudio
                          ? "EXTRACTING AUDIO"
                          : job.status}
                      </span>
                    </div>

                    <p className="text-[11px] text-slate-400 truncate font-mono">
                      {job.caption || job.source_url}
                    </p>

                    {job.error_message && (
                      <p className="text-[11px] text-rose-400 truncate">
                        {job.error_message}
                      </p>
                    )}

                    {/* Progress Bar */}
                    <div className="w-full h-1.5 rounded-full bg-[#07090E] overflow-hidden">
                      <div
                        className={`h-full transition-all duration-300 ${
                          job.status === "failed"
                            ? "bg-rose-500"
                            : job.status === "completed"
                            ? "bg-emerald-500"
                            : "master-gradient-bg"
                        }`}
                        style={{ width: `${Math.min(100, Math.max(2, pct))}%` }}
                      />
                    </div>
                  </div>
                </div>

                {/* Right Telemetry & Actions */}
                <div className="flex items-center gap-4 shrink-0">
                  <div className="text-right font-mono text-[11px] space-y-0.5 min-w-[115px]">
                    <div className="text-slate-200 font-semibold">
                      {pct.toFixed(1)}%
                    </div>
                    <div className="text-slate-400">
                      {job.status === "running" ? (
                        <>
                          {speed} {eta ? `· ${eta}` : ""}
                        </>
                      ) : job.downloaded_bytes > 0 ? (
                        formatBytes(job.downloaded_bytes)
                      ) : (
                        "—"
                      )}
                    </div>
                  </div>

                  <div className="flex items-center gap-1">
                    {job.status === "running" || job.status === "queued" ? (
                      <button
                        onClick={async () => {
                          await ipc.pauseJob(job.id);
                          refreshQueue();
                        }}
                        className="p-1.5 rounded-lg text-slate-400 hover:text-amber-300 hover:bg-white/[0.06]"
                        title="Pause"
                      >
                        <Pause className="w-3.5 h-3.5" />
                      </button>
                    ) : job.status === "paused" || job.status === "failed" ? (
                      <button
                        onClick={async () => {
                          await ipc.resumeJob(job.id);
                          refreshQueue();
                        }}
                        className="p-1.5 rounded-lg text-slate-400 hover:text-emerald-300 hover:bg-white/[0.06]"
                        title="Resume / Retry"
                      >
                        <Play className="w-3.5 h-3.5" />
                      </button>
                    ) : null}

                    <button
                      onClick={() =>
                        ipc.revealInExplorer(
                          job.media_files[0]?.file_path ?? job.output_dir
                        )
                      }
                      className="p-1.5 rounded-lg text-slate-400 hover:text-slate-200 hover:bg-white/[0.06]"
                      title="Open Output Folder"
                    >
                      <FolderOpen className="w-3.5 h-3.5" />
                    </button>

                    <button
                      onClick={async () => {
                        await ipc.deleteHistoryItem(job.id, false);
                        refreshQueue();
                      }}
                      className="p-1.5 rounded-lg text-slate-400 hover:text-rose-400 hover:bg-white/[0.06]"
                      title="Remove from Queue"
                    >
                      <Trash2 className="w-3.5 h-3.5" />
                    </button>
                  </div>
                </div>
              </div>
            );
          })
        )}
      </div>
    </div>
  );
};

export default ActiveQueueView;

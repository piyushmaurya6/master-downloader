import React from "react";
import { Activity, HardDrive, Shield, Zap } from "lucide-react";
import { formatSpeed } from "../../lib/utils";
import { useQueueStore } from "../../stores/useQueueStore";
import { useSettingsStore } from "../../stores/useSettingsStore";

export const StatusFooter: React.FC = () => {
  const jobs = useQueueStore((s) => s.jobs);
  const aggregateSpeed = useQueueStore((s) => s.getAggregateSpeedBps());
  const settings = useSettingsStore((s) => s.settings);
  const binaryStatus = useSettingsStore((s) => s.binaryStatus);

  const runningCount = jobs.filter((j) => j.status === "running").length;
  const queuedCount = jobs.filter((j) => j.status === "queued").length;

  return (
    <footer className="h-7 w-full bg-[#09090B] border-t border-white/[0.07] px-3 flex items-center justify-between text-[11px] text-zinc-400 select-none shrink-0">
      <div className="flex items-center gap-4">
        <span className="flex items-center gap-1.5">
          <Activity className="w-3 h-3 text-indigo-400" />
          <span>
            {runningCount} active · {queuedCount} queued
          </span>
        </span>

        <span className="flex items-center gap-1.5 text-zinc-500">
          <Shield className="w-3 h-3 text-emerald-400" />
          <span>
            Concurrency: {settings?.concurrency ?? 2}x · Jitter:{" "}
            {((settings?.jitter_min_ms ?? 2000) / 1000).toFixed(1)}s–
            {((settings?.jitter_max_ms ?? 4500) / 1000).toFixed(1)}s
          </span>
        </span>
      </div>

      <div className="flex items-center gap-4 font-mono">
        <span className="flex items-center gap-1 text-zinc-500 truncate max-w-[260px]">
          <HardDrive className="w-3 h-3 text-zinc-400 shrink-0" />
          <span className="truncate">{settings?.output_dir ?? "Downloads/InstaFlow"}</span>
        </span>

        <span className="text-zinc-500">
          yt-dlp:{" "}
          <strong className="text-zinc-300">
            {binaryStatus?.yt_dlp.version ?? (binaryStatus?.yt_dlp.installed ? "ready" : "missing")}
          </strong>{" "}
          · gallery-dl:{" "}
          <strong className="text-zinc-300">
            {binaryStatus?.gallery_dl.version ??
              (binaryStatus?.gallery_dl.installed ? "ready" : "missing")}
          </strong>
        </span>

        <span className="flex items-center gap-1 text-indigo-300 font-semibold">
          <Zap className="w-3 h-3 text-indigo-400" />
          {formatSpeed(aggregateSpeed)}
        </span>
      </div>
    </footer>
  );
};

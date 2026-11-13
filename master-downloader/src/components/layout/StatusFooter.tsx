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
    <footer className="h-7 w-full bg-[#07090E] border-t border-white/[0.08] px-3 flex items-center justify-between text-[11px] text-slate-400 select-none shrink-0">
      <div className="flex items-center gap-4">
        <span className="flex items-center gap-1.5">
          <Activity className="w-3 h-3 text-violet-400" />
          <span>
            {runningCount} active · {queuedCount} queued
          </span>
        </span>

        <span className="flex items-center gap-1.5 text-slate-500">
          <Shield className="w-3 h-3 text-emerald-400" />
          <span>
            Concurrency: {settings?.concurrency ?? 2}x · Auto-DL:{" "}
            <strong className="text-slate-300">
              {settings?.auto_download_on_detect ?? true ? "ON" : "OFF"}
            </strong>
          </span>
        </span>
      </div>

      <div className="flex items-center gap-4 font-mono">
        <span className="flex items-center gap-1 text-slate-500 truncate max-w-[260px]">
          <HardDrive className="w-3 h-3 text-slate-400 shrink-0" />
          <span className="truncate">
            {settings?.output_dir ?? "Downloads/MasterDownloader"}
          </span>
        </span>

        <span className="text-slate-500">
          yt-dlp:{" "}
          <strong className="text-slate-300">
            {binaryStatus?.yt_dlp.version ??
              (binaryStatus?.yt_dlp.installed ? "ready" : "missing")}
          </strong>{" "}
          · gallery-dl:{" "}
          <strong className="text-slate-300">
            {binaryStatus?.gallery_dl.installed ? "ready" : "missing"}
          </strong>{" "}
          · ffmpeg:{" "}
          <strong className="text-slate-300">
            {binaryStatus?.ffmpeg.installed ? "ready" : "missing"}
          </strong>
        </span>

        <span className="flex items-center gap-1 text-violet-300 font-semibold">
          <Zap className="w-3 h-3 text-violet-400" />
          {formatSpeed(aggregateSpeed)}
        </span>
      </div>
    </footer>
  );
};

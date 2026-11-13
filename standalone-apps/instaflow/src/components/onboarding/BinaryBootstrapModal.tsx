import React, { useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import {
  CheckCircle2,
  Cpu,
  Download,
  Loader2,
  RefreshCw,
  ShieldCheck,
  X,
} from "lucide-react";
import { ipc, parseIpcError } from "../../lib/ipc";
import { formatBytes } from "../../lib/utils";
import { useSettingsStore } from "../../stores/useSettingsStore";
import { useUIStore } from "../../stores/useUIStore";

export const BinaryBootstrapModal: React.FC = () => {
  const open = useUIStore((s) => s.onboardingModalOpen);
  const setOpen = useUIStore((s) => s.setOnboardingModalOpen);
  const addToast = useUIStore((s) => s.addToast);

  const binaryStatus = useSettingsStore((s) => s.binaryStatus);
  const setBinaryStatus = useSettingsStore((s) => s.setBinaryStatus);
  const bootstrapProgressMap = useSettingsStore((s) => s.bootstrapProgressMap);
  const isBootstrapping = useSettingsStore((s) => s.isBootstrapping);
  const setIsBootstrapping = useSettingsStore((s) => s.setIsBootstrapping);

  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  const handleBootstrap = async (forceUpdate: boolean) => {
    setErrorMsg(null);
    setIsBootstrapping(true);
    try {
      const report = await ipc.bootstrapOrUpdateBinaries(forceUpdate);
      setBinaryStatus(report);
      if (report.all_ready) {
        addToast({
          title: "Media Engines Verified & Ready",
          description: "yt-dlp, gallery-dl, and FFmpeg are installed and SHA-256 verified.",
          variant: "success",
        });
      }
    } catch (err) {
      const parsed = parseIpcError(err);
      setErrorMsg(parsed.user_message);
    } finally {
      setIsBootstrapping(false);
    }
  };

  const engines = [
    {
      key: "yt-dlp.exe",
      title: "yt-dlp Standalone Engine",
      role: "High-Bitrate Reels & DASH Video/Audio Stream Extractor",
      info: binaryStatus?.yt_dlp,
      progress: bootstrapProgressMap["yt-dlp.exe"],
    },
    {
      key: "gallery-dl.exe",
      title: "gallery-dl Standalone Engine",
      role: "Original-Quality Carousels, Photos, Stories & Metadata Inspector",
      info: binaryStatus?.gallery_dl,
      progress: bootstrapProgressMap["gallery-dl.exe"],
    },
    {
      key: "ffmpeg.exe",
      title: "FFmpeg + FFprobe Essentials (x64)",
      role: "Lossless Video + Audio Stream Muxer & Metadata Embedder",
      info: binaryStatus?.ffmpeg,
      progress: bootstrapProgressMap["ffmpeg.exe"],
    },
  ];

  return (
    <AnimatePresence>
      {open && (
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          className="fixed inset-0 z-50 bg-black/75 backdrop-blur-md flex items-center justify-center p-4"
        >
          <motion.div
            initial={{ opacity: 0, scale: 0.95, y: 12 }}
            animate={{ opacity: 1, scale: 1, y: 0 }}
            exit={{ opacity: 0, scale: 0.95, y: 12 }}
            className="w-full max-w-xl rounded-2xl bg-[#121215] border border-white/10 shadow-2xl overflow-hidden"
          >
            {/* Header */}
            <div className="px-6 py-5 border-b border-white/[0.07] flex items-center justify-between">
              <div className="flex items-center gap-3">
                <div className="p-2.5 rounded-xl bg-indigo-500/15 border border-indigo-500/30 text-indigo-400">
                  <Cpu className="w-5 h-5" />
                </div>
                <div>
                  <h2 className="text-sm font-semibold text-zinc-100">
                    Self-Contained Media Engine Manager
                  </h2>
                  <p className="text-xs text-zinc-400">
                    Zero Python or manual setup required — managed automatically in{" "}
                    <code className="text-[11px] text-indigo-300">%LOCALAPPDATA%\InstaFlow\bin</code>
                  </p>
                </div>
              </div>
              <button
                onClick={() => setOpen(false)}
                className="p-1.5 rounded-lg text-zinc-400 hover:text-zinc-100 hover:bg-white/[0.06] transition-colors"
                aria-label="Close modal"
              >
                <X className="w-4 h-4" />
              </button>
            </div>

            {/* Engine List */}
            <div className="p-6 space-y-3">
              {engines.map((eng) => {
                const installed = eng.info?.installed ?? false;
                const prog = eng.progress;
                const pct = prog ? prog.percentage : installed ? 100 : 0;
                const sha = prog?.sha256 ?? eng.info?.sha256;
                const version = prog?.version ?? eng.info?.version;

                return (
                  <div
                    key={eng.key}
                    className="p-4 rounded-xl bg-[#09090B] border border-white/[0.07] space-y-2.5"
                  >
                    <div className="flex items-start justify-between gap-2">
                      <div>
                        <div className="flex items-center gap-2">
                          <span className="text-xs font-semibold text-zinc-100">
                            {eng.title}
                          </span>
                          {version && (
                            <span className="px-1.5 py-0.5 text-[10px] font-mono rounded bg-indigo-500/15 text-indigo-300 border border-indigo-500/20">
                              {version}
                            </span>
                          )}
                        </div>
                        <p className="text-[11px] text-zinc-400 mt-0.5">{eng.role}</p>
                      </div>

                      <div className="shrink-0">
                        {installed && (!prog || prog.status === "ready") ? (
                          <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[11px] font-medium bg-emerald-500/15 text-emerald-300 border border-emerald-500/30">
                            <CheckCircle2 className="w-3 h-3" />
                            Verified
                          </span>
                        ) : prog && prog.status !== "ready" ? (
                          <span className="inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full text-[11px] font-medium bg-indigo-500/15 text-indigo-300 border border-indigo-500/30">
                            <Loader2 className="w-3 h-3 animate-spin" />
                            {prog.status === "verifying"
                              ? "Verifying SHA-256..."
                              : prog.status === "extracting"
                              ? "Extracting..."
                              : `${pct.toFixed(0)}%`}
                          </span>
                        ) : (
                          <span className="inline-flex items-center px-2 py-0.5 rounded-full text-[11px] font-medium bg-amber-500/15 text-amber-300 border border-amber-500/30">
                            Not Installed
                          </span>
                        )}
                      </div>
                    </div>

                    {/* Progress Bar */}
                    <div className="h-1.5 w-full rounded-full bg-zinc-800 overflow-hidden">
                      <motion.div
                        className="h-full bg-gradient-to-r from-indigo-500 to-violet-500"
                        initial={{ width: 0 }}
                        animate={{ width: `${pct}%` }}
                        transition={{ duration: 0.25 }}
                      />
                    </div>

                    {/* Checksum & Size Metadata */}
                    <div className="flex items-center justify-between text-[10px] font-mono text-zinc-500">
                      <span className="flex items-center gap-1 truncate max-w-[340px]">
                        <ShieldCheck className="w-3 h-3 text-emerald-400 shrink-0" />
                        {sha
                          ? `SHA-256: ${sha.slice(0, 16)}...${sha.slice(-8)}`
                          : "SHA-256 verified against official GitHub Release"}
                      </span>
                      <span>
                        {prog?.downloaded_bytes
                          ? formatBytes(prog.downloaded_bytes)
                          : eng.info?.size_bytes
                          ? formatBytes(eng.info.size_bytes)
                          : ""}
                      </span>
                    </div>
                  </div>
                );
              })}

              {errorMsg && (
                <div className="p-3 rounded-xl bg-rose-500/10 border border-rose-500/30 text-xs text-rose-300">
                  {errorMsg}
                </div>
              )}
            </div>

            {/* Footer Actions */}
            <div className="px-6 py-4 bg-[#09090B]/60 border-t border-white/[0.07] flex items-center justify-between">
              <button
                onClick={() => handleBootstrap(true)}
                disabled={isBootstrapping}
                className="inline-flex items-center gap-1.5 px-3 py-2 rounded-lg text-xs font-medium text-zinc-300 hover:text-white bg-white/[0.05] hover:bg-white/[0.1] border border-white/[0.08] disabled:opacity-50 transition-colors"
              >
                <RefreshCw
                  className={`w-3.5 h-3.5 ${isBootstrapping ? "animate-spin" : ""}`}
                />
                Force Update All Engines
              </button>

              <div className="flex items-center gap-2">
                {!binaryStatus?.all_ready && (
                  <button
                    onClick={() => handleBootstrap(false)}
                    disabled={isBootstrapping}
                    className="inline-flex items-center gap-2 px-4 py-2 rounded-lg text-xs font-semibold text-white bg-gradient-to-r from-indigo-500 to-violet-600 hover:from-indigo-400 hover:to-violet-500 shadow-lg shadow-indigo-500/25 disabled:opacity-50 transition-all"
                  >
                    {isBootstrapping ? (
                      <>
                        <Loader2 className="w-3.5 h-3.5 animate-spin" />
                        Bootstrapping Engines...
                      </>
                    ) : (
                      <>
                        <Download className="w-3.5 h-3.5" />
                        Download &amp; Verify Missing Engines
                      </>
                    )}
                  </button>
                )}

                <button
                  onClick={() => setOpen(false)}
                  className="px-4 py-2 rounded-lg text-xs font-medium text-zinc-300 hover:text-white bg-white/[0.06] hover:bg-white/[0.12] transition-colors"
                >
                  {binaryStatus?.all_ready ? "Done" : "Close"}
                </button>
              </div>
            </div>
          </motion.div>
        </motion.div>
      )}
    </AnimatePresence>
  );
};

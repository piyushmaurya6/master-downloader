import React, { useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import {
  Cpu,
  CheckCircle2,
  DownloadCloud,
  Loader2,
  ShieldCheck,
  X,
} from "lucide-react";
import { useUIStore } from "../../stores/useUIStore";
import { useSettingsStore } from "../../stores/useSettingsStore";
import { ipc, parseSerializedError } from "../../lib/ipc";
import { formatBytes } from "../../lib/utils";

export const BinaryBootstrapModal: React.FC = () => {
  const { isBootstrapModalOpen, setBootstrapModalOpen, addToast } =
    useUIStore();
  const {
    binaryStatus,
    setBinaryStatus,
    bootstrapProgress,
    isBootstrapping,
    setIsBootstrapping,
  } = useSettingsStore();

  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  if (!isBootstrapModalOpen) return null;

  const handleBootstrap = async () => {
    setIsBootstrapping(true);
    setErrorMsg(null);
    try {
      const report = await ipc.bootstrapBinaries();
      setBinaryStatus(report);
      if (report.all_ready) {
        addToast({
          type: "success",
          title: "Media Engines Ready",
          message:
            "yt-dlp, gallery-dl, Deno, and FFmpeg are verified and ready for zero-console execution.",
        });
        setBootstrapModalOpen(false);
      }
    } catch (err) {
      const parsed = parseSerializedError(err);
      setErrorMsg(parsed.user_message);
    } finally {
      setIsBootstrapping(false);
    }
  };

  const engines = [
    {
      key: "yt-dlp",
      label: "yt-dlp (YouTube 4K/8K, Instagram Reels & Facebook DASH Engine)",
      info: binaryStatus?.yt_dlp,
      progress: bootstrapProgress["yt-dlp"],
    },
    {
      key: "gallery-dl",
      label: "gallery-dl (Instagram Carousels/Stories & Facebook Photo Albums)",
      info: binaryStatus?.gallery_dl,
      progress: bootstrapProgress["gallery-dl"],
    },
    {
      key: "ffmpeg",
      label: "FFmpeg + FFprobe (Lossless DASH Muxing & EBU R128 Audio Engine)",
      info: binaryStatus?.ffmpeg,
      progress: bootstrapProgress["ffmpeg"],
    },
    {
      key: "deno",
      label: "Deno Runtime (YouTube Signature & Challenge Solver)",
      info: binaryStatus?.deno,
      progress: bootstrapProgress["deno"],
    },
  ];

  return (
    <AnimatePresence>
      <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/80 backdrop-blur-md p-4">
        <motion.div
          initial={{ opacity: 0, scale: 0.96, y: 8 }}
          animate={{ opacity: 1, scale: 1, y: 0 }}
          exit={{ opacity: 0, scale: 0.96, y: 8 }}
          className="w-full max-w-lg rounded-2xl bg-[#0F131D] border border-white/[0.1] shadow-2xl overflow-hidden"
        >
          {/* Header */}
          <div className="flex items-center justify-between px-5 py-4 border-b border-white/[0.08]">
            <div className="flex items-center gap-2.5">
              <div className="w-8 h-8 rounded-lg master-gradient-bg flex items-center justify-center text-white">
                <Cpu className="w-4 h-4" />
              </div>
              <div>
                <h2 className="text-sm font-semibold text-slate-100">
                  Self-Contained Media Engine Manager
                </h2>
                <p className="text-xs text-slate-400">
                  {binaryStatus?.bin_dir ?? "Self-Contained Native Sidecars"} — Zero Python Required
                </p>
              </div>
            </div>
            {binaryStatus?.all_ready && (
              <button
                onClick={() => setBootstrapModalOpen(false)}
                className="p-1.5 rounded-lg text-slate-400 hover:text-slate-100 hover:bg-white/[0.06]"
              >
                <X className="w-4 h-4" />
              </button>
            )}
          </div>

          {/* Body */}
          <div className="p-5 space-y-4">
            <p className="text-xs text-slate-400 leading-relaxed">
              Master Downloader bundles standalone Windows x64 binaries for{" "}
              <strong className="text-slate-200">yt-dlp</strong>,{" "}
              <strong className="text-slate-200">gallery-dl</strong>,{" "}
              <strong className="text-slate-200">Deno</strong>, and{" "}
              <strong className="text-slate-200">FFmpeg</strong>.
            </p>

            <div className="space-y-2.5">
              {engines.map((eng) => {
                const isReady =
                  eng.info?.installed || eng.progress?.status === "ready";
                const pct = eng.progress?.percentage ?? (isReady ? 100 : 0);

                return (
                  <div
                    key={eng.key}
                    className="p-3 rounded-xl bg-[#07090E] border border-white/[0.08] space-y-1.5"
                  >
                    <div className="flex items-center justify-between text-xs">
                      <span className="font-medium text-slate-200">
                        {eng.label}
                      </span>
                      {isReady ? (
                        <span className="flex items-center gap-1 text-emerald-400 font-mono text-[11px]">
                          <CheckCircle2 className="w-3.5 h-3.5" />
                          <span>
                            {eng.info?.version?.slice(0, 18) || "Ready"}
                          </span>
                        </span>
                      ) : (
                        <span className="text-slate-500 font-mono text-[11px]">
                          {eng.progress?.status || "Not provisioned"}
                        </span>
                      )}
                    </div>

                    <div className="w-full h-1.5 bg-white/[0.06] rounded-full overflow-hidden">
                      <div
                        className={`h-full transition-all duration-200 ${
                          isReady ? "bg-emerald-500" : "master-gradient-bg"
                        }`}
                        style={{ width: `${Math.min(100, pct)}%` }}
                      />
                    </div>

                    <div className="flex items-center justify-between text-[10px] font-mono text-slate-500">
                      <span>
                        {eng.info?.sha256
                          ? `SHA256: ${eng.info.sha256.slice(0, 16)}...`
                          : "Verified sidecar"}
                      </span>
                      {eng.info?.size_bytes && (
                        <span>{formatBytes(eng.info.size_bytes)}</span>
                      )}
                    </div>
                  </div>
                );
              })}
            </div>

            {errorMsg && (
              <div className="p-3 rounded-xl bg-rose-500/15 border border-rose-500/30 text-xs text-rose-300">
                {errorMsg}
              </div>
            )}

            <div className="flex items-center justify-between pt-2">
              <div className="flex items-center gap-1.5 text-[11px] text-slate-400">
                <ShieldCheck className="w-3.5 h-3.5 text-emerald-400" />
                <span>CREATE_NO_WINDOW (0x08000000) Enforced</span>
              </div>

              <button
                onClick={handleBootstrap}
                disabled={isBootstrapping}
                className="flex items-center gap-2 px-4 py-2 rounded-xl text-xs font-semibold text-white master-gradient-bg hover:opacity-95 disabled:opacity-40 transition-all"
              >
                {isBootstrapping ? (
                  <Loader2 className="w-3.5 h-3.5 animate-spin" />
                ) : (
                  <DownloadCloud className="w-3.5 h-3.5" />
                )}
                <span>
                  {binaryStatus?.all_ready
                    ? "Re-Verify / Update Engines"
                    : "Initialize Engines Now"}
                </span>
              </button>
            </div>
          </div>
        </motion.div>
      </div>
    </AnimatePresence>
  );
};

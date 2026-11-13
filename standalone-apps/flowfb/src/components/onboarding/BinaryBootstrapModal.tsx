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
            "yt-dlp, gallery-dl, and FFmpeg are verified and ready for zero-console execution.",
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
      label: "yt-dlp (Primary Facebook DASH Video & Reels Engine)",
      info: binaryStatus?.yt_dlp,
      progress: bootstrapProgress["yt-dlp"],
    },
    {
      key: "gallery-dl",
      label: "gallery-dl (Multi-Photo Post & Fallback Engine)",
      info: binaryStatus?.gallery_dl,
      progress: bootstrapProgress["gallery-dl"],
    },
    {
      key: "ffmpeg",
      label: "FFmpeg + FFprobe (Lossless DASH Remux & Audio Extractor)",
      info: binaryStatus?.ffmpeg,
      progress: bootstrapProgress["ffmpeg"],
    },
  ];

  return (
    <AnimatePresence>
      <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/80 backdrop-blur-md p-4">
        <motion.div
          initial={{ opacity: 0, scale: 0.96, y: 8 }}
          animate={{ opacity: 1, scale: 1, y: 0 }}
          exit={{ opacity: 0, scale: 0.96, y: 8 }}
          className="w-full max-w-lg rounded-2xl bg-bg-surface border border-border-subtle shadow-2xl overflow-hidden"
        >
          {/* Header */}
          <div className="flex items-center justify-between px-5 py-4 border-b border-border-subtle">
            <div className="flex items-center gap-2.5">
              <div className="w-8 h-8 rounded-lg fb-gradient-bg flex items-center justify-center text-white">
                <Cpu className="w-4 h-4" />
              </div>
              <div>
                <h2 className="text-sm font-semibold text-text-primary">
                  Self-Contained Media Engine Manager
                </h2>
                <p className="text-xs text-text-secondary">
                  Managed in %LOCALAPPDATA%\FlowFB\bin — Zero Python Required
                </p>
              </div>
            </div>
            {binaryStatus?.all_ready && (
              <button
                onClick={() => setBootstrapModalOpen(false)}
                className="p-1.5 rounded-lg text-text-muted hover:text-text-primary hover:bg-bg-elevated"
              >
                <X className="w-4 h-4" />
              </button>
            )}
          </div>

          {/* Body */}
          <div className="p-5 space-y-4">
            <p className="text-xs text-text-secondary leading-relaxed">
              FlowFB bundles standalone Windows x64 binaries for{" "}
              <strong className="text-text-primary">yt-dlp</strong>,{" "}
              <strong className="text-text-primary">gallery-dl</strong>, and{" "}
              <strong className="text-text-primary">FFmpeg</strong>. All
              processes run invisibly in the background with SHA-256 verification.
            </p>

            <div className="space-y-2.5">
              {engines.map((eng) => {
                const isReady =
                  eng.info?.installed || eng.progress?.status === "ready";
                const pct = eng.progress?.percentage ?? (isReady ? 100 : 0);

                return (
                  <div
                    key={eng.key}
                    className="p-3.5 rounded-xl bg-bg-canvas border border-border-subtle space-y-2"
                  >
                    <div className="flex items-center justify-between text-xs">
                      <span className="font-medium text-text-primary">
                        {eng.label}
                      </span>
                      {isReady ? (
                        <span className="flex items-center gap-1 text-status-success font-mono text-[11px]">
                          <CheckCircle2 className="w-3.5 h-3.5" />
                          <span>
                            {eng.info?.version?.slice(0, 18) || "Ready"}
                          </span>
                        </span>
                      ) : (
                        <span className="text-text-muted font-mono text-[11px]">
                          {eng.progress?.status || "Not provisioned"}
                        </span>
                      )}
                    </div>

                    <div className="w-full h-1.5 bg-bg-elevated rounded-full overflow-hidden">
                      <div
                        className={`h-full transition-all duration-200 ${
                          isReady ? "bg-status-success" : "fb-gradient-bg"
                        }`}
                        style={{ width: `${Math.min(100, pct)}%` }}
                      />
                    </div>

                    <div className="flex items-center justify-between text-[10px] font-mono text-text-muted">
                      <span>
                        {eng.info?.sha256
                          ? `SHA256: ${eng.info.sha256.slice(0, 16)}...`
                          : eng.progress?.sha256
                          ? `SHA256: ${eng.progress.sha256.slice(0, 16)}...`
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
              <div className="p-3 rounded-xl bg-status-error/15 border border-status-error/30 text-xs text-status-error">
                {errorMsg}
              </div>
            )}

            <div className="flex items-center justify-between pt-2">
              <div className="flex items-center gap-1.5 text-[11px] text-text-muted">
                <ShieldCheck className="w-3.5 h-3.5 text-status-success" />
                <span>CREATE_NO_WINDOW (0x08000000) Enforced</span>
              </div>

              <button
                onClick={handleBootstrap}
                disabled={isBootstrapping}
                className="flex items-center gap-2 px-4 py-2 rounded-xl text-xs font-semibold text-white fb-gradient-bg hover:opacity-95 disabled:opacity-40 transition-all"
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

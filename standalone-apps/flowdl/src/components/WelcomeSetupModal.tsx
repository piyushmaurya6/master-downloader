import React, { useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import {
  ArrowDownToLine,
  CheckCircle2,
  Loader2,
  RefreshCw,
  ShieldCheck,
  AlertCircle,
  Sparkles,
} from "lucide-react";
import { useApp } from "../stores/AppContext";

export const WelcomeSetupModal: React.FC = () => {
  const {
    showWelcomeSetup,
    setShowWelcomeSetup,
    dependencies,
    setDependenciesDeclined,
    checkDependenciesNow,
    runDependencySetup,
    settings,
    updateSettings,
  } = useApp();

  const [usePrivateCopies, setUsePrivateCopies] = useState<boolean>(
    settings.preferManagedBinaries
  );

  if (!showWelcomeSetup) return null;

  const steps = [
    {
      id: "check",
      label: "Checking system",
      done: !dependencies.checking,
      active: dependencies.checking,
      detail: dependencies.checking
        ? "Scanning AppData and system paths..."
        : "System check complete",
    },
    {
      id: "ytdlp",
      label: "yt-dlp detected",
      done: dependencies.ytdlp.functional,
      active: dependencies.installing && !dependencies.ytdlp.functional,
      detail: dependencies.ytdlp.functional
        ? `Version ${dependencies.ytdlp.version} (${dependencies.ytdlp.source})`
        : "Will be downloaded from official yt-dlp GitHub release",
    },
    {
      id: "ffmpeg",
      label: "FFmpeg detected",
      done: dependencies.ffmpeg.functional,
      active:
        dependencies.installing &&
        dependencies.ytdlp.functional &&
        !dependencies.ffmpeg.functional,
      detail: dependencies.ffmpeg.functional
        ? `Version ${dependencies.ffmpeg.version} (${dependencies.ffmpeg.source})`
        : "Will be downloaded from official yt-dlp/FFmpeg-Builds release",
    },
    {
      id: "verify",
      label: "Verifying installation & Ready to download",
      done: dependencies.ready && !dependencies.installing,
      active: dependencies.installing,
      detail: dependencies.installing
        ? dependencies.installStep
        : dependencies.ready
          ? "All engines verified and ready"
          : "Pending setup confirmation",
    },
  ];

  const handleCompleteSetup = async (forceFreshDownload = false) => {
    await updateSettings({
      preferManagedBinaries: usePrivateCopies,
      firstLaunchCompleted: true,
    });
    const ok = await runDependencySetup(forceFreshDownload);
    if (ok) {
      setShowWelcomeSetup(false);
    }
  };

  const handleDecline = async () => {
    setDependenciesDeclined(true);
    await updateSettings({ firstLaunchCompleted: true });
    setShowWelcomeSetup(false);
  };

  return (
    <AnimatePresence>
      <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-sm p-4">
        <motion.div
          initial={{ opacity: 0, y: 18, scale: 0.98 }}
          animate={{ opacity: 1, y: 0, scale: 1 }}
          exit={{ opacity: 0, y: 12, scale: 0.98 }}
          transition={{ duration: 0.28, ease: [0.2, 0, 0, 1] }}
          className="w-full max-w-xl rounded-2xl bg-surface-elevated border border-border-strong shadow-elevated overflow-hidden"
        >
          {/* Header */}
          <div className="p-6 pb-5 border-b border-border bg-gradient-to-b from-indigo-500/[0.07] to-transparent">
            <div className="flex items-center gap-3.5">
              <div className="w-11 h-11 rounded-2xl bg-indigo-600 flex items-center justify-center shadow-lg shadow-indigo-500/30">
                <ArrowDownToLine className="w-6 h-6 text-white" />
              </div>
              <div>
                <h2 className="text-xl font-bold tracking-tight text-zinc-900 dark:text-zinc-100">
                  Welcome to FlowDL
                </h2>
                <p className="text-sm text-zinc-500 dark:text-zinc-400">
                  Your videos and music, beautifully downloaded.
                </p>
              </div>
            </div>
          </div>

          {/* Body */}
          <div className="p-6 space-y-5">
            {/* Step list */}
            <div className="space-y-2.5">
              {steps.map((step) => (
                <div
                  key={step.id}
                  className="flex items-start gap-3 p-3 rounded-xl bg-surface-subtle border border-border"
                >
                  <div className="mt-0.5 shrink-0">
                    {step.active ? (
                      <Loader2 className="w-4 h-4 text-indigo-500 animate-spin" />
                    ) : step.done ? (
                      <CheckCircle2 className="w-4 h-4 text-emerald-500" />
                    ) : (
                      <div className="w-4 h-4 rounded-full border-2 border-zinc-400 dark:border-zinc-600" />
                    )}
                  </div>
                  <div className="min-w-0 flex-1">
                    <div className="text-sm font-medium text-zinc-900 dark:text-zinc-100">
                      {step.label}
                    </div>
                    <div className="text-xs text-zinc-500 dark:text-zinc-400 truncate font-mono mt-0.5">
                      {step.detail}
                    </div>
                  </div>
                </div>
              ))}
            </div>

            {/* Animated progress bar when installing */}
            {dependencies.installing && (
              <div className="space-y-1.5">
                <div className="flex justify-between text-xs text-zinc-500 dark:text-zinc-400">
                  <span>{dependencies.installStep}</span>
                  <span className="font-mono">
                    {Math.round(dependencies.installProgress)}%
                  </span>
                </div>
                <div className="h-2 w-full rounded-full bg-surface-subtle overflow-hidden">
                  <motion.div
                    className="h-full bg-indigo-600 rounded-full"
                    initial={{ width: 0 }}
                    animate={{ width: `${dependencies.installProgress}%` }}
                    transition={{ duration: 0.25 }}
                  />
                </div>
              </div>
            )}

            {/* Binary management preference */}
            <div className="p-3.5 rounded-xl bg-surface-subtle border border-border flex items-start justify-between gap-4">
              <div className="space-y-0.5">
                <div className="text-xs font-semibold text-zinc-800 dark:text-zinc-200 flex items-center gap-1.5">
                  <ShieldCheck className="w-4 h-4 text-indigo-500" />
                  Manage isolated private binaries in AppData
                </div>
                <p className="text-xs text-zinc-500 dark:text-zinc-400">
                  FlowDL keeps verified private copies in your user profile
                  without modifying system PATH or requiring administrator
                  rights.
                </p>
              </div>
              <input
                type="checkbox"
                checked={usePrivateCopies}
                onChange={(e) => setUsePrivateCopies(e.target.checked)}
                aria-label="Manage isolated private binaries"
                className="mt-1 h-4 w-4 rounded border-zinc-400 text-indigo-600 focus:ring-indigo-500"
              />
            </div>

            {/* Error display with Retry */}
            {dependencies.error && (
              <div className="p-3.5 rounded-xl bg-rose-500/10 border border-rose-500/25 flex items-start justify-between gap-3">
                <div className="flex items-start gap-2.5 text-xs text-rose-600 dark:text-rose-300">
                  <AlertCircle className="w-4 h-4 shrink-0 mt-0.5" />
                  <div>
                    <div className="font-semibold">Setup encountered an error</div>
                    <div className="mt-0.5 opacity-90">{dependencies.error}</div>
                  </div>
                </div>
                <button
                  type="button"
                  onClick={() => handleCompleteSetup(true)}
                  className="px-3 py-1.5 rounded-lg text-xs font-semibold bg-rose-600 text-white hover:bg-rose-500 transition-colors shrink-0"
                >
                  Retry
                </button>
              </div>
            )}
          </div>

          {/* Footer actions */}
          <div className="px-6 py-4 bg-surface-subtle border-t border-border flex items-center justify-between gap-3">
            <div className="flex items-center gap-2">
              <button
                type="button"
                disabled={dependencies.installing || dependencies.checking}
                onClick={() => checkDependenciesNow(false)}
                className="inline-flex items-center gap-1.5 px-3 py-2 rounded-xl text-xs font-medium text-zinc-600 dark:text-zinc-300 hover:bg-surface-hover border border-border transition-colors disabled:opacity-50"
              >
                <RefreshCw className="w-3.5 h-3.5" />
                Re-scan System
              </button>
              {!dependencies.ready && (
                <button
                  type="button"
                  disabled={dependencies.installing}
                  onClick={handleDecline}
                  className="px-3 py-2 rounded-xl text-xs font-medium text-zinc-500 hover:text-zinc-800 dark:hover:text-zinc-200 transition-colors"
                >
                  Skip for now
                </button>
              )}
            </div>

            <button
              type="button"
              disabled={dependencies.installing}
              onClick={() => handleCompleteSetup(false)}
              className="inline-flex items-center gap-2 px-5 py-2.5 rounded-xl text-sm font-semibold bg-indigo-600 hover:bg-indigo-500 text-white shadow-sm shadow-indigo-500/25 transition-all disabled:opacity-50"
            >
              {dependencies.installing ? (
                <>
                  <Loader2 className="w-4 h-4 animate-spin" />
                  Configuring Environment...
                </>
              ) : dependencies.ready ? (
                <>
                  <Sparkles className="w-4 h-4" />
                  Start Using FlowDL
                </>
              ) : (
                <>
                  <ArrowDownToLine className="w-4 h-4" />
                  Install Missing Components
                </>
              )}
            </button>
          </div>
        </motion.div>
      </div>
    </AnimatePresence>
  );
};

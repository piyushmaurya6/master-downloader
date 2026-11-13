import React, { useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import {
  ArrowUpCircle,
  CheckCircle2,
  ExternalLink,
  FileText,
  X,
} from "lucide-react";
import { tauriBridge } from "../services/tauriBridge";
import { useApp } from "../stores/AppContext";
import type { AppUpdateInfo } from "../types";
import { formatBytes } from "../utils/formatters";

interface Props {
  updateInfo: AppUpdateInfo | null;
  onDismiss: () => void;
}

export const AppUpdateBanner: React.FC<Props> = ({ updateInfo, onDismiss }) => {
  const { queue, addToast } = useApp();
  const [showReleaseNotes, setShowReleaseNotes] = useState(false);

  if (!updateInfo || !updateInfo.updateAvailable) {
    return null;
  }

  const activeDownloads = queue.filter(
    (q) =>
      q.status === "downloading" ||
      q.status === "analyzing" ||
      q.status === "postprocessing" ||
      q.status === "retrying"
  ).length;

  const handleUpdateNow = async () => {
    if (activeDownloads > 0) {
      addToast({
        type: "warning",
        title: "Active downloads in progress",
        description:
          "Please wait for active downloads to finish or pause them before installing a FlowDL application update.",
      });
      return;
    }
    const targetUrl = updateInfo.downloadUrl || updateInfo.htmlUrl;
    await tauriBridge.openExternalUrl(targetUrl);
  };

  return (
    <AnimatePresence>
      <motion.div
        initial={{ opacity: 0, y: -10 }}
        animate={{ opacity: 1, y: 0 }}
        exit={{ opacity: 0, y: -10 }}
        className="mx-6 mt-3 p-3.5 rounded-2xl bg-indigo-500/10 border border-indigo-500/25 text-zinc-900 dark:text-zinc-100 space-y-2.5"
      >
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div className="flex items-center gap-2.5">
            <ArrowUpCircle className="w-4 h-4 text-indigo-500 shrink-0" />
            <div className="text-xs">
              <span className="font-semibold">
                FlowDL {updateInfo.latestVersion} is available.
              </span>{" "}
              <span className="text-zinc-500 dark:text-zinc-400">
                (Current: v{updateInfo.currentVersion}
                {updateInfo.downloadSize
                  ? ` · ${formatBytes(updateInfo.downloadSize)}`
                  : ""}
                )
              </span>
            </div>
            {updateInfo.signatureVerified && (
              <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-medium bg-emerald-500/15 text-emerald-600 dark:text-emerald-400">
                <CheckCircle2 className="w-3 h-3" />
                Verified Release
              </span>
            )}
          </div>

          <div className="flex items-center gap-2">
            <button
              type="button"
              onClick={handleUpdateNow}
              className="inline-flex items-center gap-1.5 px-3 py-1 rounded-lg text-xs font-semibold bg-indigo-600 hover:bg-indigo-500 text-white transition-colors"
            >
              <ExternalLink className="w-3 h-3" />
              Update Now
            </button>

            <button
              type="button"
              onClick={() => setShowReleaseNotes((s) => !s)}
              className="inline-flex items-center gap-1 px-2.5 py-1 rounded-lg text-xs font-medium bg-surface-card hover:bg-surface-hover border border-border text-zinc-700 dark:text-zinc-300 transition-colors"
            >
              <FileText className="w-3 h-3" />
              View What's New
            </button>

            <button
              type="button"
              onClick={onDismiss}
              className="inline-flex items-center gap-1 px-2.5 py-1 rounded-lg text-xs font-medium text-zinc-500 hover:text-zinc-800 dark:hover:text-zinc-200 transition-colors"
            >
              Remind Me Later
              <X className="w-3 h-3" />
            </button>
          </div>
        </div>

        {showReleaseNotes && (
          <div className="p-3 rounded-xl bg-surface-card border border-border text-xs space-y-1.5">
            <div className="flex items-center justify-between text-zinc-500 dark:text-zinc-400 font-mono text-[11px]">
              <span>
                v{updateInfo.currentVersion} → v{updateInfo.latestVersion}
              </span>
              {updateInfo.releaseDate && (
                <span>
                  Published:{" "}
                  {new Date(updateInfo.releaseDate).toLocaleDateString()}
                </span>
              )}
            </div>
            <pre className="whitespace-pre-wrap font-sans text-zinc-700 dark:text-zinc-300 max-h-36 overflow-y-auto">
              {updateInfo.releaseNotes ||
                "Bug fixes, reliability improvements, and updated downloader compatibility."}
            </pre>
          </div>
        )}
      </motion.div>
    </AnimatePresence>
  );
};

import React from "react";
import { motion, AnimatePresence } from "framer-motion";
import {
  ArrowDownToLine,
  CheckCheck,
  Layers,
  PlusCircle,
} from "lucide-react";
import { DownloadCard } from "../components/DownloadCard";
import { useApp } from "../stores/AppContext";

export const DownloadsPage: React.FC = () => {
  const {
    queue,
    clearCompletedDownloads,
    settings,
    updateSettings,
    setActivePage,
  } = useApp();

  const activeItems = queue.filter(
    (q) =>
      q.status === "downloading" ||
      q.status === "analyzing" ||
      q.status === "postprocessing" ||
      q.status === "retrying"
  );
  const queuedItems = queue.filter(
    (q) => q.status === "queued" || q.status === "stopped"
  );
  const finishedItems = queue.filter(
    (q) =>
      q.status === "completed" ||
      q.status === "failed" ||
      q.status === "cancelled"
  );

  return (
    <motion.div
      initial={{ opacity: 0, y: 12 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.25, ease: [0.2, 0, 0, 1] }}
      className="max-w-4xl mx-auto space-y-6 pb-10"
    >
      {/* Header & Queue Controls */}
      <div className="flex flex-wrap items-center justify-between gap-4">
        <div>
          <h1 className="text-2xl font-bold tracking-tight text-zinc-900 dark:text-zinc-100">
            Downloads Queue
          </h1>
          <p className="text-sm text-zinc-500 dark:text-zinc-400">
            Manage active transfers, reorder pending items, and inspect completed
            jobs.
          </p>
        </div>

        <div className="flex items-center gap-2.5">
          {/* Concurrency / Sequential mode selector */}
          <div className="inline-flex items-center gap-2 px-3 py-1.5 rounded-xl bg-surface-card border border-border text-xs">
            <Layers className="w-3.5 h-3.5 text-indigo-500" />
            <span className="text-zinc-500 dark:text-zinc-400">Mode:</span>
            <select
              aria-label="Queue concurrency mode"
              value={settings.maxConcurrentDownloads}
              onChange={(e) =>
                updateSettings({
                  maxConcurrentDownloads: Number(e.target.value),
                })
              }
              className="bg-transparent font-semibold text-zinc-800 dark:text-zinc-200 focus:outline-none"
            >
              <option value={1} className="bg-zinc-900 text-white">
                Sequential (1 at a time)
              </option>
              <option value={2} className="bg-zinc-900 text-white">
                Parallel (2 simultaneous)
              </option>
              <option value={3} className="bg-zinc-900 text-white">
                Parallel (3 simultaneous)
              </option>
              <option value={4} className="bg-zinc-900 text-white">
                Parallel (4 simultaneous)
              </option>
            </select>
          </div>

          {finishedItems.length > 0 && (
            <button
              type="button"
              onClick={clearCompletedDownloads}
              className="inline-flex items-center gap-1.5 px-3.5 py-2 rounded-xl text-xs font-medium bg-surface-card hover:bg-surface-hover border border-border text-zinc-700 dark:text-zinc-300 transition-colors"
            >
              <CheckCheck className="w-3.5 h-3.5 text-emerald-500" />
              Clear Completed
            </button>
          )}
        </div>
      </div>

      {queue.length === 0 ? (
        <div className="p-12 rounded-2xl bg-surface-card border border-border text-center space-y-3">
          <div className="w-12 h-12 rounded-2xl bg-indigo-500/10 text-indigo-500 flex items-center justify-center mx-auto">
            <ArrowDownToLine className="w-6 h-6" />
          </div>
          <div className="space-y-1">
            <h3 className="text-base font-semibold text-zinc-900 dark:text-zinc-100">
              Your download queue is empty
            </h3>
            <p className="text-xs text-zinc-500 dark:text-zinc-400 max-w-sm mx-auto">
              Paste a video or music link on the Home screen to start
              downloading in the background.
            </p>
          </div>
          <button
            type="button"
            onClick={() => setActivePage("home")}
            className="inline-flex items-center gap-2 px-4 py-2 rounded-xl text-xs font-semibold bg-indigo-600 hover:bg-indigo-500 text-white transition-colors"
          >
            <PlusCircle className="w-3.5 h-3.5" />
            New Download
          </button>
        </div>
      ) : (
        <div className="space-y-6">
          {/* Active Transfers */}
          {activeItems.length > 0 && (
            <section className="space-y-2.5">
              <h2 className="text-xs font-semibold uppercase tracking-wider text-zinc-500 dark:text-zinc-400">
                Active Transfers ({activeItems.length})
              </h2>
              <div className="space-y-2.5">
                <AnimatePresence>
                  {activeItems.map((item) => (
                    <DownloadCard key={item.id} item={item} />
                  ))}
                </AnimatePresence>
              </div>
            </section>
          )}

          {/* Pending / Stopped Queue */}
          {queuedItems.length > 0 && (
            <section className="space-y-2.5">
              <h2 className="text-xs font-semibold uppercase tracking-wider text-zinc-500 dark:text-zinc-400">
                Queued & Paused ({queuedItems.length})
              </h2>
              <div className="space-y-2.5">
                <AnimatePresence>
                  {queuedItems.map((item) => (
                    <DownloadCard key={item.id} item={item} showReorder />
                  ))}
                </AnimatePresence>
              </div>
            </section>
          )}

          {/* Completed & Failed in Session */}
          {finishedItems.length > 0 && (
            <section className="space-y-2.5">
              <h2 className="text-xs font-semibold uppercase tracking-wider text-zinc-500 dark:text-zinc-400">
                Finished ({finishedItems.length})
              </h2>
              <div className="space-y-2.5">
                <AnimatePresence>
                  {finishedItems.map((item) => (
                    <DownloadCard key={item.id} item={item} />
                  ))}
                </AnimatePresence>
              </div>
            </section>
          )}
        </div>
      )}
    </motion.div>
  );
};

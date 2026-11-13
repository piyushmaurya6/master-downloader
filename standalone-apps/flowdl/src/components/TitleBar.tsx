import React from "react";
import { ArrowDownToLine, Minus, Square, X } from "lucide-react";
import { tauriBridge } from "../services/tauriBridge";
import { useApp } from "../stores/AppContext";

export const TitleBar: React.FC = () => {
  const { queue } = useApp();
  const activeCount = queue.filter(
    (q) =>
      q.status === "downloading" ||
      q.status === "analyzing" ||
      q.status === "postprocessing" ||
      q.status === "retrying"
  ).length;

  return (
    <header
      data-tauri-drag-region
      className="titlebar-drag h-10 w-full flex items-center justify-between px-3.5 bg-surface-elevated/90 backdrop-blur-md border-b border-border select-none z-40 shrink-0"
    >
      <div
        data-tauri-drag-region
        className="flex items-center gap-2.5 pointer-events-none"
      >
        <div className="w-5 h-5 rounded-md bg-indigo-600 flex items-center justify-center shadow-sm shadow-indigo-500/30">
          <ArrowDownToLine className="w-3.5 h-3.5 text-white stroke-[2.5]" />
        </div>
        <span className="text-xs font-semibold tracking-tight text-zinc-800 dark:text-zinc-200">
          FlowDL
        </span>
        {activeCount > 0 && (
          <span className="ml-1.5 inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full text-[11px] font-medium bg-indigo-500/15 text-indigo-600 dark:text-indigo-300 border border-indigo-500/20">
            <span className="w-1.5 h-1.5 rounded-full bg-indigo-500 animate-pulse" />
            {activeCount} active
          </span>
        )}
      </div>

      <div className="titlebar-nodrag flex items-center -mr-2">
        <button
          type="button"
          onClick={() => tauriBridge.windowMinimize()}
          aria-label="Minimize window"
          className="h-8 w-10 inline-flex items-center justify-center text-zinc-500 hover:text-zinc-900 dark:text-zinc-400 dark:hover:text-zinc-100 hover:bg-zinc-200/60 dark:hover:bg-white/5 transition-colors rounded-sm"
        >
          <Minus className="w-3.5 h-3.5" />
        </button>
        <button
          type="button"
          onClick={() => tauriBridge.windowToggleMaximize()}
          aria-label="Maximize window"
          className="h-8 w-10 inline-flex items-center justify-center text-zinc-500 hover:text-zinc-900 dark:text-zinc-400 dark:hover:text-zinc-100 hover:bg-zinc-200/60 dark:hover:bg-white/5 transition-colors rounded-sm"
        >
          <Square className="w-3 h-3" />
        </button>
        <button
          type="button"
          onClick={() => tauriBridge.windowClose()}
          aria-label="Close window"
          className="h-8 w-10 inline-flex items-center justify-center text-zinc-500 hover:text-white dark:text-zinc-400 hover:bg-rose-600 transition-colors rounded-sm"
        >
          <X className="w-4 h-4" />
        </button>
      </div>
    </header>
  );
};

import React from "react";
import {
  Archive,
  DownloadCloud,
  KeyRound,
  Layers,
  Settings,
} from "lucide-react";
import { useQueueStore } from "../../stores/useQueueStore";
import { useUIStore, type ActiveNavView } from "../../stores/useUIStore";

export const Sidebar: React.FC = () => {
  const activeView = useUIStore((s) => s.activeView);
  const setActiveView = useUIStore((s) => s.setActiveView);
  const setBatchModalOpen = useUIStore((s) => s.setBatchModalOpen);
  const setAuthModalOpen = useUIStore((s) => s.setAuthModalOpen);
  const jobs = useQueueStore((s) => s.jobs);

  const activeJobsCount = jobs.filter(
    (j) => j.status === "running" || j.status === "queued"
  ).length;

  const navItems: Array<{
    id: ActiveNavView;
    label: string;
    icon: React.FC<{ className?: string }>;
    badge?: number;
  }> = [
    {
      id: "workspace",
      label: "Workspace",
      icon: DownloadCloud,
      badge: activeJobsCount > 0 ? activeJobsCount : undefined,
    },
    {
      id: "history",
      label: "Media Library",
      icon: Archive,
    },
    {
      id: "settings",
      label: "Settings",
      icon: Settings,
    },
  ];

  return (
    <aside className="w-52 bg-[#09090B] border-r border-white/[0.07] flex flex-col justify-between p-3 shrink-0">
      <div className="space-y-1">
        <div className="px-2.5 py-1.5 text-[10px] font-semibold uppercase tracking-wider text-zinc-500">
          Navigation
        </div>
        {navItems.map((item) => {
          const Icon = item.icon;
          const isActive = activeView === item.id;
          return (
            <button
              key={item.id}
              onClick={() => setActiveView(item.id)}
              className={`w-full flex items-center justify-between px-3 py-2 rounded-lg text-xs font-medium transition-all ${
                isActive
                  ? "bg-indigo-500/15 text-indigo-200 border border-indigo-500/30 shadow-sm"
                  : "text-zinc-400 hover:text-zinc-200 hover:bg-white/[0.04] border border-transparent"
              }`}
            >
              <span className="flex items-center gap-2.5">
                <Icon
                  className={`w-4 h-4 ${
                    isActive ? "text-indigo-400" : "text-zinc-400"
                  }`}
                />
                {item.label}
              </span>
              {item.badge !== undefined && (
                <span className="px-1.5 py-0.5 text-[10px] font-mono rounded-full bg-indigo-500/30 text-indigo-200">
                  {item.badge}
                </span>
              )}
            </button>
          );
        })}

        <div className="pt-4 px-2.5 py-1.5 text-[10px] font-semibold uppercase tracking-wider text-zinc-500">
          Quick Actions
        </div>

        <button
          onClick={() => setBatchModalOpen(true)}
          className="w-full flex items-center justify-between px-3 py-2 rounded-lg text-xs font-medium text-zinc-400 hover:text-zinc-200 hover:bg-white/[0.04] transition-colors"
        >
          <span className="flex items-center gap-2.5">
            <Layers className="w-4 h-4 text-violet-400" />
            Batch Import
          </span>
          <kbd className="text-[10px] font-mono text-zinc-500">^⇧B</kbd>
        </button>

        <button
          onClick={() => setAuthModalOpen(true)}
          className="w-full flex items-center justify-between px-3 py-2 rounded-lg text-xs font-medium text-zinc-400 hover:text-zinc-200 hover:bg-white/[0.04] transition-colors"
        >
          <span className="flex items-center gap-2.5">
            <KeyRound className="w-4 h-4 text-emerald-400" />
            Session Vault
          </span>
        </button>
      </div>

      {/* Bottom Rate-Limit Hygiene Card */}
      <div className="p-3 rounded-xl bg-[#121215] border border-white/[0.07] space-y-1.5">
        <div className="flex items-center justify-between">
          <span className="text-[11px] font-semibold text-zinc-200">
            Anti-Ban Shield
          </span>
          <span className="w-2 h-2 rounded-full bg-emerald-400 animate-pulse" />
        </div>
        <p className="text-[10px] text-zinc-400 leading-relaxed">
          Smart jitter pacing &amp; automatic HTTP 429 circuit breaker active.
        </p>
      </div>
    </aside>
  );
};

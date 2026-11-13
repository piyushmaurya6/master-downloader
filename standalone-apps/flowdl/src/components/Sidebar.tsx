import React from "react";
import {
  ArrowDownToLine,
  CheckCircle2,
  Clock,
  Home,
  Laptop,
  Moon,
  Settings,
  Sun,
  AlertTriangle,
} from "lucide-react";
import { useApp } from "../stores/AppContext";
import type { ActivePage } from "../types";

export const Sidebar: React.FC = () => {
  const {
    activePage,
    setActivePage,
    queue,
    dependencies,
    setShowWelcomeSetup,
    settings,
    updateSettings,
  } = useApp();

  const activeOrQueuedCount = queue.filter(
    (q) =>
      q.status === "queued" ||
      q.status === "analyzing" ||
      q.status === "downloading" ||
      q.status === "postprocessing" ||
      q.status === "retrying"
  ).length;

  const navItems: {
    id: ActivePage;
    label: string;
    icon: React.ComponentType<{ className?: string }>;
    badge?: number;
  }[] = [
    { id: "home", label: "Home", icon: Home },
    {
      id: "downloads",
      label: "Downloads",
      icon: ArrowDownToLine,
      badge: activeOrQueuedCount > 0 ? activeOrQueuedCount : undefined,
    },
    { id: "history", label: "History", icon: Clock },
    { id: "settings", label: "Settings", icon: Settings },
  ];

  const cycleTheme = () => {
    const next =
      settings.theme === "dark"
        ? "light"
        : settings.theme === "light"
          ? "system"
          : "dark";
    updateSettings({ theme: next });
  };

  return (
    <aside className="w-56 shrink-0 bg-surface-elevated border-r border-border flex flex-col justify-between p-3 select-none">
      {/* Primary navigation */}
      <nav className="space-y-1" aria-label="Main Navigation">
        {navItems.map((item) => {
          const Icon = item.icon;
          const isActive = activePage === item.id;
          return (
            <button
              key={item.id}
              type="button"
              onClick={() => setActivePage(item.id)}
              className={`w-full flex items-center justify-between px-3 py-2.5 rounded-xl text-sm font-medium transition-all duration-150 ${
                isActive
                  ? "bg-indigo-600 text-white shadow-sm shadow-indigo-500/25"
                  : "text-zinc-600 dark:text-zinc-400 hover:text-zinc-900 dark:hover:text-zinc-100 hover:bg-surface-hover"
              }`}
            >
              <span className="flex items-center gap-3">
                <Icon
                  className={`w-4 h-4 ${
                    isActive
                      ? "text-white"
                      : "text-zinc-500 dark:text-zinc-400"
                  }`}
                />
                {item.label}
              </span>
              {item.badge !== undefined && (
                <span
                  className={`px-2 py-0.5 text-xs font-semibold rounded-full ${
                    isActive
                      ? "bg-white/20 text-white"
                      : "bg-indigo-500/15 text-indigo-600 dark:text-indigo-400"
                  }`}
                >
                  {item.badge}
                </span>
              )}
            </button>
          );
        })}
      </nav>

      {/* Bottom footer: Dependency health, theme switcher, version */}
      <div className="space-y-2.5 pt-3 border-t border-border">
        <button
          type="button"
          onClick={() => setShowWelcomeSetup(true)}
          className="w-full flex items-center justify-between px-3 py-2 rounded-xl bg-surface-subtle hover:bg-surface-hover border border-border transition-colors text-left"
          title="Click to view or manage yt-dlp and FFmpeg status"
        >
          <div className="flex items-center gap-2 min-w-0">
            {dependencies.ready ? (
              <CheckCircle2 className="w-4 h-4 text-emerald-500 shrink-0" />
            ) : (
              <AlertTriangle className="w-4 h-4 text-amber-500 shrink-0" />
            )}
            <div className="truncate">
              <div className="text-xs font-medium text-zinc-800 dark:text-zinc-200 truncate">
                {dependencies.ready ? "Engines Ready" : "Setup Required"}
              </div>
              <div className="text-[11px] text-zinc-500 dark:text-zinc-400 truncate font-mono">
                {dependencies.ytdlp.version
                  ? `yt-dlp ${dependencies.ytdlp.version}`
                  : "Check dependencies"}
              </div>
            </div>
          </div>
        </button>

        <div className="flex items-center justify-between px-1.5 pt-0.5">
          <div className="text-[11px] text-zinc-500 dark:text-zinc-400">
            <span className="font-semibold text-zinc-700 dark:text-zinc-300">
              FlowDL
            </span>{" "}
            <span className="font-mono">v1.0.0</span>
          </div>

          <button
            type="button"
            onClick={cycleTheme}
            aria-label={`Current theme: ${settings.theme}. Click to switch theme`}
            title={`Theme: ${settings.theme.toUpperCase()}`}
            className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-lg text-xs font-medium text-zinc-600 dark:text-zinc-300 bg-surface-subtle hover:bg-surface-hover border border-border transition-colors"
          >
            {settings.theme === "dark" && <Moon className="w-3.5 h-3.5 text-indigo-400" />}
            {settings.theme === "light" && <Sun className="w-3.5 h-3.5 text-amber-500" />}
            {settings.theme === "system" && <Laptop className="w-3.5 h-3.5 text-zinc-400" />}
            <span className="capitalize">{settings.theme}</span>
          </button>
        </div>
      </div>
    </aside>
  );
};

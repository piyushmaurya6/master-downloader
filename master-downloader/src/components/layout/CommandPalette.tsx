import React, { useState } from "react";
import {
  Archive,
  Command,
  Cpu,
  DownloadCloud,
  KeyRound,
  Layers,
  Moon,
  Pause,
  Play,
  Settings,
  Zap,
} from "lucide-react";
import { ipc } from "../../lib/ipc";
import { useSettingsStore } from "../../stores/useSettingsStore";
import { useUIStore } from "../../stores/useUIStore";

export const CommandPalette: React.FC = () => {
  const open = useUIStore((s) => s.commandPaletteOpen);
  const setOpen = useUIStore((s) => s.setCommandPaletteOpen);
  const setActiveView = useUIStore((s) => s.setActiveView);
  const setBatchModalOpen = useUIStore((s) => s.setBatchModalOpen);
  const setAuthModalOpen = useUIStore((s) => s.setAuthModalOpen);
  const setOnboardingModalOpen = useUIStore((s) => s.setOnboardingModalOpen);
  const toggleTheme = useUIStore((s) => s.toggleTheme);

  const settings = useSettingsStore((s) => s.settings);
  const setSettings = useSettingsStore((s) => s.setSettings);

  const [query, setQuery] = useState("");

  if (!open) return null;

  const actions = [
    {
      id: "toggle-autodl",
      label: `Toggle Instant Auto-Download (${
        settings?.auto_download_on_detect ? "Currently ON" : "Currently OFF"
      })`,
      icon: Zap,
      run: async () => {
        if (!settings) return;
        const next = {
          ...settings,
          auto_download_on_detect: !settings.auto_download_on_detect,
        };
        setSettings(next);
        await ipc.updateAppSettings(next);
      },
    },
    {
      id: "nav-workspace",
      label: "Go to Download Workspace & Queue",
      icon: DownloadCloud,
      run: () => setActiveView("workspace"),
    },
    {
      id: "nav-history",
      label: "Open Media Library & Download History",
      icon: Archive,
      run: () => setActiveView("history"),
    },
    {
      id: "nav-settings",
      label: "Open Application Settings",
      icon: Settings,
      run: () => setActiveView("settings"),
    },
    {
      id: "open-batch",
      label: "Open Multi-Platform Batch URL Importer (Ctrl+B)",
      icon: Layers,
      run: () => setBatchModalOpen(true),
    },
    {
      id: "open-auth",
      label: "Configure Instagram / Facebook / YouTube Session Vault",
      icon: KeyRound,
      run: () => setAuthModalOpen(true),
    },
    {
      id: "pause-all",
      label: "Pause All Active Downloads",
      icon: Pause,
      run: () => ipc.pauseAllJobs(),
    },
    {
      id: "resume-all",
      label: "Resume All Paused Downloads",
      icon: Play,
      run: () => ipc.resumeAllJobs(),
    },
    {
      id: "update-binaries",
      label: "Check / Self-Update yt-dlp, gallery-dl, Deno & FFmpeg Engines",
      icon: Cpu,
      run: () => setOnboardingModalOpen(true),
    },
    {
      id: "toggle-theme",
      label: "Toggle Dark / Light Theme",
      icon: Moon,
      run: () => toggleTheme(),
    },
  ];

  const filtered = actions.filter((a) =>
    a.label.toLowerCase().includes(query.toLowerCase())
  );

  return (
    <div
      onClick={() => setOpen(false)}
      className="fixed inset-0 z-50 bg-black/70 backdrop-blur-sm flex items-start justify-center pt-24 p-4"
    >
      <div
        onClick={(e) => e.stopPropagation()}
        className="w-full max-w-lg rounded-2xl bg-[#0F131D] border border-white/[0.12] shadow-2xl overflow-hidden"
      >
        <div className="flex items-center px-4 py-3 border-b border-white/[0.08] gap-2.5">
          <Command className="w-4 h-4 text-violet-400" />
          <input
            type="text"
            autoFocus
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Type a command or action..."
            className="w-full bg-transparent text-xs text-slate-100 placeholder:text-slate-500 focus:outline-none"
          />
          <kbd className="text-[10px] font-mono px-1.5 py-0.5 rounded bg-white/[0.06] text-slate-400">
            ESC
          </kbd>
        </div>

        <div className="max-h-72 overflow-y-auto p-2 space-y-1">
          {filtered.map((item) => {
            const Icon = item.icon;
            return (
              <button
                key={item.id}
                onClick={() => {
                  item.run();
                  setOpen(false);
                }}
                className="w-full flex items-center gap-3 px-3 py-2.5 rounded-xl text-xs text-slate-200 hover:bg-violet-500/15 hover:text-violet-200 transition-colors text-left"
              >
                <Icon className="w-4 h-4 text-violet-400 shrink-0" />
                <span>{item.label}</span>
              </button>
            );
          })}
        </div>
      </div>
    </div>
  );
};

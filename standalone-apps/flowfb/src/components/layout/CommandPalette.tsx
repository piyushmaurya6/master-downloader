import React, { useEffect, useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import {
  Archive,
  ClipboardPaste,
  Cpu,
  FolderOpen,
  KeyRound,
  Layers,
  PauseCircle,
  PlayCircle,
  RotateCcw,
  Search,
  Settings,
  SunMoon,
} from "lucide-react";
import { ipc } from "../../lib/ipc";
import { useSettingsStore } from "../../stores/useSettingsStore";
import { useUIStore } from "../../stores/useUIStore";

interface CommandItem {
  id: string;
  title: string;
  subtitle: string;
  shortcut?: string;
  icon: React.FC<{ className?: string }>;
  action: () => void;
}

export const CommandPalette: React.FC<{
  onAnalyzeUrl: (url: string) => void;
}> = ({ onAnalyzeUrl }) => {
  const open = useUIStore((s) => s.commandPaletteOpen);
  const setOpen = useUIStore((s) => s.setCommandPaletteOpen);
  const setActiveView = useUIStore((s) => s.setActiveView);
  const setBatchModalOpen = useUIStore((s) => s.setBatchModalOpen);
  const setAuthModalOpen = useUIStore((s) => s.setAuthModalOpen);
  const setOnboardingModalOpen = useUIStore((s) => s.setOnboardingModalOpen);
  const setOmnibarUrl = useUIStore((s) => s.setOmnibarUrl);
  const toggleTheme = useUIStore((s) => s.toggleTheme);

  const settings = useSettingsStore((s) => s.settings);

  const [query, setQuery] = useState("");
  const [selectedIndex, setSelectedIndex] = useState(0);

  const commands: CommandItem[] = [
    {
      id: "paste-analyze",
      title: "Paste & Inspect Facebook Link from Clipboard",
      subtitle: "Reads clipboard URL and opens the live Facebook Media Inspector",
      shortcut: "Ctrl+V",
      icon: ClipboardPaste,
      action: async () => {
        try {
          const text = await navigator.clipboard.readText();
          if (text) {
            setActiveView("workspace");
            setOmnibarUrl(text.trim());
            onAnalyzeUrl(text.trim());
          }
        } catch {
          // ignore
        }
      },
    },
    {
      id: "batch-import",
      title: "Open Batch Facebook URL Downloader",
      subtitle: "Import multiple Facebook Reels, Watch, or Post links (.txt/.csv)",
      shortcut: "Ctrl+B",
      icon: Layers,
      action: () => setBatchModalOpen(true),
    },
    {
      id: "resume-all",
      title: "Start / Resume All Queued Downloads",
      subtitle: "Resumes paused jobs and resets rate-limit cooldown",
      icon: PlayCircle,
      action: () => {
        ipc.resumeAllJobs();
      },
    },
    {
      id: "pause-all",
      title: "Pause All Active Downloads",
      subtitle: "Cleanly pauses active workers",
      icon: PauseCircle,
      action: () => {
        ipc.pauseAllJobs();
      },
    },
    {
      id: "retry-failed",
      title: "Retry Failed Downloads",
      subtitle: "Re-enqueues failed or cancelled jobs",
      icon: RotateCcw,
      action: () => {
        ipc.retryFailedJobs();
      },
    },
    {
      id: "open-output-folder",
      title: "Open Default Download Folder in Windows Explorer",
      subtitle: settings?.output_dir ?? "Downloads/FlowFB",
      icon: FolderOpen,
      action: () => {
        if (settings?.output_dir) {
          ipc.revealInExplorer(settings.output_dir);
        }
      },
    },
    {
      id: "connect-account",
      title: "Connect Facebook Account / Session Vault",
      subtitle: "Sign in via isolated WebView2 or import cookies.txt",
      icon: KeyRound,
      action: () => setAuthModalOpen(true),
    },
    {
      id: "update-engines",
      title: "Check & Update Media Engines (yt-dlp, gallery-dl, FFmpeg)",
      subtitle: "Verify SHA-256 checksums and fetch latest releases",
      icon: Cpu,
      action: () => setOnboardingModalOpen(true),
    },
    {
      id: "nav-history",
      title: "Go to Download History & Media Library",
      subtitle: "Search downloaded Facebook Reels, Watch videos, and Audio",
      icon: Archive,
      action: () => setActiveView("history"),
    },
    {
      id: "nav-settings",
      title: "Go to Settings & Diagnostic Console",
      subtitle: "Configure templates, concurrency, proxy, and logs",
      icon: Settings,
      action: () => setActiveView("settings"),
    },
    {
      id: "toggle-theme",
      title: "Toggle Dark / Light Theme",
      subtitle: "Switch between Obsidian Dark and Alabaster Light",
      icon: SunMoon,
      action: () => toggleTheme(),
    },
  ];

  const filtered = commands.filter(
    (c) =>
      c.title.toLowerCase().includes(query.toLowerCase()) ||
      c.subtitle.toLowerCase().includes(query.toLowerCase())
  );

  useEffect(() => {
    setSelectedIndex(0);
  }, [query, open]);

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        setOpen(false);
      } else if (e.key === "ArrowDown") {
        e.preventDefault();
        setSelectedIndex((i) => (filtered.length ? (i + 1) % filtered.length : 0));
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        setSelectedIndex((i) =>
          filtered.length ? (i - 1 + filtered.length) % filtered.length : 0
        );
      } else if (e.key === "Enter" && filtered[selectedIndex]) {
        e.preventDefault();
        filtered[selectedIndex].action();
        setOpen(false);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, filtered, selectedIndex, setOpen]);

  return (
    <AnimatePresence>
      {open && (
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          className="fixed inset-0 z-50 bg-black/70 backdrop-blur-sm flex items-start justify-center pt-24 px-4"
          onClick={() => setOpen(false)}
        >
          <motion.div
            initial={{ opacity: 0, scale: 0.96, y: -10 }}
            animate={{ opacity: 1, scale: 1, y: 0 }}
            exit={{ opacity: 0, scale: 0.96, y: -10 }}
            transition={{ type: "spring", stiffness: 450, damping: 32 }}
            onClick={(e) => e.stopPropagation()}
            className="w-full max-w-xl rounded-2xl bg-[#0F131D] border border-white/10 shadow-2xl overflow-hidden"
          >
            <div className="flex items-center gap-3 px-4 py-3.5 border-b border-white/[0.08]">
              <Search className="w-4 h-4 text-blue-400 shrink-0" />
              <input
                type="text"
                autoFocus
                value={query}
                onChange={(e) => setQuery(e.target.value)}
                placeholder="Type a command or paste a Facebook URL..."
                className="w-full bg-transparent text-sm text-slate-100 placeholder-slate-500 focus:outline-none"
              />
              <kbd className="px-1.5 py-0.5 text-[10px] font-mono bg-white/[0.06] text-slate-400 rounded">
                ESC
              </kbd>
            </div>

            <div className="max-h-80 overflow-y-auto p-2 space-y-1">
              {filtered.map((cmd, idx) => {
                const Icon = cmd.icon;
                const active = idx === selectedIndex;
                return (
                  <button
                    key={cmd.id}
                    onMouseEnter={() => setSelectedIndex(idx)}
                    onClick={() => {
                      cmd.action();
                      setOpen(false);
                    }}
                    className={`w-full flex items-center justify-between px-3 py-2.5 rounded-xl text-left transition-colors ${
                      active
                        ? "bg-blue-500/20 text-slate-100 border border-blue-500/30"
                        : "text-slate-300 hover:bg-white/[0.04] border border-transparent"
                    }`}
                  >
                    <div className="flex items-center gap-3 min-w-0">
                      <div className="p-2 rounded-lg bg-white/[0.04] text-blue-400">
                        <Icon className="w-4 h-4" />
                      </div>
                      <div className="min-w-0">
                        <div className="text-xs font-medium text-slate-100 truncate">
                          {cmd.title}
                        </div>
                        <div className="text-[11px] text-slate-400 truncate">
                          {cmd.subtitle}
                        </div>
                      </div>
                    </div>
                    {cmd.shortcut && (
                      <kbd className="ml-2 px-2 py-0.5 text-[10px] font-mono rounded bg-white/[0.06] text-slate-400 shrink-0">
                        {cmd.shortcut}
                      </kbd>
                    )}
                  </button>
                );
              })}
            </div>
          </motion.div>
        </motion.div>
      )}
    </AnimatePresence>
  );
};

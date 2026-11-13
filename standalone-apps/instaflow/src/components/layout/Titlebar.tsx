import React from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  Command,
  Cpu,
  Lock,
  Minus,
  Moon,
  ShieldCheck,
  Sparkles,
  Square,
  Sun,
  X,
} from "lucide-react";
import { isTauriRuntime } from "../../lib/ipc";
import { useSettingsStore } from "../../stores/useSettingsStore";
import { useUIStore } from "../../stores/useUIStore";

export const Titlebar: React.FC = () => {
  const theme = useUIStore((s) => s.theme);
  const toggleTheme = useUIStore((s) => s.toggleTheme);
  const setCommandPaletteOpen = useUIStore((s) => s.setCommandPaletteOpen);
  const setAuthModalOpen = useUIStore((s) => s.setAuthModalOpen);
  const setOnboardingModalOpen = useUIStore((s) => s.setOnboardingModalOpen);

  const authStatus = useSettingsStore((s) => s.authStatus);
  const binaryStatus = useSettingsStore((s) => s.binaryStatus);

  const handleMinimize = () => {
    if (isTauriRuntime()) getCurrentWindow().minimize();
  };

  const handleToggleMaximize = () => {
    if (isTauriRuntime()) getCurrentWindow().toggleMaximize();
  };

  const handleClose = () => {
    if (isTauriRuntime()) getCurrentWindow().close();
  };

  const allReady = binaryStatus?.all_ready ?? false;
  const isAuthenticated = authStatus?.authenticated ?? false;

  return (
    <header
      data-tauri-drag-region
      className="h-9 w-full flex items-center justify-between px-3 bg-[#09090B] border-b border-white/[0.07] select-none shrink-0 z-40"
    >
      {/* Left: Brand Identity */}
      <div className="flex items-center gap-2.5" data-tauri-drag-region>
        <div className="w-5 h-5 rounded-md bg-gradient-to-br from-indigo-500 to-violet-600 flex items-center justify-center shadow-sm shadow-indigo-500/30">
          <Sparkles className="w-3 h-3 text-white" />
        </div>
        <span className="text-xs font-semibold tracking-tight text-zinc-100">
          InstaFlow
        </span>
        <span className="text-[10px] font-mono px-1.5 py-0.5 rounded bg-white/[0.05] text-zinc-400 border border-white/[0.06]">
          v1.0.0
        </span>
      </div>

      {/* Center: Quick Command Trigger + Status Pills */}
      <div className="flex items-center gap-2">
        <button
          onClick={() => setCommandPaletteOpen(true)}
          className="flex items-center gap-2 px-2.5 py-1 rounded-md bg-[#121215] hover:bg-zinc-800/80 border border-white/[0.07] text-[11px] text-zinc-400 hover:text-zinc-200 transition-colors"
          title="Open Command Palette (Ctrl+K)"
        >
          <Command className="w-3 h-3 text-indigo-400" />
          <span>Command Palette</span>
          <kbd className="ml-1.5 px-1 py-0.2 text-[10px] font-mono bg-white/[0.06] rounded text-zinc-300">
            Ctrl+K
          </kbd>
        </button>

        {/* Engine Readiness Pill */}
        <button
          onClick={() => setOnboardingModalOpen(true)}
          className={`flex items-center gap-1.5 px-2 py-0.5 rounded-full text-[11px] font-medium border transition-colors ${
            allReady
              ? "bg-emerald-500/10 text-emerald-300 border-emerald-500/25 hover:bg-emerald-500/20"
              : "bg-amber-500/10 text-amber-300 border-amber-500/25 hover:bg-amber-500/20"
          }`}
          title="Inspect or update standalone yt-dlp, gallery-dl, and FFmpeg binaries"
        >
          <Cpu className="w-3 h-3" />
          <span>{allReady ? "Engines Ready" : "Setup Engines"}</span>
        </button>

        {/* Auth Session Pill */}
        <button
          onClick={() => setAuthModalOpen(true)}
          className={`flex items-center gap-1.5 px-2 py-0.5 rounded-full text-[11px] font-medium border transition-colors ${
            isAuthenticated
              ? "bg-indigo-500/15 text-indigo-300 border-indigo-500/30 hover:bg-indigo-500/25"
              : "bg-zinc-800/70 text-zinc-400 border-white/[0.07] hover:text-zinc-200"
          }`}
          title="Configure Instagram Session Vault"
        >
          {isAuthenticated ? (
            <>
              <ShieldCheck className="w-3 h-3 text-indigo-400" />
              <span className="max-w-[110px] truncate">
                {authStatus?.username ? `@${authStatus.username}` : "Session Active"}
              </span>
            </>
          ) : (
            <>
              <Lock className="w-3 h-3 text-zinc-400" />
              <span>Anonymous</span>
            </>
          )}
        </button>
      </div>

      {/* Right: Theme Toggle & Native Window Chrome */}
      <div className="flex items-center gap-1">
        <button
          onClick={toggleTheme}
          className="h-7 w-7 inline-flex items-center justify-center rounded-md text-zinc-400 hover:text-zinc-100 hover:bg-white/[0.06] transition-colors"
          title={`Switch to ${theme === "dark" ? "Light" : "Dark"} mode`}
        >
          {theme === "dark" ? (
            <Sun className="w-3.5 h-3.5" />
          ) : (
            <Moon className="w-3.5 h-3.5" />
          )}
        </button>

        <button
          onClick={handleMinimize}
          className="h-7 w-8 inline-flex items-center justify-center rounded-md text-zinc-400 hover:text-zinc-100 hover:bg-white/[0.06] transition-colors"
          aria-label="Minimize window"
        >
          <Minus className="w-3.5 h-3.5" />
        </button>
        <button
          onClick={handleToggleMaximize}
          className="h-7 w-8 inline-flex items-center justify-center rounded-md text-zinc-400 hover:text-zinc-100 hover:bg-white/[0.06] transition-colors"
          aria-label="Maximize window"
        >
          <Square className="w-3 h-3" />
        </button>
        <button
          onClick={handleClose}
          className="h-7 w-8 inline-flex items-center justify-center rounded-md text-zinc-400 hover:text-white hover:bg-rose-600/90 transition-colors"
          aria-label="Close window"
        >
          <X className="w-3.5 h-3.5" />
        </button>
      </div>
    </header>
  );
};

import React, { useRef } from "react";
import {
  Search,
  Loader2,
  Clipboard,
  Layers,
  X,
  ArrowRight,
  Lock,
} from "lucide-react";
import { useUIStore } from "../../stores/useUIStore";
import { classifyFacebookUrlClient } from "../../lib/utils";
import { ipc, parseSerializedError } from "../../lib/ipc";

export const SmartOmnibar: React.FC = () => {
  const inputRef = useRef<HTMLInputElement>(null);
  const {
    omnibarInput,
    setOmnibarInput,
    isInspecting,
    setIsInspecting,
    setInspectedMedia,
    setBatchModalOpen,
    setAuthModalOpen,
    addToast,
  } = useUIStore();

  const classification = classifyFacebookUrlClient(omnibarInput);

  const handleAnalyze = async (urlToAnalyze?: string) => {
    const target = (urlToAnalyze ?? omnibarInput).trim();
    if (!target) return;

    const check = classifyFacebookUrlClient(target);
    if (!check.valid) {
      addToast({
        type: "error",
        title: "Invalid Facebook URL",
        message:
          "Please paste a valid Facebook Video, Reel, Watch, fb.watch, Share, or Post link.",
      });
      return;
    }

    setIsInspecting(true);
    try {
      const result = await ipc.analyzeFacebookUrl(target);
      setInspectedMedia(result);
    } catch (err) {
      const parsed = parseSerializedError(err);
      if (parsed.code === "AUTH_REQUIRED") {
        addToast({
          type: "warning",
          title: "Authentication Required",
          message: parsed.user_message,
          actionLabel: "Connect Session",
          onAction: () => setAuthModalOpen(true),
        });
      } else {
        addToast({
          type: "error",
          title: "Analysis Failed",
          message: parsed.user_message,
        });
      }
    } finally {
      setIsInspecting(false);
    }
  };

  const handlePasteClipboard = async () => {
    try {
      const text = await navigator.clipboard.readText();
      if (text) {
        setOmnibarInput(text.trim());
        const check = classifyFacebookUrlClient(text.trim());
        if (check.valid) {
          handleAnalyze(text.trim());
        }
      }
    } catch {
      inputRef.current?.focus();
    }
  };

  const getBadgeStyle = (badge: string | null) => {
    switch (badge) {
      case "REEL":
        return "bg-sky-500/15 text-sky-400 border-sky-500/30";
      case "WATCH":
      case "VIDEO":
        return "bg-blue-500/15 text-blue-400 border-blue-500/30";
      case "FB.WATCH":
      case "SHARE":
        return "bg-indigo-500/15 text-indigo-400 border-indigo-500/30";
      case "PHOTO":
        return "bg-emerald-500/15 text-emerald-400 border-emerald-500/30";
      case "STORY":
        return "bg-amber-500/15 text-amber-400 border-amber-500/30";
      default:
        return "bg-blue-500/15 text-blue-400 border-blue-500/30";
    }
  };

  return (
    <div className="w-full">
      <div
        className={`relative flex items-center h-[52px] rounded-xl bg-bg-surface border transition-all duration-200 px-3.5 gap-3 ${
          classification.valid
            ? "border-blue-500/50 shadow-[0_0_24px_-6px_rgba(24,119,242,0.35)]"
            : "border-border-subtle focus-within:border-border-focus"
        }`}
      >
        {isInspecting ? (
          <Loader2 className="w-4 h-4 text-accent-primary animate-spin shrink-0" />
        ) : (
          <Search className="w-4 h-4 text-text-muted shrink-0" />
        )}

        {classification.valid && classification.badge && (
          <span
            className={`px-2 py-0.5 text-[10px] font-mono font-semibold uppercase tracking-wider rounded-md border shrink-0 ${getBadgeStyle(
              classification.badge
            )}`}
          >
            {classification.badge}
          </span>
        )}

        <input
          id="smart-omnibar-input"
          ref={inputRef}
          type="text"
          value={omnibarInput}
          onChange={(e) => setOmnibarInput(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              e.preventDefault();
              handleAnalyze();
            }
          }}
          placeholder="Paste Facebook Reel, Watch, Video, fb.watch, Share, or Post link... (Ctrl+L)"
          className="flex-1 bg-transparent text-[13px] text-text-primary placeholder:text-text-muted focus:outline-none font-mono"
          disabled={isInspecting}
        />

        {omnibarInput && (
          <button
            onClick={() => setOmnibarInput("")}
            title="Clear input"
            className="p-1.5 rounded-lg text-text-muted hover:text-text-primary hover:bg-bg-elevated transition-colors"
          >
            <X className="w-3.5 h-3.5" />
          </button>
        )}

        <div className="flex items-center gap-1.5 shrink-0">
          <button
            onClick={handlePasteClipboard}
            disabled={isInspecting}
            title="Paste & Analyze from Clipboard"
            className="flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg text-xs font-medium text-text-secondary hover:text-text-primary bg-bg-elevated/70 hover:bg-bg-elevated border border-border-subtle transition-colors"
          >
            <Clipboard className="w-3.5 h-3.5" />
            <span className="hidden sm:inline">Paste</span>
          </button>

          <button
            onClick={() => setBatchModalOpen(true)}
            disabled={isInspecting}
            title="Batch Multi-URL Import (Ctrl+B)"
            className="flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg text-xs font-medium text-text-secondary hover:text-text-primary bg-bg-elevated/70 hover:bg-bg-elevated border border-border-subtle transition-colors"
          >
            <Layers className="w-3.5 h-3.5 text-sky-400" />
            <span className="hidden sm:inline">Batch</span>
          </button>

          <button
            onClick={() => handleAnalyze()}
            disabled={isInspecting || !omnibarInput.trim()}
            className="flex items-center gap-1.5 px-3.5 py-1.5 rounded-lg text-xs font-semibold text-white fb-gradient-bg hover:opacity-95 disabled:opacity-40 transition-all shadow-sm"
          >
            <span>{isInspecting ? "Inspecting..." : "Inspect"}</span>
            <ArrowRight className="w-3.5 h-3.5" />
          </button>
        </div>
      </div>

      {/* Quick Hint Bar */}
      <div className="flex items-center justify-between px-1 mt-2 text-[11px] text-text-muted">
        <div className="flex items-center gap-3 flex-wrap">
          <span>Supports:</span>
          <span className="font-mono text-text-secondary">/reel/*</span>
          <span className="font-mono text-text-secondary">/watch/?v=*</span>
          <span className="font-mono text-text-secondary">fb.watch/*</span>
          <span className="font-mono text-text-secondary">/share/v|r|p/*</span>
          <span className="font-mono text-text-secondary">/videos/*</span>
        </div>
        <button
          onClick={() => setAuthModalOpen(true)}
          className="flex items-center gap-1 text-text-secondary hover:text-accent-primary transition-colors"
        >
          <Lock className="w-3 h-3" />
          <span>Auth Vault for Followers-Only / Group Media</span>
        </button>
      </div>
    </div>
  );
};

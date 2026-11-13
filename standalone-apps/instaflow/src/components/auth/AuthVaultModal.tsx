import React, { useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import {
  FileKey,
  Globe,
  KeyRound,
  Lock,
  LogOut,
  ShieldCheck,
  X,
} from "lucide-react";
import { ipc, parseIpcError } from "../../lib/ipc";
import { useSettingsStore } from "../../stores/useSettingsStore";
import { useUIStore } from "../../stores/useUIStore";

export const AuthVaultModal: React.FC = () => {
  const open = useUIStore((s) => s.authModalOpen);
  const setOpen = useUIStore((s) => s.setAuthModalOpen);
  const addToast = useUIStore((s) => s.addToast);

  const authStatus = useSettingsStore((s) => s.authStatus);
  const setAuthStatus = useSettingsStore((s) => s.setAuthStatus);

  const [cookieText, setCookieText] = useState("");
  const [usernameHint, setUsernameHint] = useState("");
  const [loading, setLoading] = useState(false);

  const handleOpenWebviewLogin = async () => {
    setLoading(true);
    try {
      await ipc.openInstagramLoginWebview();
      addToast({
        title: "Secure Instagram Webview Launched",
        description:
          "Log in naturally in the secondary window. Your session cookies will be encrypted into Windows Credential Manager automatically.",
        variant: "default",
      });
    } catch (err) {
      addToast({
        title: "Could Not Open Login Window",
        description: parseIpcError(err).user_message,
        variant: "error",
      });
    } finally {
      setLoading(false);
    }
  };

  const handleImportCookieTxt = async () => {
    if (!cookieText.trim()) return;
    setLoading(true);
    try {
      const status = await ipc.importCookiesTxt(
        cookieText,
        usernameHint.trim() || null
      );
      setAuthStatus(status);
      setCookieText("");
      addToast({
        title: "Cookies Encrypted into Vault",
        description:
          "Sealed with AES-256-GCM and stored master key in Windows Credential Manager.",
        variant: "success",
      });
    } catch (err) {
      addToast({
        title: "Cookie Import Failed",
        description: parseIpcError(err).user_message,
        variant: "error",
      });
    } finally {
      setLoading(false);
    }
  };

  const handleSelectBrowser = async (browser: string | null) => {
    setLoading(true);
    try {
      const status = await ipc.setBrowserCookieMode(browser);
      setAuthStatus(status);
      addToast({
        title: browser
          ? `Delegated to ${browser.toUpperCase()} Cookies`
          : "Switched to Anonymous Mode",
        variant: "success",
      });
    } catch (err) {
      addToast({
        title: "Failed to Update Browser Mode",
        description: parseIpcError(err).user_message,
        variant: "error",
      });
    } finally {
      setLoading(false);
    }
  };

  const handleLogout = async () => {
    setLoading(true);
    try {
      const status = await ipc.logoutAndWipeVault();
      setAuthStatus(status);
      addToast({
        title: "Session Vault Wiped",
        description:
          "Encrypted vault zeroized and master key removed from Windows Credential Manager.",
        variant: "default",
      });
    } catch (err) {
      addToast({
        title: "Vault Wipe Error",
        description: parseIpcError(err).user_message,
        variant: "error",
      });
    } finally {
      setLoading(false);
    }
  };

  return (
    <AnimatePresence>
      {open && (
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          className="fixed inset-0 z-50 bg-black/75 backdrop-blur-md flex items-center justify-center p-4"
          onClick={() => setOpen(false)}
        >
          <motion.div
            initial={{ opacity: 0, scale: 0.95, y: 10 }}
            animate={{ opacity: 1, scale: 1, y: 0 }}
            exit={{ opacity: 0, scale: 0.95, y: 10 }}
            onClick={(e) => e.stopPropagation()}
            className="w-full max-w-xl rounded-2xl bg-[#121215] border border-white/10 shadow-2xl overflow-hidden"
          >
            {/* Header */}
            <div className="px-6 py-4 border-b border-white/[0.07] flex items-center justify-between">
              <div className="flex items-center gap-3">
                <div className="p-2 rounded-xl bg-emerald-500/15 border border-emerald-500/30 text-emerald-400">
                  <KeyRound className="w-5 h-5" />
                </div>
                <div>
                  <h2 className="text-sm font-semibold text-zinc-100">
                    Instagram Authentication &amp; Encrypted Session Vault
                  </h2>
                  <p className="text-xs text-zinc-400">
                    AES-256-GCM encrypted cookie vault backed by Windows Credential Manager
                  </p>
                </div>
              </div>
              <button
                onClick={() => setOpen(false)}
                className="p-1.5 rounded-lg text-zinc-400 hover:text-zinc-100 hover:bg-white/[0.06]"
              >
                <X className="w-4 h-4" />
              </button>
            </div>

            <div className="p-6 space-y-5 max-h-[78vh] overflow-y-auto">
              {/* Current Status Banner */}
              <div className="p-4 rounded-xl bg-[#09090B] border border-white/[0.07] flex items-center justify-between">
                <div className="flex items-center gap-3">
                  {authStatus?.authenticated ? (
                    <ShieldCheck className="w-5 h-5 text-emerald-400" />
                  ) : (
                    <Lock className="w-5 h-5 text-zinc-400" />
                  )}
                  <div>
                    <div className="text-xs font-semibold text-zinc-100">
                      {authStatus?.authenticated
                        ? `Authenticated (${authStatus.username ?? authStatus.mode})`
                        : "Anonymous Mode (Public Posts & Reels Only)"}
                    </div>
                    <div className="text-[11px] text-zinc-400">
                      {authStatus?.authenticated
                        ? "Stories, Highlights, and high-resolution carousels unlocked."
                        : "Connect an account to download Stories, Highlights, or age-gated media."}
                    </div>
                  </div>
                </div>

                {authStatus?.authenticated && (
                  <button
                    onClick={handleLogout}
                    disabled={loading}
                    className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium bg-rose-500/15 text-rose-300 hover:bg-rose-500/25 border border-rose-500/30 transition-colors"
                  >
                    <LogOut className="w-3.5 h-3.5" />
                    Wipe Vault
                  </button>
                )}
              </div>

              {/* Method 1: Isolated Tauri Webview2 Login (Recommended) */}
              <div className="p-4 rounded-xl bg-[#09090B] border border-indigo-500/25 space-y-3">
                <div className="flex items-center justify-between">
                  <span className="text-xs font-semibold text-indigo-300 flex items-center gap-2">
                    <Globe className="w-4 h-4 text-indigo-400" />
                    1. Sandboxed WebView2 Login (Recommended)
                  </span>
                  <span className="text-[10px] font-mono px-2 py-0.5 rounded bg-indigo-500/20 text-indigo-300">
                    2FA Safe
                  </span>
                </div>
                <p className="text-xs text-zinc-400 leading-relaxed">
                  Opens an official Instagram login window inside an isolated WebView2 session.
                  InstaFlow never sees your password—once logged in, session cookies are sealed
                  with AES-256-GCM into <code className="text-zinc-300">vault.bin</code>.
                </p>
                <button
                  onClick={handleOpenWebviewLogin}
                  disabled={loading}
                  className="w-full py-2.5 px-4 rounded-lg text-xs font-semibold text-white bg-gradient-to-r from-indigo-500 to-violet-600 hover:from-indigo-400 hover:to-violet-500 shadow-lg shadow-indigo-500/20 transition-all"
                >
                  Connect Instagram Account via Secure Webview
                </button>
              </div>

              {/* Method 2: Local Browser Cookie Delegation */}
              <div className="p-4 rounded-xl bg-[#09090B] border border-white/[0.07] space-y-3">
                <div className="text-xs font-semibold text-zinc-200">
                  2. Delegate to Local Desktop Browser Profile
                </div>
                <div className="grid grid-cols-5 gap-2">
                  {(["none", "edge", "chrome", "firefox", "brave"] as const).map((b) => {
                    const active =
                      (b === "none" && authStatus?.mode === "anonymous") ||
                      (authStatus?.mode === "browser_cookies" &&
                        authStatus?.selected_browser === b);
                    return (
                      <button
                        key={b}
                        onClick={() => handleSelectBrowser(b === "none" ? null : b)}
                        className={`py-2 px-2.5 rounded-lg text-xs font-medium capitalize border transition-colors ${
                          active
                            ? "bg-indigo-500/20 text-indigo-200 border-indigo-500/40"
                            : "bg-white/[0.03] text-zinc-400 border-white/[0.07] hover:text-zinc-200"
                        }`}
                      >
                        {b}
                      </button>
                    );
                  })}
                </div>
              </div>

              {/* Method 3: Import Netscape cookies.txt */}
              <div className="p-4 rounded-xl bg-[#09090B] border border-white/[0.07] space-y-3">
                <div className="flex items-center justify-between">
                  <span className="text-xs font-semibold text-zinc-200 flex items-center gap-2">
                    <FileKey className="w-4 h-4 text-violet-400" />
                    3. Import Netscape cookies.txt into Encrypted Vault
                  </span>
                </div>
                <input
                  type="text"
                  value={usernameHint}
                  onChange={(e) => setUsernameHint(e.target.value)}
                  placeholder="Account label (optional, e.g. @my_handle)"
                  className="w-full px-3 py-1.5 rounded-lg bg-[#121215] border border-white/[0.08] text-xs text-zinc-200 placeholder-zinc-500 focus:outline-none focus:border-indigo-500"
                />
                <textarea
                  rows={3}
                  value={cookieText}
                  onChange={(e) => setCookieText(e.target.value)}
                  placeholder="# Netscape HTTP Cookie File&#10;.instagram.com	TRUE	/	TRUE	1900000000	sessionid	..."
                  className="w-full p-2.5 rounded-lg bg-[#121215] border border-white/[0.08] text-[11px] font-mono text-zinc-200 placeholder-zinc-600 focus:outline-none focus:border-indigo-500"
                />
                <button
                  onClick={handleImportCookieTxt}
                  disabled={!cookieText.trim() || loading}
                  className="px-4 py-2 rounded-lg text-xs font-medium bg-white/[0.07] hover:bg-white/[0.12] text-zinc-100 disabled:opacity-40 transition-colors"
                >
                  Encrypt &amp; Save to Vault
                </button>
              </div>
            </div>
          </motion.div>
        </motion.div>
      )}
    </AnimatePresence>
  );
};

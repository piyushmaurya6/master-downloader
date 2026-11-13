import React, { useState } from "react";
import {
  CheckCircle2,
  FileKey,
  Globe,
  KeyRound,
  LogOut,
  ShieldCheck,
  X,
} from "lucide-react";
import { ipc, parseIpcError } from "../../lib/ipc";
import { useSettingsStore } from "../../stores/useSettingsStore";
import { useUIStore } from "../../stores/useUIStore";

export const AuthVaultModal: React.FC = () => {
  const authModalOpen = useUIStore((s) => s.authModalOpen);
  const setAuthModalOpen = useUIStore((s) => s.setAuthModalOpen);
  const addToast = useUIStore((s) => s.addToast);

  const authStatus = useSettingsStore((s) => s.authStatus);
  const setAuthStatus = useSettingsStore((s) => s.setAuthStatus);

  const [activeTab, setActiveTab] = useState<
    "webview" | "cookies_txt" | "browser"
  >("webview");
  const [cookieText, setCookieText] = useState("");
  const [usernameHint, setUsernameHint] = useState("");
  const [selectedBrowser, setSelectedBrowser] = useState(
    authStatus?.browser_source ?? "firefox"
  );
  const [busy, setBusy] = useState(false);

  if (!authModalOpen) return null;

  const handleOpenInstagramWebview = async () => {
    setBusy(true);
    try {
      await ipc.openInstagramLoginWebview();
      addToast({
        title: "Instagram Login Window Opened",
        description:
          "Sign in inside the isolated window. Your sessionid cookie will be encrypted automatically.",
        variant: "info",
      });
    } catch (err) {
      const s = parseIpcError(err);
      addToast({
        title: "Could Not Open Login Window",
        description: s.user_message,
        variant: "error",
      });
    } finally {
      setBusy(false);
    }
  };

  const handleOpenFacebookWebview = async () => {
    setBusy(true);
    try {
      await ipc.openFacebookLoginWebview();
      addToast({
        title: "Facebook Login Window Opened",
        description:
          "Sign in inside the isolated window. Your c_user + xs cookies will be encrypted automatically.",
        variant: "info",
      });
    } catch (err) {
      const s = parseIpcError(err);
      addToast({
        title: "Could Not Open Login Window",
        description: s.user_message,
        variant: "error",
      });
    } finally {
      setBusy(false);
    }
  };

  const handleImportCookies = async () => {
    if (!cookieText.trim()) return;
    setBusy(true);
    try {
      const status = await ipc.importCookiesTxt(
        cookieText,
        usernameHint.trim() || null
      );
      setAuthStatus(status);
      setCookieText("");
      addToast({
        title: "Cookies Encrypted in Vault",
        description: `Stored ${status.cookie_count} cookies with AES-256-GCM encryption.`,
        variant: "success",
      });
    } catch (err) {
      const s = parseIpcError(err);
      addToast({
        title: "Cookie Import Failed",
        description: s.user_message,
        variant: "error",
      });
    } finally {
      setBusy(false);
    }
  };

  const handleApplyBrowserMode = async () => {
    setBusy(true);
    try {
      const status = await ipc.setBrowserCookieMode(selectedBrowser);
      setAuthStatus(status);
      addToast({
        title: "Browser Cookie Delegation Enabled",
        description: `yt-dlp and gallery-dl will read sessions from ${selectedBrowser}.`,
        variant: "success",
      });
    } catch (err) {
      const s = parseIpcError(err);
      addToast({
        title: "Browser Delegation Failed",
        description: s.user_message,
        variant: "error",
      });
    } finally {
      setBusy(false);
    }
  };

  const handleLogout = async () => {
    setBusy(true);
    try {
      const status = await ipc.logoutAndWipeVault();
      setAuthStatus(status);
      addToast({
        title: "Session Vault Wiped",
        description: "All stored cookies have been securely erased.",
        variant: "info",
      });
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 bg-black/75 backdrop-blur-sm flex items-center justify-center p-4">
      <div className="w-full max-w-xl rounded-2xl bg-[#0F131D] border border-white/[0.1] shadow-2xl overflow-hidden">
        {/* Header */}
        <div className="flex items-center justify-between px-5 py-4 border-b border-white/[0.08]">
          <div className="flex items-center gap-2.5">
            <div className="w-8 h-8 rounded-lg bg-emerald-500/15 border border-emerald-500/30 flex items-center justify-center">
              <KeyRound className="w-4 h-4 text-emerald-400" />
            </div>
            <div>
              <h3 className="text-sm font-semibold text-slate-100">
                Multi-Platform Session &amp; Cookie Vault (AES-256-GCM)
              </h3>
              <p className="text-[11px] text-slate-400">
                Required only for Instagram Stories/Highlights, Private FB Posts, or Age-Restricted YouTube
              </p>
            </div>
          </div>
          <button
            onClick={() => setAuthModalOpen(false)}
            className="p-1.5 rounded-lg text-slate-400 hover:text-slate-200 hover:bg-white/[0.06]"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* Live Platform Status Banner */}
        <div className="px-5 py-3 bg-[#07090E] border-b border-white/[0.08] flex items-center justify-between">
          <div className="flex items-center gap-2 flex-wrap">
            <ShieldCheck
              className={`w-4 h-4 ${
                authStatus?.authenticated
                  ? "text-emerald-400"
                  : "text-slate-500"
              }`}
            />
            <span
              className={`px-2 py-0.5 rounded text-[10px] font-mono border ${
                authStatus?.instagram_authenticated
                  ? "bg-fuchsia-500/20 text-fuchsia-300 border-fuchsia-500/35"
                  : "bg-white/[0.04] text-slate-500 border-white/[0.08]"
              }`}
            >
              IG: {authStatus?.instagram_authenticated ? "ACTIVE" : "PUBLIC"}
            </span>
            <span
              className={`px-2 py-0.5 rounded text-[10px] font-mono border ${
                authStatus?.facebook_authenticated
                  ? "bg-blue-500/20 text-blue-300 border-blue-500/35"
                  : "bg-white/[0.04] text-slate-500 border-white/[0.08]"
              }`}
            >
              FB: {authStatus?.facebook_authenticated ? "ACTIVE" : "PUBLIC"}
            </span>
            <span
              className={`px-2 py-0.5 rounded text-[10px] font-mono border ${
                authStatus?.youtube_authenticated
                  ? "bg-rose-500/20 text-rose-300 border-rose-500/35"
                  : "bg-white/[0.04] text-slate-500 border-white/[0.08]"
              }`}
            >
              YT: {authStatus?.youtube_authenticated ? "ACTIVE" : "PUBLIC"}
            </span>
          </div>

          {authStatus?.authenticated && (
            <button
              onClick={handleLogout}
              disabled={busy}
              className="flex items-center gap-1 px-2.5 py-1 rounded-md bg-rose-500/15 hover:bg-rose-500/25 text-rose-300 text-[11px] font-medium"
            >
              <LogOut className="w-3 h-3" />
              <span>Wipe Vault</span>
            </button>
          )}
        </div>

        {/* Method Tabs */}
        <div className="grid grid-cols-3 border-b border-white/[0.08] bg-[#0B0E17] text-xs">
          <button
            onClick={() => setActiveTab("webview")}
            className={`py-2.5 font-medium border-b-2 transition-colors ${
              activeTab === "webview"
                ? "border-violet-500 text-violet-300 bg-violet-500/5"
                : "border-transparent text-slate-400 hover:text-slate-200"
            }`}
          >
            1. Platform Login
          </button>
          <button
            onClick={() => setActiveTab("cookies_txt")}
            className={`py-2.5 font-medium border-b-2 transition-colors ${
              activeTab === "cookies_txt"
                ? "border-violet-500 text-violet-300 bg-violet-500/5"
                : "border-transparent text-slate-400 hover:text-slate-200"
            }`}
          >
            2. Import cookies.txt
          </button>
          <button
            onClick={() => setActiveTab("browser")}
            className={`py-2.5 font-medium border-b-2 transition-colors ${
              activeTab === "browser"
                ? "border-violet-500 text-violet-300 bg-violet-500/5"
                : "border-transparent text-slate-400 hover:text-slate-200"
            }`}
          >
            3. Browser Delegation
          </button>
        </div>

        {/* Tab Body */}
        <div className="p-5 space-y-4">
          {activeTab === "webview" && (
            <div className="space-y-3">
              <p className="text-xs text-slate-300 leading-relaxed">
                Log in directly inside an isolated WebView2 window. Master
                Downloader merges your Instagram and Facebook cookies into the
                encrypted vault without overwriting other platforms.
              </p>

              <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
                <button
                  onClick={handleOpenInstagramWebview}
                  disabled={busy}
                  className="flex items-center justify-center gap-2 py-2.5 px-3 rounded-xl bg-gradient-to-r from-fuchsia-600 to-pink-600 hover:opacity-95 text-xs font-semibold text-white shadow-md transition-all"
                >
                  <Globe className="w-4 h-4" />
                  <span>Login to Instagram</span>
                </button>

                <button
                  onClick={handleOpenFacebookWebview}
                  disabled={busy}
                  className="flex items-center justify-center gap-2 py-2.5 px-3 rounded-xl bg-[#1877F2] hover:bg-blue-600 text-xs font-semibold text-white shadow-md transition-all"
                >
                  <Globe className="w-4 h-4" />
                  <span>Login to Facebook</span>
                </button>
              </div>
            </div>
          )}

          {activeTab === "cookies_txt" && (
            <div className="space-y-3">
              <input
                type="text"
                value={usernameHint}
                onChange={(e) => setUsernameHint(e.target.value)}
                placeholder="Account label (optional, e.g. Personal Profile)"
                className="w-full px-3 py-2 rounded-lg bg-[#07090E] border border-white/[0.08] text-xs text-slate-200"
              />
              <textarea
                rows={5}
                value={cookieText}
                onChange={(e) => setCookieText(e.target.value)}
                placeholder="Paste Netscape HTTP Cookie File lines (.instagram.com, .facebook.com, or .youtube.com)..."
                className="w-full p-3 rounded-xl bg-[#07090E] border border-white/[0.08] text-xs font-mono text-slate-200 resize-none"
              />
              <button
                onClick={handleImportCookies}
                disabled={!cookieText.trim() || busy}
                className="w-full flex items-center justify-center gap-2 py-2 rounded-xl bg-emerald-600 hover:bg-emerald-500 disabled:opacity-40 text-xs font-semibold text-white"
              >
                <FileKey className="w-4 h-4" />
                <span>Encrypt &amp; Save to Vault</span>
              </button>
            </div>
          )}

          {activeTab === "browser" && (
            <div className="space-y-3">
              <p className="text-xs text-slate-300 leading-relaxed">
                Delegate cookies directly from an installed desktop browser profile
                across YouTube, Instagram, and Facebook.
              </p>
              <select
                value={selectedBrowser}
                onChange={(e) => setSelectedBrowser(e.target.value)}
                className="w-full px-3 py-2 rounded-lg bg-[#07090E] border border-white/[0.08] text-xs text-slate-200"
              >
                <option value="firefox">Mozilla Firefox (Recommended)</option>
                <option value="edge">Microsoft Edge</option>
                <option value="chrome">Google Chrome</option>
                <option value="brave">Brave Browser</option>
              </select>
              <button
                onClick={handleApplyBrowserMode}
                disabled={busy}
                className="w-full flex items-center justify-center gap-2 py-2 rounded-xl master-gradient-bg hover:opacity-95 text-xs font-semibold text-white"
              >
                <CheckCircle2 className="w-4 h-4" />
                <span>Use Selected Browser Profile</span>
              </button>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

import { useEffect, useRef } from "react";
import { classifyFacebookUrlClient } from "../lib/utils";
import { useSettingsStore } from "../stores/useSettingsStore";
import { useUIStore } from "../stores/useUIStore";

export function useClipboardMonitor(onDetectedUrl: (url: string) => void) {
  const clipboardMonitorEnabled = useSettingsStore(
    (s) => s.settings?.clipboard_monitor ?? true
  );
  const omnibarUrl = useUIStore((s) => s.omnibarUrl);
  const setOmnibarUrl = useUIStore((s) => s.setOmnibarUrl);
  const addToast = useUIStore((s) => s.addToast);
  const lastSeenRef = useRef<string>("");

  useEffect(() => {
    if (!clipboardMonitorEnabled) return;

    const checkClipboard = async () => {
      try {
        if (!navigator.clipboard || !document.hasFocus()) return;
        const text = await navigator.clipboard.readText();
        if (!text || text === lastSeenRef.current) return;

        const classification = classifyFacebookUrlClient(text);
        if (classification.valid && classification.canonicalUrl !== lastSeenRef.current) {
          lastSeenRef.current = classification.canonicalUrl;
          if (!omnibarUrl.trim()) {
            setOmnibarUrl(classification.canonicalUrl);
            addToast({
              title: `Detected Facebook ${classification.badge}`,
              description: classification.canonicalUrl,
              variant: "default",
              actionLabel: "Inspect",
              onAction: () => onDetectedUrl(classification.canonicalUrl),
            });
          }
        }
      } catch {
        // Clipboard permission denied or empty
      }
    };

    window.addEventListener("focus", checkClipboard);
    return () => {
      window.removeEventListener("focus", checkClipboard);
    };
  }, [clipboardMonitorEnabled, omnibarUrl, setOmnibarUrl, addToast, onDetectedUrl]);
}

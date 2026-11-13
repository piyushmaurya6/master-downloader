import { useEffect } from "react";
import { classifyFacebookUrlClient } from "../lib/utils";
import { useUIStore } from "../stores/useUIStore";

export function useKeyboardShortcuts(onPasteAnalyze: (url: string) => void) {
  const commandPaletteOpen = useUIStore((s) => s.commandPaletteOpen);
  const setCommandPaletteOpen = useUIStore((s) => s.setCommandPaletteOpen);
  const setBatchModalOpen = useUIStore((s) => s.setBatchModalOpen);
  const setActiveView = useUIStore((s) => s.setActiveView);
  const setOmnibarUrl = useUIStore((s) => s.setOmnibarUrl);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Ctrl+K or Cmd+K -> Toggle Global Command Palette
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        setCommandPaletteOpen(!commandPaletteOpen);
        return;
      }

      // Ctrl+B or Ctrl+Shift+B -> Open Batch URL Modal
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "b") {
        e.preventDefault();
        setBatchModalOpen(true);
        return;
      }
    };

    const handleGlobalPaste = (e: ClipboardEvent) => {
      const target = e.target as HTMLElement | null;
      if (
        target &&
        (target.tagName === "INPUT" ||
          target.tagName === "TEXTAREA" ||
          target.isContentEditable)
      ) {
        return;
      }

      const pasted = e.clipboardData?.getData("text/plain")?.trim();
      if (!pasted) return;

      const check = classifyFacebookUrlClient(pasted);
      if (check.valid) {
        e.preventDefault();
        setActiveView("workspace");
        setOmnibarUrl(pasted);
        onPasteAnalyze(pasted);
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    window.addEventListener("paste", handleGlobalPaste);
    return () => {
      window.removeEventListener("keydown", handleKeyDown);
      window.removeEventListener("paste", handleGlobalPaste);
    };
  }, [
    commandPaletteOpen,
    setCommandPaletteOpen,
    setBatchModalOpen,
    setActiveView,
    setOmnibarUrl,
    onPasteAnalyze,
  ]);
}

import { create } from "zustand";
import type { MediaInspectionResult } from "../types";

export type ActiveNavView = "workspace" | "history" | "settings";
export type ThemeMode = "dark" | "light";

export interface ToastMessage {
  id: string;
  title: string;
  description?: string;
  message?: string;
  variant?: "info" | "success" | "warning" | "error";
  type?: "info" | "success" | "warning" | "error";
  actionLabel?: string;
  onAction?: () => void;
}

interface UIState {
  activeView: ActiveNavView;
  theme: ThemeMode;
  omnibarUrl: string;
  isAnalyzing: boolean;
  analyzeError: string | null;
  inspectedMedia: MediaInspectionResult | null;
  selectedSlides: number[];
  selectedQuality: string;
  selectedContainer: string;
  selectedCodec: string;
  selectedAudioBitrate: string;
  exactResolution: boolean;
  audioNormalization: boolean;
  writeSubtitles: boolean;
  embedChapters: boolean;
  playlistMode: "single" | "playlist";
  saveCaption: boolean;
  saveMetadata: boolean;
  embedThumbnail: boolean;

  commandPaletteOpen: boolean;
  isCommandPaletteOpen: boolean;
  batchModalOpen: boolean;
  isBatchModalOpen: boolean;
  authModalOpen: boolean;
  isAuthModalOpen: boolean;
  onboardingModalOpen: boolean;
  isBootstrapModalOpen: boolean;
  toasts: ToastMessage[];
  lastClipboardUrl: string | null;

  setActiveView: (view: ActiveNavView) => void;
  toggleTheme: () => void;
  setOmnibarUrl: (url: string) => void;
  setIsAnalyzing: (loading: boolean) => void;
  setAnalyzeError: (err: string | null) => void;
  setInspectedMedia: (
    media: MediaInspectionResult | null,
    carouselHint?: number | null
  ) => void;
  toggleSlideSelection: (index: number) => void;
  selectAllSlides: () => void;
  deselectAllSlides: () => void;
  selectSlidesByType: (type: "image" | "video") => void;
  setSelectedQuality: (q: string) => void;
  setSelectedContainer: (c: string) => void;
  setSelectedCodec: (c: string) => void;
  setSelectedAudioBitrate: (b: string) => void;
  setExactResolution: (v: boolean) => void;
  setAudioNormalization: (v: boolean) => void;
  setWriteSubtitles: (v: boolean) => void;
  setEmbedChapters: (v: boolean) => void;
  setPlaylistMode: (m: "single" | "playlist") => void;
  setSaveCaption: (val: boolean) => void;
  setSaveMetadata: (val: boolean) => void;
  setEmbedThumbnail: (val: boolean) => void;

  setCommandPaletteOpen: (open: boolean) => void;
  setBatchModalOpen: (open: boolean) => void;
  setAuthModalOpen: (open: boolean) => void;
  setOnboardingModalOpen: (open: boolean) => void;
  setBootstrapModalOpen: (open: boolean) => void;
  addToast: (toast: Omit<ToastMessage, "id">) => void;
  dismissToast: (id: string) => void;
  setLastClipboardUrl: (url: string | null) => void;
}

export const useUIStore = create<UIState>((set, get) => ({
  activeView: "workspace",
  theme: "dark",
  omnibarUrl: "",
  isAnalyzing: false,
  analyzeError: null,
  inspectedMedia: null,
  selectedSlides: [],
  selectedQuality: "best",
  selectedContainer: "mp4",
  selectedCodec: "auto",
  selectedAudioBitrate: "320",
  exactResolution: false,
  audioNormalization: false,
  writeSubtitles: false,
  embedChapters: true,
  playlistMode: "single",
  saveCaption: false,
  saveMetadata: false,
  embedThumbnail: true,

  commandPaletteOpen: false,
  isCommandPaletteOpen: false,
  batchModalOpen: false,
  isBatchModalOpen: false,
  authModalOpen: false,
  isAuthModalOpen: false,
  onboardingModalOpen: false,
  isBootstrapModalOpen: false,
  toasts: [],
  lastClipboardUrl: null,

  setActiveView: (activeView) => set({ activeView }),

  toggleTheme: () => {
    const next = get().theme === "dark" ? "light" : "dark";
    if (typeof document !== "undefined") {
      document.documentElement.classList.remove("dark", "light");
      document.documentElement.classList.add(next);
      try {
        window.localStorage.setItem("master_dl_theme", next);
      } catch {
        // ignore storage restrictions
      }
    }
    set({ theme: next });
  },

  setOmnibarUrl: (omnibarUrl) => set({ omnibarUrl }),
  setIsAnalyzing: (isAnalyzing) => set({ isAnalyzing }),
  setAnalyzeError: (analyzeError) => set({ analyzeError }),

  setInspectedMedia: (media, carouselHint) => {
    if (!media) {
      set({ inspectedMedia: null, selectedSlides: [] });
      return;
    }
    const allIndices = media.slides.map((s) => s.index);
    const defaultSelected =
      carouselHint && allIndices.includes(carouselHint)
        ? [carouselHint]
        : allIndices;

    set({
      inspectedMedia: media,
      selectedSlides: defaultSelected,
      playlistMode: media.is_playlist ? "playlist" : "single",
      analyzeError: null,
    });
  },

  toggleSlideSelection: (index) => {
    const current = get().selectedSlides;
    if (current.includes(index)) {
      set({ selectedSlides: current.filter((i) => i !== index) });
    } else {
      set({ selectedSlides: [...current, index].sort((a, b) => a - b) });
    }
  },

  selectAllSlides: () => {
    const media = get().inspectedMedia;
    if (!media) return;
    set({ selectedSlides: media.slides.map((s) => s.index) });
  },

  deselectAllSlides: () => set({ selectedSlides: [] }),

  selectSlidesByType: (type) => {
    const media = get().inspectedMedia;
    if (!media) return;
    set({
      selectedSlides: media.slides
        .filter((s) => s.media_type === type)
        .map((s) => s.index),
    });
  },

  setSelectedQuality: (selectedQuality) => set({ selectedQuality }),
  setSelectedContainer: (selectedContainer) => set({ selectedContainer }),
  setSelectedCodec: (selectedCodec) => set({ selectedCodec }),
  setSelectedAudioBitrate: (selectedAudioBitrate) => set({ selectedAudioBitrate }),
  setExactResolution: (exactResolution) => set({ exactResolution }),
  setAudioNormalization: (audioNormalization) => set({ audioNormalization }),
  setWriteSubtitles: (writeSubtitles) => set({ writeSubtitles }),
  setEmbedChapters: (embedChapters) => set({ embedChapters }),
  setPlaylistMode: (playlistMode) => set({ playlistMode }),
  setSaveCaption: (saveCaption) => set({ saveCaption }),
  setSaveMetadata: (saveMetadata) => set({ saveMetadata }),
  setEmbedThumbnail: (embedThumbnail) => set({ embedThumbnail }),

  setCommandPaletteOpen: (open) =>
    set({ commandPaletteOpen: open, isCommandPaletteOpen: open }),
  setBatchModalOpen: (open) =>
    set({ batchModalOpen: open, isBatchModalOpen: open }),
  setAuthModalOpen: (open) =>
    set({ authModalOpen: open, isAuthModalOpen: open }),
  setOnboardingModalOpen: (open) =>
    set({ onboardingModalOpen: open, isBootstrapModalOpen: open }),
  setBootstrapModalOpen: (open) =>
    set({ onboardingModalOpen: open, isBootstrapModalOpen: open }),

  addToast: (toast) => {
    const id = `${Date.now()}-${Math.random().toString(36).slice(2, 7)}`;
    const normalized: ToastMessage = {
      ...toast,
      id,
      description: toast.description ?? toast.message,
      variant: toast.variant ?? toast.type ?? "info",
    };
    set((s) => ({ toasts: [...s.toasts.slice(-4), normalized] }));
    setTimeout(() => {
      get().dismissToast(id);
    }, 6500);
  },

  dismissToast: (id) =>
    set((s) => ({ toasts: s.toasts.filter((t) => t.id !== id) })),

  setLastClipboardUrl: (lastClipboardUrl) => set({ lastClipboardUrl }),
}));

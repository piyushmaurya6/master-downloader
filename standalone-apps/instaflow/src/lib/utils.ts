import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

export function formatBytes(bytes: number | null | undefined): string {
  if (bytes === null || bytes === undefined || bytes <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const k = 1024;
  const i = Math.min(Math.floor(Math.log(bytes) / Math.log(k)), units.length - 1);
  const val = bytes / Math.pow(k, i);
  return `${val.toFixed(i === 0 ? 0 : 1)} ${units[i]}`;
}

export function formatSpeed(bps: number | null | undefined): string {
  if (!bps || bps <= 0) return "—";
  return `${formatBytes(bps)}/s`;
}

export function formatEta(seconds: number | null | undefined): string {
  if (seconds === null || seconds === undefined || seconds < 0) return "—";
  const mins = Math.floor(seconds / 60);
  const secs = Math.floor(seconds % 60);
  return `${String(mins).padStart(2, "0")}:${String(secs).padStart(2, "0")}`;
}

export function formatRelativeDate(timestampSecOrMs: number | null | undefined): string {
  if (!timestampSecOrMs) return "";
  const ms = timestampSecOrMs < 10_000_000_000 ? timestampSecOrMs * 1000 : timestampSecOrMs;
  const d = new Date(ms);
  return d.toLocaleDateString(undefined, {
    year: "numeric",
    month: "short",
    day: "numeric",
  });
}

export interface ClientUrlClassification {
  valid: boolean;
  canonicalUrl: string;
  badge: "REEL" | "POST" | "STORY" | "HIGHLIGHT" | "TV" | "PROFILE" | null;
  shortcode: string | null;
  imgIndex: number | null;
}

const IG_REGEX =
  /^(?:https?:\/\/)?(?:www\.|m\.)?instagram\.com\/(?:([A-Za-z0-9_.]+)\/)?(?:(reel|reels|p|tv|stories|s)\/([A-Za-z0-9_-]+)(?:\/([0-9]+))?|([A-Za-z0-9_.]+))\/?(?:\?(.*))?$/i;

const RESERVED = new Set([
  "explore",
  "accounts",
  "direct",
  "about",
  "legal",
  "developer",
  "challenge",
]);

/**
 * Instant client-side validator and classifier for Instagram URLs used by the Smart Omnibar
 * and Clipboard Monitor before dispatching to Rust IPC.
 */
export function classifyInstagramUrlClient(rawInput: string): ClientUrlClassification {
  const trimmed = rawInput.trim();
  if (!trimmed) {
    return { valid: false, canonicalUrl: "", badge: null, shortcode: null, imgIndex: null };
  }

  let normalizedInput = trimmed;
  if (!/^https?:\/\//i.test(normalizedInput) && /^(?:www\.)?instagram\.com\//i.test(normalizedInput)) {
    normalizedInput = `https://${normalizedInput}`;
  }

  try {
    const u = new URL(normalizedInput);
    const host = u.hostname.toLowerCase();
    if (
      host !== "instagram.com" &&
      host !== "www.instagram.com" &&
      host !== "m.instagram.com" &&
      host !== "ddinstagram.com"
    ) {
      return { valid: false, canonicalUrl: "", badge: null, shortcode: null, imgIndex: null };
    }

    const imgIdxParam = u.searchParams.get("img_index");
    const imgIndex = imgIdxParam && /^\d+$/.test(imgIdxParam) ? parseInt(imgIdxParam, 10) : null;

    const parts = u.pathname.split("/").filter(Boolean);
    if (parts.length === 0) {
      return { valid: false, canonicalUrl: "", badge: null, shortcode: null, imgIndex: null };
    }

    if ((parts[0] === "reel" || parts[0] === "reels") && parts[1]) {
      return {
        valid: true,
        canonicalUrl: `https://www.instagram.com/reel/${parts[1]}/`,
        badge: "REEL",
        shortcode: parts[1],
        imgIndex,
      };
    }

    if (parts[0] === "p" && parts[1]) {
      return {
        valid: true,
        canonicalUrl: `https://www.instagram.com/p/${parts[1]}/`,
        badge: "POST",
        shortcode: parts[1],
        imgIndex,
      };
    }

    if (parts[0] === "tv" && parts[1]) {
      return {
        valid: true,
        canonicalUrl: `https://www.instagram.com/tv/${parts[1]}/`,
        badge: "TV",
        shortcode: parts[1],
        imgIndex,
      };
    }

    if (parts[0] === "stories" && parts[1] === "highlights" && parts[2]) {
      return {
        valid: true,
        canonicalUrl: `https://www.instagram.com/stories/highlights/${parts[2]}/`,
        badge: "HIGHLIGHT",
        shortcode: parts[2],
        imgIndex: null,
      };
    }

    if (parts[0] === "s" && parts[1]) {
      return {
        valid: true,
        canonicalUrl: `https://www.instagram.com/s/${parts[1]}/`,
        badge: "HIGHLIGHT",
        shortcode: parts[1],
        imgIndex: null,
      };
    }

    if (parts[0] === "stories" && parts[1]) {
      const sid = parts[2] ? `${parts[2]}/` : "";
      return {
        valid: true,
        canonicalUrl: `https://www.instagram.com/stories/${parts[1]}/${sid}`,
        badge: "STORY",
        shortcode: parts[2] ?? parts[1],
        imgIndex: null,
      };
    }

    if (parts.length >= 3 && (parts[1] === "p" || parts[1] === "reel" || parts[1] === "reels")) {
      const isReel = parts[1] !== "p";
      return {
        valid: true,
        canonicalUrl: `https://www.instagram.com/${isReel ? "reel" : "p"}/${parts[2]}/`,
        badge: isReel ? "REEL" : "POST",
        shortcode: parts[2],
        imgIndex,
      };
    }

    if (parts.length === 1 && !RESERVED.has(parts[0].toLowerCase()) && /^[A-Za-z0-9_.]{1,35}$/.test(parts[0])) {
      return {
        valid: true,
        canonicalUrl: `https://www.instagram.com/${parts[0]}/`,
        badge: "PROFILE",
        shortcode: parts[0],
        imgIndex: null,
      };
    }

    const fallbackMatch = IG_REGEX.exec(normalizedInput);
    if (fallbackMatch) {
      return {
        valid: true,
        canonicalUrl: normalizedInput.split("?")[0],
        badge: "POST",
        shortcode: fallbackMatch[3] ?? null,
        imgIndex,
      };
    }
  } catch {
    // Ignore URL parse error
  }

  return { valid: false, canonicalUrl: "", badge: null, shortcode: null, imgIndex: null };
}

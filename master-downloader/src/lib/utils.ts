import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";
import type { MediaUrlType, Platform } from "../types";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

export function formatBytes(bytes: number | null | undefined): string {
  if (bytes === null || bytes === undefined || bytes <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const idx = Math.min(
    Math.floor(Math.log(bytes) / Math.log(1024)),
    units.length - 1
  );
  const val = bytes / Math.pow(1024, idx);
  return `${val.toFixed(idx === 0 ? 0 : 1)} ${units[idx]}`;
}

export function formatSpeed(bps: number | null | undefined): string {
  if (!bps || bps <= 0) return "—";
  return `${formatBytes(bps)}/s`;
}

export function formatDuration(seconds: number | null | undefined): string {
  if (seconds === null || seconds === undefined || seconds <= 0) return "0:00";
  const total = Math.round(seconds);
  const hrs = Math.floor(total / 3600);
  const mins = Math.floor((total % 3600) / 60);
  const secs = total % 60;
  if (hrs > 0) {
    return `${hrs}:${mins.toString().padStart(2, "0")}:${secs
      .toString()
      .padStart(2, "0")}`;
  }
  return `${mins}:${secs.toString().padStart(2, "0")}`;
}

export function formatEta(seconds: number | null | undefined): string {
  if (seconds === null || seconds === undefined || seconds <= 0) return "";
  if (seconds < 60) return `${seconds}s`;
  const mins = Math.floor(seconds / 60);
  const secs = seconds % 60;
  return `${mins}m ${secs}s`;
}

export function formatCount(count: number | null | undefined): string {
  if (count === null || count === undefined) return "—";
  if (count >= 1_000_000) return `${(count / 1_000_000).toFixed(1)}M`;
  if (count >= 1_000) return `${(count / 1_000).toFixed(1)}K`;
  return count.toString();
}

export interface ClientUrlClassification {
  valid: boolean;
  platform: Platform | null;
  type: MediaUrlType | null;
  shortcode: string | null;
  badgeLabel: string | null;
}

/**
 * Universal client-side URL classifier across YouTube, Instagram, and Facebook
 * for instant <1ms omnibar & clipboard detection.
 */
export function classifyMediaUrlClient(raw: string): ClientUrlClassification {
  const trimmed = raw.trim();
  if (!trimmed) {
    return { valid: false, platform: null, type: null, shortcode: null, badgeLabel: null };
  }

  try {
    const withProto = /^https?:\/\//i.test(trimmed)
      ? trimmed
      : /^(www\.|m\.|music\.|web\.|mbasic\.)?(youtube\.com|youtu\.be|instagram\.com|instagr\.am|ddinstagram\.com|facebook\.com|fb\.watch|fb\.com)/i.test(
          trimmed
        )
      ? `https://${trimmed}`
      : null;

    if (!withProto) {
      return { valid: false, platform: null, type: null, shortcode: null, badgeLabel: null };
    }

    const url = new URL(withProto);
    const host = url.hostname.toLowerCase();
    const parts = url.pathname.split("/").filter(Boolean);

    // 1. YouTube Detection
    if (
      host === "youtube.com" ||
      host === "www.youtube.com" ||
      host === "m.youtube.com" ||
      host === "music.youtube.com" ||
      host === "youtu.be" ||
      host === "www.youtu.be"
    ) {
      const v = url.searchParams.get("v");
      const list = url.searchParams.get("list");

      if (host.includes("youtu.be") && parts[0]) {
        return {
          valid: true,
          platform: "youtube",
          type: "video",
          shortcode: parts[0],
          badgeLabel: "YOUTUBE",
        };
      }
      if (parts[0] === "shorts" && parts[1]) {
        return {
          valid: true,
          platform: "youtube",
          type: "shorts",
          shortcode: parts[1],
          badgeLabel: "YT SHORTS",
        };
      }
      if (parts[0] === "playlist" && list) {
        return {
          valid: true,
          platform: "youtube",
          type: "playlist",
          shortcode: list,
          badgeLabel: "YT PLAYLIST",
        };
      }
      if ((parts[0] === "live" || parts[0] === "embed") && parts[1]) {
        return {
          valid: true,
          platform: "youtube",
          type: "video",
          shortcode: parts[1],
          badgeLabel: "YT LIVE",
        };
      }
      if (v) {
        if (host === "music.youtube.com") {
          return {
            valid: true,
            platform: "youtube",
            type: "music",
            shortcode: v,
            badgeLabel: "YT MUSIC",
          };
        }
        return {
          valid: true,
          platform: "youtube",
          type: "video",
          shortcode: v,
          badgeLabel: "YOUTUBE",
        };
      }
      return { valid: false, platform: null, type: null, shortcode: null, badgeLabel: null };
    }

    // 2. Instagram Detection
    if (
      host === "instagram.com" ||
      host === "www.instagram.com" ||
      host === "instagr.am" ||
      host === "www.instagr.am" ||
      host === "ddinstagram.com" ||
      host === "www.ddinstagram.com"
    ) {
      if (parts.length === 0) {
        return { valid: false, platform: null, type: null, shortcode: null, badgeLabel: null };
      }
      if ((parts[0] === "reel" || parts[0] === "reels") && parts[1]) {
        return {
          valid: true,
          platform: "instagram",
          type: "reel",
          shortcode: parts[1],
          badgeLabel: "IG REEL",
        };
      }
      if (parts[0] === "p" && parts[1]) {
        return {
          valid: true,
          platform: "instagram",
          type: "post",
          shortcode: parts[1],
          badgeLabel: "IG POST",
        };
      }
      if (parts[0] === "tv" && parts[1]) {
        return {
          valid: true,
          platform: "instagram",
          type: "tv",
          shortcode: parts[1],
          badgeLabel: "IG TV",
        };
      }
      if (parts[0] === "stories" && parts[1] === "highlights" && parts[2]) {
        return {
          valid: true,
          platform: "instagram",
          type: "highlight",
          shortcode: parts[2],
          badgeLabel: "IG HIGHLIGHT",
        };
      }
      if (parts[0] === "s" && parts[1]) {
        return {
          valid: true,
          platform: "instagram",
          type: "highlight",
          shortcode: parts[1],
          badgeLabel: "IG HIGHLIGHT",
        };
      }
      if (parts[0] === "stories" && parts[1]) {
        return {
          valid: true,
          platform: "instagram",
          type: "stories",
          shortcode: parts[2] ?? parts[1],
          badgeLabel: "IG STORY",
        };
      }
      if (parts[1] === "p" && parts[2]) {
        return {
          valid: true,
          platform: "instagram",
          type: "post",
          shortcode: parts[2],
          badgeLabel: "IG POST",
        };
      }
      if ((parts[1] === "reel" || parts[1] === "reels") && parts[2]) {
        return {
          valid: true,
          platform: "instagram",
          type: "reel",
          shortcode: parts[2],
          badgeLabel: "IG REEL",
        };
      }
      if (parts.length === 1) {
        return {
          valid: true,
          platform: "instagram",
          type: "profile",
          shortcode: parts[0],
          badgeLabel: "IG PROFILE",
        };
      }
      return { valid: false, platform: null, type: null, shortcode: null, badgeLabel: null };
    }

    // 3. Facebook Detection
    if (
      host === "facebook.com" ||
      host === "www.facebook.com" ||
      host === "m.facebook.com" ||
      host === "web.facebook.com" ||
      host === "mbasic.facebook.com" ||
      host === "fb.watch" ||
      host === "www.fb.watch" ||
      host === "fb.com" ||
      host === "www.fb.com"
    ) {
      if (host.includes("fb.watch") && parts[0]) {
        return {
          valid: true,
          platform: "facebook",
          type: "shortlink",
          shortcode: parts[0],
          badgeLabel: "FB.WATCH",
        };
      }

      const vParam = url.searchParams.get("v");
      const storyFbid = url.searchParams.get("story_fbid");
      const fbid = url.searchParams.get("fbid");
      const setParam = url.searchParams.get("set");

      if ((parts[0] === "watch" || parts[0] === "video.php") && (vParam || parts[1])) {
        return {
          valid: true,
          platform: "facebook",
          type: "watch",
          shortcode: vParam ?? parts[1],
          badgeLabel: "FB WATCH",
        };
      }
      if ((parts[0] === "story.php" || parts[0] === "permalink.php") && storyFbid) {
        return {
          valid: true,
          platform: "facebook",
          type: "post",
          shortcode: storyFbid,
          badgeLabel: "FB POST",
        };
      }
      if ((parts[0] === "photo" || parts[0] === "photo.php") && (fbid || parts[1])) {
        return {
          valid: true,
          platform: "facebook",
          type: "photo",
          shortcode: fbid ?? parts[1],
          badgeLabel: "FB PHOTO",
        };
      }
      if (parts[0] === "media" && parts[1] === "set" && setParam) {
        return {
          valid: true,
          platform: "facebook",
          type: "photo",
          shortcode: setParam,
          badgeLabel: "FB ALBUM",
        };
      }
      if ((parts[0] === "reel" || parts[0] === "reels") && parts[1]) {
        return {
          valid: true,
          platform: "facebook",
          type: "reel",
          shortcode: parts[1],
          badgeLabel: "FB REEL",
        };
      }
      if (parts[0] === "share" && parts[1]) {
        return {
          valid: true,
          platform: "facebook",
          type: "share",
          shortcode: parts[2] ?? parts[1],
          badgeLabel: "FB SHARE",
        };
      }
      if (parts[0] === "stories" && parts[1]) {
        return {
          valid: true,
          platform: "facebook",
          type: "story",
          shortcode: parts[2] ?? parts[1],
          badgeLabel: "FB STORY",
        };
      }
      if (parts[0] === "groups" && parts[2] === "videos" && parts[3]) {
        return {
          valid: true,
          platform: "facebook",
          type: "video",
          shortcode: parts[3],
          badgeLabel: "FB VIDEO",
        };
      }
      if (parts[0] === "groups" && (parts[2] === "posts" || parts[2] === "permalink") && parts[3]) {
        return {
          valid: true,
          platform: "facebook",
          type: "post",
          shortcode: parts[3],
          badgeLabel: "FB GROUP",
        };
      }
      if (parts[1] === "videos" && parts[2]) {
        return {
          valid: true,
          platform: "facebook",
          type: "video",
          shortcode: parts[3] ?? parts[2],
          badgeLabel: "FB VIDEO",
        };
      }
      if ((parts[1] === "reel" || parts[1] === "reels") && parts[2]) {
        return {
          valid: true,
          platform: "facebook",
          type: "reel",
          shortcode: parts[2],
          badgeLabel: "FB REEL",
        };
      }
      if (parts[1] === "posts" && parts[2]) {
        return {
          valid: true,
          platform: "facebook",
          type: "post",
          shortcode: parts[2],
          badgeLabel: "FB POST",
        };
      }
      if (parts[1] === "photos" && parts[2]) {
        return {
          valid: true,
          platform: "facebook",
          type: "photo",
          shortcode: parts[3] ?? parts[2],
          badgeLabel: "FB PHOTO",
        };
      }
      return { valid: false, platform: null, type: null, shortcode: null, badgeLabel: null };
    }
  } catch {
    return { valid: false, platform: null, type: null, shortcode: null, badgeLabel: null };
  }

  return { valid: false, platform: null, type: null, shortcode: null, badgeLabel: null };
}

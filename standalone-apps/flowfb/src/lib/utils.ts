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

export function formatDuration(seconds: number | null | undefined): string {
  if (seconds === null || seconds === undefined || seconds <= 0) return "";
  const total = Math.round(seconds);
  const hrs = Math.floor(total / 3600);
  const mins = Math.floor((total % 3600) / 60);
  const secs = total % 60;
  if (hrs > 0) {
    return `${hrs}:${String(mins).padStart(2, "0")}:${String(secs).padStart(2, "0")}`;
  }
  return `${String(mins).padStart(2, "0")}:${String(secs).padStart(2, "0")}`;
}

export function formatCompactNumber(count: number | null | undefined): string {
  if (count === null || count === undefined || count < 0) return "";
  if (count >= 1_000_000) {
    return `${(count / 1_000_000).toFixed(1)}M`;
  }
  if (count >= 1_000) {
    return `${(count / 1_000).toFixed(1)}K`;
  }
  return String(count);
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
  badge: "REEL" | "WATCH" | "VIDEO" | "FB.WATCH" | "SHARE" | "POST" | "PHOTO" | "STORY" | null;
  mediaId: string | null;
}

const FB_HOSTS = new Set([
  "facebook.com",
  "www.facebook.com",
  "m.facebook.com",
  "web.facebook.com",
  "touch.facebook.com",
  "mbasic.facebook.com",
  "fb.watch",
  "www.fb.watch",
  "fb.com",
  "www.fb.com",
]);

/**
 * Instant client-side validator and classifier for Facebook URLs used by the Smart Omnibar
 * and Clipboard Monitor before dispatching to Rust IPC.
 */
export function classifyFacebookUrlClient(rawInput: string): ClientUrlClassification {
  const trimmed = rawInput.trim();
  if (!trimmed) {
    return { valid: false, canonicalUrl: "", badge: null, mediaId: null };
  }

  let normalizedInput = trimmed;
  if (
    !/^https?:\/\//i.test(normalizedInput) &&
    /^(?:www\.|m\.|web\.)?(?:facebook\.com|fb\.watch|fb\.com)\//i.test(normalizedInput)
  ) {
    normalizedInput = `https://${normalizedInput}`;
  }

  try {
    const u = new URL(normalizedInput);
    const host = u.hostname.toLowerCase();
    if (!FB_HOSTS.has(host)) {
      return { valid: false, canonicalUrl: "", badge: null, mediaId: null };
    }

    const parts = u.pathname.split("/").filter(Boolean);

    // 1. Shortlink fb.watch/{token}
    if (host === "fb.watch" || host === "www.fb.watch") {
      if (parts[0]) {
        return {
          valid: true,
          canonicalUrl: `https://fb.watch/${parts[0]}/`,
          badge: "FB.WATCH",
          mediaId: parts[0],
        };
      }
      return { valid: false, canonicalUrl: "", badge: null, mediaId: null };
    }

    // 2. /reel/{id} or /reels/{id}
    if ((parts[0]?.toLowerCase() === "reel" || parts[0]?.toLowerCase() === "reels") && parts[1]) {
      return {
        valid: true,
        canonicalUrl: `https://www.facebook.com/reel/${parts[1]}`,
        badge: "REEL",
        mediaId: parts[1],
      };
    }

    // 3. /watch/?v={id} or /watch/live/?v={id}
    const vParam = u.searchParams.get("v");
    if (parts[0]?.toLowerCase() === "watch") {
      const vid = vParam || parts[1];
      if (vid && vid.toLowerCase() !== "live") {
        return {
          valid: true,
          canonicalUrl: `https://www.facebook.com/watch/?v=${vid}`,
          badge: "WATCH",
          mediaId: vid,
        };
      }
    }
    if (vParam && (parts.length === 0 || parts[0]?.toLowerCase() === "video.php")) {
      return {
        valid: true,
        canonicalUrl: `https://www.facebook.com/watch/?v=${vParam}`,
        badge: "WATCH",
        mediaId: vParam,
      };
    }

    // 4. /share/{v|r|p}/{token} or /share/{token}
    if (parts[0]?.toLowerCase() === "share" && parts[1]) {
      if (parts.length >= 3 && ["v", "r", "p", "video", "reel", "post"].includes(parts[1].toLowerCase())) {
        const sk = parts[1].toLowerCase()[0];
        return {
          valid: true,
          canonicalUrl: `https://www.facebook.com/share/${sk}/${parts[2]}/`,
          badge: sk === "r" ? "REEL" : sk === "v" ? "VIDEO" : "SHARE",
          mediaId: parts[2],
        };
      }
      return {
        valid: true,
        canonicalUrl: `https://www.facebook.com/share/${parts[1]}/`,
        badge: "SHARE",
        mediaId: parts[1],
      };
    }

    // 5. story.php / permalink.php
    const storyFbid = u.searchParams.get("story_fbid");
    const pageIdParam = u.searchParams.get("id");
    if (
      parts[0] &&
      (parts[0].toLowerCase() === "story.php" || parts[0].toLowerCase() === "permalink.php") &&
      storyFbid
    ) {
      const canonical = pageIdParam
        ? `https://www.facebook.com/permalink.php?story_fbid=${storyFbid}&id=${pageIdParam}`
        : `https://www.facebook.com/permalink.php?story_fbid=${storyFbid}`;
      return {
        valid: true,
        canonicalUrl: canonical,
        badge: "POST",
        mediaId: storyFbid,
      };
    }

    // 6. /photo or /photo.php or /media/set
    const fbidParam = u.searchParams.get("fbid");
    const setParam = u.searchParams.get("set");
    if (parts[0] && (parts[0].toLowerCase() === "photo" || parts[0].toLowerCase() === "photo.php") && fbidParam) {
      const canonical = setParam
        ? `https://www.facebook.com/photo/?fbid=${fbidParam}&set=${setParam}`
        : `https://www.facebook.com/photo/?fbid=${fbidParam}`;
      return {
        valid: true,
        canonicalUrl: canonical,
        badge: "PHOTO",
        mediaId: fbidParam,
      };
    }
    if (parts[0]?.toLowerCase() === "media" && parts[1]?.toLowerCase() === "set" && setParam) {
      return {
        valid: true,
        canonicalUrl: `https://www.facebook.com/media/set/?set=${setParam}`,
        badge: "PHOTO",
        mediaId: setParam,
      };
    }

    // 7. /stories/{id}
    if (parts[0]?.toLowerCase() === "stories" && parts[1]) {
      const sid = parts[2] ?? parts[1];
      return {
        valid: true,
        canonicalUrl: `https://www.facebook.com/stories/${parts[1]}/${parts[2] ? `${parts[2]}/` : ""}`,
        badge: "STORY",
        mediaId: sid,
      };
    }

    // 8. /{page}/videos/{id} or /{page}/posts/{id}
    if (parts.length >= 3) {
      const mid = parts[1].toLowerCase();
      if (mid === "videos" || mid === "video") {
        const vid = parts[parts.length - 1];
        return {
          valid: true,
          canonicalUrl: `https://www.facebook.com/${parts[0]}/videos/${vid}/`,
          badge: "VIDEO",
          mediaId: vid,
        };
      }
      if (mid === "posts") {
        return {
          valid: true,
          canonicalUrl: `https://www.facebook.com/${parts[0]}/posts/${parts[2]}`,
          badge: "POST",
          mediaId: parts[2],
        };
      }
      if (mid === "photos") {
        const pid = parts[parts.length - 1];
        return {
          valid: true,
          canonicalUrl: `https://www.facebook.com/photo/?fbid=${pid}`,
          badge: "PHOTO",
          mediaId: pid,
        };
      }
    }

    // 9. /groups/{group_id}/posts/{post_id} or /groups/{group_id}/permalink/{post_id}
    if (
      parts.length >= 4 &&
      parts[0].toLowerCase() === "groups" &&
      (parts[2].toLowerCase() === "posts" || parts[2].toLowerCase() === "permalink")
    ) {
      return {
        valid: true,
        canonicalUrl: `https://www.facebook.com/groups/${parts[1]}/posts/${parts[3]}/`,
        badge: "POST",
        mediaId: parts[3],
      };
    }
  } catch {
    // Ignore URL parse error
  }

  return { valid: false, canonicalUrl: "", badge: null, mediaId: null };
}

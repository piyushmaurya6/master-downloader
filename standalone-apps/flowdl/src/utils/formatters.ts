import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";
import type { PostprocessStage, VideoQualityPreset } from "../types";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

export function formatBytes(bytes: number | null | undefined, decimals = 1): string {
  if (bytes === null || bytes === undefined || isNaN(bytes) || bytes < 0) {
    return "—";
  }
  if (bytes === 0) return "0 B";
  const k = 1024;
  const dm = decimals < 0 ? 0 : decimals;
  const sizes = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.min(Math.floor(Math.log(bytes) / Math.log(k)), sizes.length - 1);
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(dm))} ${sizes[i]}`;
}

export function formatSpeed(bytesPerSec: number | null | undefined): string {
  if (!bytesPerSec || bytesPerSec <= 0 || isNaN(bytesPerSec)) {
    return "—";
  }
  return `${formatBytes(bytesPerSec, 1)}/s`;
}

export function formatEta(seconds: number | null | undefined): string {
  if (seconds === null || seconds === undefined || isNaN(seconds) || seconds < 0) {
    return "calculating...";
  }
  if (seconds === 0) return "less than a second";
  const s = Math.round(seconds);
  if (s < 60) {
    return `${s} ${s === 1 ? "second" : "seconds"} remaining`;
  }
  const mins = Math.floor(s / 60);
  const remSecs = s % 60;
  if (mins < 60) {
    return remSecs > 0 ? `${mins}m ${remSecs}s remaining` : `${mins}m remaining`;
  }
  const hours = Math.floor(mins / 60);
  const remMins = mins % 60;
  return `${hours}h ${remMins}m remaining`;
}

export function formatDuration(seconds: number | null | undefined): string {
  if (seconds === null || seconds === undefined || isNaN(seconds) || seconds <= 0) {
    return "Live / Unknown";
  }
  const total = Math.round(seconds);
  const hrs = Math.floor(total / 3600);
  const mins = Math.floor((total % 3600) / 60);
  const secs = total % 60;
  if (hrs > 0) {
    return `${hrs}:${String(mins).padStart(2, "0")}:${String(secs).padStart(2, "0")}`;
  }
  return `${mins}:${String(secs).padStart(2, "0")}`;
}

export function formatUploadDate(rawDate: string | null | undefined): string | null {
  if (!rawDate) return null;
  if (/^\d{8}$/.test(rawDate)) {
    const year = rawDate.slice(0, 4);
    const month = rawDate.slice(4, 6);
    const day = rawDate.slice(6, 8);
    const date = new Date(`${year}-${month}-${day}T00:00:00Z`);
    if (!isNaN(date.getTime())) {
      return date.toLocaleDateString("en-US", {
        year: "numeric",
        month: "short",
        day: "numeric",
      });
    }
  }
  return rawDate;
}

export function isValidMediaUrl(input: string): boolean {
  const trimmed = input.trim();
  if (!trimmed || trimmed.length > 2048) return false;
  try {
    const parsed = new URL(trimmed);
    if (parsed.protocol !== "http:" && parsed.protocol !== "https:") {
      return false;
    }
    if (!parsed.hostname || !parsed.hostname.includes(".")) {
      return false;
    }
    return true;
  } catch {
    return false;
  }
}

export function sanitizeUrlForDisplay(url: string): string {
  try {
    const parsed = new URL(url);
    const sensitiveParams = [
      "token",
      "access_token",
      "auth",
      "key",
      "signature",
      "sig",
      "session",
      "password",
      "cookie",
    ];
    for (const param of sensitiveParams) {
      if (parsed.searchParams.has(param)) {
        parsed.searchParams.set(param, "[REDACTED]");
      }
    }
    return parsed.toString();
  } catch {
    return url;
  }
}

export function getQualityLabel(preset: VideoQualityPreset): string {
  switch (preset) {
    case "best":
      return "Best available";
    case "2160":
      return "4K — 2160p";
    case "1440":
      return "2K — 1440p";
    case "1080":
      return "Full HD — 1080p";
    case "720":
      return "HD — 720p";
    case "480":
      return "SD — 480p";
    case "360":
      return "360p";
  }
}

export function getPostprocessLabel(stage: PostprocessStage): string {
  switch (stage) {
    case "downloading_video":
      return "Downloading video stream";
    case "downloading_audio":
      return "Downloading audio stream";
    case "merging_streams":
      return "Merging video & audio streams";
    case "converting_audio":
      return "Converting audio with FFmpeg";
    case "embedding_metadata":
      return "Embedding metadata & cover art";
    case "finalizing":
      return "Finalizing output file";
    default:
      return "Processing";
  }
}

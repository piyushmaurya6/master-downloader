import React, { useEffect, useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import {
  ArrowDownToLine,
  Calendar,
  ChevronDown,
  ChevronUp,
  Film,
  FolderOpen,
  Globe,
  Info,
  ListVideo,
  Music,
  SlidersHorizontal,
  User,
} from "lucide-react";
import { tauriBridge } from "../services/tauriBridge";
import { useApp } from "../stores/AppContext";
import type {
  AudioFormat,
  AudioQualityPreset,
  DownloadMode,
  MediaMetadata,
  VideoCodecPref,
  VideoContainer,
  VideoQualityPreset,
} from "../types";
import { formatBytes, formatUploadDate } from "../utils/formatters";

interface Props {
  metadata: MediaMetadata;
}

const VIDEO_QUALITIES: {
  value: VideoQualityPreset;
  label: string;
  height: number | null;
}[] = [
  { value: "best", label: "Best available", height: null },
  { value: "2160", label: "4K — 2160p", height: 2160 },
  { value: "1440", label: "1440p QHD", height: 1440 },
  { value: "1080", label: "1080p Full HD", height: 1080 },
  { value: "720", label: "720p HD", height: 720 },
  { value: "480", label: "480p", height: 480 },
  { value: "360", label: "360p", height: 360 },
];

export const VideoPreviewCard: React.FC<Props> = ({ metadata }) => {
  const { settings, startDownload } = useApp();

  const [mode, setMode] = useState<DownloadMode>("video");
  const [videoQuality, setVideoQuality] = useState<VideoQualityPreset>("best");
  const [exactResolution, setExactResolution] = useState<boolean>(false);
  const [container, setContainer] = useState<VideoContainer>(
    settings.defaultVideoContainer || "mp4"
  );
  const [videoCodec, setVideoCodec] = useState<VideoCodecPref>("auto");
  const [preferredFps, setPreferredFps] = useState<number | null>(null);
  const [audioFormat, setAudioFormat] = useState<AudioFormat>(
    settings.defaultAudioFormat || "mp3"
  );
  const [audioQuality, setAudioQuality] = useState<AudioQualityPreset>(
    settings.defaultAudioQuality || "320"
  );
  const [downloadSubtitles, setDownloadSubtitles] = useState<boolean>(
    settings.downloadSubtitlesByDefault
  );
  const [subtitleLang, setSubtitleLang] = useState<string>(
    settings.defaultSubtitleLang || "en"
  );
  const [embedThumbnail, setEmbedThumbnail] = useState<boolean>(true);
  const [embedMetadata, setEmbedMetadata] = useState<boolean>(true);
  const [downloadChapters, setDownloadChapters] = useState<boolean>(false);
  const [normalizeAudio, setNormalizeAudio] = useState<boolean>(false);
  const [preserveOriginalFilename, setPreserveOriginalFilename] =
    useState<boolean>(false);
  const [customTemplate, setCustomTemplate] = useState<string>(
    settings.filenameTemplate
  );
  const [speedLimitKbps, setSpeedLimitKbps] = useState<number>(
    settings.speedLimitKbps
  );
  const [playlistMode, setPlaylistMode] = useState<"single" | "playlist">(
    "single"
  );
  const [customOutputDir, setCustomOutputDir] = useState<string>(
    settings.downloadDir
  );
  const [showAdvanced, setShowAdvanced] = useState<boolean>(false);

  // Pick highest available default resolution for this video
  useEffect(() => {
    const defHeight = parseInt(settings.defaultVideoQuality, 10);
    if (
      !isNaN(defHeight) &&
      metadata.availableHeights.includes(defHeight)
    ) {
      setVideoQuality(settings.defaultVideoQuality);
    } else if (metadata.availableHeights.includes(1080)) {
      setVideoQuality("1080");
    } else {
      setVideoQuality("best");
    }
  }, [metadata, settings.defaultVideoQuality]);

  const isHeightAvailable = (h: number | null): boolean => {
    if (h === null) return true;
    if (metadata.availableHeights.length === 0) return true;
    return metadata.availableHeights.includes(h);
  };

  const handleBrowseFolder = async () => {
    const picked = await tauriBridge.pickFolder(customOutputDir);
    if (picked) {
      setCustomOutputDir(picked);
    }
  };

  const handleTriggerDownload = async () => {
    await startDownload({
      url: metadata.url,
      title:
        playlistMode === "playlist" && metadata.playlistTitle
          ? metadata.playlistTitle
          : metadata.title,
      thumbnail: metadata.thumbnail,
      uploader: metadata.uploader,
      duration: metadata.duration,
      extractor: metadata.extractor,
      mode,
      videoQuality,
      exactResolution,
      container,
      videoCodec,
      preferredFps,
      audioFormat,
      audioQuality,
      downloadSubtitles,
      subtitleLang,
      embedThumbnail,
      embedMetadata,
      downloadChapters,
      normalizeAudio,
      preserveOriginalFilename,
      filenameTemplate: customTemplate,
      speedLimitKbps: speedLimitKbps > 0 ? speedLimitKbps : null,
      playlistMode,
      outputDir: customOutputDir,
    });
  };

  const selectedHeightNum =
    videoQuality === "best"
      ? metadata.availableHeights[0] ?? null
      : parseInt(videoQuality, 10);

  const estimatedBytes =
    mode === "video" && selectedHeightNum
      ? metadata.estimatedSizesByHeight[selectedHeightNum] ?? null
      : null;

  return (
    <motion.div
      initial={{ opacity: 0, y: 16 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.28, ease: [0.2, 0, 0, 1] }}
      className="rounded-2xl bg-surface-card border border-border-strong shadow-card overflow-hidden"
    >
      {/* Top horizontal media preview */}
      <div className="p-5 flex flex-col md:flex-row gap-5 border-b border-border">
        {/* Thumbnail */}
        <div className="relative w-full md:w-64 aspect-video rounded-xl overflow-hidden bg-zinc-900 shrink-0 border border-border">
          {metadata.thumbnail ? (
            <img
              src={metadata.thumbnail}
              alt={metadata.title}
              className="w-full h-full object-cover"
            />
          ) : (
            <div className="w-full h-full flex items-center justify-center text-zinc-600">
              <Film className="w-10 h-10" />
            </div>
          )}
          <div className="absolute bottom-2 right-2 px-2 py-0.5 rounded-md bg-black/80 text-white text-xs font-mono font-medium">
            {metadata.durationString}
          </div>
          <div className="absolute top-2 left-2 px-2 py-0.5 rounded-md bg-black/75 backdrop-blur-sm text-zinc-200 text-[11px] font-medium flex items-center gap-1">
            <Globe className="w-3 h-3 text-indigo-400" />
            {metadata.extractor}
          </div>
        </div>

        {/* Metadata Info */}
        <div className="flex-1 min-w-0 flex flex-col justify-between space-y-3">
          <div className="space-y-1.5">
            <h3 className="text-base font-semibold text-zinc-900 dark:text-zinc-100 leading-snug line-clamp-2">
              {metadata.title}
            </h3>

            <div className="flex flex-wrap items-center gap-x-4 gap-y-1 text-xs text-zinc-500 dark:text-zinc-400">
              {metadata.uploader && (
                <span className="inline-flex items-center gap-1.5 font-medium text-zinc-700 dark:text-zinc-300">
                  <User className="w-3.5 h-3.5 text-indigo-500" />
                  {metadata.uploader}
                </span>
              )}
              {metadata.uploadDate && (
                <span className="inline-flex items-center gap-1">
                  <Calendar className="w-3.5 h-3.5" />
                  {formatUploadDate(metadata.uploadDate)}
                </span>
              )}
              {metadata.availableHeights.length > 0 && (
                <span className="px-2 py-0.5 rounded-md bg-indigo-500/10 text-indigo-600 dark:text-indigo-300 font-mono text-[11px] font-medium">
                  Up to {metadata.availableHeights[0]}p
                </span>
              )}
            </div>

            {metadata.description && (
              <p className="text-xs text-zinc-500 dark:text-zinc-400 line-clamp-2 pt-1">
                {metadata.description}
              </p>
            )}
          </div>

          {/* Playlist prompt banner when URL is a playlist */}
          {metadata.isPlaylist && (
            <div className="p-3 rounded-xl bg-indigo-500/10 border border-indigo-500/20 flex flex-wrap items-center justify-between gap-3">
              <div className="flex items-center gap-2 text-xs text-indigo-700 dark:text-indigo-200">
                <ListVideo className="w-4 h-4 text-indigo-500 shrink-0" />
                <span>
                  Playlist detected
                  {metadata.playlistCount
                    ? ` (${metadata.playlistCount} items)`
                    : ""}
                  . Download single video or full playlist?
                </span>
              </div>
              <div className="inline-flex rounded-lg bg-surface-elevated p-0.5 border border-border">
                <button
                  type="button"
                  onClick={() => setPlaylistMode("single")}
                  className={`px-2.5 py-1 rounded-md text-xs font-medium transition-colors ${
                    playlistMode === "single"
                      ? "bg-indigo-600 text-white"
                      : "text-zinc-600 dark:text-zinc-400"
                  }`}
                >
                  Single Video
                </button>
                <button
                  type="button"
                  onClick={() => setPlaylistMode("playlist")}
                  className={`px-2.5 py-1 rounded-md text-xs font-medium transition-colors ${
                    playlistMode === "playlist"
                      ? "bg-indigo-600 text-white"
                      : "text-zinc-600 dark:text-zinc-400"
                  }`}
                >
                  Full Playlist
                </button>
              </div>
            </div>
          )}
        </div>
      </div>

      {/* Format & Quality Configuration */}
      <div className="p-5 space-y-5">
        {/* Segmented Selector: Video | Audio */}
        <div className="flex flex-wrap items-center justify-between gap-4">
          <div
            role="tablist"
            aria-label="Download mode"
            className="inline-flex p-1 rounded-xl bg-surface-subtle border border-border"
          >
            <button
              type="button"
              role="tab"
              aria-selected={mode === "video"}
              onClick={() => setMode("video")}
              className={`inline-flex items-center gap-2 px-4 py-2 rounded-lg text-xs font-semibold transition-all ${
                mode === "video"
                  ? "bg-indigo-600 text-white shadow-sm"
                  : "text-zinc-600 dark:text-zinc-400 hover:text-zinc-900 dark:hover:text-zinc-100"
              }`}
            >
              <Film className="w-3.5 h-3.5" />
              Video
            </button>
            <button
              type="button"
              role="tab"
              aria-selected={mode === "audio"}
              onClick={() => setMode("audio")}
              className={`inline-flex items-center gap-2 px-4 py-2 rounded-lg text-xs font-semibold transition-all ${
                mode === "audio"
                  ? "bg-indigo-600 text-white shadow-sm"
                  : "text-zinc-600 dark:text-zinc-400 hover:text-zinc-900 dark:hover:text-zinc-100"
              }`}
            >
              <Music className="w-3.5 h-3.5" />
              Audio
            </button>
          </div>

          {estimatedBytes !== null && estimatedBytes > 0 && (
            <div className="text-xs text-zinc-500 dark:text-zinc-400">
              Estimated size:{" "}
              <span className="font-mono font-semibold text-zinc-800 dark:text-zinc-200">
                ~{formatBytes(estimatedBytes)}
              </span>
            </div>
          )}
        </div>

        {/* Dynamic Mode Controls */}
        <AnimatePresence mode="wait">
          {mode === "video" ? (
            <motion.div
              key="video-controls"
              initial={{ opacity: 0, y: 6 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -6 }}
              transition={{ duration: 0.18 }}
              className="space-y-4"
            >
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                {/* Video Quality Selector */}
                <div className="space-y-1.5">
                  <div className="flex items-center justify-between">
                    <label
                      htmlFor="video-quality-select"
                      className="text-xs font-medium text-zinc-700 dark:text-zinc-300"
                    >
                      Video Quality ({exactResolution ? "Exact" : "Up to max"})
                    </label>
                    <button
                      type="button"
                      onClick={() => setExactResolution((v) => !v)}
                      className="text-[11px] text-indigo-600 dark:text-indigo-400 hover:underline"
                    >
                      {exactResolution
                        ? "Switch to Max resolution"
                        : "Require exact resolution"}
                    </button>
                  </div>
                  <select
                    id="video-quality-select"
                    value={videoQuality}
                    onChange={(e) =>
                      setVideoQuality(e.target.value as VideoQualityPreset)
                    }
                    className="w-full px-3.5 py-2.5 rounded-xl text-sm bg-surface-subtle border border-border text-zinc-900 dark:text-zinc-100 focus:outline-none focus:ring-2 focus:ring-indigo-500"
                  >
                    {VIDEO_QUALITIES.map((q) => {
                      const available = isHeightAvailable(q.height);
                      const sz =
                        q.height && metadata.estimatedSizesByHeight[q.height]
                          ? ` (~${formatBytes(metadata.estimatedSizesByHeight[q.height])})`
                          : "";
                      return (
                        <option
                          key={q.value}
                          value={q.value}
                          disabled={!available}
                        >
                          {q.label}
                          {sz}
                          {!available ? " — Not available for this video" : ""}
                        </option>
                      );
                    })}
                  </select>
                </div>

                {/* Output Container Selector */}
                <div className="space-y-1.5">
                  <label
                    htmlFor="video-container-select"
                    className="text-xs font-medium text-zinc-700 dark:text-zinc-300 block"
                  >
                    Output Container
                  </label>
                  <select
                    id="video-container-select"
                    value={container}
                    onChange={(e) =>
                      setContainer(e.target.value as VideoContainer)
                    }
                    className="w-full px-3.5 py-2.5 rounded-xl text-sm bg-surface-subtle border border-border text-zinc-900 dark:text-zinc-100 focus:outline-none focus:ring-2 focus:ring-indigo-500"
                  >
                    <option value="mp4">MP4 (Recommended, most compatible)</option>
                    <option value="mkv">MKV (Supports all codecs & subs)</option>
                    <option value="webm">WebM (Native VP9 / AV1 / Opus)</option>
                    <option value="auto">Automatic (Match source stream)</option>
                  </select>
                </div>
              </div>

              {/* Container + Codec compatibility hint */}
              {container === "webm" &&
                (videoCodec === "h264" || videoCodec === "h265") && (
                  <div className="p-3 rounded-xl bg-amber-500/10 border border-amber-500/20 flex items-center gap-2 text-xs text-amber-700 dark:text-amber-300">
                    <Info className="w-4 h-4 shrink-0" />
                    <span>
                      WebM containers only support VP9/AV1 video streams. Consider
                      choosing MP4 or MKV for H.264/H.265 to avoid incompatibility.
                    </span>
                  </div>
                )}

              {/* Collapsible Advanced Video Options */}
              <div className="pt-1">
                <button
                  type="button"
                  onClick={() => setShowAdvanced((s) => !s)}
                  className="inline-flex items-center gap-1.5 text-xs font-medium text-zinc-600 dark:text-zinc-400 hover:text-zinc-900 dark:hover:text-zinc-100 transition-colors"
                >
                  <SlidersHorizontal className="w-3.5 h-3.5" />
                  Advanced Video & Metadata Options
                  {showAdvanced ? (
                    <ChevronUp className="w-3.5 h-3.5" />
                  ) : (
                    <ChevronDown className="w-3.5 h-3.5" />
                  )}
                </button>

                <AnimatePresence>
                  {showAdvanced && (
                    <motion.div
                      initial={{ opacity: 0, height: 0 }}
                      animate={{ opacity: 1, height: "auto" }}
                      exit={{ opacity: 0, height: 0 }}
                      className="overflow-hidden"
                    >
                      <div className="mt-3 p-4 rounded-xl bg-surface-subtle border border-border space-y-4">
                        <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
                          <div>
                            <label className="text-xs font-medium text-zinc-600 dark:text-zinc-400 block mb-1">
                              Preferred Video Codec
                            </label>
                            <select
                              value={videoCodec}
                              onChange={(e) =>
                                setVideoCodec(e.target.value as VideoCodecPref)
                              }
                              className="w-full px-3 py-2 rounded-lg text-xs bg-surface-elevated border border-border text-zinc-900 dark:text-zinc-100"
                            >
                              <option value="auto">Auto (Best match)</option>
                              <option value="h264">H.264 (AVC)</option>
                              <option value="h265">H.265 (HEVC)</option>
                              <option value="vp9">VP9</option>
                              <option value="av1">AV1</option>
                            </select>
                          </div>

                          <div>
                            <label className="text-xs font-medium text-zinc-600 dark:text-zinc-400 block mb-1">
                              Preferred Frame Rate
                            </label>
                            <select
                              value={preferredFps ?? ""}
                              onChange={(e) =>
                                setPreferredFps(
                                  e.target.value ? Number(e.target.value) : null
                                )
                              }
                              className="w-full px-3 py-2 rounded-lg text-xs bg-surface-elevated border border-border text-zinc-900 dark:text-zinc-100"
                            >
                              <option value="">Highest available</option>
                              <option value="60">Up to 60 FPS</option>
                              <option value="30">Up to 30 FPS</option>
                            </select>
                          </div>

                          <div>
                            <label className="text-xs font-medium text-zinc-600 dark:text-zinc-400 block mb-1">
                              Subtitle Language
                            </label>
                            <input
                              type="text"
                              value={subtitleLang}
                              onChange={(e) => setSubtitleLang(e.target.value)}
                              placeholder="en, es, ja..."
                              className="w-full px-3 py-2 rounded-lg text-xs bg-surface-elevated border border-border text-zinc-900 dark:text-zinc-100 font-mono"
                            />
                          </div>
                        </div>

                        <div className="grid grid-cols-2 md:grid-cols-4 gap-3 pt-1">
                          <label className="flex items-center gap-2 text-xs text-zinc-700 dark:text-zinc-300 cursor-pointer">
                            <input
                              type="checkbox"
                              checked={downloadSubtitles}
                              onChange={(e) =>
                                setDownloadSubtitles(e.target.checked)
                              }
                              className="rounded border-zinc-400 text-indigo-600"
                            />
                            Download subtitles
                          </label>
                          <label className="flex items-center gap-2 text-xs text-zinc-700 dark:text-zinc-300 cursor-pointer">
                            <input
                              type="checkbox"
                              checked={embedThumbnail}
                              onChange={(e) =>
                                setEmbedThumbnail(e.target.checked)
                              }
                              className="rounded border-zinc-400 text-indigo-600"
                            />
                            Embed thumbnail
                          </label>
                          <label className="flex items-center gap-2 text-xs text-zinc-700 dark:text-zinc-300 cursor-pointer">
                            <input
                              type="checkbox"
                              checked={embedMetadata}
                              onChange={(e) =>
                                setEmbedMetadata(e.target.checked)
                              }
                              className="rounded border-zinc-400 text-indigo-600"
                            />
                            Embed metadata
                          </label>
                          <label className="flex items-center gap-2 text-xs text-zinc-700 dark:text-zinc-300 cursor-pointer">
                            <input
                              type="checkbox"
                              checked={downloadChapters}
                              onChange={(e) =>
                                setDownloadChapters(e.target.checked)
                              }
                              className="rounded border-zinc-400 text-indigo-600"
                            />
                            Embed chapters
                          </label>
                        </div>

                        <div className="grid grid-cols-1 md:grid-cols-2 gap-3 pt-1">
                          <div>
                            <label className="text-xs font-medium text-zinc-600 dark:text-zinc-400 block mb-1">
                              File Naming Template
                            </label>
                            <input
                              type="text"
                              value={customTemplate}
                              onChange={(e) => setCustomTemplate(e.target.value)}
                              className="w-full px-3 py-1.5 rounded-lg text-xs font-mono bg-surface-elevated border border-border text-zinc-900 dark:text-zinc-100"
                            />
                          </div>
                          <div>
                            <label className="text-xs font-medium text-zinc-600 dark:text-zinc-400 block mb-1">
                              Speed Limit (KB/s, 0 = Unlimited)
                            </label>
                            <input
                              type="number"
                              min={0}
                              value={speedLimitKbps}
                              onChange={(e) =>
                                setSpeedLimitKbps(
                                  Math.max(0, Number(e.target.value) || 0)
                                )
                              }
                              className="w-full px-3 py-1.5 rounded-lg text-xs font-mono bg-surface-elevated border border-border text-zinc-900 dark:text-zinc-100"
                            />
                          </div>
                        </div>
                      </div>
                    </motion.div>
                  )}
                </AnimatePresence>
              </div>
            </motion.div>
          ) : (
            <motion.div
              key="audio-controls"
              initial={{ opacity: 0, y: 6 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -6 }}
              transition={{ duration: 0.18 }}
              className="space-y-4"
            >
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                {/* Audio Format */}
                <div className="space-y-1.5">
                  <label
                    htmlFor="audio-format-select"
                    className="text-xs font-medium text-zinc-700 dark:text-zinc-300 block"
                  >
                    Audio Format
                  </label>
                  <select
                    id="audio-format-select"
                    value={audioFormat}
                    onChange={(e) =>
                      setAudioFormat(e.target.value as AudioFormat)
                    }
                    className="w-full px-3.5 py-2.5 rounded-xl text-sm bg-surface-subtle border border-border text-zinc-900 dark:text-zinc-100 focus:outline-none focus:ring-2 focus:ring-indigo-500"
                  >
                    <option value="mp3">MP3 (Universal compatibility)</option>
                    <option value="m4a">M4A (AAC / Native audio)</option>
                    <option value="best">
                      Original Best Available (No re-encoding)
                    </option>
                    <option value="flac">FLAC (Lossless container)</option>
                    <option value="wav">WAV (Uncompressed PCM)</option>
                    <option value="opus">OPUS (High efficiency)</option>
                  </select>
                </div>

                {/* Audio Encoding Bitrate / Quality */}
                <div className="space-y-1.5">
                  <label
                    htmlFor="audio-quality-select"
                    className="text-xs font-medium text-zinc-700 dark:text-zinc-300 block"
                  >
                    {audioFormat === "mp3"
                      ? "MP3 Target Encoding Bitrate"
                      : "Audio Quality Target"}
                  </label>
                  <select
                    id="audio-quality-select"
                    value={audioQuality}
                    disabled={
                      audioFormat === "best" ||
                      audioFormat === "flac" ||
                      audioFormat === "wav"
                    }
                    onChange={(e) =>
                      setAudioQuality(e.target.value as AudioQualityPreset)
                    }
                    className="w-full px-3.5 py-2.5 rounded-xl text-sm bg-surface-subtle border border-border text-zinc-900 dark:text-zinc-100 focus:outline-none focus:ring-2 focus:ring-indigo-500 disabled:opacity-60"
                  >
                    <option value="best">Best available source quality</option>
                    <option value="320">High — 320 kbps</option>
                    <option value="256">Medium-High — 256 kbps</option>
                    <option value="192">Medium — 192 kbps</option>
                    <option value="128">Standard — 128 kbps</option>
                  </select>
                </div>
              </div>

              <p className="text-[11px] text-zinc-500 dark:text-zinc-400 flex items-center gap-1.5">
                <Info className="w-3.5 h-3.5 text-indigo-400 shrink-0" />
                Note: Target bitrate controls FFmpeg output encoding. Re-encoding
                a lower-bitrate source stream to 320 kbps does not increase its
                original source fidelity.
              </p>

              {/* Audio options checkboxes */}
              <div className="p-3.5 rounded-xl bg-surface-subtle border border-border grid grid-cols-2 md:grid-cols-4 gap-3">
                <label className="flex items-center gap-2 text-xs text-zinc-700 dark:text-zinc-300 cursor-pointer">
                  <input
                    type="checkbox"
                    checked={embedMetadata}
                    onChange={(e) => setEmbedMetadata(e.target.checked)}
                    className="rounded border-zinc-400 text-indigo-600"
                  />
                  Embed metadata
                </label>
                <label className="flex items-center gap-2 text-xs text-zinc-700 dark:text-zinc-300 cursor-pointer">
                  <input
                    type="checkbox"
                    checked={embedThumbnail}
                    disabled={audioFormat === "wav"}
                    onChange={(e) => setEmbedThumbnail(e.target.checked)}
                    className="rounded border-zinc-400 text-indigo-600 disabled:opacity-50"
                  />
                  Embed cover art
                </label>
                <label className="flex items-center gap-2 text-xs text-zinc-700 dark:text-zinc-300 cursor-pointer">
                  <input
                    type="checkbox"
                    checked={preserveOriginalFilename}
                    onChange={(e) =>
                      setPreserveOriginalFilename(e.target.checked)
                    }
                    className="rounded border-zinc-400 text-indigo-600"
                  />
                  Clean title filename
                </label>
                <label className="flex items-center gap-2 text-xs text-zinc-700 dark:text-zinc-300 cursor-pointer">
                  <input
                    type="checkbox"
                    checked={normalizeAudio}
                    onChange={(e) => setNormalizeAudio(e.target.checked)}
                    className="rounded border-zinc-400 text-indigo-600"
                  />
                  Normalize volume
                </label>
              </div>
            </motion.div>
          )}
        </AnimatePresence>

        {/* Bottom action bar: Output destination + Primary Download button */}
        <div className="pt-3 border-t border-border flex flex-col sm:flex-row items-stretch sm:items-center justify-between gap-3">
          <div className="flex items-center gap-2 min-w-0 flex-1">
            <button
              type="button"
              onClick={handleBrowseFolder}
              className="inline-flex items-center gap-2 px-3 py-2 rounded-xl text-xs font-medium bg-surface-subtle hover:bg-surface-hover border border-border text-zinc-700 dark:text-zinc-300 transition-colors shrink-0"
            >
              <FolderOpen className="w-3.5 h-3.5 text-indigo-500" />
              Save to
            </button>
            <span
              className="text-xs font-mono text-zinc-500 dark:text-zinc-400 truncate"
              title={customOutputDir}
            >
              {customOutputDir}
            </span>
          </div>

          <button
            type="button"
            onClick={handleTriggerDownload}
            className="inline-flex items-center justify-center gap-2.5 px-6 py-3 rounded-xl text-sm font-semibold bg-indigo-600 hover:bg-indigo-500 active:scale-[0.99] text-white shadow-md shadow-indigo-500/25 transition-all"
          >
            <ArrowDownToLine className="w-4 h-4" />
            {mode === "audio" ? "Download Audio" : "Download Video"}
          </button>
        </div>
      </div>
    </motion.div>
  );
};

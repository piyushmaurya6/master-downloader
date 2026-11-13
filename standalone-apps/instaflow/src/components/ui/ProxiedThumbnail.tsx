import React, { useEffect, useState } from "react";
import { Film, Image as ImageIcon } from "lucide-react";
import { ipc, isTauriRuntime } from "../../lib/ipc";
import { cn } from "../../lib/utils";

interface ProxiedThumbnailProps {
  src?: string | null;
  alt?: string;
  mediaType?: "image" | "video" | "audio" | string;
  className?: string;
}

export const ProxiedThumbnail: React.FC<ProxiedThumbnailProps> = ({
  src,
  alt = "Instagram media preview",
  mediaType = "image",
  className,
}) => {
  const [resolvedSrc, setResolvedSrc] = useState<string | null>(null);
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    let active = true;
    setFailed(false);

    if (!src) {
      setResolvedSrc(null);
      return;
    }

    if (src.startsWith("data:") || src.startsWith("blob:") || !isTauriRuntime()) {
      setResolvedSrc(src);
      return;
    }

    ipc
      .proxyThumbnailUrl(src)
      .then((dataUri) => {
        if (active) setResolvedSrc(dataUri);
      })
      .catch(() => {
        if (active) setResolvedSrc(src);
      });

    return () => {
      active = false;
    };
  }, [src]);

  if (!resolvedSrc || failed) {
    return (
      <div
        className={cn(
          "flex flex-col items-center justify-center bg-zinc-900/80 text-zinc-500 border border-white/5",
          className
        )}
      >
        {mediaType === "video" ? (
          <Film className="w-5 h-5 text-indigo-400/70" />
        ) : (
          <ImageIcon className="w-5 h-5 text-zinc-500" />
        )}
      </div>
    );
  }

  return (
    <img
      src={resolvedSrc}
      alt={alt}
      onError={() => setFailed(true)}
      className={cn("object-cover bg-zinc-900", className)}
    />
  );
};

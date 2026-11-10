import { Film } from "lucide-react";
import { useState } from "react";
import { cn } from "@/lib/utils";

export function PosterImage({
  src,
  alt,
  title,
  className,
  imageClassName,
}: {
  src?: string | null;
  alt: string;
  title?: string;
  className?: string;
  imageClassName?: string;
}) {
  const [imageState, setImageState] = useState<{
    src?: string | null;
    loaded: boolean;
    failed: boolean;
  }>({ src, loaded: false, failed: false });

  const isCurrent = imageState.src === src;
  const loaded = isCurrent && imageState.loaded;
  const failed = isCurrent && imageState.failed;

  return (
    <div className={cn("relative h-full w-full overflow-hidden bg-muted/60", className)}>
      <div
        aria-hidden="true"
        className={cn(
          "absolute inset-0 flex h-full w-full flex-col items-center justify-center gap-2 bg-[radial-gradient(circle_at_30%_20%,oklch(0.3_0.03_20),oklch(0.16_0.005_285))] px-3 text-center",
          loaded && !failed ? "invisible" : "visible",
        )}
      >
        <Film className="size-7 text-white/30" />
        <span className="line-clamp-2 text-[0.7rem] font-medium leading-tight text-white/45">
          {title ?? alt}
        </span>
      </div>

      {src && !failed && (
        <img
          src={src}
          alt={alt}
          loading="lazy"
          onLoad={() => setImageState({ src, loaded: true, failed: false })}
          onError={() => setImageState({ src, loaded: false, failed: true })}
          className={cn(
            "relative h-full w-full object-cover transition-opacity duration-500",
            loaded ? "opacity-100" : "opacity-0",
            imageClassName,
          )}
        />
      )}
    </div>
  );
}

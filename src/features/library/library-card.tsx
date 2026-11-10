import { useMutation, useQueryClient } from "@tanstack/react-query";
import {
  Check,
  History,
  MoreVertical,
  Play,
  Trash2,
  Tv,
} from "lucide-react";
import { useState } from "react";
import { Link } from "react-router-dom";
import { toast } from "sonner";
import { ConfirmDialog } from "@/components/confirm-dialog";
import { PosterImage } from "@/components/poster-image";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { api, errorMessage } from "@/lib/api";
import { queryKeys } from "@/lib/query";
import type { LibraryItem } from "@/lib/types";
import { cn, formatDuration, progressPercent } from "@/lib/utils";

export function LibraryCard({
  item,
  showProgress = false,
  className,
}: {
  item: LibraryItem;
  showProgress?: boolean;
  className?: string;
}) {
  const queryClient = useQueryClient();
  const [confirm, setConfirm] = useState<"remove" | "history" | null>(null);

  const remove = useMutation({
    mutationFn: () => api.removeFromLibrary(item.id),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: queryKeys.library });
      await queryClient.invalidateQueries({ queryKey: queryKeys.continueWatching });
      toast.success(`Removed “${item.title}” from your library.`);
      setConfirm(null);
    },
    onError: (error) => toast.error(errorMessage(error)),
  });

  const clearHistory = useMutation({
    mutationFn: () => api.clearWatchHistory(item.id),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: queryKeys.library });
      await queryClient.invalidateQueries({ queryKey: queryKeys.continueWatching });
      toast.success(`Cleared watch history for “${item.title}”.`);
      setConfirm(null);
    },
    onError: (error) => toast.error(errorMessage(error)),
  });

  const percent = progressPercent(item.watchProgressSeconds, item.durationSeconds);
  const meta =
    showProgress && item.watchProgressSeconds > 0
      ? `${formatDuration(item.watchProgressSeconds)}${
          item.durationSeconds > 0 ? ` / ${formatDuration(item.durationSeconds)}` : ""
        }`
      : item.categoryName;

  return (
    <>
      <article className={cn("group relative min-w-0", className)}>
        <Link
          to={`/watch/${encodeURIComponent(item.identifier)}`}
          className="block outline-none"
          aria-label={`Watch ${item.title}`}
        >
          <div className="relative aspect-[2/3] overflow-hidden rounded-xl bg-card ring-1 ring-white/10 transition duration-300 poster-shadow group-hover:ring-primary/50 group-hover:brightness-105 group-focus-visible:ring-3 group-focus-visible:ring-ring/60">
            <PosterImage
              src={item.tmdbPosterUrl || item.thumbnailUrl}
              alt={item.title}
              title={item.title}
              imageClassName="transition duration-500 group-hover:scale-[1.04]"
            />

            <div className="absolute inset-0 bg-gradient-to-t from-black/80 via-black/0 to-black/0 opacity-0 transition group-hover:opacity-100" />

            <div className="absolute inset-0 grid place-items-center opacity-0 transition duration-200 group-hover:opacity-100">
              <span className="grid size-11 place-items-center rounded-full bg-primary/90 text-primary-foreground shadow-xl backdrop-blur">
                <Play className="ml-0.5 size-4 fill-current" strokeWidth={0} />
              </span>
            </div>

            <div className="absolute left-2 top-2 flex gap-1">
              {item.isCompleted && (
                <Badge className="gap-1 bg-black/70 text-white backdrop-blur">
                  <Check className="size-3" />
                  Watched
                </Badge>
              )}
              {item.mediaType.toLowerCase() === "tv" && (
                <Badge className="gap-1 bg-black/70 text-white backdrop-blur">
                  <Tv className="size-3" />
                  Series
                </Badge>
              )}
            </div>

            {showProgress && percent > 0 && (
              <div className="absolute inset-x-0 bottom-0 h-1.5 bg-black/60">
                <div
                  className="h-full rounded-r-full bg-primary"
                  style={{ width: `${percent}%` }}
                />
              </div>
            )}
          </div>

          <div className="mt-2 px-0.5">
            <h3 className="truncate text-[0.82rem] font-medium leading-snug text-foreground/95">
              {item.title}
            </h3>
            <p className="mt-0.5 truncate text-[0.7rem] text-muted-foreground">{meta}</p>
          </div>
        </Link>

        <div className="absolute right-2 top-2 opacity-0 transition group-hover:opacity-100 group-focus-within:opacity-100">
          <DropdownMenu>
            <DropdownMenuTrigger
              render={
                <Button
                  variant="secondary"
                  size="icon-xs"
                  className="rounded-full bg-black/75 text-white shadow-lg backdrop-blur hover:bg-black"
                  aria-label={`Actions for ${item.title}`}
                >
                  <MoreVertical />
                </Button>
              }
            />
            <DropdownMenuContent align="end" className="w-48">
              <DropdownMenuItem
                render={
                  <Link to={`/watch/${encodeURIComponent(item.identifier)}`}>
                    <Play className="size-3.5" />
                    Play
                  </Link>
                }
              />
              {item.watchProgressSeconds > 0 && (
                <DropdownMenuItem onClick={() => setConfirm("history")}>
                  <History className="size-3.5" />
                  Clear watch history
                </DropdownMenuItem>
              )}
              <DropdownMenuSeparator />
              <DropdownMenuItem variant="destructive" onClick={() => setConfirm("remove")}>
                <Trash2 className="size-3.5" />
                Remove from library
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      </article>

      <ConfirmDialog
        open={confirm === "remove"}
        onOpenChange={(open) => !open && setConfirm(null)}
        title="Remove from library?"
        description={
          <>
            <span className="font-medium text-foreground">“{item.title}”</span> and its watch
            history will be removed from this profile. The media stays on Archive.org.
          </>
        }
        confirmLabel="Remove"
        destructive
        pending={remove.isPending}
        onConfirm={() => remove.mutate()}
      />

      <ConfirmDialog
        open={confirm === "history"}
        onOpenChange={(open) => !open && setConfirm(null)}
        title="Clear watch history?"
        description={
          <>
            “{item.title}” will leave Continue Watching. The item stays in your library.
          </>
        }
        confirmLabel="Clear history"
        pending={clearHistory.isPending}
        onConfirm={() => clearHistory.mutate()}
      />
    </>
  );
}

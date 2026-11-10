import { useQuery, useQueryClient } from "@tanstack/react-query";
import {
  AlertTriangle,
  ArrowLeft,
  Check,
  ExternalLink,
  Gauge,
  ListVideo,
  Loader2,
  Maximize,
  Minimize,
  Pause,
  PictureInPicture2,
  Play,
  SkipForward,
  Volume2,
} from "lucide-react";
import {
  type MouseEvent as ReactMouseEvent,
  type TouchEvent as ReactTouchEvent,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { Link, useParams } from "react-router-dom";
import { toast } from "sonner";
import { PosterImage } from "@/components/poster-image";
import { EmptyState } from "@/components/empty-state";
import { PageLoader } from "@/components/spinner";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Separator } from "@/components/ui/separator";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { api, errorMessage } from "@/lib/api";
import { openExternal } from "@/lib/links";
import { queryKeys } from "@/lib/query";
import type { Episode, EpisodeProgress } from "@/lib/types";
import {
  cn,
  episodeLabel,
  formatDuration,
  humanizeFileName,
  progressPercent,
} from "@/lib/utils";

const VOLUME_KEY = "archiveplayer:volume";
const SPEEDS = [0.5, 0.75, 1, 1.25, 1.5, 1.75, 2];

function episodeDisplayName(episode: Episode | null): string {
  if (!episode) return "";
  const label = episodeLabel(episode.seasonNumber, episode.episodeNumber);
  const name = episode.name || humanizeFileName(episode.fileName);
  return label ? `${label} · ${name}` : name;
}

function readStoredVolume(): number {
  const raw = window.localStorage.getItem(VOLUME_KEY);
  const value = raw === null ? 1 : Number(raw);
  return Number.isFinite(value) ? Math.min(1, Math.max(0, value)) : 1;
}

export function WatchPage() {
  const { identifier = "" } = useParams();
  const queryClient = useQueryClient();
  const containerRef = useRef<HTMLDivElement>(null);
  const videoRef = useRef<HTMLVideoElement>(null);
  const selectedFileRef = useRef<string | null>(null);
  const progressRef = useRef<Map<string, EpisodeProgress>>(new Map());
  const lastSaveRef = useRef(0);
  const hudTimerRef = useRef<number | null>(null);
  const lastTapRef = useRef(0);

  const [selectedFile, setSelectedFile] = useState<string | null>(null);
  const [seasonFilter, setSeasonFilter] = useState("all");
  const [speed, setSpeed] = useState(1);
  const [autoplay, setAutoplay] = useState(true);
  const [progressMap, setProgressMap] = useState<Map<string, EpisodeProgress>>(new Map());
  const [countdown, setCountdown] = useState<number | null>(null);
  const [hud, setHud] = useState<string | null>(null);
  const [buffering, setBuffering] = useState(false);
  const [videoError, setVideoError] = useState<string | null>(null);
  const [isFullscreen, setIsFullscreen] = useState(false);
  const [muted, setMuted] = useState(false);

  const detailsQuery = useQuery({
    queryKey: queryKeys.playback(identifier),
    queryFn: () => api.getPlaybackDetails(identifier),
    enabled: Boolean(identifier),
    staleTime: 0,
    refetchOnWindowFocus: false,
    retry: 1,
  });

  const details = detailsQuery.data;

  const urlMap = useMemo(
    () => new Map(details?.videoFileUrls.map((file) => [file.fileName, file.url]) ?? []),
    [details],
  );

  const availableFiles = useMemo(() => {
    if (!details) return new Set<string>();
    const names = details.videoFileUrls.map((file) => file.fileName);
    return new Set(names.length > 0 ? names : details.videoFiles);
  }, [details]);

  const episodes = useMemo(
    () => details?.episodes.filter((episode) => availableFiles.has(episode.fileName)) ?? [],
    [details, availableFiles],
  );

  const currentIndex = episodes.findIndex((episode) => episode.fileName === selectedFile);
  const nextEpisode = currentIndex >= 0 ? episodes[currentIndex + 1] ?? null : null;
  const previousEpisode = currentIndex > 0 ? episodes[currentIndex - 1] : null;
  const nextEpisodeRef = useRef<Episode | null>(nextEpisode);
  nextEpisodeRef.current = nextEpisode;
  const autoplayRef = useRef(autoplay);
  autoplayRef.current = autoplay;

  const seasons = useMemo(() => {
    const values = new Set<number>();
    for (const episode of episodes) {
      if (episode.seasonNumber != null) values.add(episode.seasonNumber);
    }
    return Array.from(values).sort((a, b) => a - b);
  }, [episodes]);

  const visibleEpisodes = useMemo(
    () =>
      seasonFilter === "all"
        ? episodes
        : episodes.filter((episode) => String(episode.seasonNumber) === seasonFilter),
    [episodes, seasonFilter],
  );

  const currentEpisode = episodes.find((episode) => episode.fileName === selectedFile) ?? null;
  const currentTitle = details?.title ?? "Loading";
  const showEpisodeList = episodes.length > 1;

  const persist = useCallback(
    (fileName: string, video: HTMLVideoElement) => {
      const positionSeconds = Math.max(0, Math.floor(video.currentTime));
      const durationSeconds = Math.floor(Number.isFinite(video.duration) ? video.duration : 0);
      if (positionSeconds < 1) return;
      const entry: EpisodeProgress = {
        fileName,
        positionSeconds,
        durationSeconds,
        completed: false,
      };
      setProgressMap((previous) => {
        const next = new Map(previous);
        next.set(fileName, entry);
        return next;
      });
      void api
        .saveProgress(identifier, fileName, positionSeconds, durationSeconds)
        .catch(() => undefined);
    },
    [identifier],
  );

  const flash = useCallback((message: string) => {
    setHud(message);
    if (hudTimerRef.current) window.clearTimeout(hudTimerRef.current);
    hudTimerRef.current = window.setTimeout(() => setHud(null), 1100);
  }, []);

  const switchEpisode = useCallback((fileName: string) => {
    setSelectedFile(fileName);
    setCountdown(null);
    setVideoError(null);
    setBuffering(true);
  }, []);

  // Initialise selection, progress, and autoplay from the backend response.
  useEffect(() => {
    if (!details) return;
    const preferred = details.currentEpisodeFileName;
    setSelectedFile((previous) => {
      if (previous && availableFiles.has(previous)) return previous;
      if (preferred && availableFiles.has(preferred)) return preferred;
      return details.videoFiles.find((file) => availableFiles.has(file)) ?? null;
    });
    const map = new Map<string, EpisodeProgress>();
    for (const entry of details.episodeProgress) map.set(entry.fileName, entry);
    setProgressMap(map);
    setAutoplay(details.autoplayEnabled);
  }, [details, availableFiles]);

  useEffect(() => {
    if (!details) return;
    const current = details.episodes.find(
      (episode) => episode.fileName === details.currentEpisodeFileName,
    );
    setSeasonFilter(current?.seasonNumber != null ? String(current.seasonNumber) : "all");
  }, [details?.identifier]); // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    selectedFileRef.current = selectedFile;
  }, [selectedFile]);

  useEffect(() => {
    progressRef.current = progressMap;
  }, [progressMap]);

  // Save the captured video's position when the episode changes or the page unmounts.
  useEffect(() => {
    const video = videoRef.current;
    const file = selectedFile;
    if (!video || !file) return;
    return () => {
      const positionSeconds = Math.floor(video.currentTime);
      const durationSeconds = Math.floor(Number.isFinite(video.duration) ? video.duration : 0);
      if (positionSeconds > 1) {
        void api
          .saveProgress(identifier, file, positionSeconds, durationSeconds)
          .catch(() => undefined);
      }
    };
  }, [identifier, selectedFile]);

  // The autoplay countdown.
  useEffect(() => {
    if (countdown === null) return;
    if (countdown <= 0) {
      setCountdown(null);
      const next = nextEpisodeRef.current;
      if (next && autoplayRef.current) switchEpisode(next.fileName);
      return;
    }
    const timer = window.setTimeout(
      () => setCountdown((value) => (value === null ? null : value - 1)),
      1000,
    );
    return () => window.clearTimeout(timer);
  }, [countdown, switchEpisode]);

  // Keyboard shortcuts.
  useEffect(() => {
    function isTyping(target: EventTarget | null) {
      const element = target as HTMLElement | null;
      return Boolean(
        element?.closest(
          "input, textarea, select, button, a, [contenteditable='true'], [role='dialog'], [role='listbox']",
        ),
      );
    }

    function handler(event: KeyboardEvent) {
      if (event.metaKey || event.ctrlKey || event.altKey || isTyping(event.target)) return;
      const video = videoRef.current;
      if (!video) return;

      switch (event.key.toLowerCase()) {
        case " ":
        case "k":
          event.preventDefault();
          if (video.paused) void video.play().catch(() => undefined);
          else video.pause();
          break;
        case "arrowleft":
          event.preventDefault();
          video.currentTime = Math.max(0, video.currentTime - 10);
          flash("−10s");
          break;
        case "arrowright":
          event.preventDefault();
          video.currentTime = Math.min(video.duration || Infinity, video.currentTime + 10);
          flash("+10s");
          break;
        case "m":
          video.muted = !video.muted;
          setMuted(video.muted);
          flash(video.muted ? "Muted" : "Sound on");
          break;
        case "f":
          void toggleFullscreen();
          break;
        case "n":
          if (nextEpisodeRef.current) {
            switchEpisode(nextEpisodeRef.current.fileName);
            flash("Next episode");
          }
          break;
        case "p": {
          const current = episodes.findIndex(
            (episode) => episode.fileName === selectedFileRef.current,
          );
          if (current > 0) {
            switchEpisode(episodes[current - 1].fileName);
            flash("Previous episode");
          }
          break;
        }
        default:
          break;
      }
    }

    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [episodes, flash, switchEpisode]);

  useEffect(() => {
    function onFullscreenChange() {
      setIsFullscreen(document.fullscreenElement === containerRef.current);
    }
    document.addEventListener("fullscreenchange", onFullscreenChange);
    return () => document.removeEventListener("fullscreenchange", onFullscreenChange);
  }, []);

  useEffect(() => {
    return () => {
      if (hudTimerRef.current) window.clearTimeout(hudTimerRef.current);
    };
  }, []);

  async function toggleFullscreen() {
    const container = containerRef.current;
    if (!container) return;
    try {
      if (document.fullscreenElement) await document.exitFullscreen();
      else await container.requestFullscreen();
    } catch {
      toast.error("Fullscreen is not available.");
    }
  }

  async function togglePictureInPicture() {
    const video = videoRef.current;
    if (!video) return;
    try {
      if (document.pictureInPictureElement) await document.exitPictureInPicture();
      else await video.requestPictureInPicture();
    } catch {
      toast.error("Picture-in-picture is not available.");
    }
  }

  function seekBy(delta: number) {
    const video = videoRef.current;
    if (!video) return;
    video.currentTime = Math.max(
      0,
      Math.min(video.duration || Infinity, video.currentTime + delta),
    );
    flash(delta > 0 ? "+10s" : "−10s");
  }

  function handleDoubleClick(event: ReactMouseEvent<HTMLDivElement>) {
    const target = event.target as HTMLElement;
    if (target.closest("button, select, input, [role='dialog']")) return;
    const rect = event.currentTarget.getBoundingClientRect();
    const x = event.clientX - rect.left;
    const y = event.clientY - rect.top;
    if (y > rect.height * 0.86) return;
    if (x < rect.width * 0.35) seekBy(-10);
    else if (x > rect.width * 0.65) seekBy(10);
  }

  function handleTouchEnd(event: ReactTouchEvent<HTMLDivElement>) {
    const target = event.target as HTMLElement;
    if (target.closest("button, select, input")) return;
    const now = Date.now();
    if (now - lastTapRef.current < 300) {
      const touch = event.changedTouches[0];
      const rect = event.currentTarget.getBoundingClientRect();
      const x = touch.clientX - rect.left;
      if (x < rect.width * 0.4) seekBy(-10);
      else if (x > rect.width * 0.6) seekBy(10);
      lastTapRef.current = 0;
    } else {
      lastTapRef.current = now;
    }
  }

  function handleEnded() {
    const video = videoRef.current;
    const file = selectedFile;
    if (!video || !file) return;
    const durationSeconds = Math.floor(Number.isFinite(video.duration) ? video.duration : 0);
    setProgressMap((previous) => {
      const next = new Map(previous);
      next.set(file, {
        fileName: file,
        positionSeconds: durationSeconds,
        durationSeconds,
        completed: true,
      });
      return next;
    });
    const hasNext = Boolean(nextEpisode);
    void api
      .markCompleted(identifier, file, !hasNext)
      .then(async () => {
        await queryClient.invalidateQueries({ queryKey: queryKeys.library });
        await queryClient.invalidateQueries({ queryKey: queryKeys.continueWatching });
      })
      .catch(() => undefined);

    if (nextEpisode && autoplay) {
      setCountdown(10);
    } else if (!nextEpisode) {
      flash("Finished");
    }
  }

  if (detailsQuery.isLoading) {
    return (
      <div className="mx-auto w-full max-w-7xl px-4 py-10 lg:px-8">
        <PageLoader label="Preparing your stream" />
      </div>
    );
  }

  if (detailsQuery.isError || !details) {
    return (
      <div className="mx-auto w-full max-w-3xl px-4 py-10 lg:px-8">
        <Link
          to="/"
          className="mb-6 inline-flex items-center gap-1.5 text-sm text-muted-foreground hover:text-foreground"
        >
          <ArrowLeft className="size-4" />
          Back to library
        </Link>
        <EmptyState
          icon={AlertTriangle}
          title="This title could not be loaded"
          description={detailsQuery.isError ? errorMessage(detailsQuery.error) : "Unknown error."}
          action={
            <Button onClick={() => detailsQuery.refetch()}>
              Try again
            </Button>
          }
        />
      </div>
    );
  }

  const currentUrl = selectedFile ? urlMap.get(selectedFile) : undefined;
  const percent = progressPercent(details.watchProgressSeconds, details.durationSeconds);
  const isTv = details.mediaType.toLowerCase() === "tv";

  return (
    <div className="mx-auto w-full max-w-7xl px-4 pb-8 pt-5 lg:px-8">
      <Link
        to="/"
        className="mb-4 inline-flex items-center gap-1.5 text-sm text-muted-foreground transition hover:text-foreground"
      >
        <ArrowLeft className="size-4" />
        Back to library
      </Link>

      <div className="grid gap-6 lg:grid-cols-[minmax(0,1fr)_340px]">
        <div className="min-w-0">
          <div
            ref={containerRef}
            className="group/player relative overflow-hidden rounded-2xl bg-black ring-1 ring-white/10"
            onDoubleClick={handleDoubleClick}
            onTouchEnd={handleTouchEnd}
          >
            {currentUrl ? (
              <video
                key={selectedFile ?? "video"}
                ref={videoRef}
                src={currentUrl}
                controls
                autoPlay
                playsInline
                preload="metadata"
                className="aspect-video w-full bg-black"
                onLoadedMetadata={(event) => {
                  const video = event.currentTarget;
                  const file = selectedFile;
                  video.playbackRate = speed;
                  video.volume = readStoredVolume();
                  video.muted = muted;
                  if (file) {
                    const saved = progressRef.current.get(file);
                    if (
                      saved &&
                      !saved.completed &&
                      saved.positionSeconds > 5 &&
                      (saved.durationSeconds <= 0 ||
                        saved.positionSeconds < saved.durationSeconds - 10)
                    ) {
                      video.currentTime = saved.positionSeconds;
                    } else if (
                      !saved &&
                      !details.isCompleted &&
                      file === details.currentEpisodeFileName &&
                      details.watchProgressSeconds > 5
                    ) {
                      video.currentTime = details.watchProgressSeconds;
                    }
                  }
                  void video.play().catch(() => undefined);
                }}
                onTimeUpdate={(event) => {
                  const video = event.currentTarget;
                  if (video.paused) return;
                  const now = Date.now();
                  if (now - lastSaveRef.current < 6000) return;
                  lastSaveRef.current = now;
                  if (selectedFile) persist(selectedFile, video);
                }}
                onPause={(event) => {
                  if (selectedFile) persist(selectedFile, event.currentTarget);
                }}
                onSeeked={(event) => {
                  if (selectedFile) persist(selectedFile, event.currentTarget);
                }}
                onPlay={() => setBuffering(false)}
                onWaiting={() => setBuffering(true)}
                onCanPlay={() => setBuffering(false)}
                onVolumeChange={(event) => {
                  const video = event.currentTarget;
                  setMuted(video.muted);
                  window.localStorage.setItem(VOLUME_KEY, String(video.volume));
                }}
                onEnded={handleEnded}
                onError={() => {
                  setBuffering(false);
                  setVideoError(
                    "The browser could not play this file. It may be unavailable or use an unsupported format.",
                  );
                }}
              />
            ) : (
              <div className="grid aspect-video w-full place-items-center bg-black text-center text-sm text-muted-foreground">
                No playable video file was found for this item.
              </div>
            )}

            <div className="pointer-events-none absolute inset-x-0 top-0 z-10 flex items-start justify-between gap-3 bg-gradient-to-b from-black/80 via-black/30 to-transparent p-3">
              <div className="min-w-0 rounded-lg bg-black/45 px-2.5 py-1.5 backdrop-blur-sm">
                <p className="truncate text-xs font-medium text-white">{currentTitle}</p>
                {isTv && currentEpisode && (
                  <p className="mt-0.5 truncate text-[0.65rem] text-white/65">
                    {episodeDisplayName(currentEpisode)}
                  </p>
                )}
                {!isTv && selectedFile && (
                  <p className="mt-0.5 truncate text-[0.65rem] text-white/65">
                    {humanizeFileName(selectedFile)}
                  </p>
                )}
              </div>

              <div className="pointer-events-auto flex items-center gap-1.5">
                <Select
                  value={String(speed)}
                  onValueChange={(value) => {
                    const next = Number(value);
                    setSpeed(next);
                    if (videoRef.current) videoRef.current.playbackRate = next;
                    flash(`${next}× speed`);
                  }}
                  items={SPEEDS.map((value) => ({ value: String(value), label: `${value}×` }))}
                >
                  <SelectTrigger
                    size="sm"
                    className="h-8 w-[4.4rem] border-white/15 bg-black/50 text-xs text-white backdrop-blur hover:bg-black/70"
                    aria-label="Playback speed"
                  >
                    <Gauge className="size-3.5 opacity-70" />
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {SPEEDS.map((value) => (
                      <SelectItem key={value} value={String(value)}>
                        {value}×
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>

                <div className="hidden items-center gap-1.5 rounded-lg border border-white/15 bg-black/50 px-2 py-1.5 backdrop-blur sm:flex">
                  <span className="text-[0.65rem] text-white/70">Autoplay</span>
                  <Switch
                    checked={autoplay}
                    onCheckedChange={(checked) => {
                      setAutoplay(checked);
                      if (details.libraryItemId != null) {
                        void api
                          .setAutoplay(details.libraryItemId, checked)
                          .catch(() => undefined);
                      }
                      if (!checked) setCountdown(null);
                    }}
                    className="scale-75"
                    aria-label="Autoplay next episode"
                  />
                </div>

                <Button
                  variant="ghost"
                  size="icon-sm"
                  className="rounded-lg text-white hover:bg-white/15 hover:text-white"
                  onClick={() => void togglePictureInPicture()}
                  aria-label="Picture in picture"
                >
                  <PictureInPicture2 />
                </Button>

                <Button
                  variant="ghost"
                  size="icon-sm"
                  className="rounded-lg text-white hover:bg-white/15 hover:text-white"
                  onClick={() => void toggleFullscreen()}
                  aria-label={isFullscreen ? "Exit fullscreen" : "Fullscreen"}
                >
                  {isFullscreen ? <Minimize /> : <Maximize />}
                </Button>
              </div>
            </div>

            {buffering && currentUrl && !videoError && (
              <div className="pointer-events-none absolute inset-0 grid place-items-center bg-black/20">
                <div className="grid size-14 place-items-center rounded-full bg-black/60 backdrop-blur">
                  <Loader2 className="size-6 animate-spin text-white" />
                </div>
              </div>
            )}

            {hud && (
              <div className="pointer-events-none absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 rounded-full bg-black/70 px-4 py-2 text-sm font-medium text-white shadow-xl backdrop-blur">
                {hud}
              </div>
            )}

            {countdown !== null && nextEpisode && (
              <div className="absolute inset-x-0 bottom-14 z-20 mx-auto w-[min(92%,26rem)] overflow-hidden rounded-xl border border-white/15 bg-black/80 text-white shadow-2xl backdrop-blur-md">
                <div className="h-0.5 bg-white/15">
                  <div
                    className="h-full bg-primary transition-[width] duration-1000 ease-linear"
                    style={{ width: `${(countdown / 10) * 100}%` }}
                  />
                </div>
                <div className="p-3.5">
                  <p className="text-[0.65rem] uppercase tracking-wider text-white/55">
                    Up next in {countdown}s
                  </p>
                  <p className="mt-0.5 truncate text-sm font-medium">
                    {nextEpisode.name || humanizeFileName(nextEpisode.fileName)}
                  </p>
                  <div className="mt-2.5 flex gap-2">
                    <Button
                      size="sm"
                      className="h-7 rounded-md text-xs"
                      onClick={() => switchEpisode(nextEpisode.fileName)}
                    >
                      <Play data-icon="inline-start" />
                      Watch now
                    </Button>
                    <Button
                      size="sm"
                      variant="ghost"
                      className="h-7 rounded-md text-xs text-white/70 hover:bg-white/10 hover:text-white"
                      onClick={() => setCountdown(null)}
                    >
                      Cancel
                    </Button>
                  </div>
                </div>
              </div>
            )}

            {videoError && (
              <div className="absolute inset-0 z-20 grid place-items-center bg-black/75 p-6 text-center">
                <div className="max-w-sm">
                  <AlertTriangle className="mx-auto size-8 text-destructive" />
                  <p className="mt-3 text-sm font-medium text-white">Playback failed</p>
                  <p className="mt-1 text-xs leading-relaxed text-white/65">{videoError}</p>
                  <Button
                    size="sm"
                    variant="secondary"
                    className="mt-4"
                    onClick={() => {
                      setVideoError(null);
                      const video = videoRef.current;
                      if (video) {
                        video.load();
                        void video.play().catch(() => undefined);
                      }
                    }}
                  >
                    Try again
                  </Button>
                </div>
              </div>
            )}
          </div>

          <div className="mt-4 flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
            <div className="min-w-0">
              <div className="flex flex-wrap items-center gap-2">
                <h1 className="font-heading text-2xl font-semibold tracking-tight">
                  {details.title}
                </h1>
                <Badge variant="secondary" className="uppercase">
                  {details.mediaType}
                </Badge>
                {details.isCompleted && (
                  <Badge className="gap-1">
                    <Check className="size-3" />
                    Finished
                  </Badge>
                )}
              </div>
              <p className="mt-1.5 text-sm text-muted-foreground">
                {isTv && currentEpisode
                  ? episodeDisplayName(currentEpisode)
                  : selectedFile
                    ? humanizeFileName(selectedFile)
                    : "No file selected"}
                {details.durationSeconds > 0 && ` · ${formatDuration(details.durationSeconds)}`}
              </p>
            </div>

            <div className="flex shrink-0 items-center gap-2">
              <Button
                variant="outline"
                size="sm"
                className="rounded-full"
                onClick={() =>
                  openExternal(`https://archive.org/details/${encodeURIComponent(identifier)}`)
                }
              >
                <ExternalLink data-icon="inline-start" />
                Archive.org
              </Button>
            </div>
          </div>

          {details.description && (
            <details className="group mt-3 rounded-xl border border-border/60 bg-card/30 px-4 py-3 text-sm">
              <summary className="cursor-pointer list-none font-medium text-muted-foreground transition hover:text-foreground">
                About this title
              </summary>
              <p className="mt-3 whitespace-pre-line text-sm leading-relaxed text-muted-foreground">
                {details.description}
              </p>
            </details>
          )}

          {!showEpisodeList && (
            <div className="mt-4 flex items-center gap-2 text-xs text-muted-foreground">
              <Gauge className="size-3.5" />
              <span>
                Keyboard: Space play/pause · ←/→ skip 10s · M mute · F fullscreen
              </span>
            </div>
          )}
        </div>

        {showEpisodeList ? (
          <aside className="min-w-0">
            <div className="overflow-hidden rounded-2xl border border-border/60 bg-card/40">
              <div className="flex items-center justify-between gap-3 border-b border-border/60 px-4 py-3">
                <div className="flex items-center gap-2">
                  <ListVideo className="size-4 text-primary" />
                  <div>
                    <p className="text-sm font-semibold">Episodes</p>
                    <p className="text-[0.7rem] text-muted-foreground">
                      {episodes.length} available
                    </p>
                  </div>
                </div>
                {seasons.length > 1 && (
                  <Select
                    value={seasonFilter}
                    onValueChange={(value) => value && setSeasonFilter(value)}
                    items={[
                      { value: "all", label: "All seasons" },
                      ...seasons.map((season) => ({
                        value: String(season),
                        label: `Season ${season}`,
                      })),
                    ]}
                  >
                    <SelectTrigger size="sm" className="h-7 w-[7.5rem] text-xs">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="all">All seasons</SelectItem>
                      {seasons.map((season) => (
                        <SelectItem key={season} value={String(season)}>
                          Season {season}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                )}
              </div>

              <div className="scrollbar-thin max-h-[34rem] overflow-y-auto p-1.5">
                {visibleEpisodes.map((episode) => {
                  const active = episode.fileName === selectedFile;
                  const stored = progressMap.get(episode.fileName);
                  const entry =
                    stored ??
                    (episode.fileName === details.currentEpisodeFileName
                      ? {
                          fileName: episode.fileName,
                          positionSeconds: details.watchProgressSeconds,
                          durationSeconds: details.durationSeconds,
                          completed: details.isCompleted,
                        }
                      : null);
                  const itemPercent = entry
                    ? progressPercent(entry.positionSeconds, entry.durationSeconds)
                    : 0;
                  const label = episodeLabel(episode.seasonNumber, episode.episodeNumber);

                  return (
                    <button
                      key={episode.fileName}
                      type="button"
                      onClick={() => switchEpisode(episode.fileName)}
                      className={cn(
                        "group flex w-full items-start gap-3 rounded-xl p-2 text-left transition",
                        active ? "bg-primary/10" : "hover:bg-white/5",
                      )}
                    >
                      <div
                        className={cn(
                          "relative grid aspect-video w-[5.5rem] shrink-0 place-items-center overflow-hidden rounded-lg ring-1",
                          active
                            ? "bg-primary/15 ring-primary/40"
                            : "bg-gradient-to-br from-white/10 to-white/[0.02] ring-white/10",
                        )}
                      >
                        {active ? (
                          <span className="grid size-7 place-items-center rounded-full bg-primary text-primary-foreground">
                            <Pause className="size-3 fill-current" strokeWidth={0} />
                          </span>
                        ) : (
                          <Play className="size-4 text-white/55 transition group-hover:text-white" />
                        )}
                        {itemPercent > 0 && !entry?.completed && (
                          <div className="absolute inset-x-0 bottom-0 h-0.5 bg-black/50">
                            <div
                              className="h-full bg-primary"
                              style={{ width: `${itemPercent}%` }}
                            />
                          </div>
                        )}
                        {entry?.completed && (
                          <span className="absolute right-1 top-1 grid size-3.5 place-items-center rounded-full bg-primary text-primary-foreground">
                            <Check className="size-2.5" />
                          </span>
                        )}
                      </div>

                      <div className="min-w-0 flex-1 py-0.5">
                        <p
                          className={cn(
                            "text-[0.65rem] font-medium uppercase tracking-wider",
                            active ? "text-primary" : "text-muted-foreground",
                          )}
                        >
                          {label ?? `Episode ${visibleEpisodes.indexOf(episode) + 1}`}
                        </p>
                        <p className="mt-0.5 line-clamp-2 text-xs font-medium leading-snug">
                          {episode.name || humanizeFileName(episode.fileName)}
                        </p>
                        <p className="mt-1 text-[0.65rem] text-muted-foreground">
                          {entry?.completed
                            ? "Watched"
                            : entry && entry.positionSeconds > 0
                              ? `${formatDuration(entry.positionSeconds)} watched`
                              : "Not started"}
                        </p>
                      </div>
                    </button>
                  );
                })}
              </div>

              {nextEpisode && (
                <div className="border-t border-border/60 p-2">
                  <Button
                    variant="ghost"
                    size="sm"
                    className="w-full justify-between text-xs"
                    onClick={() => switchEpisode(nextEpisode.fileName)}
                  >
                    Next episode
                    <SkipForward />
                  </Button>
                </div>
              )}
            </div>

            {previousEpisode && (
              <p className="mt-2 px-1 text-[0.7rem] text-muted-foreground">
                P / N keys move between episodes
              </p>
            )}
          </aside>
        ) : (
          <aside className="min-w-0">
            <div className="rounded-2xl border border-border/60 bg-card/40 p-4">
              <div className="flex gap-3">
                <div className="w-20 shrink-0 overflow-hidden rounded-lg ring-1 ring-white/10">
                  <div className="aspect-[2/3]">
                    <PosterImage
                      src={details.tmdbPosterUrl || details.thumbnailUrl}
                      alt={details.title}
                      title={details.title}
                    />
                  </div>
                </div>
                <div className="min-w-0">
                  <p className="text-[0.65rem] font-medium uppercase tracking-wider text-muted-foreground">
                    Now playing
                  </p>
                  <p className="mt-1 line-clamp-2 text-sm font-medium">
                    {currentEpisode?.name ||
                      (selectedFile ? humanizeFileName(selectedFile) : details.title)}
                  </p>
                  <p className="mt-1 text-xs text-muted-foreground">
                    {details.mediaType.toUpperCase()}
                  </p>
                </div>
              </div>

              <Separator className="my-4" />

              <dl className="space-y-2.5 text-xs">
                <div className="flex items-center justify-between gap-3">
                  <dt className="text-muted-foreground">Files</dt>
                  <dd>{details.videoFiles.length}</dd>
                </div>
                <div className="flex items-center justify-between gap-3">
                  <dt className="text-muted-foreground">Progress</dt>
                  <dd>{percent > 0 ? `${Math.round(percent)}%` : "Not started"}</dd>
                </div>
                <div className="flex items-center justify-between gap-3">
                  <dt className="text-muted-foreground">Identifier</dt>
                  <dd className="max-w-[10rem] truncate font-mono text-[0.65rem]">
                    {details.identifier}
                  </dd>
                </div>
              </dl>

              <div className="mt-4 flex items-center gap-2 rounded-lg border border-border/60 bg-background/40 px-2.5 py-2 text-[0.7rem] text-muted-foreground">
                <Volume2 className="size-3.5 shrink-0" />
                <span>Space play/pause · arrows skip · F fullscreen</span>
              </div>
            </div>
          </aside>
        )}
      </div>
    </div>
  );
}

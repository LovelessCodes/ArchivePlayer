import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  AlertTriangle,
  Check,
  Database,
  ExternalLink,
  Eye,
  EyeOff,
  GitBranch,
  KeyRound,
  Loader2,
  Server,
  ShieldCheck,
  Sparkles,
  Trash2,
} from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";
import { ConfirmDialog } from "@/components/confirm-dialog";
import { EmptyState } from "@/components/empty-state";
import { PageLoader } from "@/components/spinner";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Separator } from "@/components/ui/separator";
import { api, errorMessage } from "@/lib/api";
import { openExternal } from "@/lib/links";
import { queryKeys } from "@/lib/query";

function InfoRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex flex-col gap-1 py-2.5 sm:flex-row sm:items-baseline sm:justify-between sm:gap-6">
      <dt className="shrink-0 text-xs text-muted-foreground">{label}</dt>
      <dd className="min-w-0 break-all font-mono text-[0.7rem] text-foreground/85">{value}</dd>
    </div>
  );
}

export function SettingsPage() {
  const queryClient = useQueryClient();
  const [apiKey, setApiKey] = useState("");
  const [showKey, setShowKey] = useState(false);
  const [confirmClear, setConfirmClear] = useState(false);

  const settings = useQuery({ queryKey: queryKeys.settings, queryFn: api.getSettings });

  const saveKey = useMutation({
    mutationFn: () => api.saveTmdbKey(apiKey.trim()),
    onSuccess: async () => {
      setApiKey("");
      setShowKey(false);
      await queryClient.invalidateQueries({ queryKey: queryKeys.settings });
      toast.success("TMDB key saved. Posters and genres will appear for new items.");
    },
    onError: (error) => toast.error(errorMessage(error)),
  });

  const testKey = useMutation({
    mutationFn: api.testTmdbKey,
    onSuccess: () => toast.success("TMDB connection works."),
    onError: (error) => toast.error(errorMessage(error)),
  });

  const clearKey = useMutation({
    mutationFn: api.clearTmdbKey,
    onSuccess: async () => {
      setConfirmClear(false);
      await queryClient.invalidateQueries({ queryKey: queryKeys.settings });
      toast.success("TMDB key removed.");
    },
    onError: (error) => toast.error(errorMessage(error)),
  });

  if (settings.isLoading) {
    return (
      <div className="mx-auto w-full max-w-5xl px-4 py-8 lg:px-8">
        <PageLoader label="Loading settings" />
      </div>
    );
  }

  if (settings.isError || !settings.data) {
    return (
      <div className="mx-auto w-full max-w-3xl px-4 py-8 lg:px-8">
        <EmptyState
          icon={AlertTriangle}
          title="Settings are unavailable"
          description={errorMessage(settings.error)}
          action={<Button onClick={() => settings.refetch()}>Try again</Button>}
        />
      </div>
    );
  }

  const info = settings.data;

  return (
    <div className="mx-auto w-full max-w-5xl px-4 pb-8 pt-8 lg:px-8">
      <header className="mb-8">
        <p className="mb-1.5 flex items-center gap-1.5 text-xs font-medium uppercase tracking-[0.16em] text-muted-foreground">
          <Sparkles className="size-3.5 text-primary" />
          Preferences
        </p>
        <h1 className="font-heading text-3xl font-semibold tracking-tight">Settings</h1>
        <p className="mt-2 max-w-2xl text-sm text-muted-foreground">
          Your library is stored locally. Only streaming and the optional TMDB key need
          network access; adding a key unlocks richer posters, genres, and episode details.
        </p>
      </header>

      <div className="grid gap-6 lg:grid-cols-[minmax(0,1fr)_320px]">
        <div className="space-y-6">
          <section className="overflow-hidden rounded-2xl border border-border/60 bg-card/40">
            <div className="flex items-start justify-between gap-4 border-b border-border/60 px-5 py-4">
              <div className="flex gap-3">
                <div className="mt-0.5 grid size-9 shrink-0 place-items-center rounded-lg border border-border/70 bg-background/50 text-primary">
                  <KeyRound className="size-4" />
                </div>
                <div>
                  <h2 className="text-sm font-semibold">TMDB metadata</h2>
                  <p className="mt-0.5 text-xs leading-relaxed text-muted-foreground">
                    The Movie Database provides posters, genres, and episode listings. A free API
                    key is enough.
                  </p>
                </div>
              </div>
              <Badge
                variant={info.tmdbConfigured ? "default" : "secondary"}
                className="shrink-0 gap-1"
              >
                {info.tmdbConfigured ? (
                  <>
                    <Check className="size-3" />
                    Connected
                  </>
                ) : (
                  "Optional"
                )}
              </Badge>
            </div>

            <div className="space-y-4 px-5 py-4">
              <div className="space-y-2">
                <Label htmlFor="tmdb-key">
                  {info.tmdbConfigured ? "Replace API key" : "API key"}
                </Label>
                <div className="relative">
                  <Input
                    id="tmdb-key"
                    type={showKey ? "text" : "password"}
                    value={apiKey}
                    onChange={(event) => setApiKey(event.target.value)}
                    placeholder={
                      info.tmdbConfigured
                        ? "A key is saved — paste a new one to replace it"
                        : "Paste your TMDB v3 API key"
                    }
                    className="h-10 pr-10 font-mono text-xs"
                    spellCheck={false}
                    autoComplete="off"
                  />
                  <button
                    type="button"
                    className="absolute inset-y-0 right-2 my-auto grid size-7 place-items-center rounded-md text-muted-foreground hover:bg-muted hover:text-foreground"
                    onClick={() => setShowKey((value) => !value)}
                    aria-label={showKey ? "Hide API key" : "Show API key"}
                  >
                    {showKey ? <EyeOff className="size-4" /> : <Eye className="size-4" />}
                  </button>
                </div>
                <p className="text-xs text-muted-foreground">
                  Stored only in this computer's settings file with restricted permissions.{" "}
                  <button
                    type="button"
                    className="text-foreground/80 underline-offset-2 hover:underline"
                    onClick={() => openExternal("https://www.themoviedb.org/settings/api")}
                  >
                    Get a key from TMDB
                  </button>
                  .
                </p>
              </div>

              <div className="flex flex-wrap items-center gap-2">
                <Button
                  onClick={() => saveKey.mutate()}
                  disabled={!apiKey.trim() || saveKey.isPending}
                >
                  {saveKey.isPending ? (
                    <Loader2 className="size-4 animate-spin" />
                  ) : (
                    <KeyRound data-icon="inline-start" />
                  )}
                  {info.tmdbConfigured ? "Replace key" : "Save key"}
                </Button>
                <Button
                  variant="outline"
                  onClick={() => testKey.mutate()}
                  disabled={!info.tmdbConfigured || testKey.isPending}
                >
                  {testKey.isPending && <Loader2 className="size-4 animate-spin" />}
                  Test connection
                </Button>
                {info.tmdbConfigured && (
                  <Button
                    variant="ghost"
                    className="text-muted-foreground hover:text-destructive"
                    onClick={() => setConfirmClear(true)}
                  >
                    <Trash2 data-icon="inline-start" />
                    Remove
                  </Button>
                )}
              </div>

              <div className="flex items-start gap-2 rounded-lg border border-border/60 bg-background/40 px-3 py-2.5 text-xs leading-relaxed text-muted-foreground">
                <ShieldCheck className="mt-0.5 size-3.5 shrink-0 text-primary" />
                <p>
                  Without a key, ArchivePlayer uses Archive.org thumbnails and filename-based
                  episode detection. Your library still works exactly the same.
                </p>
              </div>
            </div>
          </section>

          <section className="rounded-2xl border border-border/60 bg-card/40">
            <div className="flex gap-3 border-b border-border/60 px-5 py-4">
              <div className="mt-0.5 grid size-9 shrink-0 place-items-center rounded-lg border border-border/70 bg-background/50 text-primary">
                <Database className="size-4" />
              </div>
              <div>
                <h2 className="text-sm font-semibold">Local data</h2>
                <p className="mt-0.5 text-xs text-muted-foreground">
                  Your profiles, catalog, and watch history live on this machine.
                </p>
              </div>
            </div>
            <dl className="divide-y divide-border/50 px-5 py-1">
              <InfoRow label="Data directory" value={info.dataDir} />
              <InfoRow label="Database" value={info.databasePath} />
              <InfoRow label="Settings file" value={info.settingsPath} />
            </dl>
            <div className="border-t border-border/60 px-5 py-3 text-xs text-muted-foreground">
              Closing the app signs you out. Your library is kept for the next launch.
            </div>
          </section>

          <section className="rounded-2xl border border-border/60 bg-card/40">
            <div className="flex gap-3 border-b border-border/60 px-5 py-4">
              <div className="mt-0.5 grid size-9 shrink-0 place-items-center rounded-lg border border-border/70 bg-background/50 text-primary">
                <GitBranch className="size-4" />
              </div>
              <div>
                <h2 className="text-sm font-semibold">About ArchivePlayer</h2>
                <p className="mt-0.5 text-xs text-muted-foreground">
                  Version {info.appVersion} · MIT License with attribution
                </p>
              </div>
            </div>
            <div className="space-y-3 px-5 py-4 text-xs leading-relaxed text-muted-foreground">
              <p>
                This app is a Rust + Tauri rewrite of{" "}
                <button
                  type="button"
                  className="font-medium text-foreground underline-offset-2 hover:underline"
                  onClick={() => openExternal("https://github.com/JeppeKjeldal/ArchivePlayer")}
                >
                  ArchivePlayer by Jeppe Kjeldal
                </button>
                , used under the MIT License with Attribution Requirement. The original project is an ASP.NET Core application.
              </p>
              <p>
                Media is streamed from{" "}
                <button
                  type="button"
                  className="text-foreground/85 underline-offset-2 hover:underline"
                  onClick={() => openExternal("https://archive.org")}
                >
                  Archive.org
                </button>
                . Metadata and posters may come from{" "}
                <button
                  type="button"
                  className="text-foreground/85 underline-offset-2 hover:underline"
                  onClick={() => openExternal("https://www.themoviedb.org")}
                >
                  TMDB
                </button>
                . This project is not affiliated with either service.
              </p>
              <Button
                variant="outline"
                size="sm"
                className="rounded-full"
                onClick={() => openExternal("https://github.com/JeppeKjeldal/ArchivePlayer")}
              >
                <ExternalLink data-icon="inline-start" />
                View the original
              </Button>
            </div>
          </section>
        </div>

        <aside className="space-y-4">
          <div className="rounded-2xl border border-border/60 bg-card/40 p-4">
            <div className="flex items-center gap-2">
              <Server className="size-4 text-primary" />
              <h2 className="text-sm font-semibold">How it runs</h2>
            </div>
            <Separator className="my-3" />
            <ul className="space-y-2.5 text-xs leading-relaxed text-muted-foreground">
              <li className="flex gap-2">
                <Check className="mt-0.5 size-3.5 shrink-0 text-primary" />
                Tauri shell with a Rust backend and React frontend.
              </li>
              <li className="flex gap-2">
                <Check className="mt-0.5 size-3.5 shrink-0 text-primary" />
                SQLite catalog and Argon2-hashed local profiles.
              </li>
              <li className="flex gap-2">
                <Check className="mt-0.5 size-3.5 shrink-0 text-primary" />
                Archive.org search, metadata, and direct streaming.
              </li>
              <li className="flex gap-2">
                <Check className="mt-0.5 size-3.5 shrink-0 text-primary" />
                Optional TMDB enrichment, cached in memory.
              </li>
            </ul>
          </div>

          <div className="rounded-2xl border border-border/60 bg-card/40 p-4">
            <div className="flex items-center gap-2">
              <ShieldCheck className="size-4 text-primary" />
              <h2 className="text-sm font-semibold">Privacy</h2>
            </div>
            <Separator className="my-3" />
            <p className="text-xs leading-relaxed text-muted-foreground">
              No account, analytics, or telemetry. Network requests go only to Archive.org, and
              optionally TMDB when a key is configured.
            </p>
          </div>
        </aside>
      </div>

      <ConfirmDialog
        open={confirmClear}
        onOpenChange={setConfirmClear}
        title="Remove the TMDB key?"
        description="Posters and genre suggestions for newly added items will fall back to Archive.org metadata. Existing library entries are not changed."
        confirmLabel="Remove key"
        destructive
        pending={clearKey.isPending}
        onConfirm={() => clearKey.mutate()}
      />
    </div>
  );
}

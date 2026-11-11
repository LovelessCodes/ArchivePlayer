import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  ArrowLeft,
  Check,
  Film,
  Library,
  Link2,
  Loader2,
  MonitorPlay,
  ShieldCheck,
  TriangleAlert,
  Tv,
} from "lucide-react";
import { type FormEvent, useState } from "react";
import { Link, useSearchParams } from "react-router-dom";
import { PageLoader } from "@/components/spinner";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { useAuthQuery } from "@/hooks/use-auth";
import { api, errorMessage } from "@/lib/api";
import { queryKeys } from "@/lib/query";
import { cn } from "@/lib/utils";

const SCOPE_COPY: Record<string, { label: string; detail: string; icon: typeof Library }> = {
  "library:read": {
    label: "Browse the library",
    detail: "See your catalog, posters, and episode lists.",
    icon: Library,
  },
  "library:write": {
    label: "Manage titles",
    detail: "Add or remove titles and change autoplay.",
    icon: Film,
  },
  "progress:write": {
    label: "Save watch progress",
    detail: "Keep your place and mark episodes finished.",
    icon: MonitorPlay,
  },
};

function formatCode(value: string): string {
  const characters = value
    .toUpperCase()
    .replace(/[^A-Z0-9]/g, "")
    .slice(0, 8);
  return characters.length > 4
    ? `${characters.slice(0, 4)}-${characters.slice(4)}`
    : characters;
}

function normalizeCode(value: string | null): string {
  return formatCode(value ?? "");
}

function platformLabel(platform: string): string {
  const labels: Record<string, string> = {
    webos: "LG webOS",
    android: "Android TV",
    "android-tv": "Android TV",
    tvos: "Apple TV",
    roku: "Roku",
    tv: "Smart TV",
  };
  return labels[platform.toLowerCase()] ?? platform;
}

export function ActivatePage() {
  const auth = useAuthQuery();
  const queryClient = useQueryClient();
  const [searchParams] = useSearchParams();
  const initial = normalizeCode(searchParams.get("code"));
  const [input, setInput] = useState(initial);
  const [submitted, setSubmitted] = useState(initial.replace(/-/g, "").length === 8 ? initial : null);
  const [approved, setApproved] = useState(false);
  const [formError, setFormError] = useState<string | null>(null);

  const rawCode = input.replace(/-/g, "");
  const lookup = useQuery({
    queryKey: ["device-lookup", submitted],
    queryFn: () => api.deviceLookup(submitted ?? ""),
    enabled:
      Boolean(submitted) && !approved && auth.data?.runtime === "server",
    retry: false,
    staleTime: 0,
  });

  const approve = useMutation({
    mutationFn: () => api.deviceApprove(submitted ?? ""),
    onSuccess: async () => {
      setApproved(true);
      await queryClient.invalidateQueries({ queryKey: queryKeys.devices });
    },
    onError: (error) => setFormError(errorMessage(error)),
  });

  if (auth.isLoading) {
    return (
      <div className="app-background grid min-h-dvh place-items-center">
        <PageLoader label="Checking your session" />
      </div>
    );
  }

  const isServer = auth.data?.runtime === "server";
  const device = lookup.data;
  const lookupError = lookup.isError ? errorMessage(lookup.error) : null;

  function submit(event: FormEvent) {
    event.preventDefault();
    if (rawCode.length !== 8) {
      setFormError("Enter the eight characters shown on your TV.");
      return;
    }
    setFormError(null);
    const code = formatCode(input);
    setInput(code);
    if (code === submitted) {
      void lookup.refetch();
    } else {
      setSubmitted(code);
    }
  }

  function reset() {
    setApproved(false);
    setSubmitted(null);
    setInput("");
    setFormError(null);
  }

  return (
    <div className="app-background flex min-h-dvh flex-col">
      <header className="mx-auto flex w-full max-w-3xl items-center justify-between px-5 py-6 sm:px-8">
        <Link to="/" className="text-sm font-semibold tracking-tight">
          ArchivePlayer
        </Link>
        <Link
          to="/settings"
          className="text-xs text-muted-foreground underline-offset-4 hover:text-foreground hover:underline"
        >
          Settings
        </Link>
      </header>

      <main className="mx-auto flex w-full max-w-lg flex-1 items-start justify-center px-5 pb-16 pt-6 sm:items-center sm:pt-0">
        <div className="w-full overflow-hidden rounded-3xl border border-border/70 bg-card/70 shadow-2xl shadow-black/30 backdrop-blur">
          <div className="border-b border-border/60 px-6 py-5 sm:px-8">
            <div className="mb-3 flex items-center justify-between gap-3">
              <div className="grid size-10 place-items-center rounded-xl border border-border/70 bg-background/60 text-primary">
                <Tv className="size-5" />
              </div>
              <Badge variant="secondary" className="gap-1.5">
                <Link2 className="size-3" />
                Device pairing
              </Badge>
            </div>
            <h1 className="font-heading text-2xl font-semibold tracking-tight">
              Connect a TV
            </h1>
            <p className="mt-1.5 text-sm leading-relaxed text-muted-foreground">
              Enter the code on your TV to give it access to your library. No
              password is typed on the TV.
            </p>
          </div>

          <div className="px-6 py-6 sm:px-8">
            {!isServer ? (
              <div className="space-y-4 text-sm leading-relaxed text-muted-foreground">
                <div className="flex gap-3 rounded-xl border border-border/60 bg-background/40 p-4">
                  <ShieldCheck className="mt-0.5 size-4 shrink-0 text-primary" />
                  <p>
                    Device pairing is part of the self-hosted server edition. The
                    desktop app keeps your library on this machine and does not
                    accept remote devices.
                  </p>
                </div>
                <Link to="/">
                  <Button variant="outline" className="w-full">
                    <ArrowLeft data-icon="inline-start" />
                    Back to library
                  </Button>
                </Link>
              </div>
            ) : approved ? (
              <div className="space-y-5 text-center">
                <div className="mx-auto grid size-14 place-items-center rounded-full border border-emerald-400/30 bg-emerald-400/10 text-emerald-300">
                  <Check className="size-7" />
                </div>
                <div>
                  <h2 className="font-heading text-xl font-semibold">You're connected</h2>
                  <p className="mx-auto mt-2 max-w-sm text-sm leading-relaxed text-muted-foreground">
                    Return to your TV. It will finish signing in automatically,
                    and you can revoke it any time in Settings.
                  </p>
                </div>
                <div className="flex flex-col gap-2 sm:flex-row sm:justify-center">
                  <Link to="/">
                    <Button className="w-full sm:w-auto">Go to library</Button>
                  </Link>
                  <Button variant="outline" className="w-full sm:w-auto" onClick={reset}>
                    Pair another device
                  </Button>
                </div>
              </div>
            ) : (
              <div className="space-y-5">
                <form onSubmit={submit} className="space-y-2.5">
                  <Label htmlFor="pairing-code">Pairing code</Label>
                  <div className="flex flex-col gap-2 sm:flex-row">
                    <Input
                      id="pairing-code"
                      value={input}
                      onChange={(event) => {
                        setInput(formatCode(event.target.value));
                        setFormError(null);
                        if (submitted) setSubmitted(null);
                      }}
                      placeholder="ABCD-EFGH"
                      autoComplete="one-time-code"
                      autoCapitalize="characters"
                      spellCheck={false}
                      inputMode="text"
                      maxLength={9}
                      className="h-12 flex-1 font-mono text-lg tracking-[0.28em] uppercase"
                      aria-describedby="pairing-code-help"
                    />
                    <Button
                      type="submit"
                      size="lg"
                      className="h-12 px-6"
                      disabled={rawCode.length !== 8 || lookup.isFetching}
                    >
                      {lookup.isFetching ? (
                        <Loader2 className="size-4 animate-spin" />
                      ) : (
                        <ArrowLeft className="size-4 rotate-180" />
                      )}
                      Find device
                    </Button>
                  </div>
                  <p id="pairing-code-help" className="text-xs text-muted-foreground">
                    The code appears on your TV and expires after ten minutes.
                  </p>
                </form>

                {(formError || lookupError) && (
                  <div className="flex gap-3 rounded-xl border border-destructive/30 bg-destructive/10 p-3.5 text-sm text-destructive">
                    <TriangleAlert className="mt-0.5 size-4 shrink-0" />
                    <p>{formError ?? lookupError}</p>
                  </div>
                )}

                {device && (
                  <div className="rounded-2xl border border-border/70 bg-background/40">
                    <div className="flex items-start gap-3 border-b border-border/60 p-4">
                      <div className="grid size-10 shrink-0 place-items-center rounded-lg border border-border/70 bg-card text-primary">
                        <Tv className="size-5" />
                      </div>
                      <div className="min-w-0">
                        <p className="truncate font-medium">{device.clientName}</p>
                        <p className="mt-0.5 text-xs text-muted-foreground">
                          {platformLabel(device.platform)} · waiting for approval
                        </p>
                      </div>
                    </div>

                    <div className="space-y-3 p-4">
                      <p className="text-xs font-medium uppercase tracking-[0.14em] text-muted-foreground">
                        This device can
                      </p>
                      <ul className="space-y-3">
                        {device.scopes.map((scope) => {
                          const copy = SCOPE_COPY[scope] ?? {
                            label: scope,
                            detail: "Access granted by the server.",
                            icon: ShieldCheck,
                          };
                          const Icon = copy.icon;
                          return (
                            <li key={scope} className="flex gap-3">
                              <Icon className="mt-0.5 size-4 shrink-0 text-primary" />
                              <div>
                                <p className="text-sm font-medium">{copy.label}</p>
                                <p className="text-xs leading-relaxed text-muted-foreground">
                                  {copy.detail}
                                </p>
                              </div>
                            </li>
                          );
                        })}
                      </ul>
                      {device.status !== "pending" && (
                        <p className="rounded-lg border border-amber-400/30 bg-amber-400/10 px-3 py-2 text-xs text-amber-200">
                          This request is already {device.status}.
                        </p>
                      )}
                    </div>

                    <div className="flex flex-col gap-2 border-t border-border/60 p-4 sm:flex-row-reverse">
                      <Button
                        className="sm:flex-1"
                        onClick={() => approve.mutate()}
                        disabled={approve.isPending || device.status !== "pending"}
                      >
                        {approve.isPending ? (
                          <Loader2 className="size-4 animate-spin" />
                        ) : (
                          <Check className="size-4" />
                        )}
                        Approve device
                      </Button>
                      <Button
                        variant="ghost"
                        className="sm:flex-1"
                        onClick={reset}
                        disabled={approve.isPending}
                      >
                        Not now
                      </Button>
                    </div>
                  </div>
                )}

                <div className="flex gap-3 rounded-xl border border-border/60 bg-background/40 p-3.5 text-xs leading-relaxed text-muted-foreground">
                  <ShieldCheck className="mt-0.5 size-3.5 shrink-0 text-primary" />
                  <p>
                    Only approve a TV you can see. You can revoke access at any
                    time from Settings → Connected devices.
                  </p>
                </div>
              </div>
            )}
          </div>
        </div>
      </main>

      <footer className={cn("pb-6 text-center text-[0.7rem] text-muted-foreground")}>
        Approving as {auth.data?.currentUser?.displayName ?? auth.data?.currentUser?.email}
      </footer>
    </div>
  );
}

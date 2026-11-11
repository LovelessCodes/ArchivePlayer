import { useMutation } from "@tanstack/react-query";
import { useQueryClient } from "@tanstack/react-query";
import {
  ArrowRight,
  Database,
  Eye,
  EyeOff,
  Film,
  Loader2,
  LockKeyhole,
  ShieldCheck,
  Sparkles,
  WifiOff,
} from "lucide-react";
import { type FormEvent, useEffect, useRef, useState } from "react";
import { Navigate, useLocation, useNavigate } from "react-router-dom";
import { Logo } from "@/components/logo";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { useAuthQuery } from "@/hooks/use-auth";
import { api, errorMessage } from "@/lib/api";
import { queryKeys } from "@/lib/query";
import { cn } from "@/lib/utils";

type Mode = "login" | "register";

function featuresFor(isServer: boolean) {
  return [
    {
      icon: Database,
      title: isServer ? "Your library, on your server" : "Your library, on your machine",
      body: isServer
        ? "A private SQLite catalog on your server. No cloud, no telemetry."
        : "A local SQLite catalog with watch history. No cloud, no telemetry.",
    },
    {
      icon: Film,
      title: "Public-domain cinema",
      body: "Search Archive.org and stream directly from their archive.",
    },
    {
      icon: Sparkles,
      title: "Optional TMDB polish",
      body: isServer
        ? "The server owner can add a free TMDB key for posters, genres, and episodes."
        : "Add a free TMDB key for posters, genres, and episode metadata.",
    },
  ];
}

export function AuthPage() {
  const auth = useAuthQuery();
  const queryClient = useQueryClient();
  const navigate = useNavigate();
  const location = useLocation();
  const previous = (location.state as { from?: string } | null)?.from;
  const from = previous && previous !== "/auth" ? previous : "/";
  const isServer = auth.data?.runtime === "server";
  const features = featuresFor(isServer);
  const offline = typeof navigator !== "undefined" && !navigator.onLine;

  const refetchRef = useRef(auth.refetch);
  useEffect(() => {
    refetchRef.current = auth.refetch;
  }, [auth.refetch]);

  useEffect(() => {
    const reconnect = () => void refetchRef.current();
    window.addEventListener("online", reconnect);
    return () => window.removeEventListener("online", reconnect);
  }, []);

  const [mode, setMode] = useState<Mode | null>(null);
  const [email, setEmail] = useState("");
  const [displayName, setDisplayName] = useState("");
  const [password, setPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [remember, setRemember] = useState(true);
  const [showPassword, setShowPassword] = useState(false);
  const [formError, setFormError] = useState<string | null>(null);

  useEffect(() => {
    if (!auth.data || mode) return;
    setMode(auth.data.hasUsers ? "login" : "register");
    if (auth.data.lastEmail) setEmail(auth.data.lastEmail);
  }, [auth.data, mode]);

  const mutation = useMutation({
    mutationFn: async () => {
      if (mode === "register") {
        if (password !== confirmPassword) {
          throw new Error("The two passwords do not match.");
        }
        return api.register(email, password, displayName || undefined, remember);
      }
      return api.login(email, password, remember);
    },
    onSuccess: async () => {
      setFormError(null);
      await queryClient.invalidateQueries({ queryKey: queryKeys.auth });
      navigate(from, { replace: true });
    },
    onError: (error) => setFormError(errorMessage(error)),
  });

  if (auth.isLoading) {
    return (
      <div className="app-background grid min-h-dvh place-items-center">
        <div className="flex flex-col items-center gap-4">
          <Logo />
          <Loader2 className="size-6 animate-spin text-muted-foreground" />
        </div>
      </div>
    );
  }

  if (auth.isError) {
    return (
      <div className="app-background grid min-h-dvh place-items-center px-4">
        <div className="max-w-md rounded-2xl border border-border bg-card p-6 text-center">
          {offline && (
            <div className="mx-auto mb-3 grid size-10 place-items-center rounded-full bg-amber-400/10 text-amber-300">
              <WifiOff className="size-5" />
            </div>
          )}
          <h1 className="font-heading text-xl font-semibold">
            {offline ? "You're offline" : "ArchivePlayer could not start"}
          </h1>
          <p className="mt-2 text-sm text-muted-foreground">
            {offline
              ? "Reconnect to reach your ArchivePlayer server. The app shell is cached, but your library and streams are not."
              : errorMessage(auth.error)}
          </p>
          <Button className="mt-5" onClick={() => auth.refetch()}>
            Try again
          </Button>
        </div>
      </div>
    );
  }

  if (!mode) {
    return (
      <div className="app-background grid min-h-dvh place-items-center">
        <div className="flex flex-col items-center gap-4">
          <Logo />
          <Loader2 className="size-6 animate-spin text-muted-foreground" />
        </div>
      </div>
    );
  }

  if (auth.data?.currentUser) {
    return <Navigate to={from} replace />;
  }

  const isRegister = mode === "register";

  function submit(event: FormEvent) {
    event.preventDefault();
    setFormError(null);
    if (isRegister && password.length < 8) {
      setFormError("Passwords must be at least 8 characters long.");
      return;
    }
    mutation.mutate();
  }

  return (
    <div className="app-background min-h-dvh">
      <div className="mx-auto grid min-h-dvh w-full max-w-7xl grid-cols-1 lg:grid-cols-[1.05fr_0.95fr]">
        <section className="relative hidden flex-col justify-between overflow-hidden border-r border-border/50 p-12 lg:flex">
          <div className="absolute inset-0 -z-10 bg-[radial-gradient(circle_at_15%_10%,oklch(0.58_0.225_25/0.16),transparent_45%),radial-gradient(circle_at_85%_85%,oklch(0.4_0.12_280/0.14),transparent_45%)]" />
          <Logo />

          <div className="max-w-lg">
            <p className="mb-4 inline-flex items-center gap-2 rounded-full border border-primary/30 bg-primary/10 px-3 py-1 text-xs font-medium text-primary">
              <ShieldCheck className="size-3.5" />
              {isServer
                ? "Self-hosted · Your data stays on your server"
                : "Local-first · No accounts in the cloud"}
            </p>
            <h1 className="text-balance font-heading text-5xl font-semibold leading-[1.05] tracking-tight">
              A cinema archive that belongs to you.
            </h1>
            <p className="mt-5 max-w-md text-pretty text-base leading-relaxed text-muted-foreground">
              Browse public-domain films and shows from the Internet Archive, organize them
              your way, and pick up where you left off.{" "}
              {isServer ? "Everything stays on your server." : "Everything stays on this computer."}
            </p>

            <div className="mt-10 space-y-5">
              {features.map((feature) => (
                <div key={feature.title} className="flex gap-3.5">
                  <div className="mt-0.5 grid size-9 shrink-0 place-items-center rounded-lg border border-border/70 bg-card/70 text-primary">
                    <feature.icon className="size-4" />
                  </div>
                  <div>
                    <h2 className="text-sm font-semibold">{feature.title}</h2>
                    <p className="mt-0.5 text-sm leading-relaxed text-muted-foreground">
                      {feature.body}
                    </p>
                  </div>
                </div>
              ))}
            </div>
          </div>

          <p className="text-xs text-muted-foreground/70">
            Streams are served by Archive.org. ArchivePlayer is not affiliated with the Internet
            Archive.
          </p>
        </section>

        <section className="flex flex-col justify-center px-5 pb-[calc(2.5rem+env(safe-area-inset-bottom))] pt-[calc(2.5rem+env(safe-area-inset-top))] sm:px-10 lg:px-16 lg:py-10">
          <div className="mb-8 flex items-center justify-between lg:hidden">
            <Logo />
          </div>

          <div className="mx-auto w-full max-w-sm">
            {!auth.data?.hasUsers && (
              <p className="mb-3 inline-flex rounded-full border border-border bg-card px-2.5 py-1 text-[0.7rem] font-medium uppercase tracking-wider text-muted-foreground">
                First run
              </p>
            )}
            <h2 className="font-heading text-3xl font-semibold tracking-tight">
              {isRegister ? "Create your profile" : "Welcome back"}
            </h2>
            <p className="mt-2 text-sm text-muted-foreground">
              {isRegister
                ? isServer
                  ? "Create a profile on this server. Each profile has its own library and watch history."
                  : "Pick a local profile for your library. It never leaves this machine."
                : isServer
                  ? "Sign in to your ArchivePlayer server."
                  : "Sign in to your local ArchivePlayer profile."}
            </p>

            <form onSubmit={submit} className="mt-7 space-y-4">
              {isRegister && (
                <div className="space-y-2">
                  <Label htmlFor="display-name">Display name</Label>
                  <Input
                    id="display-name"
                    value={displayName}
                    onChange={(event) => setDisplayName(event.target.value)}
                    placeholder="Optional"
                    autoComplete="name"
                    className="h-10"
                  />
                </div>
              )}

              <div className="space-y-2">
                <Label htmlFor="email">Email</Label>
                <Input
                  id="email"
                  type="email"
                  required
                  value={email}
                  onChange={(event) => setEmail(event.target.value)}
                  placeholder="you@example.com"
                  autoComplete="email"
                  className="h-10"
                />
              </div>

              <div className="space-y-2">
                <Label htmlFor="password">
                  {isRegister ? "Password" : "Password"}
                </Label>
                <div className="relative">
                  <Input
                    id="password"
                    type={showPassword ? "text" : "password"}
                    required
                    minLength={8}
                    value={password}
                    onChange={(event) => setPassword(event.target.value)}
                    placeholder={isRegister ? "At least 8 characters" : "••••••••"}
                    autoComplete={isRegister ? "new-password" : "current-password"}
                    className="h-10 pr-10"
                  />
                  <button
                    type="button"
                    onClick={() => setShowPassword((value) => !value)}
                    className="absolute inset-y-0 right-2 my-auto grid size-7 place-items-center rounded-md text-muted-foreground hover:bg-muted hover:text-foreground"
                    aria-label={showPassword ? "Hide password" : "Show password"}
                  >
                    {showPassword ? <EyeOff className="size-4" /> : <Eye className="size-4" />}
                  </button>
                </div>
              </div>

              {isRegister && (
                <div className="space-y-2">
                  <Label htmlFor="confirm-password">Confirm password</Label>
                  <Input
                    id="confirm-password"
                    type={showPassword ? "text" : "password"}
                    required
                    minLength={8}
                    value={confirmPassword}
                    onChange={(event) => setConfirmPassword(event.target.value)}
                    placeholder="Repeat password"
                    autoComplete="new-password"
                    className="h-10"
                  />
                </div>
              )}

              <div className="flex items-center justify-between rounded-lg border border-border/60 bg-card/40 px-3 py-2.5">
                <div className="flex items-center gap-2 text-sm">
                  <LockKeyhole className="size-3.5 text-muted-foreground" />
                  <span>{isServer ? "Keep me signed in" : "Remember my email"}</span>
                </div>
                <Switch
                  checked={remember}
                  onCheckedChange={setRemember}
                  aria-label={isServer ? "Keep me signed in" : "Remember my email"}
                />
              </div>

              {(formError || mutation.isError) && (
                <p
                  role="alert"
                  className="rounded-lg border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive"
                >
                  {formError ?? errorMessage(mutation.error)}
                </p>
              )}

              <Button
                type="submit"
                size="lg"
                className="h-10 w-full rounded-lg text-sm"
                disabled={mutation.isPending}
              >
                {mutation.isPending && <Loader2 className="size-4 animate-spin" />}
                {isRegister ? "Create profile" : "Sign in"}
                {!mutation.isPending && <ArrowRight data-icon="inline-end" />}
              </Button>
            </form>

            <div className="mt-6 text-center text-sm text-muted-foreground">
              {auth.data?.hasUsers ? (
                isRegister ? (
                  <>
                    Already have a profile?{" "}
                    <button
                      type="button"
                      className={cn("font-medium text-foreground hover:underline")}
                      onClick={() => {
                        setMode("login");
                        setFormError(null);
                      }}
                    >
                      Sign in
                    </button>
                  </>
                ) : auth.data.registrationOpen ? (
                  <>
                    Need another profile?{" "}
                    <button
                      type="button"
                      className="font-medium text-foreground hover:underline"
                      onClick={() => {
                        setMode("register");
                        setFormError(null);
                      }}
                    >
                      Create one
                    </button>
                  </>
                ) : (
                  "Registration is closed on this server."
                )
              ) : isServer ? (
                "Your library, watch history, and settings live in this server's database."
              ) : (
                "Your library, watch history, and settings live in a local database."
              )}
            </div>
          </div>
        </section>
      </div>
    </div>
  );
}

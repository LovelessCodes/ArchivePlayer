import { useMutation } from "@tanstack/react-query";
import {
  ChevronDown,
  GitBranch,
  House,
  LogOut,
  Plus,
  Settings,
  UserRound,
  WifiOff,
} from "lucide-react";
import { useEffect, useState } from "react";
import { Link, NavLink, Outlet, useLocation, useNavigate } from "react-router-dom";
import { Logo } from "@/components/logo";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { api } from "@/lib/api";
import { openExternal } from "@/lib/links";
import { queryClient, queryKeys } from "@/lib/query";
import { useAuthQuery } from "@/hooks/use-auth";
import { cn, initials } from "@/lib/utils";

function navClass({ isActive }: { isActive: boolean }) {
  return cn(
    "rounded-full px-3.5 py-1.5 text-sm font-medium transition-colors",
    isActive
      ? "bg-white/10 text-foreground"
      : "text-muted-foreground hover:bg-white/5 hover:text-foreground",
  );
}

export function AppShell() {
  const { data: auth } = useAuthQuery();
  const user = auth?.currentUser;
  const location = useLocation();
  const navigate = useNavigate();
  const [online, setOnline] = useState(() =>
    typeof navigator === "undefined" ? true : navigator.onLine,
  );
  const isWatchPage = location.pathname.startsWith("/watch/");
  const hideAddButton = isWatchPage || location.pathname === "/add";
  const showMobileNav = !isWatchPage;

  useEffect(() => {
    const goOnline = () => setOnline(true);
    const goOffline = () => setOnline(false);
    window.addEventListener("online", goOnline);
    window.addEventListener("offline", goOffline);
    return () => {
      window.removeEventListener("online", goOnline);
      window.removeEventListener("offline", goOffline);
    };
  }, []);

  const logout = useMutation({
    mutationFn: api.logout,
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: queryKeys.auth });
      navigate("/auth", { replace: true });
    },
  });

  return (
    <div
      className={cn(
        "app-background flex min-h-dvh flex-col",
        showMobileNav &&
          "pb-[calc(4.75rem+env(safe-area-inset-bottom))] sm:pb-0",
      )}
    >
      <header className="sticky top-0 z-40 border-b border-border/60 bg-background/80 pt-[env(safe-area-inset-top)] backdrop-blur-xl">
        <div className="mx-auto flex h-14 w-full max-w-7xl items-center gap-3 px-4 sm:gap-5 lg:px-8">
          <Logo />
          <nav className="ml-2 hidden items-center gap-1 sm:flex">
            <NavLink to="/" end className={navClass}>
              Home
            </NavLink>
            <NavLink to="/add" className={navClass}>
              Add Content
            </NavLink>
          </nav>

          <div className="ml-auto flex items-center gap-1.5">
            {!hideAddButton && (
              <Button
                size="sm"
                className="hidden rounded-full sm:inline-flex"
                onClick={() => navigate("/add")}
              >
                <Plus data-icon="inline-start" />
                Add content
              </Button>
            )}

            <Button
              variant="ghost"
              size="icon-sm"
              className="hidden rounded-full sm:inline-flex"
              aria-label="Settings"
              onClick={() => navigate("/settings")}
            >
              <Settings />
            </Button>

            <DropdownMenu>
              <DropdownMenuTrigger
                render={
                  <button
                    type="button"
                    className="ml-0.5 flex items-center gap-2 rounded-full border border-border/70 bg-card/60 py-1 pl-1 pr-2.5 text-sm transition hover:bg-muted outline-none focus-visible:ring-3 focus-visible:ring-ring/50"
                    aria-label="Account menu"
                  >
                    <span className="grid size-6 place-items-center rounded-full bg-primary/15 text-[0.65rem] font-semibold text-primary">
                      {initials(user?.displayName || user?.email || "?")}
                    </span>
                    <span className="hidden max-w-32 truncate text-xs text-muted-foreground md:block">
                      {user?.displayName || user?.email}
                    </span>
                    <ChevronDown className="size-3 text-muted-foreground" />
                  </button>
                }
              />
              <DropdownMenuContent align="end" className="w-56">
                <DropdownMenuGroup>
                  <DropdownMenuLabel className="flex items-center gap-2 py-2">
                    <UserRound className="size-3.5" />
                    <span className="truncate">{user?.email}</span>
                  </DropdownMenuLabel>
                  <DropdownMenuItem onClick={() => navigate("/settings")}>
                    <Settings />
                    Settings
                  </DropdownMenuItem>
                  <DropdownMenuItem
                    onClick={() =>
                      openExternal("https://github.com/JeppeKjeldal/ArchivePlayer")
                    }
                  >
                    <GitBranch />
                    Original project
                  </DropdownMenuItem>
                </DropdownMenuGroup>
                <DropdownMenuSeparator />
                <DropdownMenuItem
                  variant="destructive"
                  disabled={logout.isPending}
                  onClick={() => logout.mutate()}
                >
                  <LogOut />
                  Sign out
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          </div>
        </div>
      </header>

      {!online && (
        <div
          role="status"
          className="border-b border-amber-400/20 bg-amber-400/10 px-4 py-2 text-xs text-amber-100/90"
        >
          <div className="mx-auto flex max-w-7xl items-center gap-2">
            <WifiOff className="size-3.5 shrink-0" />
            You're offline. Your library and Archive.org streams need a connection.
          </div>
        </div>
      )}

      <main className="flex-1">
        <Outlet />
      </main>

      <footer className="mx-auto mt-12 flex w-full max-w-7xl flex-col gap-2 border-t border-border/50 px-4 py-5 text-xs text-muted-foreground sm:flex-row sm:items-center sm:justify-between lg:px-8">
        <p>
          {auth?.runtime === "server" ? "Self-hosted" : "Local-first"} media library · Streams from{" "}
          <button
            type="button"
            className="text-foreground/80 underline-offset-2 hover:underline"
            onClick={() => openExternal("https://archive.org")}
          >
            Archive.org
          </button>
        </p>
        <div className="flex items-center gap-4">
          <span>Derived from ArchivePlayer by Jeppe Kjeldal</span>
          <Link to="/settings" className="hover:text-foreground">
            Settings
          </Link>
        </div>
      </footer>

      {showMobileNav && (
        <nav
          aria-label="Primary"
          className="fixed inset-x-0 bottom-0 z-40 border-t border-border/60 bg-background pb-[env(safe-area-inset-bottom)] shadow-[0_-12px_36px_-18px_rgba(0,0,0,0.9)] sm:hidden"
        >
          <div className="mx-auto grid max-w-md grid-cols-3 px-2">
            {[
              { to: "/", label: "Home", icon: House, end: true },
              { to: "/add", label: "Add", icon: Plus, end: false },
              { to: "/settings", label: "Settings", icon: Settings, end: false },
            ].map((item) => (
              <NavLink
                key={item.to}
                to={item.to}
                end={item.end}
                className={({ isActive }) =>
                  cn(
                    "flex min-h-16 flex-col items-center justify-center gap-1 text-[0.65rem] font-medium transition-colors",
                    isActive ? "text-foreground" : "text-muted-foreground",
                  )
                }
              >
                {({ isActive }) => (
                  <>
                    <span
                      className={cn(
                        "grid size-8 place-items-center rounded-full transition-colors",
                        isActive ? "bg-primary/15 text-primary" : "text-muted-foreground",
                      )}
                    >
                      <item.icon className="size-4" />
                    </span>
                    {item.label}
                  </>
                )}
              </NavLink>
            ))}
          </div>
        </nav>
      )}
    </div>
  );
}

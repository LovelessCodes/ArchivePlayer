import { useMutation } from "@tanstack/react-query";
import {
  ChevronDown,
  GitBranch,
  LogOut,
  Plus,
  Settings,
  UserRound,
} from "lucide-react";
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
  const hideAddButton =
    location.pathname.startsWith("/watch/") || location.pathname === "/add";

  const logout = useMutation({
    mutationFn: api.logout,
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: queryKeys.auth });
      navigate("/auth", { replace: true });
    },
  });

  return (
    <div className="app-background flex min-h-screen flex-col">
      <header className="sticky top-0 z-40 border-b border-border/60 bg-background/80 backdrop-blur-xl">
        <div className="mx-auto flex h-14 w-full max-w-7xl items-center gap-5 px-4 lg:px-8">
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
              className="rounded-full"
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
    </div>
  );
}

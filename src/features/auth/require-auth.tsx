import { WifiOff } from "lucide-react";
import { type ReactNode, useEffect, useRef } from "react";
import { Navigate } from "react-router-dom";
import { Logo } from "@/components/logo";
import { Spinner } from "@/components/spinner";
import { Button } from "@/components/ui/button";
import { useAuthQuery } from "@/hooks/use-auth";
import { errorMessage } from "@/lib/api";

export function RequireAuth({ children }: { children: ReactNode }) {
  const auth = useAuthQuery();
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

  if (auth.isLoading) {
    return (
      <div className="app-background grid min-h-dvh place-items-center">
        <div className="flex flex-col items-center gap-5">
          <Logo />
          <Spinner className="size-6" />
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
            {offline ? "You're offline" : "Could not reach the backend"}
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

  if (!auth.data?.currentUser) {
    return <Navigate to="/auth" replace />;
  }

  return <>{children}</>;
}

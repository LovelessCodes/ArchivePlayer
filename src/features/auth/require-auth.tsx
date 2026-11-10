import type { ReactNode } from "react";
import { Navigate } from "react-router-dom";
import { Logo } from "@/components/logo";
import { Spinner } from "@/components/spinner";
import { Button } from "@/components/ui/button";
import { useAuthQuery } from "@/hooks/use-auth";
import { errorMessage } from "@/lib/api";

export function RequireAuth({ children }: { children: ReactNode }) {
  const auth = useAuthQuery();

  if (auth.isLoading) {
    return (
      <div className="app-background grid min-h-screen place-items-center">
        <div className="flex flex-col items-center gap-5">
          <Logo />
          <Spinner className="size-6" />
        </div>
      </div>
    );
  }

  if (auth.isError) {
    return (
      <div className="app-background grid min-h-screen place-items-center px-4">
        <div className="max-w-md rounded-2xl border border-border bg-card p-6 text-center">
          <h1 className="font-heading text-xl font-semibold">Could not reach the backend</h1>
          <p className="mt-2 text-sm text-muted-foreground">{errorMessage(auth.error)}</p>
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

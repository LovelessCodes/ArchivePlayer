import { QueryClientProvider } from "@tanstack/react-query";
import { HashRouter, Link, Route, Routes } from "react-router-dom";
import { AppShell } from "@/components/app-shell";
import { ErrorBoundary } from "@/components/error-boundary";
import { Logo } from "@/components/logo";
import { Toaster } from "@/components/toaster";
import { Button } from "@/components/ui/button";
import { TooltipProvider } from "@/components/ui/tooltip";
import { AddContentPage } from "@/features/add/add-content-page";
import { AuthPage } from "@/features/auth/auth-page";
import { RequireAuth } from "@/features/auth/require-auth";
import { HomePage } from "@/features/library/home-page";
import { WatchPage } from "@/features/player/watch-page";
import { SettingsPage } from "@/features/settings/settings-page";
import { queryClient } from "@/lib/query";

function NotFoundPage() {
  return (
    <div className="app-background grid min-h-screen place-items-center px-4">
      <div className="text-center">
        <Logo className="justify-center" />
        <p className="mt-6 font-heading text-5xl font-semibold tracking-tight">404</p>
        <p className="mt-2 text-sm text-muted-foreground">This shelf does not exist.</p>
        <Link to="/" className="mt-6 inline-block">
          <Button>Back to library</Button>
        </Link>
      </div>
    </div>
  );
}

export default function App() {
  return (
    <ErrorBoundary>
      <QueryClientProvider client={queryClient}>
        <TooltipProvider>
          <HashRouter>
            <Routes>
              <Route path="/auth" element={<AuthPage />} />
              <Route
                element={
                  <RequireAuth>
                    <AppShell />
                  </RequireAuth>
                }
              >
                <Route path="/" element={<HomePage />} />
                <Route path="/add" element={<AddContentPage />} />
                <Route path="/watch/:identifier" element={<WatchPage />} />
                <Route path="/settings" element={<SettingsPage />} />
              </Route>
              <Route path="*" element={<NotFoundPage />} />
            </Routes>
          </HashRouter>
          <Toaster />
        </TooltipProvider>
      </QueryClientProvider>
    </ErrorBoundary>
  );
}

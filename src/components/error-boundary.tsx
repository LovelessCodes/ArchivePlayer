import { AlertTriangle } from "lucide-react";
import { Component, type ErrorInfo, type ReactNode } from "react";
import { Button } from "@/components/ui/button";

export class ErrorBoundary extends Component<
  { children: ReactNode },
  { error: Error | null }
> {
  state = { error: null as Error | null };

  static getDerivedStateFromError(error: Error) {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("ArchivePlayer UI error", error, info);
  }

  render() {
    if (this.state.error) {
      return (
        <div className="app-background grid min-h-screen place-items-center px-4">
          <div className="w-full max-w-md rounded-2xl border border-border bg-card p-6 text-center">
            <div className="mx-auto grid size-11 place-items-center rounded-full bg-destructive/15 text-destructive">
              <AlertTriangle className="size-5" />
            </div>
            <h1 className="mt-4 font-heading text-xl font-semibold">Something went wrong</h1>
            <p className="mt-2 text-sm text-muted-foreground">
              The interface hit an unexpected error. Reloading usually fixes it.
            </p>
            <pre className="mt-4 max-h-32 overflow-auto rounded-lg bg-background p-3 text-left font-mono text-[0.7rem] text-muted-foreground">
              {this.state.error.message}
            </pre>
            <Button className="mt-5" onClick={() => window.location.reload()}>
              Reload ArchivePlayer
            </Button>
          </div>
        </div>
      );
    }

    return this.props.children;
  }
}

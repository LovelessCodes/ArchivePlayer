import { Play } from "lucide-react";
import { Link } from "react-router-dom";
import { cn } from "@/lib/utils";

export function Logo({
  compact = false,
  className,
}: {
  compact?: boolean;
  className?: string;
}) {
  return (
    <Link
      to="/"
      aria-label="ArchivePlayer home"
      className={cn("group flex items-center gap-2.5", className)}
    >
      <span className="grid size-8 place-items-center rounded-lg bg-primary text-primary-foreground shadow-[0_0_24px_-6px_var(--primary)] transition-transform group-hover:scale-105">
        <Play className="size-4 translate-x-px fill-current" strokeWidth={0} />
      </span>
      {!compact && (
        <span className="font-heading text-[0.82rem] font-bold tracking-[0.2em] text-foreground">
          ARCHIVE<span className="text-primary">PLAYER</span>
        </span>
      )}
    </Link>
  );
}

import { useQuery } from "@tanstack/react-query";
import { Film, History, LibraryBig, Plus, Sparkles } from "lucide-react";
import { Link } from "react-router-dom";
import { EmptyState } from "@/components/empty-state";
import { PageLoader } from "@/components/spinner";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { useCurrentUser } from "@/hooks/use-auth";
import { api } from "@/lib/api";
import { queryKeys } from "@/lib/query";
import { LibraryCard } from "@/features/library/library-card";

function RowSkeleton() {
  return (
    <div className="flex flex-wrap gap-4">
      {Array.from({ length: 5 }).map((_, index) => (
        <div key={index} className="w-[150px] shrink-0 space-y-2 sm:w-[160px] xl:w-[168px]">
          <Skeleton className="aspect-[2/3] w-full rounded-xl" />
          <Skeleton className="h-3 w-4/5" />
          <Skeleton className="h-2.5 w-2/5" />
        </div>
      ))}
    </div>
  );
}

export function HomePage() {
  const user = useCurrentUser();
  const library = useQuery({ queryKey: queryKeys.library, queryFn: api.getLibrary });
  const continueWatching = useQuery({
    queryKey: queryKeys.continueWatching,
    queryFn: () => api.getContinueWatching(12),
  });

  if (library.isLoading) {
    return (
      <div className="mx-auto w-full max-w-7xl px-4 py-8 lg:px-8">
        <PageLoader label="Opening your library" />
      </div>
    );
  }

  if (library.isError) {
    return (
      <div className="mx-auto w-full max-w-7xl px-4 py-8 lg:px-8">
        <EmptyState
          icon={Film}
          title="Could not load your library"
          description={
            library.error instanceof Error ? library.error.message : "Please try again."
          }
          action={<Button onClick={() => library.refetch()}>Try again</Button>}
        />
      </div>
    );
  }

  const data = library.data!;
  const displayName = user?.displayName || user?.email?.split("@")[0] || "there";

  return (
    <div className="mx-auto w-full max-w-7xl px-4 pb-4 pt-8 lg:px-8">
      <header className="mb-9 flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
        <div>
          <p className="mb-1.5 flex items-center gap-1.5 text-xs font-medium uppercase tracking-[0.16em] text-muted-foreground">
            <Sparkles className="size-3.5 text-primary" />
            My Library
          </p>
          <h1 className="font-heading text-3xl font-semibold tracking-tight sm:text-4xl">
            Welcome back, {displayName}
          </h1>
          <p className="mt-2 text-sm text-muted-foreground">
            {data.totalItems === 0
              ? "Your shelves are waiting for their first film."
              : `${data.totalItems} ${data.totalItems === 1 ? "title" : "titles"} across ${
                  data.categories.length
                } ${data.categories.length === 1 ? "category" : "categories"}.`}
          </p>
        </div>
      </header>

      {data.totalItems === 0 ? (
        <EmptyState
          icon={LibraryBig}
          title="Your library is empty"
          description="Search Archive.org for a movie or show, or paste an identifier you already know. Everything you add stays organized here."
          action={
            <Link to="/add">
              <Button className="rounded-full">
                <Plus data-icon="inline-start" />
                Find something to watch
              </Button>
            </Link>
          }
        />
      ) : (
        <div className="space-y-10">
          {continueWatching.data && continueWatching.data.length > 0 && (
            <section aria-labelledby="continue-heading">
              <div className="mb-3 flex items-end justify-between gap-3">
                <div className="flex items-center gap-2.5">
                  <History className="size-4 text-primary" />
                  <h2 id="continue-heading" className="font-heading text-lg font-semibold">
                    Continue watching
                  </h2>
                </div>
                <p className="text-xs text-muted-foreground">
                  {continueWatching.data.length} in progress
                </p>
              </div>
              <div className="flex flex-wrap gap-4">
                {continueWatching.data.map((item) => (
                  <LibraryCard
                    key={`continue-${item.id}`}
                    item={item}
                    showProgress
                    className="w-[150px] shrink-0 sm:w-[160px] xl:w-[168px]"
                  />
                ))}
              </div>
            </section>
          )}

          {continueWatching.isLoading && <RowSkeleton />}

          {data.categories.map((category) => (
            <section key={category.name} aria-labelledby={`category-${category.name}`}>
              <div className="mb-3 flex items-end justify-between gap-3">
                <div className="flex items-baseline gap-2.5">
                  <h2
                    id={`category-${category.name}`}
                    className="font-heading text-lg font-semibold"
                  >
                    {category.name}
                  </h2>
                  <span className="text-xs text-muted-foreground">
                    {category.items.length}
                  </span>
                </div>
              </div>
              <div className="flex flex-wrap gap-4">
                {category.items.map((item) => (
                  <LibraryCard
                    key={item.id}
                    item={item}
                    className="w-[150px] shrink-0 sm:w-[160px] xl:w-[168px]"
                  />
                ))}
              </div>
            </section>
          ))}
        </div>
      )}
    </div>
  );
}

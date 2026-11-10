import { useInfiniteQuery, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  ArrowLeft,
  Check,
  Database,
  Link2,
  Loader2,
  Plus,
  Search,
  Sparkles,
  Tv,
} from "lucide-react";
import { type FormEvent, useMemo, useState } from "react";
import { Link, useNavigate } from "react-router-dom";
import { toast } from "sonner";
import { PageLoader } from "@/components/spinner";
import { PosterImage } from "@/components/poster-image";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { CategoryDialog } from "@/features/add/category-dialog";
import { api, errorMessage } from "@/lib/api";
import { queryKeys } from "@/lib/query";
import type { ArchiveSearchResult, LibraryItem } from "@/lib/types";

type SelectedResult = Pick<ArchiveSearchResult, "identifier" | "title">;

const EXAMPLES = ["night of the living dead", "Betty Boop", "The Twilight Zone", "sci-fi"];

export function AddContentPage() {
  const queryClient = useQueryClient();
  const navigate = useNavigate();
  const [queryInput, setQueryInput] = useState("");
  const [submittedQuery, setSubmittedQuery] = useState("");
  const [selected, setSelected] = useState<SelectedResult | null>(null);
  const [addedIdentifiers, setAddedIdentifiers] = useState<Set<string>>(new Set());

  const [url, setUrl] = useState("");
  const [directCategory, setDirectCategory] = useState("Uncategorized");

  const library = useQuery({ queryKey: queryKeys.library, queryFn: api.getLibrary });

  const search = useInfiniteQuery({
    queryKey: queryKeys.search(submittedQuery),
    queryFn: ({ pageParam }) => api.searchArchive(submittedQuery, pageParam),
    initialPageParam: 1,
    getNextPageParam: (lastPage) =>
      lastPage.hasMoreResults ? lastPage.currentPage + 1 : undefined,
    enabled: submittedQuery.length > 0,
    staleTime: 5 * 60_000,
    retry: 0,
  });

  const directAdd = useMutation({
    mutationFn: () => api.addToLibrary(url, directCategory.trim() || "Uncategorized"),
    onSuccess: async (item) => {
      await queryClient.invalidateQueries({ queryKey: queryKeys.library });
      await queryClient.invalidateQueries({ queryKey: queryKeys.continueWatching });
      toast.success(`Added “${item.title}” to ${item.categoryName}.`);
      navigate("/");
    },
    onError: (error) => toast.error(errorMessage(error)),
  });

  const results = useMemo(
    () => search.data?.pages.flatMap((page) => page.results) ?? [],
    [search.data],
  );

  const libraryIdentifiers = useMemo(
    () =>
      new Set(
        library.data?.categories.flatMap((category) =>
          category.items.map((item) => item.identifier),
        ) ?? [],
      ),
    [library.data],
  );

  const existingCategories = library.data?.categoryNames ?? [];

  function runSearch(nextQuery?: string) {
    const value = (nextQuery ?? queryInput).trim();
    if (!value) return;
    setQueryInput(value);
    setSubmittedQuery(value);
  }

  function submitUrl(event: FormEvent) {
    event.preventDefault();
    if (!url.trim()) {
      toast.error("Enter an Archive.org URL or identifier.");
      return;
    }
    directAdd.mutate();
  }

  function handleAdded(item: LibraryItem) {
    setAddedIdentifiers((previous) => new Set(previous).add(item.identifier));
  }

  return (
    <div className="mx-auto w-full max-w-6xl px-4 pb-6 pt-8 lg:px-8">
      <Link
        to="/"
        className="mb-6 inline-flex items-center gap-1.5 text-sm text-muted-foreground transition hover:text-foreground"
      >
        <ArrowLeft className="size-4" />
        Back to library
      </Link>

      <header className="mb-7">
        <p className="mb-1.5 flex items-center gap-1.5 text-xs font-medium uppercase tracking-[0.16em] text-muted-foreground">
          <Sparkles className="size-3.5 text-primary" />
          Discover
        </p>
        <h1 className="font-heading text-3xl font-semibold tracking-tight sm:text-4xl">
          Add something to your library
        </h1>
        <p className="mt-2 max-w-2xl text-sm leading-relaxed text-muted-foreground">
          Search the Internet Archive's public video collection, or paste an identifier you
          already have. ArchivePlayer reads the metadata and organizes it locally.
        </p>
      </header>

      <Tabs defaultValue="search" className="gap-6">
        <TabsList variant="line" className="h-auto border-b border-border/60 pb-0">
          <TabsTrigger value="search" className="h-9 gap-2 px-3 text-sm">
            <Search className="size-4" />
            Live search
          </TabsTrigger>
          <TabsTrigger value="url" className="h-9 gap-2 px-3 text-sm">
            <Link2 className="size-4" />
            Direct URL
          </TabsTrigger>
        </TabsList>

        <TabsContent value="search" className="space-y-6">
          <form
            className="flex flex-col gap-2 sm:flex-row"
            onSubmit={(event) => {
              event.preventDefault();
              runSearch();
            }}
          >
            <div className="relative flex-1">
              <Search className="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
              <Input
                value={queryInput}
                onChange={(event) => setQueryInput(event.target.value)}
                placeholder="Search Archive.org — e.g. night of the living dead"
                className="h-11 rounded-xl pl-9"
                autoFocus
              />
            </div>
            <Button
              type="submit"
              size="lg"
              className="h-11 rounded-xl px-5"
              disabled={search.isFetching && !search.isFetchingNextPage}
            >
              {search.isFetching && !search.isFetchingNextPage ? (
                <Loader2 className="size-4 animate-spin" />
              ) : (
                <Search data-icon="inline-start" />
              )}
              Search
            </Button>
          </form>

          {!submittedQuery && (
            <div className="rounded-xl border border-border/60 bg-card/40 p-5">
              <p className="text-sm font-medium">Try one of these</p>
              <div className="mt-3 flex flex-wrap gap-2">
                {EXAMPLES.map((example) => (
                  <Button
                    key={example}
                    variant="outline"
                    size="sm"
                    className="rounded-full text-xs"
                    onClick={() => runSearch(example)}
                  >
                    {example}
                  </Button>
                ))}
              </div>
              <p className="mt-4 text-xs leading-relaxed text-muted-foreground">
                Results come from Archive.org's advanced search and are filtered to public video
                items. Adding an item does not download it — playback streams from their servers.
              </p>
            </div>
          )}

          {submittedQuery && search.isLoading && (
            <PageLoader label={`Searching for “${submittedQuery}”`} />
          )}

          {submittedQuery && search.isError && (
            <div className="rounded-xl border border-destructive/30 bg-destructive/10 p-4 text-sm text-destructive">
              {errorMessage(search.error)}
            </div>
          )}

          {!search.isLoading && !search.isError && submittedQuery && results.length === 0 && (
            <div className="rounded-xl border border-dashed border-border px-6 py-14 text-center">
              <p className="font-medium">No video results found</p>
              <p className="mt-1 text-sm text-muted-foreground">
                Try a shorter query, or add the item by URL if you know its identifier.
              </p>
            </div>
          )}

          {results.length > 0 && (
            <>
              <div className="flex items-center justify-between gap-3">
                <p className="text-sm text-muted-foreground">
                  Results for <span className="text-foreground">“{submittedQuery}”</span>
                </p>
                <p className="text-xs text-muted-foreground">
                  {search.data?.pages[0]?.totalResults ?? results.length} found
                </p>
              </div>
              <div className="grid grid-cols-2 gap-4 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6">
                {results.map((result) => {
                  const inLibrary = libraryIdentifiers.has(result.identifier) || addedIdentifiers.has(result.identifier);
                  return (
                    <article key={result.identifier} className="group min-w-0">
                      <div className="relative aspect-[2/3] overflow-hidden rounded-xl bg-card ring-1 ring-white/10 transition group-hover:ring-primary/40">
                        <PosterImage
                          src={result.thumbnailUrl}
                          alt={result.title}
                          title={result.title}
                        />
                        <Badge className="absolute right-2 top-2 bg-black/70 text-[0.65rem] uppercase text-white backdrop-blur">
                          {result.mediaType.toLowerCase() === "tv" && <Tv className="size-3" />}
                          {result.mediaType}
                        </Badge>
                      </div>
                      <h3 className="mt-2 line-clamp-2 min-h-[2.5em] text-[0.8rem] font-medium leading-snug">
                        {result.title}
                      </h3>
                      <Button
                        size="sm"
                        variant={inLibrary ? "secondary" : "default"}
                        className="mt-2 w-full rounded-lg text-xs"
                        disabled={inLibrary}
                        onClick={() => setSelected(result)}
                      >
                        {inLibrary ? (
                          <>
                            <Check data-icon="inline-start" />
                            In library
                          </>
                        ) : (
                          <>
                            <Plus data-icon="inline-start" />
                            Add to library
                          </>
                        )}
                      </Button>
                    </article>
                  );
                })}
              </div>

              {search.hasNextPage && (
                <div className="flex justify-center pt-2">
                  <Button
                    variant="outline"
                    onClick={() => search.fetchNextPage()}
                    disabled={search.isFetchingNextPage}
                  >
                    {search.isFetchingNextPage && <Loader2 className="size-4 animate-spin" />}
                    Load more
                  </Button>
                </div>
              )}
            </>
          )}
        </TabsContent>

        <TabsContent value="url" className="max-w-2xl">
          <form onSubmit={submitUrl} className="space-y-5 rounded-2xl border border-border/60 bg-card/40 p-5 sm:p-6">
            <div className="space-y-2">
              <Label htmlFor="archive-url">Archive.org URL or identifier</Label>
              <Input
                id="archive-url"
                value={url}
                onChange={(event) => setUrl(event.target.value)}
                placeholder="https://archive.org/details/night_of_the_living_dead"
                className="h-10 font-mono text-xs sm:text-sm"
                spellCheck={false}
              />
              <p className="text-xs leading-relaxed text-muted-foreground">
                Paste a details page URL or just the identifier. Metadata, poster art, and episode
                files are fetched automatically.
              </p>
            </div>

            <div className="space-y-2">
              <Label htmlFor="direct-category">Category</Label>
              <Input
                id="direct-category"
                value={directCategory}
                onChange={(event) => setDirectCategory(event.target.value)}
                placeholder="Uncategorized"
                className="h-10"
                maxLength={60}
              />
              {existingCategories.length > 0 && (
                <div className="flex flex-wrap gap-1.5 pt-1">
                  {existingCategories.slice(0, 8).map((category) => (
                    <Button
                      key={category}
                      type="button"
                      size="sm"
                      variant={directCategory === category ? "default" : "outline"}
                      className="h-7 rounded-full px-3 text-xs"
                      onClick={() => setDirectCategory(category)}
                    >
                      {category}
                    </Button>
                  ))}
                </div>
              )}
            </div>

            <div className="rounded-lg border border-border/60 bg-background/40 px-3 py-2.5 text-xs text-muted-foreground">
              <Database className="mr-1.5 inline size-3.5" />
              Items are streamed from Archive.org. Only metadata and your library are stored
              locally.
            </div>

            <Button
              type="submit"
              size="lg"
              className="w-full rounded-lg"
              disabled={directAdd.isPending}
            >
              {directAdd.isPending ? (
                <Loader2 className="size-4 animate-spin" />
              ) : (
                <Plus data-icon="inline-start" />
              )}
              {directAdd.isPending ? "Reading metadata…" : "Add to library"}
            </Button>
          </form>
        </TabsContent>
      </Tabs>

      <CategoryDialog
        identifier={selected?.identifier ?? ""}
        title={selected?.title ?? ""}
        open={Boolean(selected)}
        onOpenChange={(open) => !open && setSelected(null)}
        onAdded={handleAdded}
      />
    </div>
  );
}

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Check, Loader2, Plus, Tag, WandSparkles } from "lucide-react";
import { useEffect, useState } from "react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Skeleton } from "@/components/ui/skeleton";
import { api, errorMessage } from "@/lib/api";
import { queryKeys } from "@/lib/query";
import type { LibraryItem } from "@/lib/types";
import { cn } from "@/lib/utils";

export function CategoryDialog({
  identifier,
  title,
  open,
  onOpenChange,
  onAdded,
}: {
  identifier: string;
  title: string;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onAdded?: (item: LibraryItem) => void;
}) {
  const queryClient = useQueryClient();
  const [category, setCategory] = useState("Uncategorized");

  const library = useQuery({ queryKey: queryKeys.library, queryFn: api.getLibrary });
  const suggestions = useQuery({
    queryKey: queryKeys.suggestions(identifier),
    queryFn: () => api.getCategorySuggestions(identifier),
    enabled: open && identifier.length > 0,
    staleTime: 60_000,
    retry: 0,
  });

  const add = useMutation({
    mutationFn: () => api.addToLibrary(identifier, category.trim() || "Uncategorized"),
    onSuccess: async (item) => {
      await queryClient.invalidateQueries({ queryKey: queryKeys.library });
      await queryClient.invalidateQueries({ queryKey: queryKeys.continueWatching });
      toast.success(`Added “${item.title}” to ${item.categoryName}.`);
      onAdded?.(item);
      onOpenChange(false);
    },
    onError: (error) => toast.error(errorMessage(error)),
  });

  useEffect(() => {
    if (!open) return;
    setCategory("Uncategorized");
  }, [open, identifier]);

  const existingCategories = library.data?.categoryNames ?? [];
  const suggestionNames = (suggestions.data ?? [])
    .map((suggestion) => suggestion.name)
    .filter((name, index, list) => list.indexOf(name) === index)
    .slice(0, 6);

  return (
    <Dialog open={open} onOpenChange={(next) => !add.isPending && onOpenChange(next)}>
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <DialogTitle>Add to your library</DialogTitle>
          <DialogDescription>
            Choose a shelf for <span className="font-medium text-foreground">{title}</span>. You
            can use an existing category or type a new one.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4">
          <div className="space-y-2">
            <div className="flex items-center gap-2 text-xs font-medium uppercase tracking-wider text-muted-foreground">
              <WandSparkles className="size-3.5 text-primary" />
              Suggested
            </div>
            {suggestions.isLoading ? (
              <div className="flex flex-wrap gap-1.5">
                {Array.from({ length: 4 }).map((_, index) => (
                  <Skeleton key={index} className="h-7 w-20 rounded-full" />
                ))}
              </div>
            ) : suggestionNames.length > 0 ? (
              <div className="flex flex-wrap gap-1.5">
                {suggestionNames.map((name) => (
                  <Button
                    key={name}
                    type="button"
                    size="sm"
                    variant={category === name ? "default" : "outline"}
                    className="h-7 rounded-full px-3 text-xs"
                    onClick={() => setCategory(name)}
                  >
                    {category === name && <Check data-icon="inline-start" />}
                    {name}
                  </Button>
                ))}
              </div>
            ) : (
              <p className="text-xs leading-relaxed text-muted-foreground">
                No TMDB genres are available. Add a free TMDB key in Settings for suggestions.
              </p>
            )}
          </div>

          {existingCategories.length > 0 && (
            <div className="space-y-2">
              <div className="flex items-center gap-2 text-xs font-medium uppercase tracking-wider text-muted-foreground">
                <Tag className="size-3.5" />
                Your categories
              </div>
              <div className="flex flex-wrap gap-1.5">
                {existingCategories.map((name) => (
                  <Button
                    key={name}
                    type="button"
                    size="sm"
                    variant={category === name ? "default" : "secondary"}
                    className={cn("h-7 rounded-full px-3 text-xs")}
                    onClick={() => setCategory(name)}
                  >
                    {name}
                  </Button>
                ))}
              </div>
            </div>
          )}

          <div className="space-y-2">
            <Label htmlFor="category-name">Category</Label>
            <Input
              id="category-name"
              value={category}
              onChange={(event) => setCategory(event.target.value)}
              placeholder="e.g. Movies, TV Shows, Animation"
              className="h-9"
              maxLength={60}
            />
            <p className="text-xs text-muted-foreground">
              Leave blank to use “Uncategorized”.
            </p>
          </div>
        </div>

        <DialogFooter>
          <DialogClose
            render={<Button variant="outline" disabled={add.isPending} />}
          >
            Cancel
          </DialogClose>
          <Button onClick={() => add.mutate()} disabled={add.isPending}>
            {add.isPending ? (
              <Loader2 data-icon="inline-start" className="animate-spin" />
            ) : (
              <Plus data-icon="inline-start" />
            )}
            {add.isPending ? "Adding…" : "Add to library"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

import { QueryClient } from "@tanstack/react-query";

export const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 15_000,
      refetchOnWindowFocus: false,
      retry: 1,
    },
    mutations: {
      retry: 0,
    },
  },
});

export const queryKeys = {
  auth: ["auth"] as const,
  library: ["library"] as const,
  continueWatching: ["continue-watching"] as const,
  playback: (identifier: string) => ["playback", identifier] as const,
  search: (query: string) => ["archive-search", query] as const,
  suggestions: (identifier: string) => ["category-suggestions", identifier] as const,
  settings: ["settings"] as const,
};

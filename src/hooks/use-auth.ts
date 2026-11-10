import { useQuery } from "@tanstack/react-query";
import { api } from "@/lib/api";
import { queryKeys } from "@/lib/query";

export function useAuthQuery() {
  return useQuery({
    queryKey: queryKeys.auth,
    queryFn: api.authStatus,
    staleTime: 30_000,
    retry: 1,
  });
}

export function useCurrentUser() {
  const query = useAuthQuery();
  return query.data?.currentUser ?? null;
}

import { invoke } from "@tauri-apps/api/core";
import type {
  AppInfo,
  ArchiveSearchResponse,
  AuthStatus,
  CategorySuggestion,
  LibraryItem,
  LibraryResponse,
  MediaDetails,
  SettingsInfo,
  User,
} from "@/lib/types";
import { mockInvoke } from "@/lib/mock";

export class ApiError extends Error {
  code: string;

  constructor(message: string, code = "error") {
    super(message);
    this.name = "ApiError";
    this.code = code;
  }
}

function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauri()) {
    return mockInvoke<T>(command, args);
  }

  try {
    return await invoke<T>(command, args);
  } catch (error) {
    if (error && typeof error === "object" && "message" in error) {
      const payload = error as { message?: unknown; code?: unknown };
      throw new ApiError(
        String(payload.message ?? "Something went wrong."),
        typeof payload.code === "string" ? payload.code : "error",
      );
    }
    throw new ApiError(String(error ?? "Something went wrong."));
  }
}

export function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export const api = {
  authStatus: () => call<AuthStatus>("auth_status"),
  register: (email: string, password: string, displayName?: string, remember = true) =>
    call<User>("register", { email, password, displayName, remember }),
  login: (email: string, password: string, remember = true) =>
    call<User>("login", { email, password, remember }),
  logout: () => call<void>("logout"),

  getLibrary: () => call<LibraryResponse>("get_library"),
  getContinueWatching: (limit = 10) =>
    call<LibraryItem[]>("get_continue_watching", { limit }),
  addToLibrary: (identifier: string, categoryName: string) =>
    call<LibraryItem>("add_to_library", { identifier, categoryName }),
  removeFromLibrary: (id: number) => call<void>("remove_from_library", { id }),
  clearWatchHistory: (id: number) => call<void>("clear_watch_history", { id }),
  setAutoplay: (id: number, enabled: boolean) =>
    call<void>("set_autoplay", { id, enabled }),

  searchArchive: (query: string, page = 1) =>
    call<ArchiveSearchResponse>("search_archive", { query, page }),
  getCategorySuggestions: (identifier: string) =>
    call<CategorySuggestion[]>("get_category_suggestions", { identifier }),

  getPlaybackDetails: (identifier: string) =>
    call<MediaDetails>("get_playback_details", { identifier }),
  saveProgress: (
    identifier: string,
    fileName: string,
    positionSeconds: number,
    durationSeconds: number,
  ) =>
    call<void>("save_progress", {
      identifier,
      fileName,
      positionSeconds,
      durationSeconds,
    }),
  markCompleted: (identifier: string, fileName: string, itemCompleted: boolean) =>
    call<void>("mark_completed", { identifier, fileName, itemCompleted }),

  getSettings: () => call<SettingsInfo>("get_settings"),
  saveTmdbKey: (apiKey: string) => call<void>("save_tmdb_key", { apiKey }),
  clearTmdbKey: () => call<void>("clear_tmdb_key"),
  testTmdbKey: () => call<void>("test_tmdb_key"),
  getAppInfo: () => call<AppInfo>("get_app_info"),
  openExternal: (url: string) => call<void>("open_external", { url }),
};

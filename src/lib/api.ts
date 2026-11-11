import { invoke } from "@tauri-apps/api/core";
import type {
  AppInfo,
  ArchiveSearchResponse,
  AuthStatus,
  CategorySuggestion,
  DeviceLookup,
  DeviceStartResponse,
  DeviceTokenResponse,
  LibraryItem,
  LibraryResponse,
  MediaDetails,
  Session,
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

function usingMockBackend(): boolean {
  return (
    !isTauri() &&
    import.meta.env.DEV &&
    import.meta.env.VITE_USE_MOCK !== "false"
  );
}

interface HttpRequest {
  method: "GET" | "POST" | "PUT" | "PATCH" | "DELETE";
  path: string;
  body?: unknown;
}

function segment(value: string | number): string {
  return encodeURIComponent(String(value));
}

function httpRequestFor(
  command: string,
  args: Record<string, unknown> = {},
): HttpRequest {
  switch (command) {
    case "auth_status":
      return { method: "GET", path: "/api/auth/status" };
    case "register":
      return {
        method: "POST",
        path: "/api/auth/register",
        body: {
          email: args.email,
          password: args.password,
          displayName: args.displayName,
          remember: args.remember,
        },
      };
    case "login":
      return {
        method: "POST",
        path: "/api/auth/login",
        body: {
          email: args.email,
          password: args.password,
          remember: args.remember,
        },
      };
    case "logout":
      return { method: "POST", path: "/api/auth/logout" };
    case "get_library":
      return { method: "GET", path: "/api/library" };
    case "get_continue_watching":
      return {
        method: "GET",
        path: `/api/library/continue-watching?limit=${segment(Number(args.limit ?? 10))}`,
      };
    case "add_to_library":
      return {
        method: "POST",
        path: "/api/library",
        body: {
          identifier: args.identifier,
          categoryName: args.categoryName,
        },
      };
    case "remove_from_library":
      return { method: "DELETE", path: `/api/library/${segment(Number(args.id))}` };
    case "clear_watch_history":
      return {
        method: "DELETE",
        path: `/api/library/${segment(Number(args.id))}/history`,
      };
    case "set_autoplay":
      return {
        method: "PATCH",
        path: `/api/library/${segment(Number(args.id))}/autoplay`,
        body: { enabled: args.enabled },
      };
    case "search_archive":
      return {
        method: "GET",
        path: `/api/search?q=${encodeURIComponent(String(args.query ?? ""))}&page=${segment(Number(args.page ?? 1))}`,
      };
    case "get_category_suggestions":
      return {
        method: "GET",
        path: `/api/category-suggestions/${segment(String(args.identifier ?? ""))}`,
      };
    case "get_playback_details":
      return {
        method: "GET",
        path: `/api/playback/${segment(String(args.identifier ?? ""))}`,
      };
    case "save_progress":
      return {
        method: "PUT",
        path: `/api/playback/${segment(String(args.identifier ?? ""))}/progress`,
        body: {
          fileName: args.fileName,
          positionSeconds: args.positionSeconds,
          durationSeconds: args.durationSeconds,
        },
      };
    case "mark_completed":
      return {
        method: "POST",
        path: `/api/playback/${segment(String(args.identifier ?? ""))}/completed`,
        body: {
          fileName: args.fileName,
          itemCompleted: args.itemCompleted,
        },
      };
    case "get_settings":
      return { method: "GET", path: "/api/settings" };
    case "save_tmdb_key":
      return {
        method: "PUT",
        path: "/api/settings/tmdb",
        body: { apiKey: args.apiKey },
      };
    case "clear_tmdb_key":
      return { method: "DELETE", path: "/api/settings/tmdb" };
    case "test_tmdb_key":
      return { method: "POST", path: "/api/settings/tmdb/test" };
    case "get_app_info":
      return { method: "GET", path: "/api/app-info" };
    case "list_devices":
      return { method: "GET", path: "/api/devices" };
    case "revoke_device":
      return {
        method: "DELETE",
        path: `/api/devices/${segment(String(args.id ?? ""))}`,
      };
    case "device_start":
      return {
        method: "POST",
        path: "/api/device/start",
        body: {
          clientName: args.clientName,
          platform: args.platform,
          scopes: args.scopes,
        },
      };
    case "device_token":
      return {
        method: "POST",
        path: "/api/device/token",
        body: { deviceCode: args.deviceCode },
      };
    case "device_lookup":
      return {
        method: "POST",
        path: "/api/device/lookup",
        body: { userCode: args.userCode },
      };
    case "device_approve":
      return {
        method: "POST",
        path: "/api/device/approve",
        body: { userCode: args.userCode },
      };
    default:
      throw new ApiError(`Unknown command: ${command}`, "not_found");
  }
}

async function httpCall<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  const request = httpRequestFor(command, args);
  let response: Response;

  try {
    response = await fetch(request.path, {
      method: request.method,
      credentials: "include",
      headers:
        request.body === undefined
          ? undefined
          : { "Content-Type": "application/json" },
      body:
        request.body === undefined ? undefined : JSON.stringify(request.body),
    });
  } catch {
    throw new ApiError(
      "Could not reach the ArchivePlayer server. Is it running?",
      "network",
    );
  }

  const text = await response.text();
  let payload: unknown = null;
  if (text) {
    try {
      payload = JSON.parse(text);
    } catch {
      payload = null;
    }
  }

  if (!response.ok) {
    const error = (payload ?? {}) as { message?: unknown; code?: unknown };
    throw new ApiError(
      String(error.message ?? `The server returned ${response.status}.`),
      typeof error.code === "string" ? error.code : `http_${response.status}`,
    );
  }

  if (response.status === 204 || !text) {
    return undefined as T;
  }

  return payload as T;
}

export async function call<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  if (!isTauri()) {
    return usingMockBackend() ? mockInvoke<T>(command, args) : httpCall<T>(command, args);
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

export async function openExternalUrl(url: string): Promise<void> {
  if (isTauri()) {
    await call<void>("open_external", { url });
    return;
  }
  if (usingMockBackend()) {
    await mockInvoke<void>("open_external", { url });
    return;
  }
  window.open(url, "_blank", "noopener,noreferrer");
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

  listDevices: () => call<Session[]>("list_devices"),
  revokeDevice: (id: string) => call<void>("revoke_device", { id }),
  deviceStart: (clientName: string, platform: string, scopes?: string[]) =>
    call<DeviceStartResponse>("device_start", { clientName, platform, scopes }),
  deviceToken: (deviceCode: string) =>
    call<DeviceTokenResponse>("device_token", { deviceCode }),
  deviceLookup: (userCode: string) =>
    call<DeviceLookup>("device_lookup", { userCode }),
  deviceApprove: (userCode: string) => call<void>("device_approve", { userCode }),

  openExternal: openExternalUrl,
};

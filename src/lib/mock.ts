/* Browser-only development backend.
 *
 * The real app talks to Rust through Tauri commands. When the UI is opened in
 * a plain browser (`pnpm dev`), this small in-memory/localStorage backend keeps
 * every screen usable for design work without a Tauri runtime.
 */
import type {
  ArchiveSearchResult,
  AuthStatus,
  CategorySuggestion,
  DeviceLookup,
  DeviceStartResponse,
  DeviceTokenResponse,
  Episode,
  LibraryItem,
  LibraryResponse,
  MediaDetails,
  Session,
  SettingsInfo,
  User,
} from "@/lib/types";

type MockState = {
  users: User[];
  sessionUserId: number | null;
  sessions: Session[];
  library: LibraryItem[];
  settings: { tmdbApiKey: string | null; lastEmail: string | null };
  progress: Record<number, Record<string, { position: number; duration: number; completed: boolean }>>;
};

const STORAGE_KEY = "archiveplayer.mock.v1";
const now = () => new Date().toISOString();

const DEMO_POSTERS: Record<string, string> = {
  BigBuckBunny_124: "/demo/big-buck-bunny.svg",
  night_of_the_living_dead: "/demo/night-of-the-living-dead.svg",
  plan_9_from_outer_space_1959: "/demo/plan-9-from-outer-space.svg",
  the_general_1926: "/demo/the-general-1926.svg",
  the_twilight_zone: "/demo/the-twilight-zone.svg",
  nosferatu_1922: "/demo/nosferatu-1922.svg",
  metropolis_1927: "/demo/metropolis-1927.svg",
  sherlock_holmes_1954: "/demo/sherlock-holmes.svg",
};

const poster = (identifier: string) =>
  DEMO_POSTERS[identifier] ?? `https://archive.org/services/img/${identifier}`;
const SAMPLE_VIDEO =
  "https://archive.org/download/BigBuckBunny_124/Content/big_buck_bunny_720p_surround.mp4";

const seedLibrary = (): LibraryItem[] => [
  {
    id: 1,
    identifier: "BigBuckBunny_124",
    title: "Big Buck Bunny",
    mediaType: "movies",
    thumbnailUrl: poster("BigBuckBunny_124"),
    tmdbPosterUrl: null,
    categoryName: "Animation",
    watchProgressSeconds: 245,
    durationSeconds: 596,
    currentEpisodeFileName: "Content/big_buck_bunny_720p_surround.mp4",
    lastWatched: now(),
    isCompleted: false,
    addedAt: now(),
  },
  {
    id: 2,
    identifier: "night_of_the_living_dead",
    title: "Night of the Living Dead",
    mediaType: "movies",
    thumbnailUrl: poster("night_of_the_living_dead"),
    tmdbPosterUrl: null,
    categoryName: "Movies",
    watchProgressSeconds: 840,
    durationSeconds: 5760,
    currentEpisodeFileName: "night-of-the-living-dead.mp4",
    lastWatched: now(),
    isCompleted: false,
    addedAt: now(),
  },
  {
    id: 3,
    identifier: "plan_9_from_outer_space_1959",
    title: "Plan 9 from Outer Space",
    mediaType: "movies",
    thumbnailUrl: poster("plan_9_from_outer_space_1959"),
    tmdbPosterUrl: null,
    categoryName: "Movies",
    watchProgressSeconds: 930,
    durationSeconds: 4740,
    currentEpisodeFileName: "plan-9.mp4",
    lastWatched: now(),
    isCompleted: false,
    addedAt: now(),
  },
  {
    id: 4,
    identifier: "the_general_1926",
    title: "The General",
    mediaType: "movies",
    thumbnailUrl: poster("the_general_1926"),
    tmdbPosterUrl: null,
    categoryName: "Silent Era",
    watchProgressSeconds: 1320,
    durationSeconds: 4500,
    currentEpisodeFileName: "the-general.mp4",
    lastWatched: now(),
    isCompleted: false,
    addedAt: now(),
  },
  {
    id: 5,
    identifier: "the_twilight_zone",
    title: "The Twilight Zone",
    mediaType: "tv",
    thumbnailUrl: poster("the_twilight_zone"),
    tmdbPosterUrl: null,
    categoryName: "TV Shows",
    watchProgressSeconds: 380,
    durationSeconds: 1500,
    currentEpisodeFileName: "The.Twilight.Zone.S01E03.mp4",
    lastWatched: now(),
    isCompleted: false,
    addedAt: now(),
  },
];

const searchCatalog: ArchiveSearchResult[] = [
  {
    identifier: "BigBuckBunny_124",
    title: "Big Buck Bunny",
    mediaType: "movies",
    thumbnailUrl: poster("BigBuckBunny_124"),
  },
  {
    identifier: "night_of_the_living_dead",
    title: "Night of the Living Dead",
    mediaType: "movies",
    thumbnailUrl: poster("night_of_the_living_dead"),
  },
  {
    identifier: "plan_9_from_outer_space_1959",
    title: "Plan 9 from Outer Space",
    mediaType: "movies",
    thumbnailUrl: poster("plan_9_from_outer_space_1959"),
  },
  {
    identifier: "the_general_1926",
    title: "The General",
    mediaType: "movies",
    thumbnailUrl: poster("the_general_1926"),
  },
  {
    identifier: "nosferatu_1922",
    title: "Nosferatu",
    mediaType: "movies",
    thumbnailUrl: poster("nosferatu_1922"),
  },
  {
    identifier: "metropolis_1927",
    title: "Metropolis",
    mediaType: "movies",
    thumbnailUrl: poster("metropolis_1927"),
  },
  {
    identifier: "the_twilight_zone",
    title: "The Twilight Zone",
    mediaType: "tv",
    thumbnailUrl: poster("the_twilight_zone"),
  },
  {
    identifier: "sherlock_holmes_1954",
    title: "Sherlock Holmes",
    mediaType: "tv",
    thumbnailUrl: poster("sherlock_holmes_1954"),
  },
];

const demoSession = (
  id: string,
  kind: Session["kind"],
  label: string,
  platform: string,
  scopes: string[],
): Session => {
  const seconds = Math.floor(Date.now() / 1000);
  return {
    id,
    kind,
    label,
    platform,
    scopes,
    createdAt: new Date(seconds * 1000).toISOString(),
    lastSeenAt: seconds,
    expiresAt: seconds + 180 * 24 * 60 * 60,
    current: kind === "browser",
  };
};

const initial = (): MockState => ({
  users: [],
  sessionUserId: null,
  sessions: [
    demoSession("browser-demo", "browser", "Chrome on this device", "web", ["*"]),
    demoSession("tv-demo", "device", "Living room TV", "webos", [
      "library:read",
      "progress:write",
    ]),
  ],
  library: seedLibrary(),
  settings: { tmdbApiKey: null, lastEmail: null },
  progress: {},
});

function load(): MockState {
  if (typeof localStorage === "undefined") return initial();
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return initial();
    return { ...initial(), ...(JSON.parse(raw) as MockState) };
  } catch {
    return initial();
  }
}

let state = load();

function persist() {
  if (typeof localStorage !== "undefined") {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
  }
}

function currentUser(): User | null {
  return state.users.find((user) => user.id === state.sessionUserId) ?? null;
}

function requireUser(): User {
  const user = currentUser();
  if (!user) throw { code: "unauthorized", message: "You need to sign in first." };
  return user;
}

function delay() {
  return new Promise((resolve) => setTimeout(resolve, 140));
}

function fail(message: string, code = "error"): never {
  throw { code, message };
}

function itemById(id: number): LibraryItem {
  const item = state.library.find((entry) => entry.id === id);
  if (!item) fail("Library item not found.", "not_found");
  return item;
}

function playableEpisodes(item: LibraryItem): Episode[] {
  if (item.identifier === "the_twilight_zone") {
    return Array.from({ length: 6 }, (_, index) => ({
      seasonNumber: 1,
      episodeNumber: index + 1,
      name: ["Where Is Everybody?", "One for the Angels", "Mr. Denton on Doomsday", "The Sixteen-Millimeter Shrine", "Walking Distance", "Escape Clause"][index] ?? `Episode ${index + 1}`,
      overview:
        "A classic episode of The Twilight Zone. This is demo data used when the interface is running outside Tauri.",
      stillUrl: null,
      fileName: `The.Twilight.Zone.S01E0${index + 1}.mp4`,
    }));
  }
  const movieFiles: Record<string, string> = {
    BigBuckBunny_124: "Content/big_buck_bunny_720p_surround.mp4",
    night_of_the_living_dead: "night-of-the-living-dead.mp4",
    plan_9_from_outer_space_1959: "plan-9.mp4",
    the_general_1926: "the-general.mp4",
  };
  return [
    {
      seasonNumber: null,
      episodeNumber: null,
      name: item.title,
      overview: null,
      stillUrl: null,
      fileName: movieFiles[item.identifier] ?? "sample.mp4",
    },
  ];
}

function progressFor(itemId: number, fileName: string) {
  return state.progress[itemId]?.[fileName];
}

function saveMockProgress(
  itemId: number,
  fileName: string,
  position: number,
  duration: number,
  completed = false,
) {
  state.progress[itemId] ??= {};
  state.progress[itemId][fileName] = { position, duration, completed };
  const item = state.library.find((entry) => entry.id === itemId);
  if (item) {
    item.watchProgressSeconds = position;
    item.durationSeconds = duration;
    item.currentEpisodeFileName = fileName;
    item.lastWatched = now();
    item.isCompleted = completed;
  }
  persist();
}

function libraryResponse(): LibraryResponse {
  const groups = new Map<string, LibraryItem[]>();
  for (const item of state.library) {
    const existing = groups.get(item.categoryName) ?? [];
    existing.push(item);
    groups.set(item.categoryName, existing);
  }
  const categories = [...groups.entries()]
    .map(([name, items]) => ({ name, items }))
    .sort((a, b) => {
      if (a.name === "Uncategorized") return 1;
      if (b.name === "Uncategorized") return -1;
      return a.name.localeCompare(b.name);
    });

  return {
    categories,
    categoryNames: categories.map((category) => category.name),
    totalItems: state.library.length,
  };
}

export async function mockInvoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  await delay();
  const a = args ?? {};
  const result = (() => {
    switch (command) {
      case "auth_status": {
        const status: AuthStatus = {
          hasUsers: state.users.length > 0,
          currentUser: currentUser(),
          lastEmail: state.settings.lastEmail,
          registrationOpen: true,
          runtime: "server",
        };
        return status;
      }
      case "register": {
        const email = String(a.email ?? "").trim().toLowerCase();
        if (!email.includes("@")) fail("Enter a valid email address.", "invalid");
        if (String(a.password ?? "").length < 8)
          fail("Passwords must be at least 8 characters long.", "invalid");
        if (state.users.some((user) => user.email === email))
          fail("An account with that email already exists.", "conflict");
        const user: User = {
          id: Math.max(0, ...state.users.map((entry) => entry.id)) + 1,
          email,
          displayName: a.displayName ? String(a.displayName) : null,
          createdAt: now(),
        };
        state.users.push(user);
        state.sessionUserId = user.id;
        if (a.remember !== false) state.settings.lastEmail = email;
        persist();
        return user;
      }
      case "login": {
        const email = String(a.email ?? "").trim().toLowerCase();
        const user = state.users.find((entry) => entry.email === email);
        if (!user || String(a.password ?? "").length < 8)
          fail("Invalid email or password.");
        state.sessionUserId = user.id;
        if (a.remember !== false) state.settings.lastEmail = email;
        persist();
        return user;
      }
      case "logout":
        state.sessionUserId = null;
        persist();
        return undefined;
      case "get_library":
        requireUser();
        return libraryResponse();
      case "get_continue_watching": {
        requireUser();
        const limit = Number(a.limit ?? 10);
        return state.library
          .filter((item) => item.watchProgressSeconds > 0 && !item.isCompleted)
          .sort((left, right) => (right.lastWatched ?? "").localeCompare(left.lastWatched ?? ""))
          .slice(0, limit);
      }
      case "add_to_library": {
        requireUser();
        const identifier = String(a.identifier ?? "").trim();
        if (!identifier) fail("Enter an Archive.org URL or identifier.", "invalid");
        if (state.library.some((item) => item.identifier === identifier))
          fail("That item is already in your library.", "conflict");
        const found = searchCatalog.find((entry) => entry.identifier === identifier);
        const categoryName = String(a.categoryName ?? "Uncategorized").trim() || "Uncategorized";
        const item: LibraryItem = {
          id: Math.max(0, ...state.library.map((entry) => entry.id)) + 1,
          identifier,
          title: found?.title ?? identifier.replace(/[_-]+/g, " "),
          mediaType: found?.mediaType ?? "movies",
          thumbnailUrl: found?.thumbnailUrl ?? poster(identifier),
          tmdbPosterUrl: null,
          categoryName,
          watchProgressSeconds: 0,
          durationSeconds: 0,
          currentEpisodeFileName: null,
          lastWatched: null,
          isCompleted: false,
          addedAt: now(),
        };
        state.library.push(item);
        persist();
        return item;
      }
      case "remove_from_library": {
        requireUser();
        const id = Number(a.id);
        itemById(id);
        state.library = state.library.filter((item) => item.id !== id);
        delete state.progress[id];
        persist();
        return undefined;
      }
      case "clear_watch_history": {
        requireUser();
        const item = itemById(Number(a.id));
        item.watchProgressSeconds = 0;
        item.durationSeconds = 0;
        item.currentEpisodeFileName = null;
        item.lastWatched = null;
        item.isCompleted = false;
        delete state.progress[item.id];
        persist();
        return undefined;
      }
      case "set_autoplay":
        requireUser();
        itemById(Number(a.id));
        persist();
        return undefined;
      case "search_archive": {
        requireUser();
        const query = String(a.query ?? "").toLowerCase().trim();
        const page = Number(a.page ?? 1);
        const matches = searchCatalog.filter((item) =>
          `${item.title} ${item.identifier}`.toLowerCase().includes(query),
        );
        const pageSize = 6;
        const start = (page - 1) * pageSize;
        return {
          results: matches.slice(start, start + pageSize),
          hasMoreResults: start + pageSize < matches.length,
          currentPage: page,
          totalResults: matches.length,
        };
      }
      case "get_category_suggestions": {
        requireUser();
        const identifier = String(a.identifier ?? "");
        if (identifier === "the_twilight_zone" || identifier === "sherlock_holmes_1954") {
          return [
            { name: "TV Shows", source: "TMDB" },
            { name: "Fantasy", source: "TMDB" },
            { name: "Drama", source: "TMDB" },
            { name: "Mystery", source: "TMDB" },
          ] satisfies CategorySuggestion[];
        }
        return [
          { name: "Movies", source: "TMDB" },
          { name: "Classics", source: "TMDB" },
          { name: "Animation", source: "TMDB" },
        ] satisfies CategorySuggestion[];
      }
      case "get_playback_details": {
        requireUser();
        const identifier = String(a.identifier ?? "");
        const item = state.library.find((entry) => entry.identifier === identifier);
        if (!item) fail("Media item not found.", "not_found");
        const episodes = playableEpisodes(item);
        const fileNames = episodes.map((episode) => episode.fileName);
        const current = item.currentEpisodeFileName ?? fileNames[0] ?? "sample.mp4";
        const episodeProgress = Object.entries(state.progress[item.id] ?? {}).map(
          ([fileName, progress]) => ({
            fileName,
            positionSeconds: progress.position,
            durationSeconds: progress.duration,
            completed: progress.completed,
          }),
        );
        const details: MediaDetails = {
          identifier: item.identifier,
          title: item.title,
          description:
            "Demo content shown when ArchivePlayer is running in a browser. Launch it with `pnpm tauri dev` to stream real Archive.org files.",
          mediaType: item.mediaType,
          thumbnailUrl: item.thumbnailUrl,
          tmdbPosterUrl: item.tmdbPosterUrl,
          tmdbId: null,
          tmdbMediaType: item.mediaType === "tv" ? "tv" : "movie",
          videoFiles: fileNames,
          videoFileUrls: fileNames.map((fileName) => ({
            fileName,
            url: SAMPLE_VIDEO,
          })),
          episodes,
          watchProgressSeconds: item.watchProgressSeconds,
          durationSeconds: item.durationSeconds,
          currentEpisodeFileName: current,
          lastWatched: item.lastWatched,
          isCompleted: item.isCompleted,
          autoplayEnabled: true,
          libraryItemId: item.id,
          episodeProgress,
        };
        return details;
      }
      case "save_progress": {
        requireUser();
        const identifier = String(a.identifier ?? "");
        const item = state.library.find((entry) => entry.identifier === identifier);
        if (!item) fail("Media item not found.", "not_found");
        saveMockProgress(
          item.id,
          String(a.fileName ?? ""),
          Number(a.positionSeconds ?? 0),
          Number(a.durationSeconds ?? 0),
        );
        return undefined;
      }
      case "mark_completed": {
        requireUser();
        const item = state.library.find((entry) => entry.identifier === String(a.identifier ?? ""));
        if (!item) fail("Media item not found.", "not_found");
        const existing = progressFor(item.id, String(a.fileName ?? ""));
        saveMockProgress(
          item.id,
          String(a.fileName ?? ""),
          existing?.position ?? item.watchProgressSeconds,
          existing?.duration ?? item.durationSeconds,
          Boolean(a.itemCompleted),
        );
        return undefined;
      }
      case "get_settings": {
        requireUser();
        const settings: SettingsInfo = {
          tmdbApiKey: state.settings.tmdbApiKey,
          tmdbConfigured: Boolean(state.settings.tmdbApiKey),
          lastEmail: state.settings.lastEmail,
          dataDir: "~/Library/Application Support/ArchivePlayer (demo)",
          settingsPath: "~/Library/Application Support/ArchivePlayer/settings.json",
          databasePath: "~/Library/Application Support/ArchivePlayer/archiveplayer.db",
          appVersion: "0.5.0 (browser demo)",
          canManageSettings: true,
          runtime: "server",
        };
        return settings;
      }
      case "save_tmdb_key":
        requireUser();
        state.settings.tmdbApiKey = a.apiKey ? String(a.apiKey) : null;
        persist();
        return undefined;
      case "clear_tmdb_key":
        requireUser();
        state.settings.tmdbApiKey = null;
        persist();
        return undefined;
      case "test_tmdb_key":
        requireUser();
        if (!state.settings.tmdbApiKey) fail("TMDB is not configured yet.", "invalid");
        return undefined;
      case "get_app_info":
        requireUser();
        return {
          version: "0.5.0 (browser demo)",
          dataDir: "~/Library/Application Support/ArchivePlayer (demo)",
          settingsPath: "~/Library/Application Support/ArchivePlayer/settings.json",
          databasePath: "~/Library/Application Support/ArchivePlayer/archiveplayer.db",
          runtime: "server",
        };
      case "list_devices": {
        requireUser();
        return state.sessions;
      }
      case "revoke_device": {
        requireUser();
        const id = String(a.id ?? "");
        state.sessions = state.sessions.filter((session) => session.id !== id);
        persist();
        return undefined;
      }
      case "device_start": {
        const response: DeviceStartResponse = {
          deviceCode: "demo-device-code",
          userCode: "ABCD-EFGH",
          verificationUri: `${window.location.origin}/#/activate`,
          verificationUriComplete: `${window.location.origin}/#/activate?code=ABCD-EFGH`,
          expiresIn: 600,
          interval: 5,
        };
        return response;
      }
      case "device_token": {
        requireUser();
        const response: DeviceTokenResponse = {
          token: "demo-device-token",
          user: requireUser(),
          expiresAt: Math.floor(Date.now() / 1000) + 180 * 24 * 60 * 60,
        };
        return response;
      }
      case "device_lookup": {
        requireUser();
        const response: DeviceLookup = {
          clientName: "Living room TV",
          platform: "webOS",
          scopes: ["library:read", "progress:write"],
          status: "pending",
          expiresIn: 540,
        };
        return response;
      }
      case "device_approve":
        requireUser();
        return undefined;
      case "open_external":
        window.open(String(a.url ?? ""), "_blank", "noopener,noreferrer");
        return undefined;
      default:
        fail(`Unknown demo command: ${command}`, "not_found");
    }
  })();

  return result as T;
}

export function resetMockData() {
  state = initial();
  persist();
}

export interface User {
  id: number;
  email: string;
  displayName: string | null;
  createdAt: string;
}

export type RuntimeMode = "desktop" | "server";

export interface AuthStatus {
  hasUsers: boolean;
  currentUser: User | null;
  lastEmail: string | null;
  registrationOpen: boolean;
  runtime: RuntimeMode;
}

export interface LibraryItem {
  id: number;
  identifier: string;
  title: string;
  mediaType: string;
  thumbnailUrl: string | null;
  tmdbPosterUrl: string | null;
  tmdbBackdropUrl?: string | null;
  categoryName: string;
  watchProgressSeconds: number;
  durationSeconds: number;
  currentEpisodeFileName: string | null;
  lastWatched: string | null;
  isCompleted: boolean;
  addedAt: string;
}

export interface CategoryGroup {
  name: string;
  items: LibraryItem[];
}

export interface LibraryResponse {
  categories: CategoryGroup[];
  categoryNames: string[];
  totalItems: number;
}

export interface ArchiveSearchResult {
  identifier: string;
  title: string;
  mediaType: string;
  thumbnailUrl: string;
}

export interface ArchiveSearchResponse {
  results: ArchiveSearchResult[];
  hasMoreResults: boolean;
  currentPage: number;
  totalResults: number;
}

export interface CategorySuggestion {
  name: string;
  source: string;
}

export interface Episode {
  seasonNumber: number | null;
  episodeNumber: number | null;
  name: string | null;
  overview: string | null;
  stillUrl: string | null;
  fileName: string;
}

export interface EpisodeProgress {
  fileName: string;
  positionSeconds: number;
  durationSeconds: number;
  completed: boolean;
}

export interface VideoFileUrl {
  fileName: string;
  url: string;
  format: string | null;
  mimeType: string | null;
  sizeBytes: number | null;
  lengthSeconds: number | null;
  width: number | null;
  height: number | null;
  videoCodec: string | null;
}

export interface PlaybackPlan {
  platform: string;
  fileName: string;
  url: string;
  mimeType: string | null;
  format: string | null;
  container: string | null;
  videoCodec: string | null;
  sizeBytes: number | null;
  lengthSeconds: number | null;
  width: number | null;
  height: number | null;
  compatible: boolean;
  reason: string | null;
}

export interface MediaDetails {
  identifier: string;
  title: string;
  description: string | null;
  mediaType: string;
  thumbnailUrl: string | null;
  tmdbPosterUrl: string | null;
  tmdbBackdropUrl?: string | null;
  tmdbId: number | null;
  tmdbMediaType: string | null;
  videoFiles: string[];
  videoFileUrls: VideoFileUrl[];
  playbackPlan: PlaybackPlan | null;
  episodes: Episode[];
  watchProgressSeconds: number;
  durationSeconds: number;
  currentEpisodeFileName: string | null;
  lastWatched: string | null;
  isCompleted: boolean;
  autoplayEnabled: boolean;
  episodeProgress: EpisodeProgress[];
  libraryItemId: number | null;
}

export interface SettingsInfo {
  tmdbApiKey: string | null;
  tmdbConfigured: boolean;
  lastEmail: string | null;
  dataDir: string;
  settingsPath: string;
  databasePath: string;
  appVersion: string;
  canManageSettings: boolean;
  runtime: RuntimeMode;
}

export interface AppInfo {
  version: string;
  dataDir: string;
  settingsPath: string;
  databasePath: string;
  runtime: RuntimeMode;
}

export type SessionKind = "browser" | "device";

export interface Session {
  id: string;
  kind: SessionKind;
  label: string | null;
  platform: string | null;
  scopes: string[];
  createdAt: string;
  lastSeenAt: number | null;
  expiresAt: number;
  current: boolean;
}

export interface DeviceStartResponse {
  deviceCode: string;
  userCode: string;
  verificationUri: string;
  verificationUriComplete: string;
  expiresIn: number;
  interval: number;
}

export interface DeviceTokenResponse {
  token: string;
  user: User;
  expiresAt: number;
}

export interface DeviceLookup {
  clientName: string;
  platform: string;
  scopes: string[];
  status: string;
  expiresIn: number;
}

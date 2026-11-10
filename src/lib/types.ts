export interface User {
  id: number;
  email: string;
  displayName: string | null;
  createdAt: string;
}

export interface AuthStatus {
  hasUsers: boolean;
  currentUser: User | null;
  lastEmail: string | null;
}

export interface LibraryItem {
  id: number;
  identifier: string;
  title: string;
  mediaType: string;
  thumbnailUrl: string | null;
  tmdbPosterUrl: string | null;
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
}

export interface MediaDetails {
  identifier: string;
  title: string;
  description: string | null;
  mediaType: string;
  thumbnailUrl: string | null;
  tmdbPosterUrl: string | null;
  tmdbId: number | null;
  tmdbMediaType: string | null;
  videoFiles: string[];
  videoFileUrls: VideoFileUrl[];
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
}

export interface AppInfo {
  version: string;
  dataDir: string;
  settingsPath: string;
  databasePath: string;
}

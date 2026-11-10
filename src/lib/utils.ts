import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

export function formatDuration(totalSeconds: number): string {
  if (!Number.isFinite(totalSeconds) || totalSeconds <= 0) return "0:00";
  const seconds = Math.floor(totalSeconds % 60);
  const minutes = Math.floor(totalSeconds / 60) % 60;
  const hours = Math.floor(totalSeconds / 3600);

  if (hours > 0) {
    return `${hours}:${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}`;
  }
  return `${minutes}:${String(seconds).padStart(2, "0")}`;
}

export function progressPercent(position: number, duration: number): number {
  if (!duration || duration <= 0) return 0;
  return Math.min(100, Math.max(0, (position / duration) * 100));
}

export function episodeLabel(
  seasonNumber: number | null | undefined,
  episodeNumber: number | null | undefined,
): string | null {
  if (seasonNumber == null && episodeNumber == null) return null;
  if (seasonNumber != null && episodeNumber != null) {
    return `S${String(seasonNumber).padStart(2, "0")} · E${String(episodeNumber).padStart(2, "0")}`;
  }
  if (episodeNumber != null) return `Episode ${episodeNumber}`;
  return null;
}

export function initials(value: string): string {
  const parts = value
    .replace(/@.*/, "")
    .split(/[\s._-]+/)
    .filter(Boolean);
  if (parts.length === 0) return "?";
  if (parts.length === 1) return parts[0].slice(0, 2).toUpperCase();
  return (parts[0][0] + parts[parts.length - 1][0]).toUpperCase();
}

export function humanizeFileName(fileName: string): string {
  return fileName
    .replace(/\.[a-z0-9]{2,4}$/i, "")
    .replace(/[._]+/g, " ")
    .replace(/\s+/g, " ")
    .trim();
}

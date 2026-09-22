import { invoke } from "@tauri-apps/api/core";
import type {
  Account,
  Channel,
  HistoryItem,
  LocalMedia,
  MediaItem,
  Settings,
} from "./types";

export type { Account, Channel, HistoryItem, LocalMedia, MediaItem, Settings };
export type { Platform } from "./types";

export function getSettings(): Promise<Settings> {
  return invoke("get_settings");
}

export function saveSettings(settings: Settings): Promise<void> {
  return invoke("save_settings", { settings });
}

export function setTiktokSecret(secret: string): Promise<void> {
  return invoke("set_tiktok_secret", { secret });
}

export function accountStatus(): Promise<{ youtube: Account; tiktok: Account }> {
  return invoke("account_status");
}

export function connectYoutube(): Promise<Account> {
  return invoke("connect_youtube");
}

export function connectTiktok(): Promise<Account> {
  return invoke("connect_tiktok");
}

export function disconnect(platform: "youtube" | "tiktok"): Promise<void> {
  return invoke("disconnect", { platform });
}

export function listFeed(platform: "youtube" | "tiktok"): Promise<MediaItem[]> {
  return invoke("list_feed", { platform });
}

export function listSubscriptions(
  platform: "youtube" | "tiktok",
): Promise<Channel[]> {
  return invoke("list_subscriptions", { platform });
}

export function rememberWatch(item: MediaItem): Promise<void> {
  return invoke("remember_watch", { item });
}

export function listHistory(): Promise<HistoryItem[]> {
  return invoke("list_history");
}

export function clearHistory(): Promise<void> {
  return invoke("clear_history");
}

export function toggleFavorite(item: MediaItem): Promise<boolean> {
  return invoke("toggle_favorite", { item });
}

export function listFavorites(): Promise<MediaItem[]> {
  return invoke("list_favorites");
}

export function listLocalMedia(): Promise<LocalMedia[]> {
  return invoke("list_local_media");
}

export function addLocalMedia(paths: string[]): Promise<LocalMedia[]> {
  return invoke("add_local_media", { paths });
}

export function removeLocalMedia(id: number): Promise<void> {
  return invoke("remove_local_media", { id });
}

export function openOnPlatform(url: string): Promise<void> {
  return invoke("open_on_platform", { url });
}

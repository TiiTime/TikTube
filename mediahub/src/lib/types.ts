export type Platform = "youtube" | "tiktok" | "local";

export type MediaItem = {
  platform: Platform;
  externalId: string;
  title: string;
  creator: string;
  url: string;
  thumb: string;
};

export type HistoryItem = MediaItem & { watchedAt: number };

export type Account = {
  connected: boolean;
  displayName: string;
  handle: string;
};

export type Channel = {
  name: string;
  handle: string;
  url: string;
};

export type LocalMedia = {
  id: number;
  path: string;
  title: string;
  kind: string;
};

export type Settings = {
  performanceMode: "sparsam" | "standard" | "maximal";
  language: "de" | "en";
  theme: "dunkel" | "hell";
  animations: boolean;
  autoplay: boolean;
  volume: number;
  startPage: "start" | "shorts";
  historyEnabled: boolean;
  youtubeClientId: string;
  tiktokClientKey: string;
  tiktokSecretSet: boolean;
};

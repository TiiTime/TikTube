import type { Account, MediaItem, Settings } from "./types";

export const defaultSettings: Settings = {
  performanceMode: "sparsam",
  language: "de",
  theme: "dunkel",
  animations: false,
  autoplay: false,
  volume: 80,
  startPage: "start",
  historyEnabled: true,
  youtubeClientId: "",
  tiktokClientKey: "",
  tiktokSecretSet: false,
};

const emptyAccount = (): Account => ({
  connected: false,
  displayName: "",
  handle: "",
});

class Session {
  settings = $state<Settings>({ ...defaultSettings });
  platform = $state<"youtube" | "tiktok">("youtube");
  surface = $state<"shorts" | "home" | "login">("shorts");
  youtubeLoggedIn = $state(false);
  tiktokLoggedIn = $state(false);
  accounts = $state<{ youtube: Account; tiktok: Account }>({
    youtube: emptyAccount(),
    tiktok: emptyAccount(),
  });
  pendingItem = $state<MediaItem | null>(null);
  paused = $state(false);
  ready = $state(false);
  /** Bumps when a new item should open in Shorts (even if already on /shorts). */
  playTicket = $state(0);

  setPending(item: MediaItem | null) {
    this.pendingItem = item;
    this.paused = false;
    if (item) this.playTicket += 1;
  }

  takePending(): MediaItem | null {
    const item = this.pendingItem;
    this.pendingItem = null;
    return item;
  }

  applySettings(next: Settings) {
    this.settings = { ...next };
  }

  setPlatform(p: "youtube" | "tiktok") {
    if (this.platform === p) return;
    this.platform = p;
    this.paused = false;
  }
}

export const session = new Session();

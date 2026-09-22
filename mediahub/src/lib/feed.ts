import { invoke } from "@tauri-apps/api/core";
import { isTauri } from "@tauri-apps/api/core";
import { Webview } from "@tauri-apps/api/webview";
import {
  getCurrentWindow,
  LogicalPosition,
  LogicalSize,
} from "@tauri-apps/api/window";

const LABEL = "feed";
export const BAR = 48;
export const GUTTER = 0;

export function loginUrl(platform: "youtube" | "tiktok"): string {
  if (platform === "youtube") {
    return "https://accounts.google.com/ServiceLogin?service=youtube&hl=de&continue=https%3A%2F%2Fwww.youtube.com%2F";
  }
  return "https://www.tiktok.com/login";
}

export function feedUrl(
  platform: "youtube" | "tiktok",
  surface: "shorts" | "home" | "login",
): string {
  if (surface === "login") return loginUrl(platform);
  if (platform === "youtube") {
    return surface === "shorts"
      ? "https://m.youtube.com/shorts"
      : "https://www.youtube.com/";
  }
  return surface === "shorts"
    ? "https://www.tiktok.com/foryou"
    : "https://www.tiktok.com/";
}

let queue: Promise<void> = Promise.resolve();

function enqueue(task: () => Promise<void>): Promise<void> {
  queue = queue.then(task, task);
  return queue;
}

async function area(): Promise<{ width: number; height: number }> {
  const win = getCurrentWindow();
  const factor = await win.scaleFactor();
  const inner = await win.innerSize();
  return {
    width: Math.max(320, Math.round(inner.width / factor)),
    height: Math.max(240, Math.round(inner.height / factor) - BAR),
  };
}

export async function resizeShell(): Promise<void> {
  if (!isTauri()) return;
  const win = getCurrentWindow();
  const minW = 698;
  const minH = 953;
  await win.setMinSize(new LogicalSize(minW, minH));
  const factor = await win.scaleFactor();
  const inner = await win.innerSize();
  const width = inner.width / factor;
  const height = inner.height / factor;
  if (width + 1 < minW || height + 1 < minH) {
    await win.setSize(
      new LogicalSize(Math.max(width, minW), Math.max(height, minH)),
    );
  }
}

export function fitFeed(): Promise<void> {
  return enqueue(async () => {
    if (!isTauri()) return;
    const view = await Webview.getByLabel(LABEL);
    if (!view) return;
    const box = await area();
    await view.setPosition(new LogicalPosition(GUTTER, BAR));
    await view.setSize(new LogicalSize(Math.max(280, box.width - GUTTER), box.height));
  });
}

export function hideFeed(): Promise<void> {
  return enqueue(async () => {
    if (!isTauri()) return;
    const view = await Webview.getByLabel(LABEL);
    if (view) await view.hide();
  });
}

export function openFeed(url: string): Promise<void> {
  if (!isTauri()) return Promise.resolve();
  return invoke("show_feed", { url });
}

export function openLoginWindow(platform: "youtube" | "tiktok"): Promise<void> {
  if (!isTauri()) return Promise.resolve();
  return invoke("open_login", { platform });
}

export function nudgeFeed(direction: number): Promise<void> {
  if (!isTauri()) return Promise.resolve();
  return invoke("nudge_feed", { direction });
}

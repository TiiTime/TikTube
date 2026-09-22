import type { MediaItem } from "./types";

const YT_ID = /^[A-Za-z0-9_-]{11}$/;
const TT_ID = /^\d{6,25}$/;

function youtubeItem(id: string, url: string): MediaItem | null {
  if (!YT_ID.test(id)) return null;
  return {
    platform: "youtube",
    externalId: id,
    title: id,
    creator: "",
    url,
    thumb: "",
  };
}

function tiktokItem(id: string, url: string, creator = ""): MediaItem | null {
  if (!TT_ID.test(id)) return null;
  return {
    platform: "tiktok",
    externalId: id,
    title: id,
    creator,
    url,
    thumb: "",
  };
}

/** Parse a pasted YouTube / TikTok URL into a MediaItem. No network. */
export function parseMediaUrl(raw: string): MediaItem | null {
  const trimmed = raw.trim();
  if (!trimmed) return null;

  let url: URL;
  try {
    url = new URL(trimmed.includes("://") ? trimmed : `https://${trimmed}`);
  } catch {
    return null;
  }

  const host = url.hostname.replace(/^www\./, "").toLowerCase();

  // Do not expand short TikTok links
  if (host === "vm.tiktok.com" || host === "vt.tiktok.com") {
    return null;
  }

  if (host === "youtu.be") {
    const id = url.pathname.split("/").filter(Boolean)[0]?.split("?")[0] ?? "";
    return youtubeItem(id, url.toString());
  }

  if (host === "youtube.com" || host === "m.youtube.com" || host === "music.youtube.com") {
    const v = url.searchParams.get("v");
    if (v) return youtubeItem(v, url.toString());

    const parts = url.pathname.split("/").filter(Boolean);
    if (parts[0] === "shorts" || parts[0] === "embed" || parts[0] === "live") {
      const id = parts[1]?.split("?")[0] ?? "";
      return youtubeItem(id, url.toString());
    }
  }

  if (host === "tiktok.com" || host.endsWith(".tiktok.com")) {
    // /@user/video/NUMERIC
    const m = url.pathname.match(/\/@([^/]+)\/video\/(\d{6,25})/);
    if (m) {
      return tiktokItem(m[2], url.toString(), m[1]);
    }
  }

  return null;
}

export function isShortTiktokLink(raw: string): boolean {
  try {
    const u = new URL(raw.trim().includes("://") ? raw.trim() : `https://${raw.trim()}`);
    const host = u.hostname.replace(/^www\./, "").toLowerCase();
    return host === "vm.tiktok.com" || host === "vt.tiktok.com";
  } catch {
    return false;
  }
}

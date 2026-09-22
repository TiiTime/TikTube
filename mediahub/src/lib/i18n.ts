import type { Settings } from "./types";

const de = {
  wordmark: "TikTube",
  navStart: "Start",
  navShorts: "Shorts",
  navAbos: "Abos",
  navVerlauf: "Verlauf",
  navFavoriten: "Favoriten",
  navMedien: "Medien",
  navKonten: "Konten",
  navEinstellungen: "Einstellungen",
  platformYoutube: "YouTube",
  platformTiktok: "TikTok",
  searchPaste: "Link einfügen…",
  ansehen: "Ansehen",
  startHint: "Offizielle Player. Keine Downloads.",
  lastWatched: "Zuletzt angesehen",
  parseFail: "Link konnte nicht gelesen werden. Bitte eine vollständige Video-URL verwenden.",
  shortVmTiktok: "Kurzlinks (vm.tiktok.com) werden nicht aufgelöst. Bitte die volle Video-URL einfügen.",
  emptyFeed:
    "Ohne verbundenes Konto kannst du auf Start einen Link einfügen und ansehen.",
  merken: "Merken",
  gemerkt: "Gemerkt",
  linkKopieren: "Link kopieren",
  aufPlattform: "Auf Plattform öffnen",
  erneut: "Erneut versuchen",
  youtubeUnreachable: "Die Verbindung zu YouTube ist momentan nicht verfügbar.",
  loginFailed: "Die Anmeldung konnte nicht abgeschlossen werden.",
  tiktokNoAbos:
    "TikTok zeigt hier keine Abo-Liste. Deine eigenen Videos stehen unter Shorts.",
  verlaufLoeschen: "Verlauf löschen",
  weitere: "Weitere",
  dateienWaehlen: "Dateien wählen",
  medienHint: "Nur eigene Dateien. Plattformvideos werden nicht heruntergeladen.",
  entfernen: "Entfernen",
  abspielen: "Abspielen",
  verbinden: "Verbinden",
  trennen: "Trennen",
  redirectHint:
    "Redirect-URI genau: http://127.0.0.1:38947/callback — Passwort nie in TikTube eingeben.",
  clientIdMissing: "Client-ID fehlt — unter Einstellungen eintragen.",
  speichern: "Speichern",
  allgemein: "Allgemein",
  wiedergabe: "Wiedergabe",
  performance: "Performance",
  konten: "Konten",
  datenschutz: "Datenschutz",
  startPage: "Startseite",
  language: "Sprache",
  theme: "Thema",
  animations: "Animationen",
  autoplay: "Autoplay",
  volume: "Lautstärke",
  modeSparsam: "Sparsam",
  modeStandard: "Standard",
  modeMaximal: "Maximal",
  youtubeClientId: "YouTube Client-ID",
  tiktokClientKey: "TikTok Client Key",
  tiktokSecret: "TikTok Client Secret",
  clearSecret: "Secret löschen",
  historyEnabled: "Verlauf speichern",
  disconnectBoth: "Beide Konten trennen",
  themeDunkel: "Dunkel",
  themeHell: "Hell",
  copied: "Kopiert",
  offline: "Offline — Online-Listen nicht verfügbar.",
  loading: "Laden…",
  empty: "Keine Einträge.",
  play: "Abspielen",
  pause: "Pause",
  saved: "Gespeichert.",
} as const;

const en: Record<keyof typeof de, string> = {
  wordmark: "TikTube",
  navStart: "Home",
  navShorts: "Shorts",
  navAbos: "Subs",
  navVerlauf: "History",
  navFavoriten: "Favorites",
  navMedien: "Media",
  navKonten: "Accounts",
  navEinstellungen: "Settings",
  platformYoutube: "YouTube",
  platformTiktok: "TikTok",
  searchPaste: "Paste link…",
  ansehen: "Watch",
  startHint: "Official players. No downloads.",
  lastWatched: "Last watched",
  parseFail: "Could not read this link. Please use a full video URL.",
  shortVmTiktok:
    "Short links (vm.tiktok.com) are not resolved. Paste the full video URL.",
  emptyFeed:
    "Without a connected account you can paste a link on Home and watch.",
  merken: "Save",
  gemerkt: "Saved",
  linkKopieren: "Copy link",
  aufPlattform: "Open on platform",
  erneut: "Try again",
  youtubeUnreachable: "The connection to YouTube is currently unavailable.",
  loginFailed: "Sign-in could not be completed.",
  tiktokNoAbos:
    "TikTok does not show a subscription list here. Your own videos are under Shorts.",
  verlaufLoeschen: "Clear history",
  weitere: "More",
  dateienWaehlen: "Choose files",
  medienHint: "Your own files only. Platform videos are not downloaded.",
  entfernen: "Remove",
  abspielen: "Play",
  verbinden: "Connect",
  trennen: "Disconnect",
  redirectHint:
    "Redirect URI exactly: http://127.0.0.1:38947/callback — never enter your password in TikTube.",
  clientIdMissing: "Client ID missing — set it under Settings.",
  speichern: "Save",
  allgemein: "General",
  wiedergabe: "Playback",
  performance: "Performance",
  konten: "Accounts",
  datenschutz: "Privacy",
  startPage: "Start page",
  language: "Language",
  theme: "Theme",
  animations: "Animations",
  autoplay: "Autoplay",
  volume: "Volume",
  modeSparsam: "Lean",
  modeStandard: "Standard",
  modeMaximal: "Max",
  youtubeClientId: "YouTube client ID",
  tiktokClientKey: "TikTok client key",
  tiktokSecret: "TikTok client secret",
  clearSecret: "Clear secret",
  historyEnabled: "Save history",
  disconnectBoth: "Disconnect both accounts",
  themeDunkel: "Dark",
  themeHell: "Light",
  copied: "Copied",
  offline: "Offline — online lists unavailable.",
  loading: "Loading…",
  empty: "No entries.",
  play: "Play",
  pause: "Pause",
  saved: "Saved.",
};

export type I18nKey = keyof typeof de;

export function t(lang: Settings["language"], key: I18nKey): string {
  return (lang === "en" ? en : de)[key];
}

export function mapInvokeError(
  lang: Settings["language"],
  err: unknown,
): string {
  const raw = typeof err === "string" ? err : String(err ?? "");
  const lower = raw.toLowerCase();
  if (lower.includes("youtube") && (lower.includes("unreachable") || lower.includes("unavailable") || lower.includes("network"))) {
    return t(lang, "youtubeUnreachable");
  }
  if (lower.includes("login") || lower.includes("auth") || lower.includes("anmeld")) {
    return t(lang, "loginFailed");
  }
  return raw || t(lang, "offline");
}

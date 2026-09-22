<script lang="ts">
  import { onMount } from "svelte";
  import { page } from "$app/stores";
  import { goto } from "$app/navigation";
  import { isTauri } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import "../app.css";
  import { applyPlaybackQuality, feedUrl, fitFeed, hideFeed, nudgeFeed, openFeed, openLoginWindow, resizeShell } from "$lib/feed";
  import { getSettings } from "$lib/api";
  import { t } from "$lib/i18n";
  import { defaultSettings, session } from "$lib/session.svelte";

  let { children } = $props();

  let feedErr = $state("");
  let settingsOpen = $state(false);
  let quality = $state("1080");

  const lang = $derived(session.settings.language);
  const path = $derived($page.url.pathname);
  const onFeed = $derived(path === "/" || path === "/shorts");
  const desktop = isTauri();

  function applyDomAttrs() {
    const root = document.documentElement;
    root.lang = session.settings.language;
    root.dataset.theme = session.settings.theme;
    const motionOn =
      session.settings.animations && session.settings.performanceMode !== "sparsam";
    root.dataset.motion = motionOn ? "on" : "off";
  }

  $effect(() => {
    session.settings.theme;
    session.settings.language;
    session.settings.animations;
    session.settings.performanceMode;
    if (typeof document !== "undefined") applyDomAttrs();
  });

  async function syncFeed() {
    if (!desktop) return;
    if (!onFeed || settingsOpen) {
      await hideFeed();
      return;
    }
    feedErr = "";
    try {
      await resizeShell();
      await openFeed(feedUrl(session.platform, session.surface));
      await fitFeed();
      await applyPlaybackQuality(quality);
    } catch (error) {
      feedErr = error instanceof Error ? error.message : String(error);
    }
  }

  $effect(() => {
    session.platform;
    session.surface;
    path;
    settingsOpen;
    void syncFeed();
  });

  onMount(() => {
    void (async () => {
      try {
        session.applySettings(await getSettings());
      } catch {
        session.applySettings({ ...defaultSettings });
      }
      session.ready = true;
      applyDomAttrs();
      session.youtubeLoggedIn = localStorage.getItem("mh-yt") === "1";
      session.tiktokLoggedIn = localStorage.getItem("mh-tt") === "1";
      const saved = localStorage.getItem("mh-quality");
      if (saved === "720" || saved === "1080" || saved === "1440") quality = saved;
    })();
    if (!desktop) return;
    let stopResize: (() => void) | undefined;
    let stopLogin: (() => void) | undefined;
    void getCurrentWindow()
      .onResized(() => {
        void fitFeed();
      })
      .then((unlisten) => {
        stopResize = unlisten;
      });
    void listen<string>("login-ok", (event) => {
      if (event.payload === "youtube") {
        session.youtubeLoggedIn = true;
        localStorage.setItem("mh-yt", "1");
      } else if (event.payload === "tiktok") {
        session.tiktokLoggedIn = true;
        localStorage.setItem("mh-tt", "1");
      }
      session.surface = "shorts";
      void openFeed(feedUrl(session.platform, "shorts"));
    }).then((unlisten) => {
      stopLogin = unlisten;
    });
    const onKey = (event: KeyboardEvent) => {
      const tag = (event.target as HTMLElement | null)?.tagName;
      if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return;
      if (event.key !== "ArrowDown" && event.key !== "ArrowUp") return;
      event.preventDefault();
      void nudgeFeed(event.key === "ArrowDown" ? 1 : -1);
    };
    window.addEventListener("keydown", onKey);
    return () => {
      stopResize?.();
      stopLogin?.();
      window.removeEventListener("keydown", onKey);
    };
  });

  function choosePlatform(platform: "youtube" | "tiktok") {
    session.surface = "shorts";
    session.setPlatform(platform);
    if (path !== "/") void goto("/");
  }

  function signInPlatform(platform: "youtube" | "tiktok") {
    feedErr = "";
    void openLoginWindow(platform).catch((error) => {
      feedErr = error instanceof Error ? error.message : String(error);
    });
  }

  function onQuality(event: Event) {
    const next = (event.currentTarget as HTMLSelectElement).value;
    if (next !== "720" && next !== "1080" && next !== "1440") return;
    quality = next;
  }

  function saveQuality() {
    localStorage.setItem("mh-quality", quality);
    void applyPlaybackQuality(quality);
  }

</script>

<div class="shell">
  <header class="top">
    <a class="wordmark" href="/">{t(lang, "wordmark")}</a>
    <div class="platforms" role="group" aria-label="Plattform">
      <button
        type="button"
        class:active={onFeed && session.platform === "youtube"}
        onclick={() => choosePlatform("youtube")}
      >
        YouTube
      </button>
      <span
        class="dot"
        class:on={session.youtubeLoggedIn}
        role="status"
        aria-label={session.youtubeLoggedIn ? "YouTube angemeldet" : "YouTube nicht angemeldet"}
      ></span>
      <button
        type="button"
        class:active={onFeed && session.platform === "tiktok"}
        onclick={() => choosePlatform("tiktok")}
      >
        TikTok
      </button>
      <span
        class="dot"
        class:on={session.tiktokLoggedIn}
        role="status"
        aria-label={session.tiktokLoggedIn ? "TikTok angemeldet" : "TikTok nicht angemeldet"}
      ></span>
    </div>
    <div class="end">
      <button
        type="button"
        class="gear"
        aria-label="Einstellungen"
        aria-pressed={settingsOpen}
        onclick={() => (settingsOpen = !settingsOpen)}
      >
        <svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true">
          <path
            fill="currentColor"
            d="M19.14 12.94c.04-.31.06-.63.06-.94s-.02-.63-.06-.94l2.03-1.58a.5.5 0 0 0 .12-.64l-1.92-3.32a.5.5 0 0 0-.6-.22l-2.39.96a7.2 7.2 0 0 0-1.63-.94l-.36-2.54a.5.5 0 0 0-.5-.42h-3.84a.5.5 0 0 0-.5.42l-.36 2.54c-.59.22-1.14.54-1.63.94l-2.39-.96a.5.5 0 0 0-.6.22L2.71 8.84a.5.5 0 0 0 .12.64l2.03 1.58c-.04.31-.06.63-.06.94s.02.63.06.94L2.83 14.52a.5.5 0 0 0-.12.64l1.92 3.32c.13.23.4.32.64.22l2.39-.96c.49.4 1.04.72 1.63.94l.36 2.54c.05.24.26.42.5.42h3.84c.24 0 .45-.18.5-.42l.36-2.54c.59-.22 1.14-.54 1.63-.94l2.39.96c.24.1.51 0 .64-.22l1.92-3.32a.5.5 0 0 0-.12-.64l-2.03-1.58ZM12 15.5A3.5 3.5 0 1 1 12 8.5a3.5 3.5 0 0 1 0 7Z"
          />
        </svg>
      </button>
    </div>
  </header>

  {#if feedErr}
    <p class="top-err err" role="alert">{feedErr}</p>
  {/if}

  <main class="main">
    {#if settingsOpen}
      <section class="settings">
        <h2>Einstellungen</h2>
        <label>
          Videoqualität
          <select value={quality} onchange={onQuality}>
            <option value="720">720p</option>
            <option value="1080">1080p</option>
            <option value="1440">2K</option>
          </select>
        </label>
        <p>Gilt für YouTube und TikTok. 2K nur, wenn das Video diese Stufe hat.</p>
        <button type="button" class="login settings-login" onclick={saveQuality}>Speichern</button>
        <div class="account">
          <p class="account-name">
            YouTube
            <span
              class="dot"
              class:on={session.youtubeLoggedIn}
              class:off={!session.youtubeLoggedIn}
              role="status"
              aria-label={session.youtubeLoggedIn ? "YouTube angemeldet" : "YouTube nicht angemeldet"}
            ></span>
          </p>
          <button type="button" class="login settings-login" onclick={() => signInPlatform("youtube")}>Anmelden</button>
        </div>
        <div class="account">
          <p class="account-name">
            TikTok
            <span
              class="dot"
              class:on={session.tiktokLoggedIn}
              class:off={!session.tiktokLoggedIn}
              role="status"
              aria-label={session.tiktokLoggedIn ? "TikTok angemeldet" : "TikTok nicht angemeldet"}
            ></span>
          </p>
          <button type="button" class="login settings-login" onclick={() => signInPlatform("tiktok")}>Anmelden</button>
        </div>
      </section>
    {:else if !onFeed || !desktop || feedErr}
      {@render children()}
    {/if}
  </main>
</div>

<style>
  .shell {
    position: relative;
    display: flex;
    flex-direction: column;
    height: 100vh;
    min-height: 100%;
    background: #000;
    overflow: hidden;
  }

  .top {
    height: 48px;
    box-sizing: border-box;
    display: flex;
    align-items: center;
    gap: 0.35rem;
    padding: 0 0.55rem;
    background: #000;
    flex-shrink: 0;
    overflow: hidden;
  }

  .wordmark {
    display: none;
  }

  :global(html[data-theme="hell"]) .wordmark {
    color: var(--ink);
  }

  .platforms {
    display: flex;
    gap: 0.25rem;
    flex-shrink: 0;
  }

  .top button {
    padding: 0.28em 0.7em;
    white-space: nowrap;
    border-radius: 999px;
    background: transparent;
    border-color: transparent;
  }

  .end {
    margin-left: auto;
    display: flex;
    align-items: center;
    flex: 0 0 auto;
  }

  .top button.gear {
    width: 28px;
    height: 28px;
    min-width: 28px;
    min-height: 28px;
    padding: 0;
    margin: 0;
    display: grid;
    place-items: center;
    box-sizing: border-box;
    border: 1px solid var(--line);
    border-radius: 999px;
    color: var(--paper);
    background: transparent;
    outline-offset: 0;
  }

  .top button.gear[aria-pressed="true"] {
    color: var(--amber);
    border-color: var(--amber);
  }

  .settings {
    max-width: 28rem;
    margin: 1.4rem auto;
    padding: 0 1rem 2rem;
    color: var(--paper);
  }

  .settings h2 {
    margin: 0 0 1rem;
    font-size: 1.2rem;
  }

  .settings label {
    display: grid;
    gap: 0.35rem;
    font-size: 0.92rem;
  }

  .settings select {
    width: 100%;
    padding: 0.45rem 0.55rem;
    border-radius: 8px;
    border: 1px solid var(--line);
    background: #111;
    color: var(--paper);
  }

  .settings p {
    color: var(--steel);
    font-size: 0.85rem;
  }

  .settings-login {
    margin-top: 0.2rem;
    font-size: 11px;
    padding: 0.15em 0.45em;
    border: 1px solid var(--line);
    border-radius: 999px;
    color: var(--paper);
    background: transparent;
  }

  .account {
    margin-top: 1.35rem;
  }

  .account-name {
    display: flex;
    align-items: center;
    gap: 0.45rem;
    margin: 0 0 0.45rem;
    color: var(--paper);
    font-size: 1rem;
  }

  .dot.off {
    background: #e23b3b;
  }

  .dot {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    background: #3a3f4b;
    flex: 0 0 auto;
  }

  .dot.on {
    background: #3ddc6a;
  }

  .top button.active {
    border-color: var(--amber);
    color: var(--amber);
  }

  .top a {
    color: var(--steel);
    flex-shrink: 0;
    white-space: nowrap;
    padding: 0.25em 0.35em;
  }

  .top a:hover {
    color: var(--amber);
  }

  .top-err {
    margin: 0;
    padding: 0.35rem 0.75rem;
    border-bottom: 1px solid var(--line);
  }

  .main {
    flex: 1;
    min-width: 0;
    min-height: 0;
    overflow: auto;
  }
</style>

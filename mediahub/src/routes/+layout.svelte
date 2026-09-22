<script lang="ts">
  import { onMount } from "svelte";
  import { page } from "$app/stores";
  import { goto } from "$app/navigation";
  import { isTauri } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import "../app.css";
  import { feedUrl, fitFeed, hideFeed, nudgeFeed, openFeed, openLoginWindow, resizeShell } from "$lib/feed";
  import { getSettings } from "$lib/api";
  import { t } from "$lib/i18n";
  import { defaultSettings, session } from "$lib/session.svelte";

  let { children } = $props();

  let feedErr = $state("");

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
    if (!onFeed) {
      await hideFeed();
      return;
    }
    feedErr = "";
    try {
      await resizeShell();
      await openFeed(feedUrl(session.platform, session.surface));
      await fitFeed();
    } catch (error) {
      feedErr = error instanceof Error ? error.message : String(error);
    }
  }

  $effect(() => {
    session.platform;
    session.surface;
    path;
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

  function signIn() {
    feedErr = "";
    void openLoginWindow(session.platform).catch((error) => {
      feedErr = error instanceof Error ? error.message : String(error);
    });
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
    <button type="button" class="login" onclick={signIn}>Anmelden</button>
  </header>

  {#if feedErr}
    <p class="top-err err" role="alert">{feedErr}</p>
  {/if}

  <main class="main">
    {#if !onFeed || !desktop || feedErr}
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

  .top button.login {
    margin-left: auto;
    font-size: 11px;
    padding: 0.15em 0.45em;
    border-color: var(--line);
    color: var(--paper);
    background: transparent;
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

<script lang="ts">
  import { onMount } from "svelte";
  import VideoStage from "$lib/components/VideoStage.svelte";
  import {
    listFeed,
    listFavorites,
    rememberWatch,
    toggleFavorite,
    openOnPlatform,
  } from "$lib/api";
  import { t, mapInvokeError } from "$lib/i18n";
  import { session } from "$lib/session.svelte";
  import type { MediaItem } from "$lib/types";

  let items = $state<MediaItem[]>([]);
  let index = $state(0);
  let loadErr = $state("");
  let loading = $state(true);
  let favorited = $state(false);
  let copyNote = $state("");
  let rememberedKey = $state("");
  let wheelTimer: ReturnType<typeof setTimeout> | null = null;
  let loadGen = 0;
  let held: MediaItem | null = null;

  const lang = $derived(session.settings.language);
  const current = $derived(items[index] ?? null);
  const localPath = $derived(
    current?.platform === "local" ? current.url : null,
  );
  const playing = $derived(!session.paused && !!current);

  async function loadFeed() {
    const gen = ++loadGen;
    loading = true;
    loadErr = "";
    const pending = session.takePending() ?? held;
    held = pending;

    try {
      const feed = await listFeed(session.platform);
      if (gen !== loadGen) return;
      held = null;
      if (pending) {
        const rest = feed.filter(
          (f) =>
            !(
              f.platform === pending.platform &&
              f.externalId === pending.externalId
            ),
        );
        items = [pending, ...rest];
      } else {
        items = feed;
      }
      index = 0;
    } catch (e) {
      if (gen !== loadGen) return;
      held = null;
      items = pending ? [pending] : [];
      index = 0;
      loadErr = pending ? "" : mapInvokeError(lang, e);
    } finally {
      if (gen === loadGen) loading = false;
    }
  }

  async function checkFavorite(item: MediaItem | null) {
    favorited = false;
    if (!item) return;
    try {
      const favs = await listFavorites();
      favorited = favs.some(
        (f) => f.platform === item.platform && f.externalId === item.externalId,
      );
    } catch {
      favorited = false;
    }
  }

  $effect(() => {
    const item = current;
    const key = item ? `${item.platform}:${item.externalId}` : "";
    if (!item || key === rememberedKey) return;
    rememberedKey = key;
    void checkFavorite(item);
    if (session.settings.historyEnabled) {
      void rememberWatch(item).catch(() => {});
    }
  });

  $effect(() => {
    session.platform;
    session.playTicket;
    void loadFeed();
  });

  function go(delta: number) {
    if (items.length === 0) return;
    const next = index + delta;
    if (next < 0 || next >= items.length) return;
    session.paused = false;
    index = next;
  }

  function onWheel(e: WheelEvent) {
    e.preventDefault();
    if (wheelTimer) return;
    wheelTimer = setTimeout(() => {
      wheelTimer = null;
    }, 450);
    if (e.deltaY > 0) go(1);
    else if (e.deltaY < 0) go(-1);
  }

  function onKey(e: KeyboardEvent) {
    const tag = (e.target as HTMLElement)?.tagName;
    if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return;

    if (e.code === "ArrowDown") {
      e.preventDefault();
      go(1);
    } else if (e.code === "ArrowUp") {
      e.preventDefault();
      go(-1);
    } else if (e.code === "Space") {
      e.preventDefault();
      session.paused = !session.paused;
    } else if (e.code === "KeyF") {
      e.preventDefault();
      const el = document.querySelector("[data-mediahub-gate]") as HTMLElement | null;
      if (!el) return;
      if (!document.fullscreenElement) void el.requestFullscreen?.();
      else void document.exitFullscreen?.();
    }
  }

  let rootEl: HTMLDivElement | null = $state(null);

  onMount(() => {
    window.addEventListener("keydown", onKey);
    const el = rootEl;
    el?.addEventListener("wheel", onWheel, { passive: false });
    return () => {
      window.removeEventListener("keydown", onKey);
      el?.removeEventListener("wheel", onWheel);
      if (wheelTimer) {
        clearTimeout(wheelTimer);
        wheelTimer = null;
      }
    };
  });

  async function onMerken() {
    if (!current) return;
    try {
      favorited = await toggleFavorite(current);
    } catch (e) {
      loadErr = mapInvokeError(lang, e);
    }
  }

  async function onCopy() {
    if (!current?.url) return;
    try {
      await navigator.clipboard.writeText(current.url);
      copyNote = t(lang, "copied");
    } catch {
      copyNote = "";
    }
  }

  async function onOpen() {
    if (!current?.url || current.platform === "local") return;
    try {
      await openOnPlatform(current.url);
    } catch (e) {
      loadErr = mapInvokeError(lang, e);
    }
  }

  function onPlayingChange(p: boolean) {
    session.paused = !p;
  }
</script>

<div class="shorts-view" bind:this={rootEl}>
  {#if loading}
    <p class="muted pad">{t(lang, "loading")}</p>
  {:else if !current}
    <div class="pad">
      {#if loadErr}
        <p class="err" role="alert">{loadErr}</p>
      {/if}
      <p class="muted">{t(lang, "emptyFeed")}</p>
    </div>
  {:else}
    {#key `${session.platform}:${current.platform}:${current.externalId}`}
      <VideoStage
        item={current}
        localPath={localPath}
        playing={playing}
        volume={session.settings.volume}
        autoplay={session.settings.autoplay}
        onPlayingChange={onPlayingChange}
      />
    {/key}

    <div class="meta-bar">
      <div class="info">
        <p class="creator">{current.creator || "—"}</p>
        <p class="title">{current.title}</p>
        <p class="idx muted">{index + 1} / {items.length}</p>
      </div>
      <div class="actions">
        <button type="button" onclick={onMerken}>
          {favorited ? t(lang, "gemerkt") : t(lang, "merken")}
        </button>
        {#if current.platform !== "local"}
          <button type="button" onclick={onCopy}>{t(lang, "linkKopieren")}</button>
          <button type="button" onclick={onOpen}>{t(lang, "aufPlattform")}</button>
        {/if}
        {#if copyNote}
          <span class="muted">{copyNote}</span>
        {/if}
      </div>
      {#if loadErr}
        <p class="err feed-err" role="alert">{loadErr}</p>
      {/if}
    </div>
  {/if}
</div>

<style>
  .shorts-view {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    overflow: hidden;
  }

  .pad {
    padding: 1.25rem;
  }

  .meta-bar {
    flex-shrink: 0;
    padding: 0.75rem 1rem 1rem;
    border-top: 1px solid var(--line);
    background: var(--panel);
  }

  .creator {
    margin: 0;
    color: var(--steel);
    font-size: 0.85rem;
  }

  .title {
    margin: 0.15rem 0 0;
    font-family: var(--font-display);
  }

  .idx {
    margin: 0.35rem 0 0;
    font-size: 0.8rem;
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem;
    margin-top: 0.65rem;
    align-items: center;
  }

  .feed-err {
    margin-top: 0.5rem;
  }
</style>

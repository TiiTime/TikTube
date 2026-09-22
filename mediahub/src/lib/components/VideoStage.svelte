<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import type { MediaItem } from "$lib/types";
  import { t, mapInvokeError } from "$lib/i18n";
  import { openOnPlatform } from "$lib/api";
  import { session } from "$lib/session.svelte";

  type Props = {
    item: MediaItem | null;
    localPath: string | null;
    playing: boolean;
    volume: number;
    autoplay: boolean;
    onPlayingChange?: (playing: boolean) => void;
  };

  let {
    item,
    localPath = null,
    playing = false,
    volume = 80,
    autoplay = false,
    onPlayingChange,
  }: Props = $props();

  let iframeEl: HTMLIFrameElement | null = $state(null);
  let videoEl: HTMLVideoElement | null = $state(null);
  let retryNonce = $state(0);

  const YT_ID = /^[A-Za-z0-9_-]{11}$/;
  const TT_ID = /^\d{6,25}$/;

  const lang = $derived(session.settings.language);
  const tallyOn = $derived(playing && !!item);

  const frameSrc = $derived.by(() => {
    retryNonce;
    if (!item) return "";
    if (item.platform === "youtube") {
      if (!YT_ID.test(item.externalId)) return "";
      let src =
        `https://www.youtube-nocookie.com/embed/${item.externalId}?enablejsapi=1&rel=0&modestbranding=1&playsinline=1`;
      if (autoplay) src += "&autoplay=1";
      return src;
    }
    if (item.platform === "tiktok") {
      if (!TT_ID.test(item.externalId)) return "";
      return `https://www.tiktok.com/embed/v2/${item.externalId}`;
    }
    return "";
  });

  const videoSrc = $derived.by(() => {
    retryNonce;
    if (item?.platform !== "local") return "";
    const path = localPath || item.url;
    if (!path) return "";
    try {
      return convertFileSrc(path);
    } catch {
      return "";
    }
  });

  const errorMsg = $derived.by(() => {
    if (!item) return "";
    if (item.platform === "youtube" && !YT_ID.test(item.externalId)) {
      return "Ungültige YouTube-ID.";
    }
    if (item.platform === "tiktok" && !TT_ID.test(item.externalId)) {
      return "Ungültige TikTok-ID.";
    }
    if (item.platform === "local" && !videoSrc) return "Kein lokaler Pfad.";
    return "";
  });

  function postYt(func: "playVideo" | "pauseVideo") {
    if (!iframeEl?.contentWindow) return;
    iframeEl.contentWindow.postMessage(
      JSON.stringify({ event: "command", func, args: [] }),
      "https://www.youtube-nocookie.com",
    );
  }

  $effect(() => {
    const vol = Math.max(0, Math.min(100, volume)) / 100;
    if (videoEl) videoEl.volume = vol;
  });

  $effect(() => {
    if (!item) return;
    if (item.platform === "local" && videoEl) {
      if (playing) {
        void videoEl.play().catch(() => onPlayingChange?.(false));
      } else {
        videoEl.pause();
      }
      return;
    }
    if (item.platform === "youtube" && frameSrc) {
      postYt(playing ? "playVideo" : "pauseVideo");
    }
  });

  function retry() {
    retryNonce += 1;
  }

  let actionErr = $state("");

  async function openPlatform() {
    if (!item?.url) return;
    try {
      await openOnPlatform(item.url);
      actionErr = "";
    } catch (e) {
      actionErr = mapInvokeError(lang, e);
    }
  }
</script>

<div class="stage" data-playing={tallyOn ? "1" : "0"}>
  <div class="gate" data-mediahub-gate aria-label="Player">
    <span class="tally" class:lit={tallyOn} aria-hidden="true"></span>

    {#if errorMsg || actionErr}
      <div class="player-err">
        <p>{errorMsg || actionErr}</p>
        <div class="err-actions">
          <button type="button" onclick={retry}>{t(lang, "erneut")}</button>
          {#if item?.url && item.platform !== "local"}
            <button type="button" onclick={openPlatform}>{t(lang, "aufPlattform")}</button>
          {/if}
        </div>
      </div>
    {:else if item?.platform === "local" && videoSrc}
      <video
        bind:this={videoEl}
        src={videoSrc}
        controls
        playsinline
        preload={session.settings.performanceMode === "maximal" ? "metadata" : "none"}
        onplay={() => onPlayingChange?.(true)}
        onpause={() => onPlayingChange?.(false)}
      >
        <track kind="captions" label="Keine Untertitel" srclang="de" />
      </video>
    {:else if (item?.platform === "youtube" || item?.platform === "tiktok") && frameSrc}
      {#key frameSrc}
        <iframe
          bind:this={iframeEl}
          title={item.title || "Player"}
          src={frameSrc}
          allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture"
          allowfullscreen
        ></iframe>
      {/key}
    {:else}
      <div class="empty muted">{t(lang, "empty")}</div>
    {/if}
  </div>
</div>

<style>
  .stage {
    display: flex;
    justify-content: center;
    align-items: center;
    background: var(--stage);
    min-height: 0;
    flex: 1;
    padding: 0.75rem;
  }

  .gate {
    position: relative;
    width: min(100%, 360px);
    aspect-ratio: 9 / 16;
    background: #000;
    border: 1px solid var(--line);
    border-radius: var(--radius);
    overflow: hidden;
  }

  .tally {
    position: absolute;
    top: 8px;
    left: 8px;
    z-index: 2;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--steel);
  }

  .tally.lit {
    background: var(--amber);
  }

  :global(html[data-motion="on"]) .tally.lit {
    box-shadow: 0 0 6px var(--amber);
  }

  iframe,
  video {
    width: 100%;
    height: 100%;
    border: 0;
    display: block;
    background: #000;
  }

  .player-err,
  .empty {
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    height: 100%;
    padding: 1rem;
    text-align: center;
    gap: 0.75rem;
  }

  .err-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    justify-content: center;
  }
</style>

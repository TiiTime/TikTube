<script lang="ts">
  import { listSubscriptions, openOnPlatform } from "$lib/api";
  import { t, mapInvokeError } from "$lib/i18n";
  import { session } from "$lib/session.svelte";
  import type { Channel } from "$lib/types";

  let channels = $state<Channel[]>([]);
  let err = $state("");
  let loading = $state(true);

  const lang = $derived(session.settings.language);

  async function load() {
    loading = true;
    err = "";
    try {
      channels = await listSubscriptions(session.platform);
    } catch (e) {
      channels = [];
      err = mapInvokeError(lang, e);
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    session.platform;
    void load();
  });

  async function open(url: string) {
    try {
      await openOnPlatform(url);
    } catch (e) {
      err = mapInvokeError(lang, e);
    }
  }
</script>

<section class="page">
  <h1>{t(lang, "navAbos")}</h1>

  {#if loading}
    <p class="muted">{t(lang, "loading")}</p>
  {:else if session.platform === "tiktok" && channels.length === 0 && !err}
    <p class="muted">{t(lang, "tiktokNoAbos")}</p>
  {:else if err}
    <p class="err" role="alert">{err}</p>
  {:else if channels.length === 0}
    <p class="muted">{t(lang, "empty")}</p>
  {:else}
    <ul class="list">
      {#each channels as ch}
        <li class="list-row">
          <button type="button" class="linkish" onclick={() => open(ch.url)}>
            <span class="title">{ch.name}</span>
            <span class="meta">@{ch.handle}</span>
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  .page {
    padding: 1.25rem;
  }

  h1 {
    font-family: var(--font-display);
    font-size: 1.25rem;
    margin: 0 0 1rem;
  }

  .list {
    list-style: none;
    margin: 0;
    padding: 0;
  }
</style>

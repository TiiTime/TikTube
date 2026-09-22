<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { listFavorites, toggleFavorite } from "$lib/api";
  import { t, mapInvokeError } from "$lib/i18n";
  import { session } from "$lib/session.svelte";
  import type { MediaItem } from "$lib/types";

  const PAGE = 40;

  let all = $state<MediaItem[]>([]);
  let shown = $state(PAGE);
  let err = $state("");
  let loading = $state(true);

  const lang = $derived(session.settings.language);
  const visible = $derived(all.slice(0, shown));

  async function load() {
    loading = true;
    err = "";
    try {
      all = await listFavorites();
      shown = PAGE;
    } catch (e) {
      all = [];
      err = mapInvokeError(lang, e);
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    void load();
  });

  function openItem(item: MediaItem) {
    session.setPending(item);
    if (item.platform === "youtube" || item.platform === "tiktok") {
      session.setPlatform(item.platform);
    }
    void goto("/shorts?play=1");
  }

  async function unmark(item: MediaItem) {
    try {
      await toggleFavorite(item);
      all = all.filter(
        (f) => !(f.platform === item.platform && f.externalId === item.externalId),
      );
    } catch (e) {
      err = mapInvokeError(lang, e);
    }
  }

  function more() {
    shown = Math.min(shown + PAGE, all.length);
  }
</script>

<section class="page">
  <h1>{t(lang, "navFavoriten")}</h1>

  {#if loading}
    <p class="muted">{t(lang, "loading")}</p>
  {:else if err}
    <p class="err" role="alert">{err}</p>
  {:else if all.length === 0}
    <p class="muted">{t(lang, "empty")}</p>
  {:else}
    <ul class="list">
      {#each visible as item}
        <li class="list-row">
          <button type="button" class="linkish" onclick={() => openItem(item)}>
            <span class="title">{item.title}</span>
            <span class="meta">{item.creator || item.platform}</span>
          </button>
          <button type="button" onclick={() => unmark(item)}>{t(lang, "gemerkt")}</button>
        </li>
      {/each}
    </ul>
    {#if shown < all.length}
      <button type="button" class="more" onclick={more}>{t(lang, "weitere")}</button>
    {/if}
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

  .more {
    margin-top: 0.75rem;
  }
</style>

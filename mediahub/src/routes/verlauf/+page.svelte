<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { clearHistory, listHistory } from "$lib/api";
  import { t, mapInvokeError } from "$lib/i18n";
  import { session } from "$lib/session.svelte";
  import type { HistoryItem } from "$lib/types";

  const PAGE = 40;

  let all = $state<HistoryItem[]>([]);
  let shown = $state(PAGE);
  let err = $state("");
  let loading = $state(true);

  const lang = $derived(session.settings.language);
  const visible = $derived(all.slice(0, shown));

  async function load() {
    loading = true;
    err = "";
    try {
      all = await listHistory();
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

  function openItem(item: HistoryItem) {
    session.setPending(item);
    if (item.platform === "youtube" || item.platform === "tiktok") {
      session.setPlatform(item.platform);
    }
    void goto("/shorts?play=1");
  }

  async function clear() {
    try {
      await clearHistory();
      all = [];
      shown = PAGE;
    } catch (e) {
      err = mapInvokeError(lang, e);
    }
  }

  function more() {
    shown = Math.min(shown + PAGE, all.length);
  }
</script>

<section class="page">
  <div class="head">
    <h1>{t(lang, "navVerlauf")}</h1>
    {#if all.length > 0}
      <button type="button" onclick={clear}>{t(lang, "verlaufLoeschen")}</button>
    {/if}
  </div>

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

  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    margin-bottom: 1rem;
  }

  h1 {
    font-family: var(--font-display);
    font-size: 1.25rem;
    margin: 0;
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

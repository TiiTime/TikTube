<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { open } from "@tauri-apps/plugin-dialog";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import {
    addLocalMedia,
    listLocalMedia,
    removeLocalMedia,
  } from "$lib/api";
  import { t, mapInvokeError } from "$lib/i18n";
  import { session } from "$lib/session.svelte";
  import type { LocalMedia, MediaItem } from "$lib/types";

  let items = $state<LocalMedia[]>([]);
  let err = $state("");
  let loading = $state(true);

  const lang = $derived(session.settings.language);

  async function load() {
    loading = true;
    err = "";
    try {
      items = await listLocalMedia();
    } catch (e) {
      items = [];
      err = mapInvokeError(lang, e);
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    void load();

    let unlisten: (() => void) | undefined;
    let gone = false;
    void (async () => {
      try {
        const stop = await getCurrentWebview().onDragDropEvent(async (event) => {
          if (event.payload.type !== "drop") return;
          const paths = event.payload.paths ?? [];
          if (paths.length === 0) return;
          try {
            await addLocalMedia(paths);
            items = await listLocalMedia();
          } catch (e) {
            err = mapInvokeError(lang, e);
          }
        });
        if (gone) stop();
        else unlisten = stop;
      } catch {
        /* browser preview */
      }
    })();

    return () => {
      gone = true;
      unlisten?.();
    };
  });

  async function chooseFiles() {
    try {
      const selected = await open({
        multiple: true,
        filters: [
          {
            name: "Video",
            extensions: ["mp4", "webm", "m4v"],
          },
        ],
      });
      if (!selected) return;
      const paths = Array.isArray(selected) ? selected : [selected];
      await addLocalMedia(paths);
      items = await listLocalMedia();
    } catch (e) {
      err = mapInvokeError(lang, e);
    }
  }

  function play(m: LocalMedia) {
    const item: MediaItem = {
      platform: "local",
      externalId: String(m.id),
      title: m.title,
      creator: "",
      url: m.path,
      thumb: "",
    };
    session.setPending(item);
    void goto("/shorts?play=1");
  }

  async function remove(id: number) {
    try {
      await removeLocalMedia(id);
      items = items.filter((m) => m.id !== id);
    } catch (e) {
      err = mapInvokeError(lang, e);
    }
  }
</script>

<section class="page">
  <div class="head">
    <h1>{t(lang, "navMedien")}</h1>
    <button type="button" class="primary" onclick={chooseFiles}>
      {t(lang, "dateienWaehlen")}
    </button>
  </div>
  <p class="muted hint">{t(lang, "medienHint")}</p>

  {#if loading}
    <p class="muted">{t(lang, "loading")}</p>
  {:else if err}
    <p class="err" role="alert">{err}</p>
  {:else if items.length === 0}
    <p class="muted">{t(lang, "empty")}</p>
  {:else}
    <ul class="list">
      {#each items as m}
        <li class="list-row">
          <button type="button" class="linkish" onclick={() => play(m)}>
            <span class="title">{m.title}</span>
            <span class="meta">{m.kind} · {m.path}</span>
          </button>
          <button type="button" onclick={() => play(m)}>{t(lang, "abspielen")}</button>
          <button type="button" onclick={() => remove(m.id)}>{t(lang, "entfernen")}</button>
        </li>
      {/each}
    </ul>
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
  }

  h1 {
    font-family: var(--font-display);
    font-size: 1.25rem;
    margin: 0;
  }

  .hint {
    margin: 0.75rem 0 1rem;
  }

  .list {
    list-style: none;
    margin: 0;
    padding: 0;
  }
</style>

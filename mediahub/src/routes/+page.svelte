<script lang="ts">
  import { isTauri } from "@tauri-apps/api/core";
  import { session } from "$lib/session.svelte";
  import { t } from "$lib/i18n";

  const lang = $derived(session.settings.language);
  const desktop = isTauri();
</script>

<section class="page">
  {#if desktop}
    <p class="muted">Die Seite liegt im Fenster unter der Leiste.</p>
  {:else}
    <h1>{t(lang, "wordmark")}</h1>
    <p>
      YouTube und TikTok öffnen im Desktop-Fenster. Anmelden passiert dort auf der Seite, über Google bzw. TikTok.
    </p>
  {/if}
</section>

<style>
  .page {
    padding: 1.25rem;
    max-width: 36rem;
  }

  h1 {
    font-family: var(--font-display);
    font-size: 1.25rem;
    margin: 0 0 0.75rem;
  }
</style>

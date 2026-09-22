<script lang="ts">
  import { clearHistory, saveSettings } from "$lib/api";
  import { t, mapInvokeError } from "$lib/i18n";
  import { session } from "$lib/session.svelte";
  import type { Settings } from "$lib/types";

  let draft = $state<Settings>({ ...session.settings });
  let err = $state("");
  let saved = $state(false);
  let saving = $state(false);
  let synced = false;

  const lang = $derived(draft.language);

  $effect(() => {
    // One-shot sync after layout loads settings
    if (!synced && session.ready) {
      draft = { ...session.settings };
      synced = true;
    }
  });

  function applyImmediate(partial: Partial<Settings>) {
    draft = { ...draft, ...partial };
    session.applySettings(draft);
  }

  async function save() {
    err = "";
    saved = false;
    saving = true;
    try {
      await saveSettings(draft);
      session.applySettings(draft);
      saved = true;
    } catch (e) {
      err = mapInvokeError(lang, e);
    } finally {
      saving = false;
    }
  }

  async function clearHist() {
    try {
      await clearHistory();
    } catch (e) {
      err = mapInvokeError(lang, e);
    }
  }

</script>

<section class="page">
  <h1>{t(lang, "navEinstellungen")}</h1>

  {#if err}
    <p class="err" role="alert">{err}</p>
  {/if}
  {#if saved}
    <p class="muted">{t(lang, "saved")}</p>
  {/if}

  <fieldset class="group">
    <legend>{t(lang, "allgemein")}</legend>

    <div class="field">
      <label for="start-page">{t(lang, "startPage")}</label>
      <select
        id="start-page"
        bind:value={draft.startPage}
        onchange={() => applyImmediate({ startPage: draft.startPage })}
      >
        <option value="start">{t(lang, "navStart")}</option>
        <option value="shorts">{t(lang, "navShorts")}</option>
      </select>
    </div>

    <div class="field">
      <label for="language">{t(lang, "language")}</label>
      <select
        id="language"
        bind:value={draft.language}
        onchange={() => applyImmediate({ language: draft.language })}
      >
        <option value="de">Deutsch</option>
        <option value="en">English</option>
      </select>
    </div>

    <div class="field">
      <label for="theme">{t(lang, "theme")}</label>
      <select
        id="theme"
        bind:value={draft.theme}
        onchange={() => applyImmediate({ theme: draft.theme })}
      >
        <option value="dunkel">{t(lang, "themeDunkel")}</option>
        <option value="hell">{t(lang, "themeHell")}</option>
      </select>
    </div>

    <div class="field check">
      <label>
        <input
          type="checkbox"
          bind:checked={draft.animations}
          onchange={() => applyImmediate({ animations: draft.animations })}
        />
        {t(lang, "animations")}
      </label>
    </div>
  </fieldset>

  <fieldset class="group">
    <legend>{t(lang, "wiedergabe")}</legend>

    <div class="field check">
      <label>
        <input type="checkbox" bind:checked={draft.autoplay} />
        {t(lang, "autoplay")}
      </label>
    </div>

    <div class="field">
      <label for="volume">{t(lang, "volume")}: {draft.volume}</label>
      <input id="volume" type="range" min="0" max="100" bind:value={draft.volume} />
    </div>
  </fieldset>

  <fieldset class="group">
    <legend>{t(lang, "performance")}</legend>
    <div class="radios" role="radiogroup" aria-label={t(lang, "performance")}>
      <label>
        <input
          type="radio"
          name="perf"
          value="sparsam"
          checked={draft.performanceMode === "sparsam"}
          onchange={() => applyImmediate({ performanceMode: "sparsam" })}
        />
        {t(lang, "modeSparsam")}
      </label>
      <label>
        <input
          type="radio"
          name="perf"
          value="standard"
          checked={draft.performanceMode === "standard"}
          onchange={() => applyImmediate({ performanceMode: "standard" })}
        />
        {t(lang, "modeStandard")}
      </label>
      <label>
        <input
          type="radio"
          name="perf"
          value="maximal"
          checked={draft.performanceMode === "maximal"}
          onchange={() => applyImmediate({ performanceMode: "maximal" })}
        />
        {t(lang, "modeMaximal")}
      </label>
    </div>
  </fieldset>

  <fieldset class="group">
    <legend>{t(lang, "datenschutz")}</legend>

    <div class="field check">
      <label>
        <input type="checkbox" bind:checked={draft.historyEnabled} />
        {t(lang, "historyEnabled")}
      </label>
    </div>

    <div class="actions">
      <button type="button" onclick={clearHist}>{t(lang, "verlaufLoeschen")}</button>
    </div>
  </fieldset>

  <button type="button" class="primary save" disabled={saving} onclick={save}>
    {saving ? t(lang, "loading") : t(lang, "speichern")}
  </button>
</section>

<style>
  .page {
    padding: 1.25rem;
    max-width: 480px;
  }

  h1 {
    font-family: var(--font-display);
    font-size: 1.25rem;
    margin: 0 0 1rem;
  }

  .group {
    border: 1px solid var(--line);
    border-radius: var(--radius);
    margin: 0 0 1rem;
    padding: 0.75rem 1rem 0.25rem;
    background: var(--panel);
  }

  legend {
    padding: 0 0.35rem;
    color: var(--steel);
    font-size: 0.85rem;
  }

  .check label {
    display: flex;
    align-items: center;
    gap: 0.45rem;
    color: inherit;
    font-size: 0.95rem;
  }

  .radios {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    margin-bottom: 0.75rem;
  }

  .radios label {
    display: flex;
    align-items: center;
    gap: 0.45rem;
    color: inherit;
    font-size: 0.95rem;
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem;
    margin-bottom: 0.75rem;
  }

  .save {
    margin-top: 0.25rem;
  }

  input[type="range"] {
    width: 100%;
  }
</style>

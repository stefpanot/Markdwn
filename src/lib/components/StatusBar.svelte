<script lang="ts">
  import { app } from "$lib/state.svelte";

  const modeLabel = { read: "Lecture", split: "Split", zen: "Zen" };
  const words = $derived(app.rendered?.words ?? 0);
  const minutes = $derived(app.rendered?.reading_minutes ?? 0);
  const nf = new Intl.NumberFormat("fr-FR");
</script>

<div class="status">
  <span class="path" title={app.active?.path}>
    {app.active?.path || "Document non enregistré"}
  </span>
  {#if app.dirty}
    <span class="dirty">Modifié</span>
  {/if}

  <div class="spacer"></div>

  <span>{nf.format(words)} mots · {minutes} min de lecture</span>
  {#if app.mode === "read"}
    <span>
      {app.readingSize} px · {app.readingWidth === "full" ? "pleine largeur" : "colonne centrée"}
    </span>
  {:else}
    <span>Ln {app.cursorLine}, Col {app.cursorCol}</span>
  {/if}
  <span>UTF-8</span>
  <span class="mode">{modeLabel[app.mode]}</span>
</div>

<style>
  .status {
    height: var(--h-status);
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 0 14px;
    border-top: 1px solid var(--border);
    font-size: 11.5px;
    color: var(--fg-3);
  }
  .spacer {
    flex: 1;
  }
  .path {
    max-width: 40%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .dirty {
    color: var(--accent-2);
  }
  .mode {
    color: var(--fg-2);
  }
</style>

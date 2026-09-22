<script lang="ts">
  import Icon from "./Icon.svelte";
  import { app } from "$lib/state.svelte";

  const modeLabel = { read: "Lecture", split: "Split", zen: "Zen" };
  const words = $derived(app.rendered?.words ?? 0);
  const minutes = $derived(app.rendered?.reading_minutes ?? 0);
  const nf = new Intl.NumberFormat("fr-FR");
</script>

<!-- Barre d'état basse : le point d'accès à la barre latérale est TOUJOURS
     ici, à gauche, quel que soit le contexte (sauf Zen, qui garde ses
     outils propres). Fini le bouton qui sautait de la titlebar aux outils
     zen selon le mode. Le reste de la barre ne porte que les infos du
     document courant, d'où le test sur app.active. -->
<div class="status">
  <button
    class="toggle"
    class:on={app.sidebarVisible}
    onclick={() => (app.sidebarVisible = !app.sidebarVisible)}
    aria-pressed={app.sidebarVisible}
    title="Barre de dossiers — Ctrl+B"
  >
    <Icon name="sidebar" size={15} width={1.4} />
    <span>Dossiers</span>
  </button>

  {#if app.active}
    <span class="sep"></span>
    <span class="path" title={app.active.path}>
      {app.active.path || "Document non enregistré"}
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
  {/if}
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
  .toggle {
    display: flex;
    align-items: center;
    gap: 7px;
    margin-left: -8px;
    padding: 3px 9px;
    border-radius: var(--r-md);
    font-size: 11.5px;
    font-weight: 500;
    color: var(--fg-2);
  }
  .toggle:hover {
    background: var(--accent-soft);
    color: var(--accent-on);
  }
  /* État reflété : la barre est visible, le bouton le dit. */
  .toggle.on {
    color: var(--accent-fg);
  }
  .sep {
    width: 1px;
    height: 14px;
    background: var(--border);
    flex-shrink: 0;
  }
</style>

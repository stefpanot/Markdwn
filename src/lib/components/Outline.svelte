<script lang="ts">
  import Icon from "./Icon.svelte";
  import { app } from "$lib/state.svelte";

  /* Le plan du document n'existe qu'en UN exemplaire à la fois : variant
     "panel" en bas de la sidebar (Split/Zen), variant "rail" en colonne
     autonome du mode Lecture. Deux exemplaires simultanés semaient le doute
     sur lequel suivre. */
  interface Props {
    onGoto: (line: number) => void;
    /** "rail" = colonne autonome du mode Lecture, "panel" = bas de sidebar. */
    variant?: "panel" | "rail";
    activeLine?: number;
  }
  let { onGoto, variant = "panel", activeLine = 0 }: Props = $props();

  /** Le titre courant est le dernier dont la ligne est au-dessus du curseur. */
  const currentSlug = $derived.by(() => {
    let slug = "";
    for (const h of app.headings) {
      if (h.line > activeLine) break;
      slug = h.slug;
    }
    return slug;
  });

  const minLevel = $derived(
    app.headings.length ? Math.min(...app.headings.map((h) => h.level)) : 1,
  );

  /* Repli du rail : le mode Lecture remplace alors le plan par une fine
     colonne avec un bouton pour le rouvrir (+page.svelte) — le contenu ne
     doit jamais devenir introuvable. */
  function collapseRail() {
    app.tocCollapsed = true;
  }
</script>

<div
  class="outline {variant}"
  style:height={variant === "panel" && !app.outlineCollapsed
    ? `${app.outlineHeight * 100}%`
    : undefined}
>
  {#if variant === "rail"}
    <!-- L'en-tête fait exactement la hauteur de celui de la barre de dossiers
         (38 px) : le bouton de repli s'aligne sur ses icônes quand les deux
         sont visibles. -->
    <div class="head rail-head">
      <span class="rail-title">Sur cette page</span>
      <button class="icon-btn small" onclick={collapseRail} title="Masquer le plan">
        <Icon name="toc-collapse" size={14} width={1.4} />
      </button>
    </div>
  {:else}
    <!-- En-tête cliquable : le geste clavier/tactile du repli, pendant du
         drag de la séparation arbre/plan pour la souris. -->
    <button
      class="head panel-head"
      onclick={() => (app.outlineCollapsed = !app.outlineCollapsed)}
      aria-expanded={!app.outlineCollapsed}
    >
      <span class="twisty" class:open={!app.outlineCollapsed}>
        <Icon name="chevron-right" size={12} width={1.6} />
      </span>
      Plan du document
    </button>
  {/if}

  {#if variant === "rail" || !app.outlineCollapsed}
    {#if app.headings.length === 0}
      <p class="empty">Aucun titre dans ce document.</p>
    {:else}
      <div class="items">
        {#each app.headings as h (h.line)}
          <button
            class="item"
            class:on={h.slug === currentSlug}
            style:padding-left="{12 + (h.level - minLevel) * 14}px"
            onclick={() => onGoto(h.line)}
            title={h.text}
          >
            {h.text}
          </button>
        {/each}
      </div>
    {/if}
  {/if}
</div>

<style>
  .outline {
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  .outline.panel {
    flex-shrink: 0;
    border-top: 1px solid var(--border);
    padding: 8px;
  }
  .outline.rail {
    width: var(--w-toc);
    flex-shrink: 0;
    min-height: 0;
    /* Colonne flex : l'en-tête reste fixe, la liste défile à l'intérieur. */
  }

  .head {
    font-size: 10.5px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--fg-3);
    padding: 4px 10px 10px;
    flex-shrink: 0;
  }
  .rail-head {
    height: 38px;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 8px 0 14px;
    border-left: 1px solid var(--border);
  }
  .rail-title {
    flex: 1;
    font-size: 11.5px;
    font-weight: 600;
    letter-spacing: 0.04em;
    color: var(--fg-2);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .panel-head {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    text-align: left;
    border-radius: var(--r-md);
  }
  .panel-head:hover {
    color: var(--fg-1);
    background: var(--hover);
  }
  .panel-head .twisty {
    display: flex;
    color: var(--fg-2);
    transition: transform 120ms ease;
  }
  .panel-head .twisty.open {
    transform: rotate(90deg);
  }
  .icon-btn.small {
    width: 20px;
    height: 20px;
    border-radius: var(--r-sm);
  }

  .empty {
    margin: 0;
    padding: 0 10px 6px;
    font-size: 12px;
    color: var(--fg-3);
  }
  .rail .empty {
    padding: 2px 10px 6px 14px;
    border-left: 1px solid var(--border);
  }

  .items {
    display: flex;
    flex-direction: column;
    gap: 2px;
    overflow-y: auto;
    min-height: 0;
    overscroll-behavior: contain;
  }
  /* Dans le rail, la liste est le seul conteneur scrollant : l'en-tête
     reste visible en permanence. */
  .rail .items {
    flex: 1;
    padding: 2px 10px 16px 14px;
    border-left: 1px solid var(--border);
  }

  .item {
    display: block;
    width: 100%;
    /* Garde critique : `overflow: hidden` ci-dessous ramène le min-height
       automatique de ce flex item à zéro (spec flexbox). Sans flex-shrink: 0,
       un document à beaucoup de titres écrase chaque bouton à quelques pixels
       et le texte devient illisible. */
    flex-shrink: 0;
    padding: 5px 10px;
    border-radius: var(--r-md);
    font-size: 12.5px;
    font-weight: 450;
    color: var(--fg-2);
    text-align: left;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .item:hover {
    background: var(--hover);
    color: var(--fg-1);
  }
  .item.on {
    background: var(--accent-soft);
    box-shadow: inset 2px 0 0 var(--accent);
    color: var(--accent-on);
    font-weight: 500;
  }
</style>

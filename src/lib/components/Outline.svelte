<script lang="ts">
  import { app } from "$lib/state.svelte";

  /* Le plan du document vit ici, en bas de la sidebar, et pas dans un panneau
     séparé : c'est ce qu'on consulte le plus dans un long .md. */
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
</script>

<div class="outline {variant}">
  <div class="head">{variant === "rail" ? "Sur cette page" : "Plan du document"}</div>

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
    max-height: 40%;
  }
  .outline.rail {
    width: var(--w-toc);
    flex-shrink: 0;
    padding: 30px 20px 20px 26px;
    overflow-y: auto;
  }

  .head {
    font-size: 10.5px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--fg-3);
    padding: 4px 10px 10px;
  }
  .rail .head {
    padding: 0 0 14px;
  }

  .empty {
    margin: 0;
    padding: 0 10px 6px;
    font-size: 12px;
    color: var(--fg-3);
  }

  .items {
    display: flex;
    flex-direction: column;
    gap: 2px;
    overflow-y: auto;
    min-height: 0;
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

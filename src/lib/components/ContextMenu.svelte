<script lang="ts">
  import type { MenuItem } from "$lib/menu";

  interface Props {
    x: number;
    y: number;
    items: MenuItem[];
    onClose: () => void;
  }
  let { x, y, items, onClose }: Props = $props();

  let menu: HTMLDivElement;
  let size = $state({ w: 0, h: 0 });

  /** Replie le menu à l'intérieur de la fenêtre s'il déborde. On le garde
      invisible le temps de la mesure : une frame décalée se verrait. */
  const pos = $derived({
    left: size.w ? Math.min(x, window.innerWidth - size.w - 8) : x,
    top: size.h ? Math.min(y, window.innerHeight - size.h - 8) : y,
  });

  $effect(() => {
    if (!menu || size.w) return;
    const r = menu.getBoundingClientRect();
    size = { w: r.width, h: r.height };
  });

  function choose(item: MenuItem) {
    if (item.disabled) return;
    onClose();
    item.run();
  }
</script>

<svelte:window
  onpointerdown={(e) => {
    if (menu && !menu.contains(e.target as Node)) onClose();
  }}
  onkeydown={(e) => e.key === "Escape" && onClose()}
  onblur={onClose}
/>

<div
  class="menu"
  bind:this={menu}
  style:left="{pos.left}px"
  style:top="{pos.top}px"
  style:visibility={size.w ? "visible" : "hidden"}
  role="menu"
  tabindex="-1"
>
  {#each items as item, i (i)}
    {#if item.separatorBefore}
      <div class="sep"></div>
    {/if}
    <button class="item" role="menuitem" disabled={item.disabled} onclick={() => choose(item)}>
      <span class="tick" class:on={item.checked}>
        {#if item.checked}
          <svg width="13" height="13" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M3.5 8.5l3 3 6-7" /></svg>
        {/if}
      </span>
      <span class="text">{item.label}</span>
      {#if item.keys}
        <span class="keys">{item.keys}</span>
      {/if}
    </button>
  {/each}
</div>

<style>
  .menu {
    position: fixed;
    z-index: 50;
    min-width: 208px;
    padding: 5px;
    border-radius: var(--r-xl);
    background: var(--surface-raised);
    box-shadow:
      0 0 0 1px var(--border-strong),
      0 12px 34px rgb(0 0 0 / 0.36);
  }
  .item {
    display: flex;
    align-items: center;
    gap: 9px;
    width: 100%;
    padding: 7px 11px;
    border-radius: var(--r-md);
    font-size: 12.5px;
    color: var(--fg-1);
    text-align: left;
  }
  /* Largeur réservée en permanence : cocher une entrée ne doit pas décaler
     le libellé des autres. */
  .tick {
    width: 13px;
    flex-shrink: 0;
    display: flex;
    color: var(--accent-fg);
  }
  .text {
    flex: 1;
    white-space: nowrap;
  }
  .keys {
    font-family: var(--font-mono);
    font-size: 10.5px;
    color: var(--fg-3);
    flex-shrink: 0;
  }
  .item:hover:not(:disabled) .keys {
    color: var(--accent-fg);
  }
  .item:hover:not(:disabled) {
    background: var(--accent-soft);
    color: var(--accent-on);
  }
  .item:disabled {
    color: var(--fg-4);
  }
  .sep {
    height: 1px;
    margin: 4px 6px;
    background: var(--border-strong);
  }
</style>

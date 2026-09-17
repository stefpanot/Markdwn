<script lang="ts">
  import Icon from "./Icon.svelte";
  import { app } from "$lib/state.svelte";
  import { minimise, toggleMaximise, closeWindow } from "$lib/window";

  interface Props {
    onNew: () => void;
    onCloseTab: (index: number) => void;
    onTabMenu: (index: number, x: number, y: number) => void;
    /** Menu applicatif, ouvert depuis le logo — façon Zed. */
    onAppMenu: (x: number, y: number) => void;
  }
  let { onNew, onCloseTab, onTabMenu, onAppMenu }: Props = $props();

  let tabsStrip: HTMLDivElement;

  /* Beaucoup d'onglets ouverts : la bande scrolle horizontalement au lieu de
     tronquer en silence. La molette verticale devient un défilement
     horizontal, geste attendu sur une barre d'onglets. */
  function onTabsWheel(e: WheelEvent) {
    if (Math.abs(e.deltaY) <= Math.abs(e.deltaX)) return;
    e.preventDefault();
    tabsStrip.scrollLeft += e.deltaY;
  }

  /* Un changement d'onglet au clavier (palette, Alt+↑↓) doit ramener l'onglet
     actif dans la partie visible de la bande. */
  $effect(() => {
    app.activeIndex;
    tabsStrip
      ?.querySelector(".tab.active")
      ?.scrollIntoView({ block: "nearest", inline: "nearest" });
  });

  function openAppMenu(e: MouseEvent) {
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    onAppMenu(r.left, r.bottom + 6);
  }
</script>

<!-- Les onglets vivent DANS la titlebar : 40px de hauteur d'écriture gagnés.
     Le drag region de Tauri ne s'active que si la CIBLE du clic porte
     l'attribut — pas un ancêtre. D'où la zone de préhension explicite : sans
     elle, la barre est presque impossible à saisir. -->
<!-- L'attribut est posé sur les PARENTS aussi : Tauri teste la cible du clic,
     donc un parent porteur rend saisissable tout ce qu'aucun enfant ne couvre
     (marges, interstices entre onglets, bandes au-dessus et en dessous) sans
     rendre les onglets ni les boutons moins cliquables. -->
<div class="titlebar" data-tauri-drag-region>
  <!-- Le bouton de barre de dossiers vit ICI et nulle part ailleurs : la
       titlebar est le seul chrome présent dans tous les modes ET sur l'écran
       d'accueil. Dans la barre d'outils, il devenait inatteignable dès qu'on
       fermait tous les fichiers avec la barre masquée. -->
  <div class="left" data-tauri-drag-region>
    <button class="mark" onclick={openAppMenu} title="Menu — actions et paramètres">
      <span class="mark-glyph"><Icon name="wordmark" size={13} width={1.9} /></span>
    </button>
    <button
      class="icon-btn"
      onclick={() => (app.sidebarVisible = !app.sidebarVisible)}
      aria-pressed={app.sidebarVisible}
      title="Barre de dossiers — Ctrl+B"
    >
      <Icon name="sidebar" size={16} width={1.4} />
    </button>
  </div>

  <div class="tabs" data-tauri-drag-region bind:this={tabsStrip} onwheel={onTabsWheel}>
    {#each app.docs as doc, i (doc.id)}
      <div
        class="tab"
        class:active={i === app.activeIndex}
        class:dirty={doc.content !== doc.savedContent}
      >
        <!-- Le clic droit est porté par le bouton, pas par le div : un
             élément statique avec un gestionnaire n'est pas accessible. -->
        <button
          class="tab-main"
          onclick={() => (app.activeIndex = i)}
          oncontextmenu={(e) => {
            e.preventDefault();
            onTabMenu(i, e.clientX, e.clientY);
          }}
          title={doc.path || doc.name}
        >
          <span class="tab-icon" class:accent={i === app.activeIndex}>
            <Icon name="file" size={14} width={1.4} />
          </span>
          <span class="tab-name">{doc.name}</span>
        </button>

        <!-- Largeur réservée : le point et la croix se relaient sans décaler
             l'onglet, et un onglet modifié reste fermable. -->
        <span class="tab-end">
          <span class="dot" title="Non enregistré"></span>
          <button class="tab-close" onclick={() => onCloseTab(i)} aria-label="Fermer {doc.name}">
            <Icon name="close" size={11} width={1.5} />
          </button>
        </span>
      </div>
    {/each}
  </div>

  <!-- Hors de la bande scrollable : « nouveau » reste visible même quand les
       onglets débordent. -->
  <button class="new" onclick={onNew} title="Nouveau document — Ctrl+N">
    <Icon name="plus" size={15} />
  </button>

  <!-- Toute la place restante est saisissable, et le double-clic agrandit. -->
  <div class="grip" data-tauri-drag-region></div>

  <div class="controls">
    <button class="wc" onclick={minimise} aria-label="Réduire">
      <Icon name="minimise" size={11} width={1.2} />
    </button>
    <button class="wc" onclick={toggleMaximise} aria-label="Agrandir">
      <Icon name="maximise" size={11} width={1.2} />
    </button>
    <button class="wc close" onclick={closeWindow} aria-label="Fermer la fenêtre">
      <Icon name="close" size={11} width={1.2} />
    </button>
  </div>
</div>

<style>
  .titlebar {
    height: var(--h-titlebar);
    flex-shrink: 0;
    display: flex;
    align-items: center;
    padding-left: 12px;
    gap: 10px;
  }

  .left {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-shrink: 0;
  }

  .mark {
    width: 22px;
    height: 22px;
    border-radius: var(--r-md);
    background: linear-gradient(150deg, var(--accent), color-mix(in oklab, var(--accent) 80%, black));
    color: #fff;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    box-shadow: 0 1px 3px rgb(0 0 0 / 0.3);
  }
  .mark:hover {
    filter: brightness(1.12);
  }
  /* Le glyphe ne doit pas intercepter le clic destiné au bouton. */
  .mark-glyph {
    display: flex;
    pointer-events: none;
  }

  /* Dimensionné au contenu, pas flex:1 : c'est ce qui libère .grip.
     Scrollable plutôt que tronqué : un onglet caché sans indicateur est
     inatteignable à la souris. La scrollbar resterait trop haute pour une
     titlebar de 40px : masquée, la molette (ci-dessus) fait le travail. */
  .tabs {
    display: flex;
    align-items: center;
    gap: 3px;
    min-width: 0;
    overflow-x: auto;
    overflow-y: hidden;
    scrollbar-width: none;
    flex: 0 1 auto;
  }
  .tabs::-webkit-scrollbar {
    display: none;
  }

  /* Garantie d'une cible réelle même avec beaucoup d'onglets ouverts. */
  .grip {
    flex: 1 1 auto;
    align-self: stretch;
    min-width: 90px;
  }

  .tab {
    height: 30px;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 7px 0 11px;
    border-radius: var(--r-lg);
    color: var(--fg-2);
    font-size: 12.5px;
    font-weight: 450;
    max-width: 220px;
    flex-shrink: 0;
  }
  .tab:hover {
    background: var(--hover);
  }
  .tab.active {
    background: var(--active);
    box-shadow: inset 0 0 0 1px var(--border);
    color: var(--fg-1);
    font-weight: 500;
  }
  .tab-main {
    display: flex;
    align-items: center;
    gap: 7px;
    min-width: 0;
    color: inherit;
    font: inherit;
  }
  .tab-icon {
    display: flex;
    color: var(--fg-3);
  }
  .tab-icon.accent {
    color: var(--accent);
  }
  .tab-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .tab-end {
    width: 16px;
    height: 16px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent-2);
    display: none;
  }
  .tab.dirty .dot {
    display: block;
  }
  .tab.dirty:hover .dot {
    display: none;
  }
  .tab-close {
    display: none;
    align-items: center;
    justify-content: center;
    width: 16px;
    height: 16px;
    border-radius: 4px;
    color: var(--fg-3);
  }
  .tab:hover .tab-close {
    display: flex;
  }
  .tab-close:hover {
    background: var(--border-strong);
    color: var(--fg-1);
  }

  .new {
    width: 26px;
    height: 26px;
    margin-left: 3px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--r-md);
    color: var(--fg-3);
    flex-shrink: 0;
  }
  .new:hover {
    background: var(--hover);
    color: var(--fg-1);
  }

  .controls {
    display: flex;
    align-self: stretch;
    flex-shrink: 0;
  }
  .wc {
    width: 46px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--fg-1);
  }
  .wc:hover {
    background: var(--hover);
  }
  .wc.close:hover {
    background: #c42b1c;
    color: #fff;
  }
</style>

<script lang="ts">
  import Icon from "./Icon.svelte";
  import TreeNode from "./TreeNode.svelte";
  import Outline from "./Outline.svelte";
  import { app } from "$lib/state.svelte";

  interface Props {
    onOpenFolder: () => void;
    onOpenPath: (path: string) => void;
    onGoto: (line: number) => void;
    /** Rafraîchit l'arborescence depuis le disque (bouton + watcher). */
    onRefresh: () => void;
    /** Le plan du document n'a qu'une seule place à la fois : en bas de la
        sidebar en Split/Zen, dans le rail « Sur cette page » en Lecture.
        La sidebar du mode Lecture passe donc withOutline à false. */
    withOutline?: boolean;
  }
  let { onOpenFolder, onOpenPath, onGoto, onRefresh, withOutline = true }: Props = $props();

  let asideEl = $state<HTMLElement>();

  const folderName = $derived(
    app.folderPath ? app.folderPath.split(/[\\/]/).filter(Boolean).pop() : "",
  );

  /* ----- redimensionnement de la barre (poignée sur le bord droit) ----- */
  const SIDEBAR_MIN = 180;
  const SIDEBAR_MAX = 480;

  function startSidebarDrag(e: PointerEvent) {
    e.preventDefault();
    // bind:this sur un élément toujours rendu : le drag ne peut pas démarrer
    // avant que la poignée existe.
    const el = asideEl!;
    const startX = e.clientX;
    const startWidth = el.getBoundingClientRect().width;
    el.setPointerCapture(e.pointerId);
    // Verrouiller le curseur dès le premier mouvement, même si la poignée ne
    // fait que quelques pixels de large.
    document.body.style.cursor = "col-resize";

    const onMove = (ev: PointerEvent) => {
      const w = Math.min(SIDEBAR_MAX, Math.max(SIDEBAR_MIN, startWidth + ev.clientX - startX));
      app.sidebarWidth = Math.round(w);
    };
    const onUp = () => {
      document.body.style.cursor = "";
      el.removeEventListener("pointermove", onMove);
      el.removeEventListener("pointerup", onUp);
      el.removeEventListener("pointercancel", onUp);
    };
    el.addEventListener("pointermove", onMove);
    el.addEventListener("pointerup", onUp);
    el.addEventListener("pointercancel", onUp);
  }

  /* ----- plan en bas : hauteur ajustable par drag sur la séparation ----- */
  const OUTLINE_MIN = 0.2;
  const OUTLINE_MAX = 0.7;

  function startOutlineDrag(e: PointerEvent) {
    e.preventDefault();
    const handle = e.currentTarget as HTMLElement;
    handle.setPointerCapture(e.pointerId);
    document.body.style.cursor = "row-resize";

    const onMove = (ev: PointerEvent) => {
      const box = asideEl!.getBoundingClientRect();
      const h = (box.bottom - ev.clientY) / box.height;
      app.outlineHeight = Math.min(OUTLINE_MAX, Math.max(OUTLINE_MIN, h));
    };
    const onUp = () => {
      document.body.style.cursor = "";
      handle.removeEventListener("pointermove", onMove);
      handle.removeEventListener("pointerup", onUp);
      handle.removeEventListener("pointercancel", onUp);
    };
    handle.addEventListener("pointermove", onMove);
    handle.addEventListener("pointerup", onUp);
    handle.addEventListener("pointercancel", onUp);
  }
</script>

<aside class="sidebar" bind:this={asideEl}>
  <div class="head">
    <span class="title">{folderName || "Aucun dossier"}</span>
    <button
      class="icon-btn small"
      onclick={onRefresh}
      disabled={!app.folderPath}
      title="Rafraîchir l'arborescence"
    >
      <Icon name="sync" size={13} width={1.5} />
    </button>
    <button class="icon-btn small" onclick={onOpenFolder} title="Ouvrir un dossier">
      <Icon name="folder" size={14} width={1.4} />
    </button>
  </div>

  <div class="tree">
    {#if app.folderEntries.length === 0}
      <button class="empty" onclick={onOpenFolder}>
        <Icon name="folder" size={20} width={1.3} />
        <span>Ouvrir un dossier</span>
      </button>
    {:else}
      {#each app.folderEntries as entry (entry.path)}
        <TreeNode {entry} depth={0} onOpen={onOpenPath} />
      {/each}
    {/if}
  </div>

  <!-- Pas de plan quand il n'y a pas de document : un panneau qui annonce
       « aucun titre » sur l'écran d'accueil n'informe de rien. -->
  {#if app.active && withOutline}
    <!-- Séparation arbre/plan : drag vertical pour régler la hauteur du plan
         (20–70 % de la sidebar). Réservée à la souris : le clavier passe par
         l'en-tête cliquable du plan pour le replier. -->
    <div
      class="sep"
      class:hidden={app.outlineCollapsed}
      role="separator"
      aria-orientation="horizontal"
      aria-label="Hauteur du plan"
      onpointerdown={startOutlineDrag}
    ></div>
    <Outline {onGoto} activeLine={app.cursorLine} />
  {/if}

  <!-- Poignée de redimensionnement : à l'extérieur du flux pour ne pas
       manger de place dans la colonne. -->
  <div
    class="drag"
    role="separator"
    aria-orientation="vertical"
    aria-label="Largeur de la barre de dossiers"
    onpointerdown={startSidebarDrag}
  ></div>
</aside>

<style>
  .sidebar {
    width: var(--w-sidebar);
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    min-height: 0;
    position: relative;
    border-right: 1px solid var(--border);
    background: color-mix(in oklab, var(--surface-editor) 40%, transparent);
  }

  /* Zone de saisie généreuse (6 px) autour d'une ligne visible de 1 px au
     survol : le bord droit de la barre reste facile à attraper sans afficher
     en permanence une poignée lourde. */
  .drag {
    position: absolute;
    top: 0;
    right: -3px;
    width: 6px;
    height: 100%;
    cursor: col-resize;
    z-index: 2;
  }
  .drag::after {
    content: "";
    position: absolute;
    top: 0;
    left: 3px;
    width: 1px;
    height: 100%;
    background: var(--accent);
    opacity: 0;
    transition: opacity 120ms ease;
  }
  .drag:hover::after,
  .drag:active::after {
    opacity: 1;
  }

  .head {
    height: 38px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 0 8px 0 12px;
  }
  .title {
    flex: 1;
    font-size: 11.5px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--fg-2);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .icon-btn.small {
    width: 22px;
    height: 22px;
    border-radius: var(--r-sm);
  }

  .tree {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 0 8px 8px;
    display: flex;
    flex-direction: column;
    gap: 1px;
    /* La roulette sur l'arborescence ne doit pas enchaîner ailleurs. */
    overscroll-behavior: contain;
  }

  .sep {
    height: 5px;
    flex-shrink: 0;
    cursor: row-resize;
    /* La zone de saisie déborde sur les deux panneaux voisins : attraper la
       séparation ne demande pas de viser au pixel. */
    margin: -2px 0;
    position: relative;
    z-index: 2;
  }
  .sep::after {
    content: "";
    position: absolute;
    left: 8px;
    right: 8px;
    top: 2px;
    height: 1px;
    background: var(--border-strong);
    transition: background 120ms ease;
  }
  .sep:hover::after,
  .sep:active::after {
    background: var(--accent);
  }
  .sep.hidden {
    display: none;
  }

  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
    margin: 24px 4px;
    padding: 22px 12px;
    border-radius: var(--r-xl);
    border: 1px dashed var(--border-strong);
    color: var(--fg-3);
    font-size: 12.5px;
  }
  .empty:hover {
    color: var(--fg-1);
    border-color: var(--accent);
    background: var(--hover);
  }
</style>

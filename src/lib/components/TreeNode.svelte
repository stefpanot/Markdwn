<script lang="ts">
  import Icon from "./Icon.svelte";
  import { listDir, type DirEntryInfo } from "$lib/api";
  import { app, isPathUnder } from "$lib/state.svelte";
  import Self from "./TreeNode.svelte";

  interface Props {
    entry: DirEntryInfo;
    depth: number;
    onOpen: (path: string) => void;
  }
  let { entry, depth, onOpen }: Props = $props();

  let rowEl: HTMLButtonElement;

  /* L'expansion et les enfants chargés vivent dans le store, indexés par
     chemin : un refresh de l'arbre remplace les données sans refermer
     l'arborescence (item « expansion préservée » du cadrage K2.8). */
  const expanded = $derived(!!app.treeExpanded[entry.path]);
  const children = $derived(app.treeChildren[entry.path] ?? []);

  const isActive = $derived(!!app.active && app.active.path === entry.path);

  /** Charge les enfants si le cache du store ne les porte pas déjà. Les
     appels concurrents (effet de révélation + effet d'expansion) écrivent
     la même valeur : le store n'en garde qu'une. */
  async function ensureChildren() {
    if (app.treeChildren[entry.path] !== undefined) return;
    try {
      app.treeChildren[entry.path] = await listDir(entry.path);
    } catch {
      app.treeChildren[entry.path] = [];
    }
  }

  /* Révélation demandée depuis le fil d'Ariane ou le suivi du document
     actif : chaque dossier sur le chemin s'ouvre et charge ses enfants ; le
     nœud ciblé se défile au centre. Les enfants chargés en retard montent
     chacun leur tour et poursuivent la chaîne : c'est l'effet du niveau le
     plus profond qui scrolle. */
  $effect(() => {
    const target = app.revealPath;
    const tick = app.revealTick;
    if (!tick || !target) return;
    if (entry.is_dir && isPathUnder(target, entry.path)) {
      app.treeExpanded[entry.path] = true;
      void ensureChildren();
    }
    if (entry.path === target) {
      rowEl?.scrollIntoView({ block: "center" });
    }
  });

  // Dossier déplié (clic ou révélation) : ses enfants doivent exister.
  $effect(() => {
    if (entry.is_dir && expanded) void ensureChildren();
  });

  function toggle() {
    if (!entry.is_dir) {
      onOpen(entry.path);
      return;
    }
    app.treeExpanded[entry.path] = !expanded;
    // Le chargement des enfants dépliés est piloté par l'effet ci-dessus.
  }
</script>

<button
  class="row"
  class:active={isActive}
  style:padding-left="{8 + depth * 16}px"
  onclick={toggle}
  title={entry.path}
  bind:this={rowEl}
>
  {#if entry.is_dir}
    <span class="twisty" class:open={expanded}>
      <Icon name="chevron-right" size={13} width={1.6} />
    </span>
    <span class="ico folder"><Icon name="folder" size={15} width={1.4} /></span>
  {:else}
    <span class="twisty spacer"></span>
    <span class="ico" class:accent={isActive}><Icon name="file" size={14} width={1.4} /></span>
  {/if}
  <span class="name">{entry.name}</span>
</button>

{#if entry.is_dir && expanded}
  {#each children as child (child.path)}
    <Self entry={child} depth={depth + 1} {onOpen} />
  {/each}
{/if}

<style>
  .row {
    width: 100%;
    height: 27px;
    display: flex;
    align-items: center;
    gap: 7px;
    padding-right: 8px;
    border-radius: var(--r-md);
    font-size: 12.5px;
    font-weight: 450;
    color: var(--fg-2);
    text-align: left;
  }
  .row:hover {
    background: var(--hover);
  }
  .row.active {
    background: var(--accent-soft);
    box-shadow: inset 0 0 0 1px var(--accent-line);
    color: var(--accent-on);
    font-weight: 500;
  }
  .twisty {
    display: flex;
    width: 13px;
    flex-shrink: 0;
    color: var(--fg-2);
    transition: transform 120ms ease;
  }
  .twisty.open {
    transform: rotate(90deg);
  }
  .ico {
    display: flex;
    flex-shrink: 0;
    color: var(--fg-3);
  }
  .ico.folder {
    color: var(--accent);
  }
  .ico.accent {
    color: var(--accent-fg);
  }
  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>

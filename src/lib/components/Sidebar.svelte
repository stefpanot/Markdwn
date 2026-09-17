<script lang="ts">
  import Icon from "./Icon.svelte";
  import TreeNode from "./TreeNode.svelte";
  import Outline from "./Outline.svelte";
  import { app } from "$lib/state.svelte";

  interface Props {
    onOpenFolder: () => void;
    onOpenPath: (path: string) => void;
    onGoto: (line: number) => void;
    /** Le plan du document n'a qu'une seule place à la fois : en bas de la
        sidebar en Split/Zen, dans le rail « Sur cette page » en Lecture.
        La sidebar du mode Lecture passe donc withOutline à false. */
    withOutline?: boolean;
  }
  let { onOpenFolder, onOpenPath, onGoto, withOutline = true }: Props = $props();

  const folderName = $derived(
    app.folderPath ? app.folderPath.split(/[\\/]/).filter(Boolean).pop() : "",
  );
</script>

<aside class="sidebar">
  <div class="head">
    <span class="title">{folderName || "Aucun dossier"}</span>
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
    <Outline {onGoto} activeLine={app.cursorLine} />
  {/if}
</aside>

<style>
  .sidebar {
    width: var(--w-sidebar);
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    min-height: 0;
    border-right: 1px solid var(--border);
    background: color-mix(in oklab, var(--surface-editor) 40%, transparent);
  }

  .head {
    height: 38px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 6px;
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

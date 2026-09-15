<script lang="ts">
  import Icon from "./Icon.svelte";
  import { listDir, type DirEntryInfo } from "$lib/api";
  import { app } from "$lib/state.svelte";
  import Self from "./TreeNode.svelte";

  interface Props {
    entry: DirEntryInfo;
    depth: number;
    onOpen: (path: string) => void;
  }
  let { entry, depth, onOpen }: Props = $props();

  let expanded = $state(false);
  let children = $state<DirEntryInfo[]>([]);
  let loaded = false;

  const isActive = $derived(!!app.active && app.active.path === entry.path);

  async function toggle() {
    if (!entry.is_dir) {
      onOpen(entry.path);
      return;
    }
    expanded = !expanded;
    if (expanded && !loaded) {
      try {
        children = await listDir(entry.path);
        loaded = true;
      } catch {
        children = [];
      }
    }
  }
</script>

<button
  class="row"
  class:active={isActive}
  style:padding-left="{8 + depth * 16}px"
  onclick={toggle}
  title={entry.path}
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

<script lang="ts">
  import Icon from "./Icon.svelte";
  import ModeSwitch from "./ModeSwitch.svelte";
  import FormatButtons from "./FormatButtons.svelte";
  import ContextMenu from "./ContextMenu.svelte";
  import { app } from "$lib/state.svelte";
  import type { MenuItem } from "$lib/menu";
  import type { EditAction, FormatKind } from "$lib/formats";

  interface Props {
    onOpenFolder: () => void;
    onOpenFile: () => void;
    onSave: () => void;
    onFormat: (kind: FormatKind) => void;
    /** Couper / copier / coller / tout sélectionner (menu du bouton ☺). */
    onEditAction: (action: EditAction) => void;
  }
  let { onOpenFolder, onOpenFile, onSave, onFormat, onEditAction }: Props = $props();

  /* Menu d'actions d'édition, ouvert par le bouton smiley : équivalent du
     menu contextuel (le menu natif ne peut pas être déclenché par code). */
  let editMenu: { x: number; y: number } | null = $state(null);

  function openEditMenu(e: MouseEvent) {
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    editMenu = { x: r.right - 208, y: r.bottom + 6 };
  }

  const editItems: MenuItem[] = [
    { label: "Couper", keys: "Ctrl+X", run: () => onEditAction("cut") },
    { label: "Copier", keys: "Ctrl+C", run: () => onEditAction("copy") },
    { label: "Coller", keys: "Ctrl+V", run: () => onEditAction("paste") },
    {
      label: "Tout sélectionner",
      keys: "Ctrl+A",
      separatorBefore: true,
      run: () => onEditAction("selectAll"),
    },
  ];
</script>

<div class="bar">
  <button class="icon-btn" onclick={onOpenFolder} title="Ouvrir un dossier">
    <Icon name="folder" size={16} width={1.4} />
  </button>
  <button class="icon-btn" onclick={onOpenFile} title="Ouvrir un fichier — Ctrl+O">
    <Icon name="file" size={16} width={1.4} />
  </button>
  <button class="icon-btn" onclick={onSave} title="Enregistrer — Ctrl+S">
    <Icon name="save" size={16} width={1.4} />
  </button>

  <div class="divider"></div>

  <FormatButtons {onFormat} />

  <!-- L'espace vide de la barre sert aussi à déplacer la fenêtre. -->
  <div class="spacer" data-tauri-drag-region></div>

  <button
    class="chip"
    class:on={app.syncScroll}
    onclick={() => (app.syncScroll = !app.syncScroll)}
    aria-pressed={app.syncScroll}
    title="Scroll synchronisé entre source et aperçu"
  >
    <Icon name="sync" size={14} />
    <span>Sync</span>
  </button>

  <ModeSwitch />

  <div class="divider"></div>

  <button
    class="icon-btn"
    onclick={() => (app.theme = app.theme === "dark" ? "light" : "dark")}
    title="Thème clair / sombre"
  >
    <Icon name={app.theme === "dark" ? "moon" : "sun"} size={16} />
  </button>
  <button
    class="icon-btn"
    onclick={() => (app.settingsOpen = !app.settingsOpen)}
    aria-pressed={app.settingsOpen}
    title="Paramètres — Ctrl+,"
  >
    <Icon name="settings" size={16} width={1.4} />
  </button>
  <button class="icon-btn" onclick={openEditMenu} title="Actions d'édition — comme le clic droit">
    <Icon name="smile" size={16} width={1.4} />
  </button>
</div>

{#if editMenu}
  <ContextMenu x={editMenu.x} y={editMenu.y} items={editItems} onClose={() => (editMenu = null)} />
{/if}

<style>
  .bar {
    height: var(--h-bar);
    flex-shrink: 0;
    display: flex;
    align-items: center;
    padding: 0 10px 0 14px;
    gap: 2px;
    border-bottom: 1px solid var(--border);
  }
  .spacer {
    flex: 1;
  }
  .chip {
    height: 26px;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 9px;
    border-radius: var(--r-md);
    font-size: 11.5px;
    font-weight: 500;
    color: var(--fg-2);
    margin-right: 8px;
    flex-shrink: 0;
  }
  .chip:hover {
    background: var(--hover);
  }
  .chip.on {
    background: var(--accent-soft);
    color: var(--accent-fg);
  }
</style>

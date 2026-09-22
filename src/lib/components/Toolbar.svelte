<script lang="ts">
  import Icon from "./Icon.svelte";
  import ModeSwitch from "./ModeSwitch.svelte";
  import { app } from "$lib/state.svelte";

  interface Props {
    onOpenFolder: () => void;
    onOpenFile: () => void;
    onSave: () => void;
    onFormat: (
      kind:
        | "bold"
        | "italic"
        | "code"
        | "codeblock"
        | "strike"
        | "mark"
        | "link"
        | "list"
        | "quote",
    ) => void;
  }
  let { onOpenFolder, onOpenFile, onSave, onFormat }: Props = $props();
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

  <button class="icon-btn" onclick={() => onFormat("bold")} title="Gras">
    <Icon name="bold" size={16} width={1.6} />
  </button>
  <button class="icon-btn" onclick={() => onFormat("italic")} title="Italique">
    <Icon name="italic" size={16} />
  </button>
  <button class="icon-btn" onclick={() => onFormat("link")} title="Lien">
    <Icon name="link" size={16} />
  </button>
  <button class="icon-btn" onclick={() => onFormat("code")} title="Code inline">
    <Icon name="code" size={16} />
  </button>
  <button class="icon-btn" onclick={() => onFormat("codeblock")} title="Bloc de code — ```langage">
    <Icon name="code-block" size={16} width={1.4} />
  </button>
  <button class="icon-btn" onclick={() => onFormat("strike")} title="Barré">
    <Icon name="strike" size={16} width={1.4} />
  </button>
  <button class="icon-btn" onclick={() => onFormat("mark")} title="Surligner">
    <Icon name="mark" size={16} width={1.4} />
  </button>
  <button class="icon-btn" onclick={() => onFormat("list")} title="Liste">
    <Icon name="list" size={16} />
  </button>
  <button class="icon-btn" onclick={() => onFormat("quote")} title="Citation">
    <Icon name="quote" size={16} />
  </button>

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
</div>

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

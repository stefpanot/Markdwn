<script lang="ts">
  import Icon from "./Icon.svelte";
  import { app } from "$lib/state.svelte";

  /* Affiché quand aucun document n'est ouvert. Pas d'onglet fantôme, et pas de
     « fichiers récents » : ils demandent la configuration persistée, qui
     n'existe pas encore. Mieux vaut ne rien promettre. */
  interface Props {
    onOpenFolder: () => void;
    onOpenFile: () => void;
    onNew: () => void;
    onDiscover: () => void;
  }
  let { onOpenFolder, onOpenFile, onNew, onDiscover }: Props = $props();

  const folderName = $derived(
    app.folderPath ? app.folderPath.split(/[\\/]/).filter(Boolean).pop() : "",
  );

  const shortcuts: [string, string][] = [
    ["Ctrl+1 / 2 / 3", "Lecture · Split · Zen"],
    ["Ctrl+B", "Barre de dossiers"],
    ["Alt+↑ / Alt+↓", "Fichier précédent / suivant"],
    ["Ctrl+S", "Enregistrer"],
    ["Ctrl+W", "Fermer l'onglet"],
  ];
</script>

<div class="welcome">
  <div class="card">
    <div class="mark" aria-hidden="true">
      <Icon name="wordmark" size={24} width={1.8} />
    </div>

    <h1>Markdwn</h1>
    <p class="tagline">Éditeur et lecteur Markdown natif.</p>

    <div class="actions">
      <button class="action primary" onclick={onOpenFolder}>
        <Icon name="folder" size={16} width={1.4} />
        <span class="label">Ouvrir un dossier</span>
        <span class="hint">arborescence et navigation</span>
      </button>

      <button class="action" onclick={onOpenFile}>
        <Icon name="file" size={16} width={1.4} />
        <span class="label">Ouvrir un fichier</span>
        <span class="kbd">Ctrl+O</span>
      </button>

      <button class="action" onclick={onNew}>
        <Icon name="plus" size={16} width={1.5} />
        <span class="label">Nouveau document</span>
        <span class="kbd">Ctrl+N</span>
      </button>

      <button class="action" onclick={onDiscover}>
        <Icon name="view-preview" size={16} width={1.4} />
        <span class="label">Découvrir Markdwn</span>
        <span class="hint">ouvre un document de démonstration</span>
      </button>
    </div>

    {#if folderName && app.sidebarVisible}
      <p class="context">
        <Icon name="folder" size={13} width={1.4} />
        <span><strong>{folderName}</strong> est ouvert — choisis un fichier dans la barre latérale.</span>
      </p>
    {:else if folderName}
      <!-- Conseiller « choisis un fichier dans la barre latérale » quand elle
           est masquée serait un conseil impossible à suivre. -->
      <button class="context clickable" onclick={() => (app.sidebarVisible = true)}>
        <Icon name="sidebar" size={13} width={1.4} />
        <span><strong>{folderName}</strong> est ouvert — afficher la barre de dossiers</span>
        <span class="kbd">Ctrl+B</span>
      </button>
    {/if}

    <div class="rule"></div>

    <dl class="shortcuts">
      {#each shortcuts as [keys, what] (keys)}
        <dt><span class="kbd">{keys}</span></dt>
        <dd>{what}</dd>
      {/each}
    </dl>
  </div>
</div>

<style>
  .welcome {
    flex: 1;
    min-height: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    overflow-y: auto;
    padding: 32px;
  }

  .card {
    width: 100%;
    max-width: 420px;
  }

  .mark {
    width: 42px;
    height: 42px;
    border-radius: 11px;
    background: linear-gradient(150deg, var(--accent), color-mix(in oklab, var(--accent) 78%, black));
    color: #fff;
    display: flex;
    align-items: center;
    justify-content: center;
    box-shadow: 0 2px 10px rgb(0 0 0 / 0.28);
    margin-bottom: 20px;
  }

  h1 {
    margin: 0 0 4px;
    font-size: 22px;
    font-weight: 650;
    letter-spacing: -0.02em;
    color: var(--fg-bright);
  }
  .tagline {
    margin: 0 0 26px;
    font-size: 13.5px;
    color: var(--fg-2);
  }

  .actions {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .action {
    display: flex;
    align-items: center;
    gap: 11px;
    height: 42px;
    padding: 0 13px;
    border-radius: var(--r-xl);
    color: var(--fg-2);
    text-align: left;
  }
  .action:hover {
    background: var(--hover);
    color: var(--fg-1);
  }
  .action.primary {
    background: var(--accent-soft);
    box-shadow: inset 0 0 0 1px var(--accent-line);
    color: var(--accent-on);
  }
  .action.primary:hover {
    background: color-mix(in oklab, var(--accent-soft) 70%, var(--accent));
  }
  .label {
    font-size: 13.5px;
    font-weight: 500;
    color: inherit;
  }
  .hint {
    margin-left: auto;
    font-size: 11.5px;
    color: var(--fg-3);
  }
  .action .kbd {
    margin-left: auto;
  }

  .context {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 22px 0 0;
    padding: 10px 13px;
    border-radius: var(--r-xl);
    background: var(--chip);
    box-shadow: inset 0 0 0 1px var(--border);
    font-size: 12.5px;
    color: var(--fg-2);
  }
  .context strong {
    font-weight: 600;
    color: var(--fg-1);
  }
  .context.clickable {
    width: 100%;
    text-align: left;
  }
  .context.clickable:hover {
    background: var(--hover);
    color: var(--fg-1);
  }
  .context .kbd {
    margin-left: auto;
  }

  .rule {
    height: 1px;
    background: var(--border-strong);
    margin: 26px 0 20px;
  }

  .shortcuts {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 9px 14px;
    margin: 0;
    align-items: center;
  }
  .shortcuts dt {
    margin: 0;
  }
  .shortcuts dd {
    margin: 0;
    font-size: 12.5px;
    color: var(--fg-3);
  }
</style>

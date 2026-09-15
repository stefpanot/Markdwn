<script lang="ts">
  import Icon from "./Icon.svelte";
  import { app } from "$lib/state.svelte";

  /* Panneau modal, et non onglet : le modèle d'onglets ne contient que des
     documents, et les réglages doivent être atteignables depuis le Zen aussi. */
  interface Props {
    onClose: () => void;
    onReveal: () => void;
  }
  let { onClose, onReveal }: Props = $props();

  let panel: HTMLDivElement;

  const sizes = [15, 17, 19, 21];

  // Le dossier, pas le fichier : c'est ce qu'on veut révéler.
  const configDir = $derived(app.configPath.replace(/[\\/][^\\/]+$/, ""));
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && onClose()} />

<div class="scrim" role="presentation" onpointerdown={(e) => e.target === e.currentTarget && onClose()}>
  <div class="panel" bind:this={panel} role="dialog" aria-modal="true" aria-label="Paramètres">
    <header>
      <h2>Paramètres</h2>
      <button class="icon-btn" onclick={onClose} aria-label="Fermer">
        <Icon name="close" size={14} />
      </button>
    </header>

    <div class="body">
      <section>
        <h3>Apparence</h3>
        <div class="row">
          <span class="lab">Thème</span>
          <div class="seg">
            <button class:on={app.theme === "dark"} onclick={() => (app.theme = "dark")}>
              Sombre
            </button>
            <button class:on={app.theme === "light"} onclick={() => (app.theme = "light")}>
              Clair
            </button>
          </div>
        </div>
      </section>

      <section>
        <h3>Lecture</h3>
        <div class="row">
          <span class="lab">Largeur</span>
          <div class="seg">
            <button
              class:on={app.readingWidth === "centered"}
              onclick={() => (app.readingWidth = "centered")}
            >
              Colonne centrée
            </button>
            <button
              class:on={app.readingWidth === "full"}
              onclick={() => (app.readingWidth = "full")}
            >
              Pleine largeur
            </button>
          </div>
        </div>
        <div class="row">
          <span class="lab">Taille du texte</span>
          <div class="seg">
            {#each sizes as s (s)}
              <button class:on={app.readingSize === s} onclick={() => (app.readingSize = s)}>
                {s}
              </button>
            {/each}
          </div>
        </div>
      </section>

      <section>
        <h3>Espace de travail</h3>
        <div class="row">
          <span class="lab">
            Mode au démarrage
            <span class="sub">le dernier mode utilisé est mémorisé</span>
          </span>
          <div class="seg">
            <button class:on={app.mode === "read"} onclick={() => (app.mode = "read")}>
              Lecture
            </button>
            <button class:on={app.mode === "split"} onclick={() => (app.mode = "split")}>
              Split
            </button>
            <button class:on={app.mode === "zen"} onclick={() => (app.mode = "zen")}>Zen</button>
          </div>
        </div>

        <label class="check">
          <input type="checkbox" bind:checked={app.restoreLastFolder} />
          <span class="lab">
            Rouvrir le dernier dossier au démarrage
            {#if app.folderPath}
              <span class="sub">{app.folderPath}</span>
            {/if}
          </span>
        </label>

        <label class="check">
          <input type="checkbox" bind:checked={app.sidebarVisible} />
          <span class="lab">Afficher la barre de dossiers</span>
        </label>

        <label class="check">
          <input type="checkbox" bind:checked={app.syncScroll} />
          <span class="lab">
            Scroll synchronisé en mode Split
            <span class="sub">entre la source et l'aperçu</span>
          </span>
        </label>
      </section>
    </div>

    <footer>
      <button class="link" onclick={() => app.resetSettings()}>Réinitialiser</button>
      <div class="spacer"></div>
      {#if app.version}
        <span class="version" title="Version lue depuis Cargo.toml, seule source">
          v{app.version}
        </span>
      {/if}
      {#if app.configPath}
        <button class="link reveal" onclick={onReveal} title={app.configPath}>
          <Icon name="folder" size={13} width={1.4} />
          <span>Révéler le fichier</span>
        </button>
      {/if}
    </footer>

    {#if configDir}
      <p class="where">Les réglages sont enregistrés dans <code>{configDir}</code>, hors du dossier d'installation : une mise à jour ne les écrase pas.</p>
    {/if}
  </div>
</div>

<style>
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 60;
    background: rgb(10 11 14 / 0.62);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 40px 24px;
  }

  .panel {
    width: 100%;
    max-width: 560px;
    max-height: 100%;
    display: flex;
    flex-direction: column;
    border-radius: 14px;
    background: var(--surface-raised);
    box-shadow:
      0 0 0 1px var(--border-strong),
      0 28px 80px rgb(0 0 0 / 0.62);
    overflow: hidden;
  }

  header {
    display: flex;
    align-items: center;
    height: 52px;
    padding: 0 12px 0 20px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  h2 {
    flex: 1;
    margin: 0;
    font-size: 15px;
    font-weight: 600;
    letter-spacing: -0.012em;
    color: var(--fg-bright);
  }

  .body {
    padding: 6px 20px 16px;
    overflow-y: auto;
    min-height: 0;
  }

  section {
    padding: 16px 0;
    border-bottom: 1px solid var(--border);
  }
  section:last-child {
    border-bottom: 0;
  }
  h3 {
    margin: 0 0 12px;
    font-size: 10.5px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--fg-3);
  }

  .row {
    display: flex;
    align-items: center;
    gap: 16px;
    margin-bottom: 10px;
  }
  .row:last-child {
    margin-bottom: 0;
  }
  .lab {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: 13px;
    color: var(--fg-1);
  }
  .sub {
    font-size: 11.5px;
    color: var(--fg-3);
    word-break: break-all;
  }

  .seg {
    display: flex;
    align-items: center;
    padding: 2px;
    gap: 2px;
    border-radius: var(--r-lg);
    background: var(--chip);
    box-shadow: inset 0 0 0 1px var(--border);
    flex-shrink: 0;
  }
  .seg button {
    height: 24px;
    padding: 0 10px;
    border-radius: var(--r-sm);
    font-size: 12px;
    font-weight: 500;
    color: var(--fg-2);
  }
  .seg button:hover {
    color: var(--fg-1);
  }
  .seg button.on {
    background: var(--accent-soft);
    color: var(--accent-fg);
    box-shadow: 0 1px 2px rgb(0 0 0 / 0.2);
  }

  .check {
    display: flex;
    align-items: flex-start;
    gap: 11px;
    padding: 7px 0;
  }
  .check input {
    margin: 2px 0 0;
    accent-color: var(--accent);
    width: 15px;
    height: 15px;
    flex-shrink: 0;
  }

  footer {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 20px;
    border-top: 1px solid var(--border);
    flex-shrink: 0;
  }
  .spacer {
    flex: 1;
  }
  .link {
    display: flex;
    align-items: center;
    gap: 7px;
    font-size: 12.5px;
    color: var(--fg-2);
    padding: 5px 9px;
    border-radius: var(--r-md);
  }
  .link:hover {
    background: var(--hover);
    color: var(--fg-1);
  }
  .reveal {
    color: var(--accent-fg);
  }
  .version {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--fg-3);
  }

  .where {
    margin: 0;
    padding: 0 20px 16px;
    font-size: 11.5px;
    line-height: 1.55;
    color: var(--fg-3);
    flex-shrink: 0;
  }
  .where code {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--fg-2);
    word-break: break-all;
  }
</style>

<script lang="ts">
  import Icon from "./Icon.svelte";
  import ModeSwitch from "./ModeSwitch.svelte";
  import { app } from "$lib/state.svelte";

  /* La barre du mode Lecture ne propose pas d'outils d'écriture : historique
     entre documents, fil d'Ariane, et réglages de typo à portée de clic. */
  interface Props {
    canBack: boolean;
    canForward: boolean;
    onBack: () => void;
    onForward: () => void;
    /** Fichier suivant / précédent dans le dossier, dans l'ordre de l'arbre. */
    canPrevFile: boolean;
    canNextFile: boolean;
    onPrevFile: () => void;
    onNextFile: () => void;
  }
  let {
    canBack,
    canForward,
    onBack,
    onForward,
    canPrevFile,
    canNextFile,
    onPrevFile,
    onNextFile,
  }: Props = $props();

  const crumbs = $derived.by(() => {
    const doc = app.active;
    if (!doc) return [];
    if (!doc.path) return [doc.name];
    return doc.path.split(/[\\/]/).filter(Boolean).slice(-2);
  });

  const sizes = [15, 17, 19, 21];
  function step(dir: -1 | 1) {
    const i = sizes.indexOf(app.readingSize);
    const next = sizes[Math.min(Math.max((i < 0 ? 1 : i) + dir, 0), sizes.length - 1)];
    app.readingSize = next;
  }
</script>

<div class="bar">
  <!-- Toute la navigation est regroupée à gauche : historique entre documents
       puis fichier précédent/suivant. Le fil d'Ariane, de longueur variable,
       vient APRÈS : les boutons ne bougent plus quand le chemin change. -->
  <div class="history">
    <button class="icon-btn" onclick={onBack} disabled={!canBack} title="Reculer dans l'historique">
      <Icon name="arrow-left" size={16} width={1.6} />
    </button>
    <button
      class="icon-btn"
      onclick={onForward}
      disabled={!canForward}
      title="Avancer dans l'historique"
    >
      <Icon name="arrow-right" size={16} width={1.6} />
    </button>
  </div>

  <!-- Parcourir le dossier sans quitter la lecture. -->
  <div class="history">
    <button
      class="icon-btn"
      onclick={onPrevFile}
      disabled={!canPrevFile}
      title="Fichier précédent du dossier — Alt+↑"
    >
      <Icon name="chevron-up" size={16} width={1.6} />
    </button>
    <button
      class="icon-btn"
      onclick={onNextFile}
      disabled={!canNextFile}
      title="Fichier suivant du dossier — Alt+↓"
    >
      <Icon name="chevron-down" size={16} width={1.6} />
    </button>
  </div>

  <div class="divider"></div>

  <div class="crumbs">
    <span class="crumb-icon"><Icon name="folder" size={14} width={1.4} /></span>
    {#each crumbs as part, i (i)}
      {#if i > 0}
        <span class="sep"><Icon name="chevron-right" size={12} width={1.6} /></span>
      {/if}
      <span class="crumb" class:last={i === crumbs.length - 1}>{part}</span>
    {/each}
  </div>

  <!-- L'espace vide sert aussi à déplacer la fenêtre. -->
  <div class="spacer" data-tauri-drag-region></div>

  <div class="steppers" role="group" aria-label="Taille du texte">
    <button class="step" onclick={() => step(-1)} title="Réduire le texte — {app.readingSize} px">
      <span class="a sm">A</span>
    </button>
    <button class="step" onclick={() => step(1)} title="Agrandir le texte — {app.readingSize} px">
      <span class="a lg">A</span>
    </button>
  </div>

  <!-- Largeur de lecture : colonne mesurée pour la prose, pleine largeur pour
       les tableaux et le code. -->
  <div class="steppers" role="group" aria-label="Largeur de lecture">
    <button
      class="step wide"
      class:on={app.readingWidth === "centered"}
      onclick={() => (app.readingWidth = "centered")}
      aria-pressed={app.readingWidth === "centered"}
      title="Colonne centrée — mesure d'environ 68 caractères"
    >
      <Icon name="width-centered" size={15} width={1.4} />
    </button>
    <button
      class="step wide"
      class:on={app.readingWidth === "full"}
      onclick={() => (app.readingWidth = "full")}
      aria-pressed={app.readingWidth === "full"}
      title="Pleine largeur — occupe tout l'espace disponible"
    >
      <Icon name="width-full" size={15} width={1.4} />
    </button>
  </div>

  <div class="divider"></div>

  <ModeSwitch />

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
    height: var(--h-readingbar);
    flex-shrink: 0;
    display: flex;
    align-items: center;
    padding: 0 12px 0 14px;
    gap: 10px;
    border-bottom: 1px solid var(--border);
  }
  .history {
    display: flex;
    gap: 2px;
  }
  .crumbs {
    display: flex;
    align-items: center;
    gap: 7px;
    min-width: 0;
    overflow: hidden;
  }
  .crumb-icon,
  .sep {
    display: flex;
    color: var(--fg-3);
    flex-shrink: 0;
  }
  .crumb {
    font-size: 12.5px;
    color: var(--fg-2);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .crumb.last {
    font-weight: 500;
    color: var(--fg-1);
  }
  .spacer {
    flex: 1;
  }
  .steppers {
    height: 26px;
    display: flex;
    align-items: center;
    padding: 2px;
    gap: 1px;
    border-radius: var(--r-lg);
    background: var(--chip);
    box-shadow: inset 0 0 0 1px var(--border);
  }
  .step {
    width: 26px;
    height: 22px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--r-sm);
    color: var(--fg-2);
  }
  .step:hover {
    background: var(--hover);
    color: var(--fg-1);
  }
  .a {
    font-weight: 600;
    line-height: 1;
  }
  .a.sm {
    font-size: 11px;
  }
  .a.lg {
    font-size: 14px;
  }
  .step.wide {
    width: 30px;
  }
  .step.on {
    background: var(--accent-soft);
    color: var(--accent-fg);
    box-shadow: 0 1px 2px rgb(0 0 0 / 0.2);
  }
</style>

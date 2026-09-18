<script lang="ts">
  import Icon from "./Icon.svelte";

  interface Props {
    query: string;
    replacement: string;
    caseSensitive: boolean;
    wholeWord: boolean;
    /** Indice de l'occurrence courante et nombre total. */
    current: number;
    total: number;
    onQueryChange: (value: string) => void;
    onReplacementChange: (value: string) => void;
    onToggleCase: () => void;
    onToggleWord: () => void;
    onNext: () => void;
    onPrev: () => void;
    onReplace: () => void;
    onReplaceAll: () => void;
    onClose: () => void;
  }
  let {
    query,
    replacement,
    caseSensitive,
    wholeWord,
    current,
    total,
    onQueryChange,
    onReplacementChange,
    onToggleCase,
    onToggleWord,
    onNext,
    onPrev,
    onReplace,
    onReplaceAll,
    onClose,
  }: Props = $props();

  let queryInput = $state<HTMLInputElement | undefined>(undefined);
  let replaceInput = $state<HTMLInputElement | undefined>(undefined);

  // À l'ouverture : focus direct sur la saisie, contenu sélectionné pour
  // retaper une recherche sans effacer la précédente à la main.
  $effect(() => {
    queryInput?.focus();
    queryInput?.select();
  });

  /** Re-focus la recherche (Ctrl+H quand le panneau est déjà ouvert). */
  export function focusQuery() {
    queryInput?.focus();
    queryInput?.select();
  }

  /* Cliquer un bouton vole le focus : le rendre au champ concerné pour que la
     frappe continue sans reclic. */
  function act(run: () => void, back: HTMLInputElement | undefined) {
    run();
    back?.focus();
  }

  function onFindKeydown(e: KeyboardEvent) {
    if (e.key === "Enter") {
      e.preventDefault();
      if (e.shiftKey) onPrev();
      else onNext();
      queryInput?.focus();
    } else if (e.key === "Escape") {
      onClose();
    }
  }

  function onReplaceKeydown(e: KeyboardEvent) {
    if (e.key === "Enter") {
      e.preventDefault();
      onReplace();
      replaceInput?.focus();
    } else if (e.key === "Escape") {
      onClose();
    }
  }
</script>

<div class="find-panel" role="search">
  <div class="row">
    <span class="lead"><Icon name="search" size={14} /></span>
    <input
      bind:this={queryInput}
      class="field"
      type="text"
      placeholder="Rechercher"
      value={query}
      oninput={(e) => onQueryChange(e.currentTarget.value)}
      onkeydown={onFindKeydown}
      spellcheck="false"
    />
    <button
      class="toggle"
      aria-pressed={caseSensitive}
      title="Respecter la casse (Aa)"
      onclick={onToggleCase}>Aa</button
    >
    <button
      class="toggle"
      aria-pressed={wholeWord}
      title="Mot entier"
      onclick={onToggleWord}>ab|</button
    >
    <span class="count" class:dim={query !== "" && total === 0}>
      {query === "" ? "" : total === 0 ? "Aucun résultat" : `${current + 1} / ${total}`}
    </span>
    <button
      class="icon-btn"
      disabled={total === 0}
      title="Occurrence précédente — Maj+Entrée"
      onclick={() => act(onPrev, queryInput)}
    >
      <Icon name="chevron-up" size={14} />
    </button>
    <button
      class="icon-btn"
      disabled={total === 0}
      title="Occurrence suivante — Entrée"
      onclick={() => act(onNext, queryInput)}
    >
      <Icon name="chevron-down" size={14} />
    </button>
    <button class="icon-btn" title="Fermer — Échap" onclick={onClose}>
      <Icon name="close" size={13} />
    </button>
  </div>
  <div class="row">
    <span class="lead"><Icon name="replace" size={14} /></span>
    <input
      bind:this={replaceInput}
      class="field"
      type="text"
      placeholder="Remplacer"
      value={replacement}
      oninput={(e) => onReplacementChange(e.currentTarget.value)}
      onkeydown={onReplaceKeydown}
      spellcheck="false"
    />
    <button
      class="icon-btn"
      disabled={total === 0}
      title="Remplacer l'occurrence courante — Entrée"
      onclick={() => act(onReplace, replaceInput)}
    >
      <Icon name="replace" size={14} />
    </button>
    <button
      class="icon-btn"
      disabled={total === 0}
      title="Remplacer toutes les occurrences"
      onclick={() => act(onReplaceAll, replaceInput)}
    >
      <Icon name="replace-all" size={14} />
    </button>
  </div>
</div>

<style>
  .find-panel {
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    gap: 5px;
    padding: 7px 10px 8px;
    background: var(--surface-sunken);
    border-bottom: 1px solid var(--border);
  }
  .row {
    display: flex;
    align-items: center;
    gap: 5px;
  }
  .lead {
    width: 22px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--fg-3);
    flex-shrink: 0;
  }
  .field {
    height: var(--h-control);
    flex: 1;
    max-width: 340px;
    padding: 0 8px;
    font: inherit;
    font-size: 12.5px;
    color: var(--fg-1);
    background: var(--chip);
    border: 1px solid var(--border-strong);
    border-radius: var(--r-md);
    outline: none;
  }
  .field::placeholder {
    color: var(--fg-4);
  }
  .field:focus {
    border-color: var(--accent-line);
    background: var(--surface-raised);
  }
  .toggle {
    height: 22px;
    padding: 0 7px;
    border-radius: var(--r-sm);
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--fg-2);
    flex-shrink: 0;
  }
  .toggle:hover {
    background: var(--hover);
    color: var(--fg-1);
  }
  .toggle[aria-pressed="true"] {
    background: var(--accent-soft);
    color: var(--accent-fg);
  }
  .count {
    min-width: 86px;
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--fg-3);
    text-align: right;
    flex-shrink: 0;
  }
  .count.dim {
    color: var(--accent-2-fg);
  }
</style>

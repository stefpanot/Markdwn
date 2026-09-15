<script lang="ts">
  import Icon from "./Icon.svelte";
  import { app, type Mode } from "$lib/state.svelte";

  /* Le sélecteur de mode est le cœur de l'interface, pas un bouton de toolbar :
     lire, écrire+vérifier, écrire sont trois postures de rang égal. */
  const modes: { id: Mode; icon: string; label: string; keys: string }[] = [
    { id: "read", icon: "view-preview", label: "Lecture", keys: "Ctrl+1" },
    { id: "split", icon: "view-split", label: "Split", keys: "Ctrl+2" },
    { id: "zen", icon: "view-editor", label: "Zen", keys: "Ctrl+3" },
  ];
</script>

<div class="switch" role="group" aria-label="Mode d'affichage">
  {#each modes as m (m.id)}
    <button
      class="seg"
      class:on={app.mode === m.id}
      onclick={() => (app.mode = m.id)}
      title="{m.label} — {m.keys}"
      aria-pressed={app.mode === m.id}
    >
      <Icon name={m.icon} size={15} width={1.4} />
    </button>
  {/each}
</div>

<style>
  .switch {
    height: 26px;
    display: flex;
    align-items: center;
    padding: 2px;
    gap: 2px;
    border-radius: var(--r-lg);
    background: var(--chip);
    box-shadow: inset 0 0 0 1px var(--border);
    flex-shrink: 0;
  }
  .seg {
    width: 30px;
    height: 22px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--r-sm);
    color: var(--fg-3);
  }
  .seg:hover {
    color: var(--fg-1);
  }
  .seg.on {
    background: var(--accent-soft);
    color: var(--accent-fg);
    box-shadow: 0 1px 2px rgb(0 0 0 / 0.2);
  }
</style>

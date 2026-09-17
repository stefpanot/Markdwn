<script lang="ts">
  import Icon from "./Icon.svelte";
  import { app } from "$lib/state.svelte";

  /* Palette de commandes, ouverte sur Ctrl+K. Deux groupes : les fichiers du
     dossier ouvert (↵ les ouvre, Ctrl+↵ force un nouvel onglet) puis les
     commandes de l'app. Un « > » initial restreint aux commandes. */
  export interface PaletteCommand {
    id: string;
    label: string;
    icon: string;
    keys?: string;
    /** Mots supplémentaires participants au filtre, sans être affichés. */
    keywords?: string;
    run: () => void;
  }

  interface Props {
    commands: PaletteCommand[];
    onClose: () => void;
    onOpenPath: (path: string, newTab: boolean) => void;
  }
  let { commands, onClose, onOpenPath }: Props = $props();

  let query = $state("");
  let sel = $state(0);
  let list: HTMLDivElement;
  let input: HTMLInputElement;

  // Le composant n'est monté que tant que la palette est ouverte : tout
  // l'état repart à zéro à chaque ouverture.
  $effect(() => {
    input?.focus();
  });

  /* ---------- correspondance floue : sous-suite, bonus au segment contigu -- */
  type Range = [number, number];

  function matchRanges(text: string, needle: string): Range[] | null {
    if (!needle) return [];
    const t = text.toLowerCase();
    const i = t.indexOf(needle);
    if (i >= 0) return [[i, i + needle.length]];
    const ranges: Range[] = [];
    let from = 0;
    for (const ch of needle) {
      const j = t.indexOf(ch, from);
      if (j < 0) return null;
      ranges.push([j, j + 1]);
      from = j + 1;
    }
    return ranges;
  }

  function merge(ranges: Range[]): Range[] {
    const out: Range[] = [];
    for (const r of ranges) {
      const last = out[out.length - 1];
      if (last && r[0] <= last[1]) last[1] = Math.max(last[1], r[1]);
      else out.push([...r] as Range);
    }
    return out;
  }

  /** Découpe le texte en segments à rendre, hit marquant la correspondance. */
  function segments(text: string, ranges: Range[]): { t: string; hit: boolean }[] {
    if (ranges.length === 0) return [{ t: text, hit: false }];
    const out: { t: string; hit: boolean }[] = [];
    let at = 0;
    for (const [a, b] of ranges) {
      if (a > at) out.push({ t: text.slice(at, a), hit: false });
      out.push({ t: text.slice(a, b), hit: true });
      at = b;
    }
    if (at < text.length) out.push({ t: text.slice(at), hit: false });
    return out;
  }

  /* ---------- requête ---------- */
  const commandsOnly = $derived(query.startsWith(">"));
  const needle = $derived((commandsOnly ? query.slice(1) : query).trim().toLowerCase());

  /* ---------- fichiers du dossier ouvert ---------- */
  interface FileHit {
    kind: "file";
    path: string;
    name: string;
    dir: string;
    score: number;
    ranges: Range[];
  }

  const files = $derived.by((): FileHit[] => {
    if (commandsOnly) return [];
    const root = app.folderPath;
    return app.folderFiles
      .map((path): FileHit | null => {
        const rel =
          root && path.startsWith(root + "\\") || root && path.startsWith(root + "/")
            ? path.slice(root.length + 1)
            : path;
        const parts = rel.split(/[\\/]/);
        const name = parts[parts.length - 1];
        const dir = parts.length > 1 ? parts.slice(0, -1).join("/") + "/" : "";
        const m = matchRanges(rel, needle);
        if (!m) return null;
        // La surbrillance ne porte que sur le nom : on reclampe les bornes
        // du nom dans rel vers des offsets relatifs au nom.
        const off = rel.length - name.length;
        const ranges: Range[] = merge(m)
          .filter((r) => r[1] > off)
          .map(([a, b]) => [Math.max(a, off) - off, b - off]);
        const contiguous = m.length === 1;
        const at = contiguous ? m[0][0] : 1000 + m[0][0];
        return { kind: "file", path, name, dir, score: at, ranges };
      })
      .filter((h): h is FileHit => h !== null)
      .sort((a, b) => a.score - b.score || a.path.localeCompare(b.path))
      .slice(0, 20);
  });

  /* ---------- commandes ---------- */
  interface CommandHit {
    kind: "command";
    cmd: PaletteCommand;
    ranges: Range[];
  }

  const commandHits = $derived.by((): CommandHit[] =>
    commands
      .map((cmd): CommandHit | null => {
        const m = matchRanges(cmd.label, needle) ??
          (cmd.keywords ? matchRanges(cmd.keywords, needle) : null);
        if (!m) return null;
        return { kind: "command", cmd, ranges: m };
      })
      .filter((h): h is CommandHit => h !== null)
      .slice(0, 12),
  );

  /* ---------- liste aplatie : fichiers d'abord, comme dans la maquette ----- */
  const hits = $derived<(FileHit | CommandHit)[]>([...files, ...commandHits]);

  $effect(() => {
    // Réinitialiser la sélection à chaque changement de filtre.
    needle;
    commandsOnly;
    sel = 0;
  });

  $effect(() => {
    // Garder la ligne sélectionnée visible au clavier.
    sel;
    list?.querySelector(`[data-idx="${sel}"]`)?.scrollIntoView({ block: "nearest" });
  });

  function run(hit: FileHit | CommandHit, newTab: boolean) {
    onClose();
    if (hit.kind === "file") onOpenPath(hit.path, newTab);
    else hit.cmd.run();
  }

  function onInputKeydown(e: KeyboardEvent) {
    if (e.key === "ArrowDown" && hits.length > 0) {
      sel = (sel + 1) % hits.length;
      e.preventDefault();
    } else if (e.key === "ArrowUp" && hits.length > 0) {
      sel = (sel - 1 + hits.length) % hits.length;
      e.preventDefault();
    } else if (e.key === "Enter" && hits[sel]) {
      run(hits[sel], e.ctrlKey || e.metaKey);
      e.preventDefault();
    } else if (e.key === "Escape") {
      onClose();
      e.preventDefault();
    }
  }
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && onClose()} />

<div
  class="scrim"
  role="presentation"
  onpointerdown={(e) => e.target === e.currentTarget && onClose()}
>
  <div class="palette" role="dialog" aria-modal="true" aria-label="Palette de commandes">
    <!-- ---------- saisie ---------- -->
    <div class="input-row">
      <Icon name="search" size={17} />
      <input
        bind:this={input}
        bind:value={query}
        onkeydown={onInputKeydown}
        placeholder={commandsOnly ? "Commande…" : "Fichier ou commande…"}
        spellcheck="false"
        aria-label="Rechercher"
      />
      {#if hits.length > 0}
        <span class="count">
          {hits.length} résultat{hits.length > 1 ? "s" : ""}
        </span>
      {/if}
      <span class="kbd">Esc</span>
    </div>

    <!-- ---------- résultats ---------- -->
    <div class="results" bind:this={list}>
      {#if files.length > 0}
        <div class="group">Fichiers</div>
        {#each files as hit, i (hit.path)}
          <button
            class="row"
            class:on={sel === i}
            data-idx={i}
            onpointermove={() => (sel = i)}
            onclick={() => run(hit, false)}
          >
            <Icon name="file" size={16} width={1.4} />
            <span class="label">
              {#each segments(hit.name, hit.ranges) as s (s.t + s.hit)}
                {#if s.hit}<mark>{s.t}</mark>{:else}{s.t}{/if}
              {/each}
            </span>
            {#if hit.dir}<span class="hint">{hit.dir}</span>{/if}
            {#if sel === i}<Icon name="arrow-right" size={15} />{/if}
          </button>
        {/each}
      {/if}

      {#if files.length > 0 && commandHits.length > 0}
        <div class="sep"></div>
      {/if}

      {#if commandHits.length > 0}
        <div class="group">Commandes</div>
        {#each commandHits as hit, i (hit.cmd.id)}
          {@const idx = files.length + i}
          <button
            class="row"
            class:on={sel === idx}
            data-idx={idx}
            onpointermove={() => (sel = idx)}
            onclick={() => run(hit, false)}
          >
            <Icon name={hit.cmd.icon} size={16} width={1.4} />
            <span class="label">
              {#each segments(hit.cmd.label, hit.ranges) as s (s.t + s.hit)}
                {#if s.hit}<mark>{s.t}</mark>{:else}{s.t}{/if}
              {/each}
            </span>
            {#if hit.cmd.keys}<span class="kbd">{hit.cmd.keys}</span>{/if}
          </button>
        {/each}
      {/if}

      {#if hits.length === 0}
        <div class="empty">Aucun résultat pour « {commandsOnly ? query.slice(1) : query} »</div>
      {/if}
    </div>

    <!-- ---------- rappels clavier ---------- -->
    <div class="footer">
      <span><span class="mono">↑↓</span> naviguer</span>
      <span><span class="mono">↵</span> ouvrir</span>
      <span><span class="mono">Ctrl ↵</span> nouvel onglet</span>
      <div class="spacer"></div>
      <span><span class="mono">&gt;</span> commandes seules</span>
    </div>
  </div>
</div>

<style>
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 70;
    background: rgb(10 11 14 / 0.62);
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding-top: 112px;
  }

  .palette {
    width: 640px;
    max-width: calc(100vw - 48px);
    max-height: min(560px, calc(100vh - 160px));
    display: flex;
    flex-direction: column;
    border-radius: 14px;
    background: var(--surface-raised);
    box-shadow:
      0 0 0 1px var(--border-strong),
      0 2px 6px rgb(0 0 0 / 0.3),
      0 28px 80px rgb(0 0 0 / 0.62);
    overflow: hidden;
  }

  /* ---------- saisie ---------- */
  .input-row {
    height: 54px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 0 16px;
    border-bottom: 1px solid var(--border);
    color: var(--accent-fg);
  }
  input {
    flex: 1;
    min-width: 0;
    background: none;
    border: 0;
    outline: none;
    padding: 0;
    font-family: var(--font-ui);
    font-size: 16px;
    font-weight: 450;
    letter-spacing: -0.01em;
    color: var(--fg-1);
    user-select: text;
    cursor: text;
  }
  input::placeholder {
    color: var(--fg-3);
  }
  .count {
    font-size: 11.5px;
    color: var(--fg-3);
    flex-shrink: 0;
  }

  /* ---------- résultats ---------- */
  .results {
    padding: 8px;
    overflow-y: auto;
    min-height: 0;
  }
  .group {
    padding: 8px 10px 5px;
    font-size: 10.5px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--fg-3);
  }
  .sep {
    height: 1px;
    background: var(--border);
    margin: 8px 10px;
  }

  .row {
    width: 100%;
    height: 38px;
    display: flex;
    align-items: center;
    gap: 11px;
    padding: 0 10px;
    border-radius: 8px;
    color: var(--fg-2);
    text-align: left;
    flex-shrink: 0;
  }
  .row.on {
    background: var(--accent-soft);
    box-shadow: inset 0 0 0 1px var(--accent-line);
    color: var(--accent-fg);
  }
  .row:not(.on):hover {
    background: var(--hover);
    color: var(--fg-1);
  }
  .label {
    flex: 1;
    min-width: 0;
    font-size: 13.5px;
    color: var(--fg-1);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .row.on .label {
    color: var(--accent-on);
  }
  .label mark {
    background: color-mix(in srgb, var(--accent) 30%, transparent);
    color: var(--fg-bright);
    font-weight: 650;
    border-radius: 3px;
    padding: 0 1px;
  }
  .hint {
    font-size: 11.5px;
    color: var(--fg-3);
    flex-shrink: 0;
  }
  .row.on .hint {
    color: var(--accent-fg);
  }

  .empty {
    padding: 22px 10px;
    text-align: center;
    font-size: 13px;
    color: var(--fg-3);
  }

  /* ---------- rappels clavier ---------- */
  .footer {
    height: 36px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 18px;
    padding: 0 16px;
    border-top: 1px solid var(--border);
    background: var(--chip);
    font-size: 11px;
    color: var(--fg-3);
  }
  .footer .spacer {
    flex: 1;
  }
  .mono {
    font-family: var(--font-mono);
    font-size: 10.5px;
    color: var(--fg-4);
  }
</style>

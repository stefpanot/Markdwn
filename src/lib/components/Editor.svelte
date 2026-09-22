<script lang="ts">
  import { EditorState } from "@codemirror/state";
  import { EditorView, keymap, highlightActiveLine, drawSelection } from "@codemirror/view";
  import {
    history,
    defaultKeymap,
    historyKeymap,
    indentLess,
    indentWithTab,
    undo as cmUndo,
    redo as cmRedo,
  } from "@codemirror/commands";
  import { markdown, markdownLanguage } from "@codemirror/lang-markdown";
  import { languages } from "@codemirror/language-data";
  import { editorTheme, highlighting } from "$lib/editor-theme";
  import { app } from "$lib/state.svelte";

  interface Props {
    /** Remonte la première ligne visible, pour la synchro de scroll. */
    onScrollLine?: (line: number) => void;
  }
  let { onScrollLine }: Props = $props();

  let host: HTMLDivElement;
  let view: EditorView | undefined;
  /** Identité du document que l'EditorView porte actuellement. Suivre l'INDEX
      serait faux : les index sont réutilisés dès qu'un onglet se ferme, donc
      « tout fermer » laisserait le texte précédent à l'écran, et fermer un
      onglet à gauche de l'actif rechargerait le même document pour rien. */
  let loadedId = -1;

  function extensions() {
    return [
      history(),
      drawSelection(),
      highlightActiveLine(),
      // La keymap par défaut de CodeMirror lie Ctrl+K à « supprimer jusqu'à la
      // fin de ligne » (héritage Emacs) : conflit avec notre palette (Ctrl+K).
      // On l'avale ici, le handler fenêtre ouvre la palette.
      keymap.of([{ key: "Mod-k", run: () => true }]),
      // Tab/Maj+Tab AVANT le keymap par défaut : celui-ci ne lie pas Tab, et
      // le comportement natif (sortir le focus de l'éditeur) casse l'écriture.
      // Tab indente la ligne ou la sélection — c'est ce qui décale une puce
      // markdown ; Maj+Tab désindente.
      keymap.of([indentWithTab, { key: "Shift-Tab", run: indentLess }]),
      keymap.of([...defaultKeymap, ...historyKeymap]),
      markdown({ base: markdownLanguage, codeLanguages: languages }),
      editorTheme,
      highlighting,
      EditorView.lineWrapping,
      EditorView.updateListener.of((u) => {
        if (u.docChanged && app.active) {
          app.active.content = u.state.doc.toString();
        }
        if (u.docChanged || u.selectionSet) {
          const head = u.state.selection.main.head;
          const line = u.state.doc.lineAt(head);
          app.cursorLine = line.number;
          app.cursorCol = head - line.from + 1;
        }
      }),
    ];
  }

  // Créé vide, sans lire l'état applicatif : cet effet ne doit dépendre de
  // rien, sinon il détruirait et recréerait l'éditeur. L'effet de chargement
  // ci-dessous y met le document actif dans la foulée.
  $effect(() => {
    if (!host) return;
    view = new EditorView({
      parent: host,
      state: EditorState.create({ doc: "", extensions: extensions() }),
    });
    loadedId = -1;

    const scroller = view.scrollDOM;
    const report = () => {
      if (!view || !onScrollLine) return;
      const box = scroller.getBoundingClientRect();
      const pos = view.posAtCoords({ x: box.left + 40, y: box.top + 2 }, false);
      onScrollLine(view.state.doc.lineAt(pos).number);
    };
    scroller.addEventListener("scroll", report, { passive: true });

    return () => {
      scroller.removeEventListener("scroll", report);
      view?.destroy();
      view = undefined;
      loadedId = -1;
    };
  });

  /** Changer de document remplace le contenu sans recréer l'éditeur.
      Le garde sort AVANT de lire `content`, pour que la frappe ne fasse pas de
      cet effet un dépendant du texte. */
  $effect(() => {
    const doc = app.active;
    if (!view || !doc || doc.id === loadedId) return;
    view.dispatch({
      changes: { from: 0, to: view.state.doc.length, insert: doc.content },
      selection: { anchor: 0 },
    });
    loadedId = doc.id;
  });

  export function scrollToLine(line: number) {
    if (!view) return;
    const clamped = Math.min(Math.max(line, 1), view.state.doc.lines);
    const target = view.state.doc.line(clamped);
    view.dispatch({ effects: EditorView.scrollIntoView(target.from, { y: "start" }) });
  }

  export function focus() {
    view?.focus();
  }

  /** Entoure la sélection — gras, italique, code inline. */
  export function wrap(before: string, after = before) {
    if (!view) return;
    const { from, to } = view.state.selection.main;
    const text = view.state.sliceDoc(from, to);
    view.dispatch({
      changes: { from, to, insert: `${before}${text}${after}` },
      selection: { anchor: from + before.length, head: from + before.length + text.length },
    });
    view.focus();
  }

  /** Préfixe chaque ligne de la sélection — titres, listes, citations. */
  export function prefixLines(prefix: string) {
    if (!view) return;
    const { state } = view;
    const { from, to } = state.selection.main;
    const first = state.doc.lineAt(from).number;
    const last = state.doc.lineAt(to).number;
    const changes = [];
    for (let n = first; n <= last; n++) {
      changes.push({ from: state.doc.line(n).from, insert: prefix });
    }
    view.dispatch({ changes });
    view.focus();
  }

  /** Texte de la sélection courante. */
  export function selectedText(): string {
    if (!view) return "";
    const { from, to } = view.state.selection.main;
    return view.state.sliceDoc(from, to);
  }

  /** Remplace la sélection courante (couper, coller). */
  export function replaceSelection(text: string) {
    if (!view) return;
    const { from, to } = view.state.selection.main;
    view.dispatch({
      changes: { from, to, insert: text },
      selection: { anchor: from + text.length },
    });
    view.focus();
  }

  /** Insère un bloc de code clôturé autour de la sélection, qui prend sa
      propre ligne. Le curseur se pose après la clôture ouvrante : on y tape
      le langage (ts, rust…) qui déclenche la coloration. */
  export function codeBlock() {
    if (!view) return;
    const { state } = view;
    const { from, to } = state.selection.main;
    const first = state.doc.lineAt(from);
    const last = state.doc.lineAt(to);
    view.dispatch({
      changes: [
        { from: first.from, insert: "```\n" },
        { from: last.to, insert: "\n```" },
      ],
      selection: { anchor: first.from + 3 },
    });
    view.focus();
  }

  export function undoEdit() {
    if (view) cmUndo(view);
  }

  export function redoEdit() {
    if (view) cmRedo(view);
  }

  /* Positions ci-dessous en unités UTF-16 (celles de JS et de CodeMirror),
     fournies telles quelles par le moteur de recherche Rust. */

  /** Sélectionne un intervalle et le fait défiler — navigation des occurrences.
      Sans focus : le panneau de recherche garde la main, comme chez VS Code. */
  export function selectRange(from: number, to: number) {
    if (!view) return;
    const len = view.state.doc.length;
    const clamp = (p: number) => Math.min(Math.max(p, 0), len);
    const f = clamp(from);
    const t = clamp(to);
    view.dispatch({
      selection: { anchor: f, head: t },
      effects: EditorView.scrollIntoView(f, { y: "center" }),
    });
  }

  /** Remplace un intervalle — occurrence courante. Ne focalise pas non plus. */
  export function replaceRange(from: number, to: number, insert: string) {
    if (!view) return;
    view.dispatch({ changes: { from, to, insert } });
  }

  /** Remplace tout le contenu — « remplacer tout ». */
  export function setContent(content: string) {
    if (!view) return;
    view.dispatch({
      changes: { from: 0, to: view.state.doc.length, insert: content },
    });
  }
</script>

<div class="editor" bind:this={host}></div>

<style>
  .editor {
    flex: 1;
    min-width: 0;
    min-height: 0;
    background: var(--surface-editor);
    overflow: hidden;
  }
  .editor :global(.cm-editor) {
    height: 100%;
  }
</style>

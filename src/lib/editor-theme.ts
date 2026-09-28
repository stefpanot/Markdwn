import { EditorView } from "@codemirror/view";
import { HighlightStyle, syntaxHighlighting } from "@codemirror/language";
import { tags as t } from "@lezer/highlight";

/** Le thème lit les tokens CSS, donc il suit le thème clair/sombre sans
    reconstruire l'éditeur. */
export const editorTheme = EditorView.theme({
  "&": {
    height: "100%",
    backgroundColor: "var(--surface-editor)",
    color: "var(--syn-text)",
    fontFamily: "var(--font-mono)",
    fontSize: "var(--editor-size)",
  },
  ".cm-scroller": {
    fontFamily: "var(--font-mono)",
    lineHeight: "var(--editor-leading)",
    padding: "26px 0 40vh 0",
    overflow: "auto",
    /* La roulette sur le texte reste dans le texte : pas d'enchaînement vers
       un autre panneur scrollable aux extrémités du document. */
    overscrollBehavior: "contain",
  },
  ".cm-content": {
    padding: "0",
    caretColor: "var(--accent-fg)",
    userSelect: "text",
  },
  ".cm-line": {
    padding: "0 34px",
  },
  "&.cm-focused": { outline: "none" },
  ".cm-activeLine": {
    backgroundColor: "var(--line-active)",
    boxShadow: "inset 2px 0 0 var(--accent)",
  },
  "&.cm-focused .cm-cursor": {
    borderLeftColor: "var(--accent-fg)",
    borderLeftWidth: "1.6px",
  },
  "&.cm-focused .cm-selectionBackground, ::selection": {
    backgroundColor: "var(--accent-soft)",
  },
  ".cm-selectionBackground": {
    backgroundColor: "var(--accent-soft)",
  },
  ".cm-gutters": { display: "none" },
});

/** Coloration de la source, calquée sur les contrastes de VS Code : le corps
    de texte reste neutre et très lisible, la structure (titres, liens, code)
    prend les deux accents. Les marqueurs (`#`, `*`, `` ` ``) restent discrets. */
export const markdownHighlight = HighlightStyle.define([
  { tag: t.heading1, color: "var(--accent-fg)", fontWeight: "700" },
  { tag: t.heading2, color: "var(--accent-fg)", fontWeight: "700" },
  { tag: t.heading3, color: "var(--accent-fg)", fontWeight: "700" },
  { tag: t.heading4, color: "var(--accent-fg)", fontWeight: "700" },
  { tag: t.heading5, color: "var(--accent-fg)", fontWeight: "700" },
  { tag: t.heading6, color: "var(--accent-fg)", fontWeight: "700" },
  { tag: t.processingInstruction, color: "var(--syn-marker)" },
  { tag: t.strong, color: "var(--syn-strong)", fontWeight: "700" },
  { tag: t.emphasis, color: "var(--syn-text)", fontStyle: "italic" },
  { tag: t.strikethrough, color: "var(--fg-3)", textDecoration: "line-through" },
  { tag: t.link, color: "var(--accent-fg)", textDecoration: "underline" },
  { tag: t.url, color: "var(--accent-2-fg)" },
  { tag: t.labelName, color: "var(--accent-2-fg)" },
  { tag: t.monospace, color: "var(--accent-2-fg)" },
  { tag: t.quote, color: "var(--fg-2)" },
  { tag: t.list, color: "var(--accent-2)" },
  { tag: t.meta, color: "var(--syn-punct)" },
  { tag: t.keyword, color: "var(--accent)" },
  { tag: t.string, color: "var(--accent-2-fg)" },
  { tag: t.comment, color: "var(--fg-4)", fontStyle: "italic" },
  { tag: t.number, color: "var(--accent-2-fg)" },
  { tag: t.typeName, color: "var(--syn-strong)" },
  { tag: t.variableName, color: "var(--syn-text)" },
  // Séparateurs et détails qui n'étaient pas couverts : `---`, entités HTML,
  // échappements `\*` — restent visibles sans prendre le pas sur le texte.
  { tag: t.contentSeparator, color: "var(--syn-punct)" },
  { tag: t.character, color: "var(--accent-2-fg)" },
  { tag: t.escape, color: "var(--syn-punct)" },
]);

export const highlighting = syntaxHighlighting(markdownHighlight);

<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { resolveAsset } from "$lib/api";
  import { app } from "$lib/state.svelte";

  interface Props {
    /** "read" élargit la mesure et passe le corps en serif. */
    variant?: "split" | "read";
    onScrollLine?: (line: number) => void;
    /** Tous les clics sur un lien remontent ici : c'est la page qui décide
        d'ouvrir un document, de sauter à une ancre ou de sortir vers le
        navigateur. */
    onLink?: (href: string) => void;
  }
  let { variant = "split", onScrollLine, onLink }: Props = $props();

  let scroller: HTMLDivElement;

  const html = $derived(app.rendered?.html ?? "");

  function markers(): HTMLElement[] {
    return Array.from(scroller?.querySelectorAll<HTMLElement>(".srcmap") ?? []);
  }

  function report() {
    if (!onScrollLine || !scroller) return;
    const top = scroller.getBoundingClientRect().top;
    let line = 1;
    for (const m of markers()) {
      // Le dernier marqueur encore au-dessus du haut de la zone visible.
      if (m.getBoundingClientRect().top - top > 2) break;
      line = Number(m.dataset.line) || line;
    }
    onScrollLine(line);
  }

  function onClick(e: MouseEvent) {
    const anchor = (e.target as HTMLElement | null)?.closest("a");
    const href = anchor?.getAttribute("href");
    if (!href) return;
    // Aucun lien ne doit naviguer la webview : elle n'a nulle part où aller.
    e.preventDefault();
    onLink?.(href);
  }

  /* Délégation attachée impérativement : le HTML de l'aperçu est régénéré à
     chaque rendu, donc on écoute le conteneur. L'activation clavier d'un lien
     émet un click qui remonte ici, donc rien à ajouter pour le clavier. */
  $effect(() => {
    const host = scroller;
    if (!host) return;
    host.addEventListener("click", onClick);
    return () => host.removeEventListener("click", onClick);
  });

  /* Images relatives : l'origine de la webview (localhost ou tauri://) ne
     connaît pas le dossier du document, donc `assets/logo.svg` serait cassé.
     On résout chaque src en Rust (percent-décodage, `..`, convention « /… »
     relatif au document) puis on le réécrit vers le protocole asset: de Tauri.
     Ça couvre aussi les <img> en HTML brut, que pulldown-cmark laisse passer
     tels quels. Les résolutions sont mémorisées : un rendu débouncé ne
     redemande pas ce qu'on sait déjà. */
  const assetCache = new Map<string, string | null>();

  $effect(() => {
    void html; // dépendance : repatcher après chaque nouveau rendu
    const host = scroller;
    // Document jamais enregistré : pas de dossier de référence, rien à faire.
    const docPath = app.active?.path ?? "";
    if (!host || !docPath) return;
    for (const img of host.querySelectorAll<HTMLImageElement>("img")) {
      const src = img.getAttribute("src") ?? "";
      if (!src) continue;
      const key = `${docPath}${src}`;
      const apply = (abs: string | null) => {
        // L'élément a pu être remplacé par un rendu plus récent entre-temps.
        if (abs && img.isConnected) img.src = convertFileSrc(abs);
      };
      if (assetCache.has(key)) {
        apply(assetCache.get(key) ?? null);
      } else {
        resolveAsset(docPath, src)
          .then((abs) => {
            assetCache.set(key, abs);
            apply(abs);
          })
          .catch(() => {
            /* Hors Tauri (Vite nu) : on laisse le src tel quel. */
          });
      }
    }
  });

  /** Amène l'ancre en haut de la zone lisible. Retourne false si l'ancre
      n'existe pas dans ce document, pour que l'appelant puisse le dire. */
  export function scrollToAnchor(slug: string): boolean {
    if (!scroller) return false;
    let target: HTMLElement | null = null;
    try {
      target = scroller.querySelector(`#${CSS.escape(slug)}`);
    } catch {
      return false;
    }
    if (!target) return false;
    const delta = target.getBoundingClientRect().top - scroller.getBoundingClientRect().top;
    scroller.scrollTop += delta - 12;
    return true;
  }

  export function scrollToLine(line: number) {
    if (!scroller) return;
    let target: HTMLElement | undefined;
    for (const m of markers()) {
      if ((Number(m.dataset.line) || 1) > line) break;
      target = m;
    }
    if (!target) {
      scroller.scrollTop = 0;
      return;
    }
    const delta = target.getBoundingClientRect().top - scroller.getBoundingClientRect().top;
    scroller.scrollTop += delta;
  }
</script>

<div
  class="preview {variant}"
  class:full={variant === "read" && app.readingWidth === "full"}
  bind:this={scroller}
  onscroll={report}
  role="document"
  style:--read-size="{app.readingSize}px"
>
  <!-- Le HTML vient du rendu Rust, pas d'une saisie externe. -->
  <div class="doc">{@html html}</div>
</div>

<style>
  .preview {
    flex: 1;
    min-width: 0;
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
    background: var(--surface-preview);
    user-select: text;
  }
  .preview.read {
    background: transparent;
    display: flex;
    justify-content: center;
  }

  .doc {
    padding: 34px 44px;
    max-width: 660px;
    font-size: 15.5px;
    line-height: 1.72;
    color: var(--syn-text);
    /* Coloration des blocs de code (syntect émet des classes, pas des styles
       en ligne) : les couleurs vivent ici et suivent le thème comme le reste.
       Palette resserrée : indigo pour la structure, terre cuite pour les
       chaînes, sable pour les nombres, neutres pour le reste. */
    --code-comment: var(--syn-punct);
    --code-string: var(--accent-2-fg);
    --code-keyword: var(--accent-fg);
    --code-ident: var(--syn-strong);
    --code-number: #d9b77c;
    --code-type: #b9c0fa;
    --code-invalid: #e06c75;
  }
  :global(:root[data-theme="light"]) .doc {
    --code-number: #8a6a1f;
    --code-type: #5a63d6;
    --code-invalid: #c13e37;
  }
  .read .doc {
    padding: 34px 0 45vh;
    width: 712px;
    max-width: 712px;
    font-family: var(--font-read);
    font-size: var(--read-size);
    line-height: 1.78;
  }
  /* Pleine largeur : les tableaux et blocs de code respirent enfin. La prose
     y gagne des lignes plus longues — c'est le compromis assumé du mode. */
  .read.full {
    justify-content: stretch;
  }
  .read.full .doc {
    width: 100%;
    max-width: none;
    padding: 34px 48px 45vh;
  }

  /* Marqueurs de cartographie source : invisibles, mais mesurables. */
  .doc :global(.srcmap) {
    display: block;
    height: 0;
    overflow: hidden;
  }

  .doc :global(h1) {
    margin: 0 0 12px;
    font-size: 30px;
    line-height: 1.24;
    font-weight: 650;
    letter-spacing: -0.024em;
    color: var(--fg-bright);
  }
  .read .doc :global(h1) {
    font-family: var(--font-read);
    font-size: 34px;
    line-height: 1.22;
    font-weight: 600;
    letter-spacing: -0.018em;
  }
  .doc :global(h2) {
    margin: 28px 0 12px;
    font-size: 20px;
    line-height: 1.3;
    font-weight: 600;
    letter-spacing: -0.016em;
    color: var(--syn-strong);
  }
  .read .doc :global(h2) {
    font-family: var(--font-read);
    font-size: 23px;
    font-weight: 600;
    letter-spacing: -0.012em;
  }
  .doc :global(h3) {
    margin: 24px 0 10px;
    font-size: 17px;
    font-weight: 600;
    color: var(--syn-strong);
  }
  .doc :global(p) {
    margin: 0 0 22px;
    text-wrap: pretty;
  }
  .doc :global(a) {
    color: var(--accent-fg);
    text-decoration: underline;
    text-decoration-color: color-mix(in oklab, var(--accent-fg) 40%, transparent);
    text-underline-offset: 3px;
  }
  .doc :global(a:hover) {
    text-decoration-color: var(--accent-fg);
  }
  .doc :global(strong) {
    font-weight: 650;
    color: var(--fg-bright);
  }
  .doc :global(del),
  .doc :global(s) {
    color: var(--fg-3);
  }
  /* ==surligné== : une couleur « surligneur » tirée du sable existant, pour
     ne pas introduire d'accent tiers. */
  .doc :global(mark) {
    background: color-mix(in oklab, var(--code-number) 26%, transparent);
    color: var(--fg-bright);
    padding: 0 3px;
    border-radius: 3px;
  }
  .doc :global(ul),
  .doc :global(ol) {
    margin: 0 0 24px;
    padding-left: 22px;
  }
  .doc :global(li) {
    margin-bottom: 7px;
  }
  .doc :global(li::marker) {
    color: var(--accent);
  }
  .doc :global(blockquote) {
    margin: 0 0 26px;
    padding: 2px 0 2px 20px;
    box-shadow: inset 2px 0 0 var(--accent);
    color: var(--fg-2);
  }
  .read .doc :global(blockquote) {
    font-style: italic;
  }
  .doc :global(code) {
    font-family: var(--font-mono);
    font-size: 0.86em;
    padding: 1.5px 5px;
    border-radius: 4px;
    background: var(--accent-2-soft);
    color: var(--accent-2-fg);
  }
  .doc :global(pre) {
    margin: 0 0 26px;
    padding: 13px 15px;
    border-radius: var(--r-xl);
    background: var(--surface-sunken);
    box-shadow: 0 0 0 1px var(--border-strong);
    overflow-x: auto;
  }
  .doc :global(pre code) {
    font-size: 13px;
    line-height: 1.62;
    padding: 0;
    background: none;
    color: var(--syn-text);
  }
  /* Scopes syntect (ClassStyle::Spaced) : chaque atome du scope devient une
     classe — `keyword.control.rust` donne class="keyword control rust", donc
     on cible les racines communes. Règles génériques d'abord : la chaîne
     l'emporte sur la ponctuation à spécificité égale. */
  .doc :global(pre code .punctuation) {
    color: var(--code-comment);
  }
  .doc :global(pre code .comment) {
    color: var(--code-comment);
    font-style: italic;
  }
  .doc :global(pre code .string),
  .doc :global(pre code .constant.character) {
    color: var(--code-string);
  }
  .doc :global(pre code .keyword),
  .doc :global(pre code .storage),
  .doc :global(pre code .variable.language),
  .doc :global(pre code .entity.name.tag) {
    color: var(--code-keyword);
  }
  .doc :global(pre code .constant.numeric),
  .doc :global(pre code .constant.language),
  .doc :global(pre code .support.constant),
  .doc :global(pre code .entity.other.attribute-name) {
    color: var(--code-number);
  }
  .doc :global(pre code .entity.name.function),
  .doc :global(pre code .support.function),
  .doc :global(pre code .support.macro) {
    color: var(--code-ident);
  }
  .doc :global(pre code .entity.name.type),
  .doc :global(pre code .support.type),
  .doc :global(pre code .support.class) {
    color: var(--code-type);
  }
  .doc :global(pre code .markup.heading) {
    color: var(--code-keyword);
  }
  .doc :global(pre code .markup.bold) {
    font-weight: 650;
  }
  .doc :global(pre code .markup.italic) {
    font-style: italic;
  }
  .doc :global(pre code .markup.underline.link) {
    color: var(--code-keyword);
  }
  .doc :global(pre code .invalid) {
    color: var(--code-invalid);
  }
  .doc :global(table) {
    width: 100%;
    margin: 0 0 26px;
    border-collapse: collapse;
    border-radius: var(--r-xl);
    overflow: hidden;
    box-shadow: 0 0 0 1px var(--border-strong);
    font-family: var(--font-ui);
    font-size: 14.5px;
  }
  .doc :global(th) {
    padding: 9px 14px;
    text-align: left;
    background: var(--chip);
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.03em;
    text-transform: uppercase;
    color: var(--fg-2);
  }
  .doc :global(td) {
    padding: 10px 14px;
    border-top: 1px solid var(--border);
  }
  .doc :global(hr) {
    margin: 30px 0;
    border: 0;
    border-top: 1px solid var(--border-strong);
  }
  .doc :global(img) {
    max-width: 100%;
    border-radius: var(--r-md);
  }
  .doc :global(input[type="checkbox"]) {
    accent-color: var(--accent);
    margin-right: 8px;
  }
</style>

/** Rendu Mermaid des blocs ```mermaid de l'aperçu.
 *
 * Le HTML vient du rendu Rust (pulldown-cmark) : un bloc ```mermaid inconnu
 * de syntect traverse la pipeline comme un bloc de code ordinaire,
 * `<pre><code class="language-mermaid">`. Ce module reprend la main côté
 * webview : il extrait le source de chaque bloc, le passe à mermaid.js et
 * remplace le bloc par le SVG produit.
 *
 * Choix d'architecture :
 * - mermaid est chargé à la demande (dynamic import) : ~1,5 Mo de bundle ne
 *   doivent pas peser sur le démarrage d'une app qui n'affiche souvent aucun
 *   diagramme. Le chargement n'arrive qu'au premier document contenant un
 *   bloc ```mermaid.
 * - Les rendus sont mis en cache par (thème, source). Le rendu étant relancé
 *   à chaque frappe (débouncée), taper dans le document ne re-rend pas les
 *   diagrammes inchangés — seuls les blocs modifiés (et les nouveaux) passent
 *   par mermaid.render, qui est coûteux.
 * - Les rendus sont séquentiels : mermaid.render n'est pas réentrant.
 * - La palette est tirée des tokens CSS de l'app : les diagrammes suivent le
 *   thème clair/sombre comme le reste de l'aperçu. Un changement de thème ne
 *   régénère pas le HTML (l'effet doit donc savoir re-rendre les diagrammes
 *   déjà rendus : leur source est gardée dans `sources`).
 * - securityLevel "strict" (défaut de mermaid) : le SVG passe au DOMPurify
 *   interne avant de nous revenir, cohérent avec la sanification ammonia
 *   côté Rust qui protège déjà {@html}.
 */

import type { Theme } from "./state.svelte";

type Mermaid = typeof import("mermaid").default;

let mod: Mermaid | null = null;
let configuredFor: Theme | null = null;
let seq = 0;

/** Source texte de chaque élément (pre ou div.mermaid) déjà rencontré. */
const sources = new WeakMap<HTMLElement, string>();

/** Cache des rendus : clé = thème + source. La promesse couvre les succès
    comme les échecs (une syntaxe invalide ne doit pas être re-tentée à
    chaque frappe). */
const cache = new Map<string, Promise<{ svg: string } | { error: string }>>();
const CACHE_MAX = 100;

async function loadMermaid(): Promise<Mermaid> {
  if (!mod) mod = (await import("mermaid")).default;
  return mod;
}

function cssVar(name: string): string {
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim();
}

/** (Ré)initialise mermaid pour le thème courant. L'app n'a que deux thèmes :
    on re-configure au changement plutôt que de payer une détection continue. */
function applyTheme(mermaid: Mermaid, theme: Theme) {
  if (configuredFor === theme) return;
  configuredFor = theme;
  mermaid.initialize({
    startOnLoad: false,
    securityLevel: "strict",
    theme: "base",
    themeVariables: {
      background: "transparent",
      fontFamily: "inherit",
      // Nœuds : fond discret, texte courant, bordure visible.
      primaryColor: cssVar("--chip"),
      primaryTextColor: cssVar("--syn-text"),
      primaryBorderColor: cssVar("--border-strong"),
      secondaryColor: cssVar("--surface-sunken"),
      tertiaryColor: cssVar("--accent-soft"),
      // Arêtes et labels d'arêtes.
      lineColor: cssVar("--fg-3"),
      textColor: cssVar("--syn-text"),
      // Titres de sous-graphes et acteurs (diagrammes de séquence).
      titleColor: cssVar("--fg-bright"),
      actorBkg: cssVar("--chip"),
      actorBorder: cssVar("--border-strong"),
      actorTextColor: cssVar("--syn-text"),
      actorLineColor: cssVar("--fg-3"),
      labelBoxBkgColor: cssVar("--chip"),
      labelBoxBorderColor: cssVar("--border-strong"),
      labelTextColor: cssVar("--syn-text"),
      loopTextColor: cssVar("--syn-text"),
      noteBkgColor: cssVar("--accent-soft"),
      noteBorderColor: cssVar("--accent-line"),
      noteTextColor: cssVar("--accent-fg"),
      // Alternatives (alt/else) des diagrammes de séquence.
      altBackground: cssVar("--surface-sunken"),
      // Flèches d'activation.
      signalColor: cssVar("--fg-3"),
      // CSL (états, classes, entité-relation) : bordures.
      clusterBkg: cssVar("--surface-sunken"),
      clusterBorder: cssVar("--border-strong"),
      edgeLabelBackground: cssVar("--surface-sunken"),
    },
    flowchart: { htmlLabels: true, useMaxWidth: false },
    sequence: { useMaxWidth: false, wrap: true },
    // Gantt / pie / mindmap : valeurs de base suffisantes.
  });
}

function renderOne(mermaid: Mermaid, source: string): Promise<{ svg: string } | { error: string }> {
  const id = `mmd-${++seq}`;
  return mermaid
    .render(id, source)
    .then(({ svg }) => ({ svg }))
    .catch((e: unknown) => {
      /* En cas d'échec de parse, mermaid jette après avoir rendu son diagramme
         d'erreur dans son bac à sable temporaire #d<id>, sans le nettoyer
         (le retrait arrive après le throw). On le détruit ici : sans ça,
         chaque bloc invalide laisserait un SVG d'erreur orphelin dans le
         <body> pour la durée de la session. */
      document.getElementById(`d${id}`)?.remove();
      return {
        error: e instanceof Error ? e.message.replace(/\s+/g, " ").slice(0, 300) : String(e),
      };
    });
}

/** Remplace chaque bloc ```mermaid de `host` par son SVG rendu. Sans effet
    si aucun bloc n'est présent (mermaid n'est alors même pas chargé).
    Appelé après chaque nouveau rendu HTML et à chaque changement de thème. */
export async function renderMermaidBlocks(host: HTMLElement, theme: Theme): Promise<void> {
  // 1) Thème changé, HTML identique : les div.mermaid déjà rendues ne
  //    correspondraient plus à la palette. On les repasse en bloc de code
  //    brut (le source est conservé), la suite du pipeline les re-rend.
  for (const el of Array.from(host.querySelectorAll<HTMLElement>("div.mermaid"))) {
    if (el.dataset.theme === theme) continue;
    const src = sources.get(el);
    if (src == null) continue;
    const pre = document.createElement("pre");
    const code = document.createElement("code");
    code.className = "language-mermaid";
    code.textContent = src;
    pre.append(code);
    el.replaceWith(pre);
  }

  // 2) Indexation des blocs frais (le HTML a été régénéré : ce sont des <pre>).
  const blocks = Array.from(host.querySelectorAll<HTMLPreElement>("pre")).filter((pre) => {
    const code = pre.querySelector("code.language-mermaid");
    if (!code) return false;
    sources.set(pre, code.textContent ?? "");
    return true;
  });
  if (blocks.length === 0) return;

  const mermaid = await loadMermaid();
  applyTheme(mermaid, theme);

  for (const pre of blocks) {
    // Remplacé entre-temps par un rendu plus récent : on ne touche pas au
    // nouveau DOM (même logique que la résolution d'images de Preview).
    if (!pre.isConnected) continue;
    const source = sources.get(pre) ?? "";
    const key = `${theme}${source}`;
    let p = cache.get(key);
    if (!p) {
      p = renderOne(mermaid, source);
      if (cache.size >= CACHE_MAX) {
        // Éviction FIFO : les premières entrées sont les plus anciennes.
        const oldest = cache.keys().next().value;
        if (oldest !== undefined) cache.delete(oldest);
      }
      cache.set(key, p);
    }
    const result = await p;
    if (!pre.isConnected) continue;
    if ("svg" in result) {
      const div = document.createElement("div");
      div.className = "mermaid";
      div.dataset.theme = theme;
      div.innerHTML = result.svg;
      sources.set(div, source);
      pre.replaceWith(div);
    } else {
      // Syntaxe invalide : le message s'affiche, le source reste lisible
      // dessous (dans un dépliant) pour corriger sans repasser à l'éditeur.
      const wrap = document.createElement("div");
      wrap.className = "mermaid-error";
      const msg = document.createElement("p");
      msg.textContent = `Diagramme Mermaid invalide : ${result.error}`;
      const details = document.createElement("details");
      const summary = document.createElement("summary");
      summary.textContent = "Voir le source";
      const srcPre = document.createElement("pre");
      const srcCode = document.createElement("code");
      srcCode.textContent = source;
      srcPre.append(srcCode);
      details.append(summary, srcPre);
      wrap.append(msg, details);
      sources.set(wrap, source);
      pre.replaceWith(wrap);
    }
  }
}

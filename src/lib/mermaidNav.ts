/** Navigation pan/zoom/ajuster pour chaque diagramme Mermaid rendu.
 *
 * Un diagramme large déborde de la colonne de l'aperçu ; la scrollbar
 * horizontale, en bas du bloc, est pénible quand le diagramme est haut.
 * Cette module ajoute à chaque `.mermaid` :
 * - une barre d'outils (au survol) : Ajuster / − / pourcentage / + / Main ;
 * - un mode « Ajuster » : le diagramme tient sur la largeur de la colonne ;
 * - le zoom (boutons ou Ctrl + molette, ancré sur le curseur) ;
 * - la « main » : glisser pour se déplacer horizontalement (comme un PDF) ;
 * - Molette + Maj : déplacement horizontal ; double-clic : ajuster/100 % ;
 * - clavier quand le diagramme a le focus : ← → déplacent, 0 ajuste.
 *
 * Le panneau vertical reste celui du document (molette normale) : on ne
 * capture que ce qui manque. Le diagramme est contenu dans un porteur
 * `.mnav` transformé (translate + scale, origine 0 0) ; la hauteur du
 * conteneur est recalculée à chaque application car un transform ne
 * participe pas à la mise en page — sans ça, un diagramme rétréci laisserait
 * un grand vide.
 */

interface ViewState {
  natW: number;
  natH: number;
  scale: number;
  /** déplacement horizontal libre (≤ 0 quand le contenu déborde). */
  px: number;
  /** mode ajusté : le scale suit la largeur du conteneur (resize inclus). */
  fit: boolean;
  /** mode main : le glisser déplace le diagramme. */
  pan: boolean;
}

const states = new WeakMap<HTMLElement, ViewState>();

const MIN_SCALE = 0.1;
const MAX_SCALE = 4;

/** Borne le déplacement horizontal : on ne peut pas perdre le diagramme. */
function clampPan(host: HTMLElement, st: ViewState): number {
  const overflow = st.natW * st.scale - host.clientWidth;
  return overflow > 0 ? Math.min(0, Math.max(st.px, -overflow)) : 0;
}

/** Recalcule le transform, la hauteur du conteneur et l'indicateur. */
function apply(host: HTMLElement, view: HTMLElement, st: ViewState, pct: HTMLElement): void {
  const availW = host.clientWidth;
  // Quand le diagramme (zoomé) est plus étroit que la colonne, on le centre.
  const ox = Math.max(0, (availW - st.natW * st.scale) / 2);
  st.px = clampPan(host, st);
  view.style.transform = `translate(${ox + st.px}px, 0) scale(${st.scale})`;
  host.style.height = `${Math.ceil(st.natH * st.scale)}px`;
  pct.textContent = `${Math.round(st.scale * 100)}%`;
  host.classList.toggle("fitted", st.fit);
}

/** Scale « ajuster » : tient dans la largeur, sans jamais agrandir. */
function fitScale(host: HTMLElement, st: ViewState): number {
  return Math.min(1, host.clientWidth / st.natW);
}

export function enhanceMermaidDiagram(host: HTMLElement): void {
  if (host.dataset.nav) return;
  const svg = host.querySelector("svg");
  if (!svg) return;
  host.dataset.nav = "1";

  // Porteur transformé autour du SVG.
  const view = document.createElement("div");
  view.className = "mnav";
  view.append(...Array.from(host.childNodes));
  host.append(view);

  // Taille naturelle : le SVG porte des attributs width/height en px
  // (useMaxWidth false) ; repli sur le viewBox puis sur la mesure layout.
  const vb = svg.viewBox?.baseVal;
  const st: ViewState = {
    natW: parseFloat(svg.getAttribute("width") ?? "") || (vb && vb.width) || svg.clientWidth || 1,
    natH: parseFloat(svg.getAttribute("height") ?? "") || (vb && vb.height) || svg.clientHeight || 1,
    scale: 1,
    px: 0,
    fit: false,
    pan: false,
  };
  states.set(host, st);

  // Barre d'outils.
  const bar = document.createElement("div");
  bar.className = "mtoolbar";
  const mkBtn = (label: string, title: string, onClick: () => void) => {
    const b = document.createElement("button");
    b.type = "button";
    b.textContent = label;
    b.title = title;
    b.addEventListener("click", (e) => {
      e.stopPropagation();
      onClick();
    });
    bar.append(b);
    return b;
  };
  const btnFit = mkBtn("Ajuster", "Ajuster à la largeur (double-clic, touche 0)", () => {
    st.fit = !st.fit;
    st.px = 0;
    st.scale = st.fit ? fitScale(host, st) : 1;
    btnFit.classList.toggle("on", st.fit);
    apply(host, view, st, pct);
  });
  mkBtn("−", "Zoom arrière", () => zoomBy(host, view, st, pct, 1 / 1.25));
  const pct = document.createElement("button");
  pct.type = "button";
  pct.className = "mpct";
  pct.title = "Revenir à 100 %";
  pct.addEventListener("click", (e) => {
    e.stopPropagation();
    st.fit = false;
    btnFit.classList.remove("on");
    st.scale = 1;
    st.px = 0;
    apply(host, view, st, pct);
  });
  bar.append(pct);
  mkBtn("+", "Zoom avant", () => zoomBy(host, view, st, pct, 1.25));
  const btnPan = mkBtn("Main", "Déplacer le diagramme en glissant", () => {
    st.pan = !st.pan;
    host.classList.toggle("pan", st.pan);
    btnPan.classList.toggle("on", st.pan);
  });
  host.append(bar);

  // État initial : ajuster si (et seulement si) le diagramme déborde.
  if (st.natW > host.clientWidth + 4) {
    st.fit = true;
    st.scale = fitScale(host, st);
    btnFit.classList.add("on");
  }
  apply(host, view, st, pct);

  // Zoom ancré sur un point (curseur), comme un lecteur PDF.
  function zoomAt(host: HTMLElement, cx: number, factor: number): void {
    const availW = host.clientWidth;
    const ox = Math.max(0, (availW - st.natW * st.scale) / 2);
    const contentX = (cx - ox - st.px) / st.scale;
    st.scale = Math.min(MAX_SCALE, Math.max(MIN_SCALE, st.scale * factor));
    st.px = cx - ox - contentX * st.scale;
  }

  function zoomBy(h: HTMLElement, v: HTMLElement, s: ViewState, p: HTMLElement, f: number): void {
    zoomAt(h, h.clientWidth / 2, f);
    // Un zoom manuel sort du mode ajusté (le resize ne doit pas l'écraser).
    s.fit = false;
    btnFit.classList.remove("on");
    apply(h, v, s, p);
  }

  // Molette : Ctrl = zoom ancré, Maj = déplacement horizontal, sinon le
  // défilement reste celui du document (on ne capture pas).
  host.addEventListener(
    "wheel",
    (e) => {
      if (e.ctrlKey) {
        e.preventDefault();
        const rect = host.getBoundingClientRect();
        zoomAt(host, e.clientX - rect.left, Math.pow(1.2, -e.deltaY / 100));
        st.fit = false;
        btnFit.classList.remove("on");
        apply(host, view, st, pct);
      } else if (e.shiftKey) {
        e.preventDefault();
        st.px = clampPan(host, { ...st, px: st.px - e.deltaY });
        apply(host, view, st, pct);
      }
    },
    { passive: false },
  );

  // Double-clic : bascule ajuster / 100 %.
  view.addEventListener("dblclick", () => btnFit.click());

  // La main : glisser déplace (pointer capture, donc sortie de l'élément OK).
  view.addEventListener("pointerdown", (e) => {
    if (!st.pan || e.button !== 0) return;
    e.preventDefault();
    view.setPointerCapture(e.pointerId);
    view.classList.add("grabbing");
    let lastX = e.clientX;
    const move = (ev: PointerEvent) => {
      st.px = clampPan(host, { ...st, px: st.px + (ev.clientX - lastX) });
      lastX = ev.clientX;
      apply(host, view, st, pct);
    };
    const up = (ev: PointerEvent) => {
      view.releasePointerCapture(ev.pointerId);
      view.removeEventListener("pointermove", move);
      view.removeEventListener("pointerup", up);
      view.classList.remove("grabbing");
    };
    view.addEventListener("pointermove", move);
    view.addEventListener("pointerup", up);
  });

  // Clavier : ← → déplacent, 0 ajuste, +/− zooment.
  host.tabIndex = 0;
  host.addEventListener("keydown", (e) => {
    if (e.key === "ArrowLeft" || e.key === "ArrowRight") {
      e.preventDefault();
      st.px = clampPan(host, { ...st, px: st.px + (e.key === "ArrowLeft" ? -60 : 60) });
      apply(host, view, st, pct);
    } else if (e.key === "0") {
      btnFit.click();
    } else if (e.key === "+" || e.key === "=") {
      zoomBy(host, view, st, pct, 1.25);
    } else if (e.key === "-") {
      zoomBy(host, view, st, pct, 1 / 1.25);
    }
  });

  // En mode ajusté, le scale suit les redimensionnements de la colonne
  // (split ratio, repli de la sidebar, mode pleine largeur).
  if (typeof ResizeObserver !== "undefined") {
    const ro = new ResizeObserver(() => {
      if (!host.isConnected) {
        ro.disconnect();
        return;
      }
      if (st.fit) {
        st.scale = fitScale(host, st);
        apply(host, view, st, pct);
      }
    });
    ro.observe(host);
  }
}

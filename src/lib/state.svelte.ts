import type { AppConfig, DirEntryInfo, Heading, Rendered } from "./api";

export type Mode = "read" | "split" | "zen";
export type Theme = "dark" | "light";

export interface OpenDoc {
  /** Identité stable du document ouvert.
      Les index de `docs` sont réutilisés dès qu'un onglet se ferme : ils ne
      peuvent donc pas servir à savoir quel document l'éditeur porte. */
  id: number;
  /** Vide pour un document jamais enregistré. */
  path: string;
  name: string;
  content: string;
  /** Contenu au dernier enregistrement, pour savoir si c'est sale. */
  savedContent: string;
  /** Offset UTF-16 du curseur, en unités JS/CodeMirror. Tenu à jour à chaque
      déplacement ; restauré au montage de l'éditeur : changer de mode
      (Zen -> Split notamment) ne doit pas renvoyer le curseur en haut du
      document — il reste où l'utilisateur l'avait laissé. */
  cursorPos: number;
  /** Scroll mémorisé par document ET par mode : on ne reprend pas une lecture
      là où on avait laissé le curseur d'édition. */
  scroll: Record<Mode, number>;
}

const WELCOME = `# Markdwn

Éditeur et lecteur Markdown natif. Trois postures, un raccourci chacune.

## Les modes

| Mode    | Raccourci | Pour                    |
| ------- | --------- | ----------------------- |
| Lecture | Ctrl+1    | lire, plein cadre       |
| Split   | Ctrl+2    | écrire en vérifiant     |
| Zen     | Ctrl+3    | écrire, le reste s'efface |

## Au quotidien

| Action                    | Raccourci    |
| ------------------------- | ------------ |
| Palette de commandes      | Ctrl+K       |
| Rechercher et remplacer   | Ctrl+H       |
| Ouvrir un fichier         | Ctrl+O       |
| Enregistrer               | Ctrl+S       |
| Enregistrer sous (copie)  | Ctrl+Shift+S |

Un double-clic sur un \`.md\` dans l'Explorateur l'ouvre directement ici.

## Et aussi

Coloration syntaxique, plan du document cliquable, aperçu en direct et
navigation entre les fichiers d'un dossier.
`;

let nextId = 1;

function makeDoc(content: string): OpenDoc {
  return {
    id: nextId++,
    path: "",
    name: "Sans titre.md",
    // savedContent identique au contenu : un document qu'on vient d'ouvrir ne
    // doit pas se déclarer « Modifié » avant la première frappe.
    content,
    savedContent: content,
    cursorPos: 0,
    scroll: { read: 0, split: 0, zen: 0 },
  };
}

/** Une feuille réellement vide, sur Ctrl+N. */
const blankDoc = () => makeDoc("");

class AppState {
  theme = $state<Theme>("dark");
  mode = $state<Mode>("split");

  /** Peut être VIDE : aucun document ouvert affiche l'écran d'accueil, pas un
      onglet fantôme. Tout ce qui lit `active` doit donc gérer `undefined`. */
  docs = $state<OpenDoc[]>([]);
  activeIndex = $state(0);

  /** Résultat du rendu Rust pour le document actif. */
  rendered = $state<Rendered | null>(null);

  cursorLine = $state(1);
  cursorCol = $state(1);

  /** Racine de l'arborescence, vide tant qu'aucun dossier n'est ouvert. */
  folderPath = $state("");
  folderEntries = $state<DirEntryInfo[]>([]);
  /** Liste plate ordonnée des .md du dossier, pour fichier suivant/précédent. */
  folderFiles = $state<string[]>([]);

  /** Dernier chemin demandé pour la révélation dans l'arborescence. */
  revealPath = $state("");
  /** Incrémenté à CHAQUE demande de révélation : rejoue l'expansion des
      dossiers et le défilement même si le chemin n'a pas changé. */
  revealTick = $state(0);

  /** La barre de dossiers se masque dans TOUS les modes, lecture comprise :
      passer au fichier suivant ne doit pas obliger à changer de posture. */
  sidebarVisible = $state(true);

  /** Réglages de lecture, exposés dans la barre du mode Lecture.
      « centered » garde une mesure de ~68 caractères, bon pour la prose ;
      « full » occupe toute la largeur, bon pour les tableaux et le code. */
  readingSize = $state(17);
  readingWidth = $state<"centered" | "full">("centered");
  /** Corps du texte d'édition, en px — voir --editor-size. */
  editorSize = $state(14.5);
  syncScroll = $state(true);
  restoreLastFolder = $state(true);
  /** Vérification silencieuse des mises à jour au lancement. */
  autoUpdate = $state(true);

  /* --- Panneaux latéraux (K2.8). Toutes ces valeurs sont posées en
     variables inline sur .app (--w-sidebar, --w-toc) ou consommées en
     flex-basis : la mise en page suit sans code supplémentaire. --- */
  /** Largeur de la barre de dossiers, en px (bornes 180–480). */
  sidebarWidth = $state(250);
  /** Rail « Sur cette page » replié (mode Lecture). Replié par défaut : le
     plan est une aide à la navigation ponctuelle, pas un élément de décor
     permanent — c'est au lecteur de l'ouvrir quand il en a besoin. */
  tocCollapsed = $state(true);
  /** Largeur du rail « Sur cette page », en px (bornes 200–420). */
  tocWidth = $state(240);
  /** Ratio éditeur/aperçu du mode Split (bornes 0.2–0.8). */
  splitRatio = $state(0.5);
  /** Plan du document replié en bas de la sidebar (Split/Zen). */
  outlineCollapsed = $state(false);
  /** Hauteur du plan, en fraction de la sidebar (bornes 0.2–0.7). */
  outlineHeight = $state(0.4);
  /** Révéler le document actif dans l'arborescence à chaque changement. */
  followActive = $state(true);

  /* --- État de l'arborescence, indexé par chemin. Sorti des TreeNode :
     un refresh (manuel, focus, watcher) remplace les données sans jamais
     détruire l'expansion, et les dossiers disparus du disque sont simplement
     oubliés (leur entrée reste en mémoire, inoffensive). --- */
  /** Dossiers dépliés. */
  treeExpanded = $state<Record<string, boolean>>({});
  /** Enfants chargés par dossier ; `undefined` = pas encore lus. */
  treeChildren = $state<Record<string, DirEntryInfo[]>>({});

  /** Faux tant que la configuration n'est pas chargée : il ne faut surtout pas
      réécrire le fichier avec les valeurs par défaut entre-temps. */
  hydrated = $state(false);
  configPath = $state("");
  settingsOpen = $state(false);
  paletteOpen = $state(false);
  /** Vide hors Tauri ; sinon la version résolue depuis Cargo.toml. */
  version = $state("");

  active = $derived<OpenDoc | undefined>(this.docs[this.activeIndex]);
  dirty = $derived(!!this.active && this.active.content !== this.active.savedContent);
  headings = $derived<Heading[]>(this.rendered?.headings ?? []);

  /** Photographie des réglages à persister. Lire tous ces champs ici est ce
      qui fait que l'effet d'enregistrement se déclenche au moindre changement. */
  toConfig(): AppConfig {
    return {
      // Rust réécrit ce numéro : le front n'a pas à connaître le format.
      schemaVersion: 0,
      theme: this.theme,
      mode: this.mode,
      readingSize: this.readingSize,
      readingWidth: this.readingWidth,
      editorSize: this.editorSize,
      sidebarVisible: this.sidebarVisible,
      syncScroll: this.syncScroll,
      lastFolder: this.folderPath,
      restoreLastFolder: this.restoreLastFolder,
      autoUpdate: this.autoUpdate,
      // Optionnels : absents d'une config ancienne, lus quand présents.
      sidebarWidth: this.sidebarWidth,
      tocCollapsed: this.tocCollapsed,
      tocWidth: this.tocWidth,
      splitRatio: this.splitRatio,
      outlineCollapsed: this.outlineCollapsed,
      outlineHeight: this.outlineHeight,
      followActive: this.followActive,
    };
  }

  applyConfig(c: AppConfig) {
    if (c.theme === "dark" || c.theme === "light") this.theme = c.theme;
    if (c.mode === "read" || c.mode === "split" || c.mode === "zen") this.mode = c.mode;
    if (Number.isFinite(c.readingSize)) this.readingSize = c.readingSize;
    if (Number.isFinite(c.editorSize)) this.editorSize = c.editorSize;
    if (c.readingWidth === "centered" || c.readingWidth === "full") {
      this.readingWidth = c.readingWidth;
    }
    this.sidebarVisible = !!c.sidebarVisible;
    this.syncScroll = !!c.syncScroll;
    this.restoreLastFolder = !!c.restoreLastFolder;
    this.autoUpdate = c.autoUpdate !== false;
    // Champs optionnels : une config ancienne ne les porte pas — clampNum
    // retombe alors sur le défaut, sans écraser la valeur courante.
    this.sidebarWidth = clampNum(c.sidebarWidth, 180, 480, 250);
    if (c.tocCollapsed !== undefined) this.tocCollapsed = !!c.tocCollapsed;
    this.tocWidth = clampNum(c.tocWidth, 200, 420, 240);
    this.splitRatio = clampNum(c.splitRatio, 0.2, 0.8, 0.5);
    if (c.outlineCollapsed !== undefined) this.outlineCollapsed = !!c.outlineCollapsed;
    this.outlineHeight = clampNum(c.outlineHeight, 0.2, 0.7, 0.4);
    if (c.followActive !== undefined) this.followActive = !!c.followActive;
  }

  resetSettings() {
    this.theme = "dark";
    this.mode = "split";
    this.readingSize = 17;
    this.readingWidth = "centered";
    this.editorSize = 14.5;
    this.sidebarVisible = true;
    this.syncScroll = true;
    this.restoreLastFolder = true;
    this.autoUpdate = true;
    this.sidebarWidth = 250;
    this.tocCollapsed = true;
    this.tocWidth = 240;
    this.splitRatio = 0.5;
    this.outlineCollapsed = false;
    this.outlineHeight = 0.4;
    this.followActive = true;
  }

  /** Oublie tout l'état de l'arbre : changement de dossier racine. L'état
      d'expansion d'un autre dossier n'a aucune raison de survivre. */
  resetTree() {
    this.treeExpanded = {};
    this.treeChildren = {};
  }

  /** Marque les enfants chargés comme périmés : au prochain affichage, chaque
      dossier déplié se relit depuis le disque. L'expansion, elle, survit —
      c'est tout l'intérêt de cet état indexé par chemin. */
  invalidateTree() {
    this.treeChildren = {};
  }

  /** Ouvre le document de découverte, à la demande depuis l'écran d'accueil. */
  openWelcome() {
    this.docs.push(makeDoc(WELCOME));
    this.activeIndex = this.docs.length - 1;
  }

  newBlank() {
    this.docs.push(blankDoc());
    this.activeIndex = this.docs.length - 1;
  }

  open(doc: { path: string; name: string; content: string }, opts: { forceNew?: boolean } = {}) {
    if (!opts.forceNew) {
      const existing = this.docs.findIndex((d) => d.path !== "" && d.path === doc.path);
      if (existing >= 0) {
        this.activeIndex = existing;
        return;
      }
    }
    this.docs.push({ ...makeDoc(doc.content), path: doc.path, name: doc.name });
    this.activeIndex = this.docs.length - 1;
  }

  close(index: number) {
    this.closeMany([index]);
  }

  /** Ferme un lot d'onglets et garde le document actif visé s'il survit. */
  closeMany(indices: number[]) {
    const doomed = new Set(indices);
    const survivor = this.active;
    const keep = this.docs.filter((_, i) => !doomed.has(i));

    // Plus rien d'ouvert : on retombe sur l'écran d'accueil, sans onglet.
    if (keep.length === 0) {
      this.docs = [];
      this.activeIndex = 0;
      return;
    }

    const stillThere = survivor ? keep.indexOf(survivor) : -1;
    this.docs = keep;
    this.activeIndex = stillThere >= 0 ? stillThere : keep.length - 1;
  }

  /** Indices concernés par « fermer », « fermer les autres », « fermer tout ». */
  indicesFor(scope: "one" | "others" | "all", index: number): number[] {
    if (scope === "one") return [index];
    if (scope === "all") return this.docs.map((_, i) => i);
    return this.docs.map((_, i) => i).filter((i) => i !== index);
  }

  markSaved(path: string, name: string) {
    const d = this.active;
    if (!d) return;
    d.path = path;
    d.name = name;
    d.savedContent = d.content;
  }
}

export const app = new AppState();

/** Vrai si `path` vit dans le dossier `dir`. Le test exige un séparateur
    juste après `dir` : C:\foo ne « contient » pas C:\foobar. */
export function isPathUnder(path: string, dir: string): boolean {
  if (!dir) return false;
  return (
    path.length > dir.length &&
    path.startsWith(dir) &&
    (path[dir.length] === "\\" || path[dir.length] === "/")
  );
}

/** Dossier parent d'un chemin de fichier ("C:\a\b.md" → "C:\a"). */
export function parentDir(path: string): string {
  return path.replace(/[\\/][^\\/]+$/, "");
}

/** Borne un nombre lu depuis la config : une main sur le fichier JSON (ou un
    vieux réglage) ne doit pas pouvoir casser la mise en page. Absent ou non
    fini : le défaut, sans toucher à la valeur courante. */
function clampNum(v: number | undefined, min: number, max: number, fallback: number): number {
  if (typeof v !== "number" || !Number.isFinite(v)) return fallback;
  return Math.min(max, Math.max(min, v));
}

<script lang="ts">
  import { open as openDialog, save as saveDialog, ask } from "@tauri-apps/plugin-dialog";
  // `openPath` est déjà le nom de notre ouverture de document dans l'app ;
  // celui-ci délègue au système, d'où l'alias.
  import { openUrl, openPath as openWithSystem } from "@tauri-apps/plugin-opener";
  import Icon from "$lib/components/Icon.svelte";
  import TitleBar from "$lib/components/TitleBar.svelte";
  import Toolbar from "$lib/components/Toolbar.svelte";
  import ReadingBar from "$lib/components/ReadingBar.svelte";
  import Sidebar from "$lib/components/Sidebar.svelte";
  import Outline from "$lib/components/Outline.svelte";
  import Editor from "$lib/components/Editor.svelte";
  import Preview from "$lib/components/Preview.svelte";
  import StatusBar from "$lib/components/StatusBar.svelte";
  import EmptyState from "$lib/components/EmptyState.svelte";
  import ResizeEdges from "$lib/components/ResizeEdges.svelte";
  import ContextMenu from "$lib/components/ContextMenu.svelte";
  import type { MenuItem } from "$lib/menu";
  import SettingsPanel from "$lib/components/SettingsPanel.svelte";
  import CommandPalette, {
    type PaletteCommand,
  } from "$lib/components/CommandPalette.svelte";
  import {
    appVersion,
    consumeInitialFile,
    findInDocument,
    listDir,
    listMarkdownTree,
    loadConfig,
    readDocument,
    renderMarkdown,
    replaceInDocument,
    resolveLink,
    saveConfig,
    writeDocument,
    type SearchMatch,
  } from "$lib/api";
  import { app, isPathUnder, parentDir } from "$lib/state.svelte";
  import { htmlToMarkdown } from "$lib/html2md";
  import { checkForUpdates } from "$lib/updater";
  import { listen } from "@tauri-apps/api/event";
  import FindPanel from "$lib/components/FindPanel.svelte";

  let editor = $state<ReturnType<typeof Editor> | undefined>();
  let preview = $state<ReturnType<typeof Preview> | undefined>();
  let error = $state("");

  /* ---------- configuration : charger avant d'écrire quoi que ce soit ------ */
  // Drapeau NON réactif : un `$state` ici ferait boucler l'effet sur lui-même.
  let hydrationStarted = false;

  $effect(() => {
    if (hydrationStarted) return;
    hydrationStarted = true;
    (async () => {
      try {
        const loaded = await loadConfig();
        app.applyConfig(loaded.config);
        app.configPath = loaded.path;
        app.version = await appVersion();
        if (loaded.warning) error = loaded.warning;
        if (loaded.config.restoreLastFolder && loaded.config.lastFolder) {
          // Le dossier a pu être déplacé ou supprimé depuis : ce n'est pas une
          // erreur à afficher, juste un dossier qu'on ne rouvre pas.
          try {
            await refreshFolder(loaded.config.lastFolder);
          } catch {
            /* ignoré volontairement */
          }
        }
        // Sans dossier ouvert, la barre latérale n'a rien à montrer : masquée
        // par défaut au lancement. Ouvrir un dossier la réaffiche.
        if (!app.folderPath) app.sidebarVisible = false;
      } catch {
        /* Hors Tauri (serveur Vite nu) : on reste sur les valeurs par défaut. */
      } finally {
        app.hydrated = true;
      }
      // Vérification silencieuse des mises à jour, une fois hydraté. Rien ne
      // s'affiche tant qu'il n'y a pas de mise à jour disponible.
      if (app.autoUpdate) void checkForUpdates(false);
      // Fichier passé au lancement (association Windows « ouvrir avec ») :
      // ouvert une fois hydraté. L'écoute « open-file » couvre les
      // double-clics suivants, déroutés par le plugin single-instance vers
      // l'instance existante au lieu d'en lancer une seconde.
      try {
        const initial = await consumeInitialFile();
        if (initial) await openPath(initial);
        await listen<string>("open-file", (e) => {
          void openPath(e.payload);
        });
      } catch {
        /* Hors Tauri : ni argument ni événement à traiter. */
      }
    })();
  });

  // Enregistrement débonucé. Le garde `hydrated` est essentiel : sans lui, le
  // premier rendu écraserait le fichier avec les valeurs par défaut.
  $effect(() => {
    const snapshot = app.toConfig();
    if (!app.hydrated) return;
    const handle = setTimeout(() => {
      saveConfig(snapshot).catch(() => {
        /* disque plein, droits : on ne bloque pas l'édition pour autant */
      });
    }, 400);
    return () => clearTimeout(handle);
  });

  /* ---------- rendu : debounce, jamais de reparse à chaque frappe ---------- */
  async function renderNow(source: string) {
    try {
      app.rendered = await renderMarkdown(source);
      error = "";
    } catch (e) {
      error = String(e);
    }
  }

  $effect(() => {
    const doc = app.active;
    // Aucun document : purger le rendu, sinon l'écran d'accueil garderait un
    // sommaire et des compteurs qui ne correspondent plus à rien.
    if (!doc) {
      app.rendered = null;
      return;
    }
    const source = doc.content;
    const handle = setTimeout(() => renderNow(source), 80);
    return () => clearTimeout(handle);
  });

  /* ---------- scroll synchronisé, avec verrou de direction ---------- */
  let lock: "editor" | "preview" | null = null;
  let lockTimer: ReturnType<typeof setTimeout>;

  function acquire(who: "editor" | "preview") {
    if (lock && lock !== who) return false;
    lock = who;
    clearTimeout(lockTimer);
    lockTimer = setTimeout(() => (lock = null), 150);
    return true;
  }

  function fromEditor(line: number) {
    if (!app.syncScroll || app.mode !== "split") return;
    if (!acquire("editor")) return;
    preview?.scrollToLine(line);
  }

  function fromPreview(line: number) {
    if (!app.syncScroll || app.mode !== "split") return;
    if (!acquire("preview")) return;
    editor?.scrollToLine(line);
  }

  /* ---------- historique entre documents ---------- */
  let history = $state<string[]>([]);
  let hIndex = $state(-1);
  const canBack = $derived(hIndex > 0);
  const canForward = $derived(hIndex >= 0 && hIndex < history.length - 1);

  function pushHistory(path: string) {
    if (!path || history[hIndex] === path) return;
    history = [...history.slice(0, hIndex + 1), path];
    hIndex = history.length - 1;
  }

  function activatePath(path: string) {
    const i = app.docs.findIndex((d) => d.path === path);
    if (i >= 0) app.activeIndex = i;
    else openPath(path);
  }

  function goBack() {
    if (!canBack) return;
    hIndex -= 1;
    activatePath(history[hIndex]);
  }

  function goForward() {
    if (!canForward) return;
    hIndex += 1;
    activatePath(history[hIndex]);
  }

  /* ---------- parcours du dossier, dans l'ordre de l'arborescence ---------- */
  const fileIndex = $derived(app.folderFiles.indexOf(app.active?.path ?? ""));
  const canPrevFile = $derived(app.folderFiles.length > 0 && fileIndex !== 0);
  const canNextFile = $derived(
    app.folderFiles.length > 0 && fileIndex !== app.folderFiles.length - 1,
  );

  function stepFile(dir: -1 | 1) {
    const files = app.folderFiles;
    if (files.length === 0) return;
    const i = files.indexOf(app.active?.path ?? "");
    // Document hors du dossier ouvert : on entre par le bout correspondant.
    const next = i < 0 ? (dir === 1 ? 0 : files.length - 1) : i + dir;
    if (next < 0 || next >= files.length) return;
    openPath(files[next]);
  }

  /* ---------- fichiers ---------- */
  async function openPath(path: string, opts: { newTab?: boolean } = {}) {
    try {
      const doc = await readDocument(path);
      app.open(doc, { forceNew: opts.newTab });
      pushHistory(path);
      error = "";
    } catch (e) {
      error = String(e);
    }
  }

  async function openFile() {
    let picked: string | string[] | null = null;
    try {
      picked = await openDialog({
        multiple: false,
        filters: [{ name: "Markdown", extensions: ["md", "markdown", "mdown", "mkd"] }],
      });
    } catch {
      // Serveur Vite nu : la boîte de dialogue est un plugin Tauri, absent
      // hors de l'app. Sans ce garde, le rejet serait invisible.
      error = "Ouvrir un fichier n'est disponible que dans l'application de bureau.";
      return;
    }
    if (typeof picked === "string") await openPath(picked);
  }

  async function refreshFolder(path: string) {
    app.folderEntries = await listDir(path);
    app.folderFiles = await listMarkdownTree(path);
    app.folderPath = path;
  }

  /** Révèle le document actif dans la barre latérale. Hors du dossier ouvert
      (ou sans dossier ouvert) : on charge le dossier parent du fichier —
      l'arbre ne doit pas rester déconnecté du document affiché. Dedans :
      simple expansion et sélection. */
  async function revealActiveInSidebar() {
    const doc = app.active;
    if (!doc?.path) return;
    if (!isPathUnder(doc.path, app.folderPath)) {
      try {
        await refreshFolder(parentDir(doc.path));
      } catch (e) {
        error = String(e);
        return;
      }
    }
    app.sidebarVisible = true;
    app.revealPath = doc.path;
    app.revealTick += 1;
  }

  async function openFolder() {
    let picked: string | string[] | null = null;
    try {
      picked = await openDialog({ directory: true, multiple: false });
    } catch {
      // Même garde que openFile : le dialogue est un plugin Tauri.
      error = "Ouvrir un dossier n'est disponible que dans l'application de bureau.";
      return;
    }
    if (typeof picked !== "string") return;
    try {
      await refreshFolder(picked);
      app.sidebarVisible = true;
      error = "";
    } catch (e) {
      error = String(e);
    }
  }

  // Dialogue natif commun à Enregistrer (document sans chemin) et Enregistrer
  // sous. `null` = dialogue annulé ou indisponible (hors Tauri).
  async function pickSavePath(defaultPath: string): Promise<string | null> {
    let picked: string | null = null;
    try {
      picked = await saveDialog({
        defaultPath,
        filters: [{ name: "Markdown", extensions: ["md"] }],
      });
    } catch {
      // Même garde que openFile : le dialogue est un plugin Tauri.
      error = "Enregistrer sous n'est disponible que dans l'application de bureau.";
      return null;
    }
    return typeof picked === "string" ? picked : null;
  }

  async function writeTo(path: string) {
    const doc = app.active;
    if (!doc) return;
    try {
      await writeDocument(path, doc.content);
      // L'onglet suit le nouveau chemin : après un « sous », les Ctrl+S
      // suivants écrivent à la copie, l'original reste intact.
      app.markSaved(path, path.split(/[\\/]/).pop() ?? doc.name);
      if (app.folderPath) await refreshFolder(app.folderPath);
      error = "";
    } catch (e) {
      error = String(e);
    }
  }

  async function save() {
    const doc = app.active;
    if (!doc) return;
    // `path` vaut "" (et non undefined) pour un document jamais enregistré :
    // le ?? ne déclencherait jamais le dialogue, et « Enregistrer » sur un
    // nouveau document ne ferait rien.
    const path = doc.path || (await pickSavePath(doc.name));
    if (path) await writeTo(path);
  }

  async function saveAs() {
    const doc = app.active;
    if (!doc) return;
    // Chemin courant par défaut : permet de renommer en un clic sans naviguer.
    const path = await pickSavePath(doc.path || doc.name);
    if (path) await writeTo(path);
  }

  /* ---------- recherche / remplacement (Ctrl+H) ---------- */
  let findOpen = $state(false);
  let findQuery = $state("");
  let findReplacement = $state("");
  let findCase = $state(false);
  let findWord = $state(false);
  let findMatches = $state<SearchMatch[]>([]);
  let findCurrent = $state(0);
  let findPanel = $state<ReturnType<typeof FindPanel> | undefined>(undefined);
  /** Signature de la dernière recherche qui a SÉLECTIONNÉ : la frappe dans le
      document recalcule les occurrences sans voler la sélection ; seule une
      requête ou une option différente re-sélectionne. */
  let findSignature = "";
  let findSeq = 0;

  function openFind() {
    if (app.mode !== "split") return;
    findOpen = true;
    // Ctrl+H sur un panneau déjà ouvert : recentrer la saisie.
    findPanel?.focusQuery();
  }

  function closeFind() {
    findOpen = false;
    editor?.focus();
  }

  function selectFindMatch() {
    const m = findMatches[findCurrent];
    if (m) editor?.selectRange(m.index, m.index + m.len);
  }

  // Recalcul des occurrences : debounce comme le rendu, lecture réactive de la
  // requête, des options et du contenu. Le moteur vit en Rust (Piste 2).
  $effect(() => {
    const query = findQuery;
    const options = { caseSensitive: findCase, wholeWord: findWord };
    const content = app.active?.content;
    const seq = ++findSeq;
    if (!findOpen || !query || content === undefined) {
      // Réinitialiser la signature : rouvrir le panneau avec la même requête
      // doit re-sélectionner, pas rester sur « aucun résultat » figé.
      findSignature = "";
      findMatches = [];
      findCurrent = 0;
      return;
    }
    const handle = setTimeout(async () => {
      try {
        const found = await findInDocument(content, query, options);
        // Réponse périmée : une frappe plus récente a relancé le calcul.
        if (seq !== findSeq) return;
        // Contenu modifié entre-temps : le prochain déclenchement corrigera.
        if (app.active?.content !== content) return;
        findMatches = found;
        if (findCurrent > found.length - 1) findCurrent = Math.max(0, found.length - 1);
        const signature = JSON.stringify([query, findCase, findWord]);
        if (signature !== findSignature) {
          findSignature = signature;
          selectFindMatch();
        }
      } catch (e) {
        error = String(e);
      }
    }, 80);
    return () => clearTimeout(handle);
  });

  function nextFind() {
    if (findMatches.length === 0) return;
    findCurrent = (findCurrent + 1) % findMatches.length;
    selectFindMatch();
  }

  function prevFind() {
    if (findMatches.length === 0) return;
    findCurrent = (findCurrent - 1 + findMatches.length) % findMatches.length;
    selectFindMatch();
  }

  function replaceFind() {
    const m = findMatches[findCurrent];
    const doc = app.active;
    if (!m || !doc) return;
    // Garde contre les offsets périmés : le recalcul est débouncé, un double
    // clic rapide ne doit pas écrire à un endroit qui a bougé.
    if (doc.content.slice(m.index, m.index + m.len) !== m.text) return;
    editor?.replaceRange(m.index, m.index + m.len, findReplacement);
    // Le changement de contenu relance le recalcul ; l'occurrence suivante se
    // retrouve à l'indice courant — comportement « avancer ».
  }

  async function replaceAllFind() {
    const content = app.active?.content;
    if (!content || !findQuery) return;
    try {
      const options = { caseSensitive: findCase, wholeWord: findWord };
      const next = await replaceInDocument(content, findQuery, findReplacement, options);
      if (next !== content) editor?.setContent(next);
    } catch (e) {
      error = String(e);
    }
  }

  function newDocument() {
    app.newBlank();
  }

  /* ---------- suivre les liens : c'est ce qui fait un navigateur de doc ------ */
  function splitFragment(href: string): [string, string] {
    const i = href.indexOf("#");
    if (i < 0) return [href, ""];
    return [href.slice(0, i), decodeURIComponent(href.slice(i + 1))];
  }

  async function jumpToAnchor(slug: string) {
    if (!slug) return;
    // Laisser le DOM se peindre avant de mesurer la position de l'ancre.
    await new Promise(requestAnimationFrame);
    if (!preview?.scrollToAnchor(slug)) error = `Ancre introuvable : #${slug}`;
  }

  async function handleLink(href: string) {
    // Schéma explicite : ça sort de l'app, vers le navigateur ou le client mail.
    if (/^(https?|mailto|tel):/i.test(href)) {
      try {
        await openUrl(href);
      } catch (e) {
        error = String(e);
      }
      return;
    }

    const [target, fragment] = splitFragment(href);

    // Ancre pure : on reste dans le document courant.
    if (!target) {
      jumpToAnchor(fragment);
      return;
    }

    const from = app.active?.path;
    if (!from) {
      error = "Enregistre ce document pour que ses liens relatifs soient résolus.";
      return;
    }

    try {
      const link = await resolveLink(from, target);
      if (!link.exists) {
        error = `Introuvable : ${link.path}`;
        return;
      }
      // Une image, un PDF, un .csv : ça part vers l'application système.
      if (!link.is_markdown) {
        await openWithSystem(link.path);
        return;
      }

      const doc = await readDocument(link.path);
      app.open(doc);
      pushHistory(link.path);
      // Rendu immédiat plutôt qu'attendre le debounce : l'ancre a besoin du
      // DOM final pour être mesurée.
      await renderNow(doc.content);
      if (fragment) await jumpToAnchor(fragment);
    } catch (e) {
      error = String(e);
    }
  }

  /* ---------- fermeture d'onglets ---------- */
  async function closeTabs(scope: "one" | "others" | "all", index: number) {
    const indices = app.indicesFor(scope, index);
    const unsaved = indices.filter((i) => {
      const d = app.docs[i];
      return d.content !== d.savedContent;
    });

    // Ne jamais jeter du travail sans le dire.
    if (unsaved.length > 0) {
      const names = unsaved.map((i) => app.docs[i].name).join(", ");
      const message =
        unsaved.length === 1
          ? `« ${names} » contient des modifications non enregistrées.`
          : `${unsaved.length} documents contiennent des modifications non enregistrées :\n${names}`;
      let confirmed = false;
      try {
        confirmed = await ask(message, {
          title: "Fermer sans enregistrer ?",
          kind: "warning",
          okLabel: "Fermer sans enregistrer",
          cancelLabel: "Annuler",
        });
      } catch (e) {
        error = String(e);
        return;
      }
      if (!confirmed) return;
    }

    app.closeMany(indices);
  }

  let menu = $state<{ x: number; y: number; index: number } | null>(null);
  /* ---------- barre de menus façon Zed : logo + entrées inline ----------
     Ultra-compacte dans la titlebar, chaque entrée ouvre son dropdown.
     Le logo ouvre le menu « Application » (paramètres, mises à jour…). */
  let appMenu = $state<{ label: string; x: number; y: number } | null>(null);

  const appMenus = $derived.by((): Record<string, MenuItem[]> => ({
    Fichier: [
      { label: "Nouveau document", keys: "Ctrl+N", run: newDocument },
      { label: "Ouvrir un fichier…", keys: "Ctrl+O", run: openFile },
      { label: "Ouvrir un dossier…", run: openFolder },
      {
        label: "Révéler dans la barre latérale",
        disabled: !app.active?.path,
        run: () => void revealActiveInSidebar(),
      },
      { label: "Enregistrer", keys: "Ctrl+S", disabled: !app.active, run: save },
      {
        label: "Enregistrer sous…",
        keys: "Ctrl+Shift+S",
        disabled: !app.active,
        run: saveAs,
      },
    ],
    Édition: [
      {
        label: "Annuler",
        keys: "Ctrl+Z",
        disabled: !app.active,
        run: () => editor?.undoEdit(),
      },
      {
        label: "Rétablir",
        keys: "Ctrl+Y",
        disabled: !app.active,
        run: () => editor?.redoEdit(),
      },
      { label: "Couper", disabled: !app.active, run: cutSelection },
      { label: "Copier", disabled: !app.active, run: copySelection },
      { label: "Coller", disabled: !app.active, run: pasteClipboard },
      {
        label: "Coller HTML en Markdown",
        disabled: !app.active,
        run: pasteHtmlAsMarkdown,
      },
      {
        label: "Copier le Markdown en HTML",
        disabled: !app.active,
        run: copyMarkdownAsHtml,
      },
      {
        label: "Rechercher et remplacer…",
        keys: "Ctrl+H",
        disabled: app.mode !== "split",
        run: openFind,
      },
    ],
    Affichage: [
      {
        label: "Lecture",
        keys: "Ctrl+1",
        checked: app.mode === "read",
        run: () => (app.mode = "read"),
      },
      {
        label: "Split",
        keys: "Ctrl+2",
        checked: app.mode === "split",
        run: () => (app.mode = "split"),
      },
      {
        label: "Zen",
        keys: "Ctrl+3",
        checked: app.mode === "zen",
        run: () => (app.mode = "zen"),
      },
      {
        label: "Barre de dossiers",
        keys: "Ctrl+B",
        checked: app.sidebarVisible,
        run: () => (app.sidebarVisible = !app.sidebarVisible),
      },
      {
        label: "Thème sombre",
        checked: app.theme === "dark",
        run: () => (app.theme = app.theme === "dark" ? "light" : "dark"),
      },
    ],
    Application: [
      {
        label: "Paramètres…",
        keys: "Ctrl+,",
        run: () => (app.settingsOpen = true),
      },
      {
        label: "Rechercher des mises à jour…",
        run: () => void checkForUpdates(true),
      },
      {
        label: "Palette de commandes…",
        keys: "Ctrl+K",
        run: () => (app.paletteOpen = true),
      },
    ],
  }));

  const menuBarEntries = ["Fichier", "Édition", "Affichage"];

  function openAppMenu(label: string, x: number, y: number) {
    appMenu = { label, x, y };
  }

  /* Bascule au survol : quand un menu est ouvert, glisser sur une autre
     entrée ouvre la sienne — comportement des barres de menus classiques. */
  function hoverAppMenu(label: string, x: number, y: number) {
    if (appMenu) appMenu = { label, x, y };
  }

  /* ---------- palette de commandes (Ctrl+K) : mêmes actions que les menus -- */
  const paletteCommands = $derived.by((): PaletteCommand[] => [
    { id: "new", label: "Nouveau document", icon: "plus", keys: "Ctrl+N", run: newDocument },
    { id: "open-file", label: "Ouvrir un fichier…", icon: "file", keys: "Ctrl+O", run: openFile },
    { id: "open-folder", label: "Ouvrir un dossier…", icon: "folder", run: openFolder },
    ...(app.active
      ? [
          {
            id: "save",
            label: "Enregistrer",
            icon: "save",
            keys: "Ctrl+S",
            run: save,
          } satisfies PaletteCommand,
          {
            id: "save-as",
            label: "Enregistrer sous…",
            icon: "save",
            keywords: "dupliquer copie copier renommer export",
            keys: "Ctrl+Shift+S",
            run: saveAs,
          } satisfies PaletteCommand,
          ...(app.mode === "split"
            ? [
                {
                  id: "find",
                  label: "Rechercher et remplacer…",
                  icon: "search",
                  keywords: "chercher search replace",
                  keys: "Ctrl+H",
                  run: openFind,
                } satisfies PaletteCommand,
              ]
            : []),
          {
            id: "close-tab",
            label: "Fermer l'onglet",
            icon: "close",
            keys: "Ctrl+W",
            run: () => closeTabs("one", app.activeIndex),
          } satisfies PaletteCommand,
          {
            id: "reveal",
            label: "Révéler dans la barre latérale",
            icon: "locate",
            keywords: "arborescence dossier chemin tree breadcrumb",
            run: () => void revealActiveInSidebar(),
          } satisfies PaletteCommand,
        ]
      : []),
    {
      id: "mode-read",
      label: "Basculer en mode Lecture",
      icon: "view-preview",
      keys: "Ctrl+1",
      keywords: "lire read",
      run: () => (app.mode = "read"),
    },
    {
      id: "mode-split",
      label: "Basculer en mode Split",
      icon: "view-split",
      keys: "Ctrl+2",
      run: () => (app.mode = "split"),
    },
    {
      id: "mode-zen",
      label: "Basculer en mode Zen",
      icon: "zen",
      keys: "Ctrl+3",
      keywords: "écrire focus",
      run: () => (app.mode = "zen"),
    },
    {
      id: "sidebar",
      label: "Afficher / masquer la barre de dossiers",
      icon: "sidebar",
      keys: "Ctrl+B",
      run: () => (app.sidebarVisible = !app.sidebarVisible),
    },
    {
      id: "theme",
      label: app.theme === "dark" ? "Passer au thème clair" : "Passer au thème sombre",
      icon: app.theme === "dark" ? "sun" : "moon",
      keywords: "theme dark light sombre clair",
      run: () => (app.theme = app.theme === "dark" ? "light" : "dark"),
    },
    {
      id: "settings",
      label: "Paramètres…",
      icon: "settings",
      keys: "Ctrl+,",
      run: () => (app.settingsOpen = true),
    },
  ]);

  async function revealConfig() {
    const dir = app.configPath.replace(/[\\/][^\\/]+$/, "");
    if (!dir) return;
    try {
      await openWithSystem(dir);
    } catch (e) {
      error = String(e);
    }
  }

  const menuItems = $derived.by((): MenuItem[] => {
    if (!menu) return [];
    const i = menu.index;
    return [
      { label: "Fermer", run: () => closeTabs("one", i) },
      {
        label: "Fermer les autres",
        disabled: app.docs.length < 2,
        run: () => closeTabs("others", i),
      },
      { label: "Fermer tout", separatorBefore: true, run: () => closeTabs("all", i) },
    ];
  });

  /* ---------- formatage ---------- */
  function format(
    kind:
      | "bold"
      | "italic"
      | "code"
      | "codeblock"
      | "strike"
      | "mark"
      | "link"
      | "list"
      | "quote",
  ) {
    switch (kind) {
      case "bold":
        editor?.wrap("**");
        break;
      case "italic":
        editor?.wrap("*");
        break;
      case "code":
        editor?.wrap("`");
        break;
      case "codeblock":
        editor?.codeBlock();
        break;
      case "strike":
        editor?.wrap("~~");
        break;
      case "mark":
        editor?.wrap("==");
        break;
      case "link":
        editor?.wrap("[", "](url)");
        break;
      case "list":
        editor?.prefixLines("- ");
        break;
      case "quote":
        editor?.prefixLines("> ");
        break;
    }
  }

  function gotoLine(line: number) {
    if (app.mode === "read") preview?.scrollToLine(line);
    else {
      editor?.scrollToLine(line);
      if (app.syncScroll) preview?.scrollToLine(line);
    }
  }

  /* ---------- presse-papier ---------- */
  async function copySelection() {
    const text = editor?.selectedText() ?? "";
    if (!text) return;
    try {
      await navigator.clipboard.writeText(text);
    } catch (e) {
      error = String(e);
    }
  }

  async function cutSelection() {
    const text = editor?.selectedText() ?? "";
    if (!text) return;
    try {
      await navigator.clipboard.writeText(text);
      editor?.replaceSelection("");
    } catch (e) {
      error = String(e);
    }
  }

  async function pasteClipboard() {
    try {
      const text = await navigator.clipboard.readText();
      if (text) editor?.replaceSelection(text);
    } catch {
      error = "Lecture du presse-papier impossible — coller avec Ctrl+V.";
    }
  }

  /** Colle le presse-papier en le convertissant : le HTML (copié depuis une
      page web, un mail…) devient Markdown ; un presse-papier texte pur est
      collé tel quel. */
  async function pasteHtmlAsMarkdown() {
    try {
      const items = await navigator.clipboard.read();
      const htmlItem = items.find((i) => i.types.includes("text/html"));
      const blob = htmlItem ? await htmlItem.getType("text/html") : null;
      const html = blob ? await blob.text() : "";
      if (html.trim()) {
        editor?.replaceSelection(htmlToMarkdown(html));
      } else {
        const text = await navigator.clipboard.readText();
        if (text) editor?.replaceSelection(text);
      }
    } catch {
      error = "Lecture du presse-papier impossible — coller avec Ctrl+V.";
    }
  }

  /** Copie le document rendu en HTML : un traitement de texte ou un mail le
      reçoivent mis en forme ; un éditeur de texte reçoit la source Markdown. */
  async function copyMarkdownAsHtml() {
    const doc = app.active;
    const html = app.rendered?.html ?? "";
    if (!doc || !html) return;
    try {
      await navigator.clipboard.write([
        new ClipboardItem({
          "text/html": new Blob([html], { type: "text/html" }),
          "text/plain": new Blob([doc.content], { type: "text/plain" }),
        }),
      ]);
    } catch {
      // WebView2 refusant le type text/html : le HTML brut reste utile.
      try {
        await navigator.clipboard.writeText(html);
      } catch (e) {
        error = String(e);
      }
    }
  }

  /* ---------- raccourcis ---------- */
  function onKeydown(e: KeyboardEvent) {
    // La palette capture tout quand elle est ouverte : ses flèches, son Entrée
    // et son Échap ne doivent pas déclencher les raccourcis de l'app (sinon
    // Échap dans le Zen quitterait le mode au lieu de fermer la palette).
    if (app.paletteOpen) return;

    if (e.altKey && !e.ctrlKey && !e.metaKey) {
      if (e.key === "ArrowDown") stepFile(1);
      else if (e.key === "ArrowUp") stepFile(-1);
      else return;
      e.preventDefault();
      return;
    }

    const ctrl = e.ctrlKey || e.metaKey;
    if (!ctrl) {
      // F5 rechargerait la webview (accélérateur WebView2) et ferait perdre
      // l'état en mémoire, documents non enregistrés compris. On avale.
      if (e.key === "F5") {
        e.preventDefault();
        return;
      }
      // Échap ferme d'abord la recherche ouverte (depuis l'éditeur ; depuis le
      // panneau, celui-ci ferme aussi et ce garde devient un no-op).
      if (e.key === "Escape" && findOpen) {
        e.preventDefault();
        closeFind();
        return;
      }
      // Échap ferme d'abord les paramètres : sinon on sortirait du Zen sans
      // même voir le panneau se fermer.
      if (e.key === "Escape" && app.mode === "zen" && !app.settingsOpen) app.mode = "split";
      return;
    }

    switch (e.key) {
      case "1":
        app.mode = "read";
        break;
      case "2":
        app.mode = "split";
        break;
      case "3":
        app.mode = "zen";
        break;
      case "b":
        app.sidebarVisible = !app.sidebarVisible;
        break;
      case "s":
        save();
        break;
      // Avec Maj enfoncée, `key` est la lettre en capitale.
      case "S":
        saveAs();
        break;
      case "o":
        openFile();
        break;
      case "k":
        app.paletteOpen = true;
        break;
      // WebView2 traite Ctrl+R et Ctrl+Shift+R comme un navigateur :
      // recharger la page effacerait l'état en mémoire. On avale (pareil
      // pour Ctrl+F5 dans le cas "F5" ci-dessous).
      case "r":
      case "R":
        break;
      case "F5":
        break;
      case "h":
        openFind();
        break;
      case "n":
        newDocument();
        break;
      case "w":
        if (app.active) closeTabs("one", app.activeIndex);
        break;
      case ",":
        app.settingsOpen = !app.settingsOpen;
        break;
      default:
        return;
    }
    e.preventDefault();
  }

  /* Le menu contextuel du webview (Recharger, Inspecter) n'a rien à faire dans
     une app ; on le garde là où un lecteur veut copier du texte. */
  function onContextMenu(e: MouseEvent) {
    const el = e.target as HTMLElement | null;
    if (el?.closest(".preview, .cm-editor")) return;
    e.preventDefault();
  }
</script>

<svelte:window onkeydown={onKeydown} oncontextmenu={onContextMenu} />

<div class="app" class:zen={app.mode === "zen" && !!app.active}>
  {#if !app.active}
    <!-- Aucun document : écran d'accueil, sans onglet fantôme. La barre de
         dossiers reste là si un dossier est ouvert, pour choisir un fichier. -->
    <TitleBar
      onNew={newDocument}
      onCloseTab={(i) => closeTabs("one", i)}
      onTabMenu={(index, x, y) => (menu = { index, x, y })}
      menuEntries={menuBarEntries}
      menusVisible={appMenu !== null}
      onMenu={openAppMenu}
      onMenuHover={hoverAppMenu}
    />
    <div class="body">
      {#if app.sidebarVisible}
        <Sidebar onOpenFolder={openFolder} onOpenPath={openPath} onGoto={gotoLine} />
      {/if}
      <EmptyState
        onOpenFolder={openFolder}
        onOpenFile={openFile}
        onNew={newDocument}
        onDiscover={() => app.openWelcome()}
      />
    </div>
  {:else if app.mode === "zen"}
    <!-- Décalé quand la barre de dossiers est visible, sinon elle passe dessous. -->
    <div
      class="zen-name"
      style:left={app.sidebarVisible ? "calc(var(--w-sidebar) + 26px)" : "26px"}
    >
      {#if app.dirty}<span class="zen-dot"></span>{/if}
      <span>{app.active?.path || app.active?.name}</span>
    </div>
    <div class="zen-tools">
      <!-- Le Zen n'a pas de titlebar : sans ce bouton, la barre de dossiers
           masquée ne serait rattrapable qu'au clavier ici aussi. -->
      <button
        class="icon-btn"
        onclick={() => (app.sidebarVisible = !app.sidebarVisible)}
        aria-pressed={app.sidebarVisible}
        title="Barre de dossiers — Ctrl+B"
      >
        <Icon name="sidebar" size={16} width={1.4} />
      </button>
      <button
        class="icon-btn"
        onclick={() => (app.theme = app.theme === "dark" ? "light" : "dark")}
        title="Thème clair / sombre"
      >
        <Icon name={app.theme === "dark" ? "moon" : "sun"} size={16} />
      </button>
      <button class="zen-exit" onclick={() => (app.mode = "split")}>
        <Icon name="zen-exit" size={15} />
        <span>Quitter</span>
        <span class="kbd">Esc</span>
      </button>
    </div>

    <div class="body">
      {#if app.sidebarVisible}
        <Sidebar onOpenFolder={openFolder} onOpenPath={openPath} onGoto={gotoLine} />
      {/if}
      <div class="zen-column">
        <Editor bind:this={editor} />
      </div>
    </div>

    <div class="zen-pill">
      <strong>{new Intl.NumberFormat("fr-FR").format(app.rendered?.words ?? 0)} mots</strong>
      <span class="pipe"></span>
      <span>{app.rendered?.reading_minutes ?? 0} min de lecture</span>
      <span class="pipe"></span>
      <span>Ln {app.cursorLine}</span>
    </div>
  {:else}
    <TitleBar
      onNew={newDocument}
      onCloseTab={(i) => closeTabs("one", i)}
      onTabMenu={(index, x, y) => (menu = { index, x, y })}
      menuEntries={menuBarEntries}
      menusVisible={appMenu !== null}
      onMenu={openAppMenu}
      onMenuHover={hoverAppMenu}
    />

    {#if app.mode === "read"}
      <ReadingBar
        {canBack}
        {canForward}
        onBack={goBack}
        onForward={goForward}
        {canPrevFile}
        {canNextFile}
        onPrevFile={() => stepFile(-1)}
        onNextFile={() => stepFile(1)}
      />
      <div class="body">
        {#if app.sidebarVisible}
          <!-- Le plan vit dans le rail « Sur cette page » en Lecture : la
               sidebar n'affiche que l'arborescence, jamais les deux. -->
          <Sidebar onOpenFolder={openFolder} onOpenPath={openPath} onGoto={gotoLine} withOutline={false} />
        {/if}
        <Outline onGoto={gotoLine} variant="rail" activeLine={app.cursorLine} />
        <Preview bind:this={preview} variant="read" onLink={handleLink} />
      </div>
    {:else}
      <Toolbar onOpenFolder={openFolder} onOpenFile={openFile} onSave={save} onFormat={format} />
      {#if findOpen}
        <FindPanel
          bind:this={findPanel}
          query={findQuery}
          replacement={findReplacement}
          caseSensitive={findCase}
          wholeWord={findWord}
          current={findCurrent}
          total={findMatches.length}
          onQueryChange={(v) => (findQuery = v)}
          onReplacementChange={(v) => (findReplacement = v)}
          onToggleCase={() => (findCase = !findCase)}
          onToggleWord={() => (findWord = !findWord)}
          onNext={nextFind}
          onPrev={prevFind}
          onReplace={replaceFind}
          onReplaceAll={replaceAllFind}
          onClose={closeFind}
        />
      {/if}
      <div class="body">
        {#if app.sidebarVisible}
          <Sidebar onOpenFolder={openFolder} onOpenPath={openPath} onGoto={gotoLine} />
        {/if}
        <div class="editor-col">
          <!-- Fil d'Ariane : un fichier ouvert hors du dossier en cours ne
               laissait aucune trace de son emplacement. Le bouton révèle le
               fichier dans l'arborescence (ou charge son dossier parent). -->
          <div class="crumbs">
            <Icon name="file" size={12} width={1.4} />
            <span class="crumb-path" title={app.active?.path || app.active?.name}>
              {app.active?.path || app.active?.name}
            </span>
            {#if app.active?.path}
              <button
                class="icon-btn small"
                onclick={() => void revealActiveInSidebar()}
                title="Révéler dans la barre latérale"
              >
                <Icon name="locate" size={13} width={1.4} />
              </button>
            {/if}
          </div>
          <Editor bind:this={editor} onScrollLine={fromEditor} />
        </div>
        <div class="splitter"></div>
        <Preview
          bind:this={preview}
          variant="split"
          onScrollLine={fromPreview}
          onLink={handleLink}
        />
      </div>
    {/if}
  {/if}

  <!-- Barre d'état globale : écran d'accueil et modes classiques, jamais en
       Zen (qui garde ses outils propres dans son coin). -->
  {#if !(app.mode === "zen" && !!app.active)}
    <StatusBar />
  {/if}

  <ResizeEdges />

  {#if menu}
    <ContextMenu x={menu.x} y={menu.y} items={menuItems} onClose={() => (menu = null)} />
  {/if}

  {#if appMenu}
    <ContextMenu
      x={appMenu.x}
      y={appMenu.y}
      items={appMenus[appMenu.label] ?? []}
      onClose={() => (appMenu = null)}
    />
  {/if}

  {#if app.settingsOpen}
    <SettingsPanel onClose={() => (app.settingsOpen = false)} onReveal={revealConfig} />
  {/if}

  {#if app.paletteOpen}
    <CommandPalette
      commands={paletteCommands}
      onClose={() => (app.paletteOpen = false)}
      onOpenPath={(path, newTab) => openPath(path, { newTab })}
    />
  {/if}

  {#if error}
    <div class="error" role="alert">
      <span>{error}</span>
      <button class="icon-btn" onclick={() => (error = "")} aria-label="Fermer">
        <Icon name="close" size={13} />
      </button>
    </div>
  {/if}
</div>

<style>
  .app {
    height: 100vh;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    position: relative;
  }
  .app.zen {
    background: radial-gradient(120% 100% at 50% -20%, #24252c 0%, #1b1c20 55%, #17181a 100%);
  }
  :global(:root[data-theme="light"]) .app.zen {
    background: radial-gradient(120% 100% at 50% -20%, #fdfdfe 0%, #f4f4f7 55%, #eceef1 100%);
  }

  .body {
    flex: 1;
    display: flex;
    min-height: 0;
  }

  .splitter {
    width: 1px;
    flex-shrink: 0;
    background: var(--border-strong);
    cursor: col-resize;
  }

  /* La colonne éditeur embarque son fil d'Ariane : la barre ne porte que sur
     l'éditeur, pas sur la barre latérale ni l'aperçu. */
  .editor-col {
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .crumbs {
    height: 30px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 0 8px 0 14px;
    border-bottom: 1px solid var(--border);
    color: var(--fg-3);
    font-size: 11.5px;
    background: color-mix(in oklab, var(--surface-editor) 40%, transparent);
  }
  .crumb-path {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    /* Tronque le début (la racine) plutôt que la fin : c'est la fin qui
       situe le fichier. Le survol donne le chemin complet. */
    direction: rtl;
    text-align: left;
    font-family: var(--font-mono, monospace);
  }
  .crumbs .icon-btn.small {
    width: 22px;
    height: 22px;
    border-radius: var(--r-sm);
  }

  /* ---------- Zen ---------- */
  .zen-name {
    position: absolute;
    top: 24px;
    left: 26px;
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 11.5px;
    font-weight: 450;
    color: var(--fg-4);
    letter-spacing: 0.01em;
    z-index: 3;
  }
  .zen-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent-2);
  }
  .zen-tools {
    position: absolute;
    top: 18px;
    right: 22px;
    display: flex;
    align-items: center;
    gap: 4px;
    z-index: 3;
  }
  .zen-exit {
    height: 30px;
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 0 10px;
    border-radius: var(--r-lg);
    background: var(--chip);
    box-shadow: inset 0 0 0 1px var(--border);
    font-size: 11.5px;
    font-weight: 500;
    color: var(--fg-2);
  }
  .zen-exit:hover {
    color: var(--fg-1);
    background: var(--hover);
  }

  .zen-column {
    flex: 1;
    display: flex;
    justify-content: center;
    min-height: 0;
    padding-top: 60px;
  }
  .zen-column :global(.editor) {
    flex: 0 1 780px;
    background: transparent;
    font-size: 16px;
  }
  .zen-column :global(.cm-editor) {
    background: transparent;
  }
  .zen-column :global(.cm-activeLine) {
    background: transparent;
    box-shadow: none;
  }

  .zen-pill {
    position: absolute;
    bottom: 30px;
    left: 50%;
    transform: translateX(-50%);
    display: flex;
    align-items: center;
    gap: 14px;
    height: 32px;
    padding: 0 15px;
    border-radius: var(--r-xl);
    background: var(--chip);
    box-shadow: inset 0 0 0 1px var(--border);
    font-size: 11.5px;
    color: var(--fg-3);
    z-index: 3;
  }
  .zen-pill strong {
    font-weight: 500;
    color: var(--fg-2);
  }
  .pipe {
    width: 1px;
    height: 13px;
    background: var(--border-strong);
  }

  .error {
    position: absolute;
    bottom: 44px;
    left: 50%;
    transform: translateX(-50%);
    display: flex;
    align-items: center;
    gap: 12px;
    max-width: 70%;
    padding: 10px 10px 10px 16px;
    border-radius: var(--r-xl);
    background: var(--surface-raised);
    box-shadow:
      0 0 0 1px var(--border-strong),
      0 12px 34px rgb(0 0 0 / 0.4);
    font-size: 12.5px;
    color: var(--fg-1);
    z-index: 5;
  }
</style>

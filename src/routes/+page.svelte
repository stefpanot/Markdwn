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
  import {
    appVersion,
    listDir,
    listMarkdownTree,
    loadConfig,
    readDocument,
    renderMarkdown,
    resolveLink,
    saveConfig,
    writeDocument,
  } from "$lib/api";
  import { app } from "$lib/state.svelte";

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
      } catch {
        /* Hors Tauri (serveur Vite nu) : on reste sur les valeurs par défaut. */
      } finally {
        app.hydrated = true;
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
  async function openPath(path: string) {
    try {
      const doc = await readDocument(path);
      app.open(doc);
      pushHistory(path);
      error = "";
    } catch (e) {
      error = String(e);
    }
  }

  async function openFile() {
    const picked = await openDialog({
      multiple: false,
      filters: [{ name: "Markdown", extensions: ["md", "markdown", "mdown", "mkd"] }],
    });
    if (typeof picked === "string") await openPath(picked);
  }

  async function refreshFolder(path: string) {
    app.folderEntries = await listDir(path);
    app.folderFiles = await listMarkdownTree(path);
    app.folderPath = path;
  }

  async function openFolder() {
    const picked = await openDialog({ directory: true, multiple: false });
    if (typeof picked !== "string") return;
    try {
      await refreshFolder(picked);
      app.sidebarVisible = true;
      error = "";
    } catch (e) {
      error = String(e);
    }
  }

  async function save() {
    const doc = app.active;
    if (!doc) return;
    let path = doc.path;
    if (!path) {
      const picked = await saveDialog({
        defaultPath: doc.name,
        filters: [{ name: "Markdown", extensions: ["md"] }],
      });
      if (typeof picked !== "string") return;
      path = picked;
    }
    try {
      await writeDocument(path, doc.content);
      app.markSaved(path, path.split(/[\\/]/).pop() ?? doc.name);
      if (app.folderPath) await refreshFolder(app.folderPath);
      error = "";
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
  let appMenu = $state<{ x: number; y: number } | null>(null);

  /* ---------- menu applicatif, sur le logo ---------- */
  const appMenuItems = $derived.by((): MenuItem[] => [
    { label: "Nouveau document", keys: "Ctrl+N", run: newDocument },
    { label: "Ouvrir un fichier…", keys: "Ctrl+O", run: openFile },
    { label: "Ouvrir un dossier…", run: openFolder },
    {
      label: "Enregistrer",
      keys: "Ctrl+S",
      disabled: !app.active,
      separatorBefore: true,
      run: save,
    },
    {
      label: "Lecture",
      keys: "Ctrl+1",
      checked: app.mode === "read",
      separatorBefore: true,
      run: () => (app.mode = "read"),
    },
    { label: "Split", keys: "Ctrl+2", checked: app.mode === "split", run: () => (app.mode = "split") },
    { label: "Zen", keys: "Ctrl+3", checked: app.mode === "zen", run: () => (app.mode = "zen") },
    {
      label: "Barre de dossiers",
      keys: "Ctrl+B",
      checked: app.sidebarVisible,
      separatorBefore: true,
      run: () => (app.sidebarVisible = !app.sidebarVisible),
    },
    {
      label: "Thème sombre",
      checked: app.theme === "dark",
      run: () => (app.theme = app.theme === "dark" ? "light" : "dark"),
    },
    {
      label: "Paramètres…",
      keys: "Ctrl+,",
      separatorBefore: true,
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
  function format(kind: "bold" | "italic" | "code" | "link" | "list" | "quote") {
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

  /* ---------- raccourcis ---------- */
  function onKeydown(e: KeyboardEvent) {
    if (e.altKey && !e.ctrlKey && !e.metaKey) {
      if (e.key === "ArrowDown") stepFile(1);
      else if (e.key === "ArrowUp") stepFile(-1);
      else return;
      e.preventDefault();
      return;
    }

    const ctrl = e.ctrlKey || e.metaKey;
    if (!ctrl) {
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
      case "o":
        openFile();
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
      onAppMenu={(x, y) => (appMenu = { x, y })}
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
      onAppMenu={(x, y) => (appMenu = { x, y })}
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
          <Sidebar onOpenFolder={openFolder} onOpenPath={openPath} onGoto={gotoLine} />
        {/if}
        <Outline onGoto={gotoLine} variant="rail" activeLine={app.cursorLine} />
        <Preview bind:this={preview} variant="read" onLink={handleLink} />
      </div>
    {:else}
      <Toolbar onOpenFolder={openFolder} onOpenFile={openFile} onSave={save} onFormat={format} />
      <div class="body">
        {#if app.sidebarVisible}
          <Sidebar onOpenFolder={openFolder} onOpenPath={openPath} onGoto={gotoLine} />
        {/if}
        <Editor bind:this={editor} onScrollLine={fromEditor} />
        <div class="splitter"></div>
        <Preview
          bind:this={preview}
          variant="split"
          onScrollLine={fromPreview}
          onLink={handleLink}
        />
      </div>
    {/if}

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
      items={appMenuItems}
      onClose={() => (appMenu = null)}
    />
  {/if}

  {#if app.settingsOpen}
    <SettingsPanel onClose={() => (app.settingsOpen = false)} onReveal={revealConfig} />
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

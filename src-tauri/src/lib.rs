mod config;
mod markdown;
mod search;

use serde::Serialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::Emitter;
use tauri::Manager;

/// Réceptacle du watcher de dossier (K2.8) : une seule surveillance à la
/// fois, celle du dossier racine courant. Remplacer l'option arrête
/// l'ancienne avant de poser la nouvelle.
struct FolderWatcher(Mutex<Option<notify::RecommendedWatcher>>);

#[derive(Debug, Serialize)]
pub struct LoadedConfig {
    config: config::Config,
    /// Message à montrer à l'utilisateur si le fichier a dû être écarté.
    warning: Option<String>,
    /// Chemin réel, pour que l'écran de paramètres puisse le révéler.
    path: String,
}

#[derive(Debug, Serialize)]
pub struct Document {
    path: String,
    name: String,
    content: String,
}

#[derive(Debug, Serialize)]
pub struct DirEntryInfo {
    path: String,
    name: String,
    is_dir: bool,
}

/// Registre des chemins explicitement ouverts par l'utilisateur (issue #4) :
/// `read_document`, `write_document`, `list_dir` et `list_markdown_tree`
/// refusent tout ce qui n'y figure pas, et le protocole asset: n'est plus
/// borné qu'à ces racines au lieu du `**` statique d'avant.
struct AllowedPaths(Mutex<HashSet<PathBuf>>);

/// Un chemin passe s'il EST une entrée du registre ou s'il est SOUS une
/// entrée (un dossier ouvert couvre tout son sous-arbre). Comparaison sur des
/// chemins normalisés lexicalement ; `starts_with` raisonne par composants,
/// donc « C:\notes » n'englobe jamais « C:\notes2 ».
fn is_allowed(allowed: &HashSet<PathBuf>, p: &Path) -> bool {
    let p = normalise(p);
    allowed.contains(&p) || allowed.iter().any(|root| p.starts_with(root))
}

/// Ajoute un chemin au registre et son dossier au scope du protocole asset:.
/// Partagé par `allow_path` (front) et la navigation par liens. C'est un étage
/// de défense en profondeur : le front n'appelle `allow_path` qu'après de
/// vraies actions utilisateur (dialogue, « ouvrir avec »), et bloquer
/// l'exécution de script dans la webview est le rôle de la CSP + d'ammonia.
fn register_allowed(state: &AllowedPaths, app: &tauri::AppHandle, p: &Path) -> Result<(), String> {
    let p = normalise(p);
    // Pour un dossier, c'est lui-même qui rejoint le scope ; pour un fichier,
    // c'est son parent — c'est ce qui permet aux images du document de
    // charger même sans dossier ouvert.
    let dir = if p.is_dir() {
        p.clone()
    } else {
        p.parent().map(Path::to_path_buf).unwrap_or_default()
    };
    state.0.lock().map_err(|e| e.to_string())?.insert(p);
    if !dir.as_os_str().is_empty() {
        app.asset_protocol_scope()
            .allow_directory(&dir, true)
            .map_err(|e| format!("scope asset : {e}"))?;
    }
    Ok(())
}

/// Garde des commandes de lecture/écriture : refuse un chemin hors registre.
fn ensure_allowed(state: &AllowedPaths, path: &str) -> Result<PathBuf, String> {
    let p = PathBuf::from(path);
    let allowed = state.0.lock().map_err(|e| e.to_string())?;
    if !is_allowed(&allowed, &p) {
        return Err(format!("ce chemin n'a pas été ouvert dans Markdwn : {path}"));
    }
    Ok(p)
}

/* Fil d'exécution des commandes. Une commande Tauri SANS `async` s'exécute
   sur le thread principal — celui de la boucle d'événements de la fenêtre :
   tant qu'elle tourne, l'interface est gelée (ni clic, ni défilement, ni
   repeinte). Tout ce qui touche au disque ou au rendu est donc déclaré
   `#[tauri::command(async)]` : même corps synchrone, mais exécuté sur le pool
   de threads de Tauri. Le front tient compte de l'ordre d'arrivée, qui n'est
   plus garanti (gardes de séquence dans +page.svelte).

   Restent synchrones, exprès : `write_document` et `save_config`, car deux
   écritures lancées coup sur coup doivent atterrir dans l'ordre (sinon une
   version plus ancienne pourrait écraser la plus récente) ; `watch_folder`,
   pour la même raison d'ordre ; et les commandes triviales. */

/// Enregistre un chemin explicitement ouvert par l'utilisateur — dialogue
/// (fichier, dossier, enregistrer sous) ou fichier reçu par « ouvrir avec ».
#[tauri::command(async)]
fn allow_path(
    path: String,
    state: tauri::State<'_, AllowedPaths>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    register_allowed(&state, &app, Path::new(&path))
}

/// Markdown -> HTML, plus le sommaire et les compteurs. Le front n'a aucune
/// logique de parsing : il patche le DOM à partir de ce que renvoie Rust.
#[tauri::command(async)]
fn render_markdown(source: String) -> markdown::Rendered {
    markdown::render(&source)
}

#[tauri::command(async)]
fn read_document(path: String, state: tauri::State<'_, AllowedPaths>) -> Result<Document, String> {
    let p = ensure_allowed(&state, &path)?;
    let content = std::fs::read_to_string(&p).map_err(|e| format!("{path} : {e}"))?;
    Ok(Document {
        name: file_name_of(&p),
        path,
        content,
    })
}

#[tauri::command]
fn write_document(
    path: String,
    content: String,
    state: tauri::State<'_, AllowedPaths>,
) -> Result<(), String> {
    let p = ensure_allowed(&state, &path)?;
    std::fs::write(&p, content).map_err(|e| format!("{path} : {e}"))
}

/// Occurrences d'une query dans le document courant, positions UTF-16 prêtes
/// pour CodeMirror. Socle de la recherche multi-fichiers (Piste 3).
#[tauri::command(async)]
fn find_in_document(
    source: String,
    query: String,
    options: search::SearchOptions,
) -> Vec<search::Match> {
    search::find_matches(&source, &query, options)
}

/// Document avec toutes les occurrences remplacées (littéral, non chevauchant).
#[tauri::command(async)]
fn replace_in_document(
    source: String,
    query: String,
    replacement: String,
    options: search::SearchOptions,
) -> String {
    search::replace_all(&source, &query, &replacement, options)
}

/// Premier fichier Markdown trouvé dans une liste d'arguments (ligne de
/// commande Windows, y compris celle construite par l'association « ouvrir
/// avec »). Filtre les flags et tout ce qui n'est pas un fichier existant.
fn markdown_arg(args: impl IntoIterator<Item = String>) -> Option<String> {
    args.into_iter()
        .filter(|a| !a.starts_with('-'))
        .find(|a| {
            let p = Path::new(a);
            p.is_file() && is_markdown(p)
        })
}

/// Chemin du fichier passé au lancement, consommé par la webview juste après
/// l'hydratation. `take` : un second appel ne rouvre pas le document.
#[tauri::command]
fn consume_initial_file(state: tauri::State<Mutex<Option<String>>>) -> Option<String> {
    state.lock().ok()?.take()
}

/// Dossiers et fichiers Markdown seulement — c'est un éditeur Markdown, pas un
/// explorateur de fichiers générique.
#[tauri::command(async)]
fn list_dir(path: String, state: tauri::State<'_, AllowedPaths>) -> Result<Vec<DirEntryInfo>, String> {
    let base = ensure_allowed(&state, &path)?;
    let entries = tree_entries(&base).map_err(|e| format!("{path} : {e}"))?;
    Ok(entries
        .into_iter()
        .map(|e| DirEntryInfo {
            path: e.path.to_string_lossy().into_owned(),
            name: e.name,
            is_dir: e.is_dir,
        })
        .collect())
}

/// Une entrée de dossier telle que l'arborescence l'affiche.
struct TreeEntry {
    path: PathBuf,
    name: String,
    is_dir: bool,
}

/// Dossier ou non, SANS appel système supplémentaire dans le cas courant :
/// le type vient des données déjà renvoyées par l'énumération (sous Windows,
/// FindNextFileW), alors que `Path::is_dir` refait un stat complet à chaque
/// appel. Un lien symbolique ou une jonction garde le comportement d'avant
/// (on suit la cible) : c'est le seul cas où l'on paie le stat.
fn entry_is_dir(entry: &std::fs::DirEntry) -> bool {
    match entry.file_type() {
        Ok(ft) if ft.is_symlink() => entry.path().is_dir(),
        Ok(ft) => ft.is_dir(),
        Err(_) => entry.path().is_dir(),
    }
}

/// Entrées visibles d'un dossier, dans l'ordre affiché : dossiers d'abord,
/// puis alphabétique insensible à la casse. Filtres communs à l'arbre et à la
/// liste plate : ni fichiers cachés, ni autre chose que dossiers et Markdown.
///
/// Partagé par `list_dir` et `walk`, et pensé pour les gros dossiers : le type
/// de chaque entrée est lu une fois (voir `entry_is_dir`) et la clé de tri
/// calculée une fois. L'ancienne version rappelait `is_dir()` — un appel
/// système — et `to_lowercase()` dans le comparateur, soit O(n log n) stats :
/// mesuré à ~2 à 4,7 s sur un dossier de projets réel, contre ~0,2 s ici.
fn tree_entries(dir: &Path) -> std::io::Result<Vec<TreeEntry>> {
    let mut keyed: Vec<(String, TreeEntry)> = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        // Une entrée illisible (droits, course avec une suppression) ne doit
        // pas masquer tout le reste du dossier.
        let Ok(entry) = entry else { continue };
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        let path = entry.path();
        let is_dir = entry_is_dir(&entry);
        if !is_dir && !is_markdown(&path) {
            continue;
        }
        keyed.push((name.to_lowercase(), TreeEntry { path, name, is_dir }));
    }
    keyed.sort_by(|(ka, a), (kb, b)| b.is_dir.cmp(&a.is_dir).then_with(|| ka.cmp(kb)));
    Ok(keyed.into_iter().map(|(_, e)| e).collect())
}

fn config_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map_err(|e| format!("dossier de configuration introuvable : {e}"))
}

/// Version de l'application, telle que Tauri l'a résolue au build — donc
/// depuis `Cargo.toml`, seule source. Rien à synchroniser à la main.
#[tauri::command]
fn app_version(app: tauri::AppHandle) -> String {
    app.package_info().version.to_string()
}

#[tauri::command]
fn load_config(app: tauri::AppHandle) -> Result<LoadedConfig, String> {
    let dir = config_dir(&app)?;
    let (cfg, warning) = config::load_from(&dir);
    Ok(LoadedConfig {
        config: cfg,
        warning,
        path: config::path_in(&dir).to_string_lossy().into_owned(),
    })
}

#[tauri::command]
fn save_config(app: tauri::AppHandle, config: config::Config) -> Result<(), String> {
    let dir = config_dir(&app)?;
    config::save_to(&dir, &config)
}

/// Un événement du watcher change-t-il la FORME de l'arbre ? Seuls comptent
/// créations, suppressions et renommages : l'arbre n'affiche que des noms.
///
/// C'est le filtre décisif pour les performances. Sous Windows,
/// ReadDirectoryChangesW signale une modification pour chaque écriture d'un
/// fichier ET pour son dossier parent (sa date de modification bouge). Sans ce
/// tri, la moindre activité disque dans le dossier ouvert — un build, un
/// `git status`, un simple Ctrl+S — passait le filtre des chemins (le parent
/// est un dossier) et relançait un refresh complet de l'arborescence.
fn changes_tree_shape(kind: &notify::EventKind) -> bool {
    use notify::event::ModifyKind;
    use notify::EventKind;
    match kind {
        EventKind::Create(_) | EventKind::Remove(_) => true,
        EventKind::Modify(ModifyKind::Name(_)) => true,
        // Contenu, métadonnées, accès : rien qui touche à l'arbre.
        EventKind::Modify(_) | EventKind::Access(_) => false,
        // Nature inconnue : on rafraîchit par prudence.
        EventKind::Any | EventKind::Other => true,
    }
}

/// Un composant de chemin caché (`.git`, `.obsidian`…).
fn is_hidden_component(c: std::path::Component<'_>) -> bool {
    c.as_os_str().as_encoded_bytes().first() == Some(&b'.')
}

/// Un chemin d'événement du watcher doit-il déclencher un refresh de
/// l'arborescence ? Mêmes filtres que `list_dir` : rien de ce qui vit SOUS un
/// dossier caché n'est dans l'arbre — `.git/objects/ab`, créé à chaque commit,
/// porte un nom anodin, mais son parent est caché ; seuls comptent ensuite les
/// dossiers et les Markdown. Une cible absente du disque (suppression) passe
/// dès lors qu'elle ne porte pas d'extension — un répertoire supprimé n'a pas
/// de moyen fiable de se déclarer dossier.
fn event_is_relevant(root: &Path, p: &Path) -> bool {
    let hidden = match p.strip_prefix(root) {
        Ok(rel) => rel.components().any(is_hidden_component),
        // Hors de la racine (ne devrait pas arriver) : on juge le nom seul.
        Err(_) => file_name_of(p).starts_with('.'),
    };
    if hidden {
        return false;
    }
    p.is_dir() || is_markdown(p) || (!p.exists() && p.extension().is_none())
}

/// Intervalle minimal entre deux notifications à la webview. Chaque `emit`
/// est un aller-retour vers le thread de la fenêtre : un `git checkout` qui
/// touche mille fichiers en enverrait mille. Le front débounce de 300 ms
/// APRÈS la dernière notification reçue ; un événement écarté ici survient
/// donc moins de 100 ms après une notification émise, et le refresh qu'elle
/// déclenche — au plus tôt 300 ms plus tard — le voit forcément.
const EMIT_MIN_INTERVAL: std::time::Duration = std::time::Duration::from_millis(100);

/// Surveille récursivement le dossier ouvert et notifie la webview à chaque
/// changement pertinent (événement "folder-changed", payload = la racine).
/// Le front débounce (~300 ms) : notify émet par rafales. Relancer la commande
/// sur un autre dossier arrête la surveillance précédente ; un chemin vide
/// arrête tout (fenêtre sans dossier ouvert).
#[tauri::command]
fn watch_folder(
    path: String,
    watcher: tauri::State<'_, FolderWatcher>,
    guard: tauri::State<'_, AllowedPaths>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let mut slot = watcher.0.lock().map_err(|e| e.to_string())?;
    // Toujours libérer l'ancien watcher, quel que soit le sort de la suite.
    *slot = None;
    if path.is_empty() {
        return Ok(());
    }
    let base = ensure_allowed(&guard, &path)?;
    let root = base.to_string_lossy().into_owned();
    let watched = base.clone();
    let mut last_emit: Option<std::time::Instant> = None;
    let mut w = notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
        let Ok(event) = res else { return };
        // Débordement du tampon du système : des événements ont été perdus,
        // seul un refresh complet remet l'arbre d'aplomb.
        let relevant = event.need_rescan()
            || (changes_tree_shape(&event.kind)
                && event.paths.iter().any(|p| event_is_relevant(&watched, p)));
        if !relevant {
            return;
        }
        let now = std::time::Instant::now();
        if last_emit.is_some_and(|t| now.duration_since(t) < EMIT_MIN_INTERVAL) {
            return;
        }
        last_emit = Some(now);
        // La racine est le seul identifiant dont le front a besoin :
        // c'est elle qu'il relit au complet.
        let _ = app.emit("folder-changed", root.clone());
    })
    .map_err(|e| format!("watcher : {e}"))?;
    notify::Watcher::watch(&mut w, &base, notify::RecursiveMode::Recursive)
        .map_err(|e| format!("{path} : {e}"))?;
    *slot = Some(w);
    Ok(())
}

/// Ouvre les outils de développement WebView2. DEV uniquement : en release,
/// le menu natif — leur ancienne porte d'entrée — est désactivé, et on ne
/// laisse pas de porte dérobée dans une app signée.
#[tauri::command]
fn open_devtools(window: tauri::WebviewWindow) -> Result<(), String> {
    #[cfg(debug_assertions)]
    {
        window.open_devtools();
        Ok(())
    }
    #[cfg(not(debug_assertions))]
    {
        let _ = window;
        Err("outils de développement indisponibles en release".into())
    }
}

#[derive(Debug, Serialize)]
pub struct ResolvedLink {
    /// Chemin absolu, normalisé lexicalement (les `..` sont résolus).
    path: String,
    exists: bool,
    is_markdown: bool,
}

/// Cœur partagé de la résolution : un href/src relatif au document `from`
/// devient un chemin absolu, percent-décodé et normalisé.
///
/// La normalisation est LEXICALE, pas via `canonicalize` : celui-ci exige que
/// la cible existe et renvoie des chemins préfixés `\\?\` sous Windows, ce
/// qu'on ne veut ni afficher ni comparer.
fn resolve_relative(from: &str, href: &str) -> Result<PathBuf, String> {
    let base = Path::new(from)
        .parent()
        .ok_or_else(|| format!("document sans dossier parent : {from}"))?;

    let decoded = percent_encoding::percent_decode_str(href)
        .decode_utf8()
        .map_err(|e| format!("lien mal encodé : {e}"))?;

    // Les séparateurs Markdown sont des `/`, y compris sous Windows.
    let relative = decoded.replace('/', std::path::MAIN_SEPARATOR_STR);
    Ok(normalise(&base.join(&relative)))
}

/// Résout un lien relatif d'un document Markdown vers un chemin absolu.
/// La cible est enregistrée au passage : naviguer vers un document lié
/// (« ../../notes/autre.md ») est un flux légitime, le garde des chemins ne
/// doit pas le briser.
#[tauri::command(async)]
fn resolve_link(
    from: String,
    href: String,
    state: tauri::State<'_, AllowedPaths>,
    app: tauri::AppHandle,
) -> Result<ResolvedLink, String> {
    let resolved = resolve_link_inner(&from, &href)?;
    register_allowed(&state, &app, Path::new(&resolved.path))?;
    Ok(resolved)
}

fn resolve_link_inner(from: &str, href: &str) -> Result<ResolvedLink, String> {
    let mut resolved = resolve_relative(from, href)?;

    // Beaucoup de wikis lient sans extension : `./local-setup` -> `local-setup.md`.
    if !resolved.exists() && resolved.extension().is_none() {
        let with_md = resolved.with_extension("md");
        if with_md.exists() {
            resolved = with_md;
        }
    }

    Ok(ResolvedLink {
        exists: resolved.exists(),
        is_markdown: is_markdown(&resolved),
        path: resolved.to_string_lossy().into_owned(),
    })
}

/// Une URL avec scheme (`https:`, `data:`, `asset:`…) ne doit pas être
/// réécrite. Piège : `C:\…` ressemble à un scheme d'une lettre — c'est un
/// lecteur Windows, donc un chemin absolu qu'on garde tel quel.
fn has_url_scheme(s: &str) -> bool {
    let Some(colon) = s.find(':') else {
        return false;
    };
    if colon == 1 && s.as_bytes()[0].is_ascii_alphabetic() {
        return false;
    }
    let scheme = &s[..colon];
    !scheme.is_empty()
        && scheme.starts_with(|c: char| c.is_ascii_alphabetic())
        && scheme
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
}

/// Résout le `src` d'une image du document `from` vers un chemin absolu, que
/// le front charge ensuite via le protocole `asset:` de Tauri (`convertFileSrc`).
///
/// Renvoie `None` quand il ne faut PAS toucher au src : URL distante, data URI,
/// src vide. Le front peut donc appeler cette commande sur chaque image sans
/// filtrer lui-même.
#[tauri::command]
fn resolve_asset(from: String, src: String) -> Result<Option<String>, String> {
    let src = src.trim();
    if src.is_empty() || has_url_scheme(src) {
        return Ok(None);
    }
    // Convention des éditeurs (et de GitHub) : un src « /images/x.png » est
    // relatif au document, pas à la racine du disque. Sans ce retrait, le
    // join produirait un chemin ancré à la racine du lecteur courant.
    // Exception : un chemin UNC Windows (« \\serveur\partage ») garde ses deux
    // séparateurs initiaux, il EST absolu.
    let src = if let Some(unc) = src.strip_prefix("\\\\") {
        format!("\\\\{}", unc.trim_start_matches('\\'))
    } else {
        src.trim_start_matches(['/', '\\']).to_string()
    };
    let resolved = resolve_relative(&from, &src)?;
    Ok(Some(resolved.to_string_lossy().into_owned()))
}

fn normalise(p: &Path) -> PathBuf {
    use std::path::Component;
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// Liste plate et ordonnée de tous les .md sous `path`, dans le MÊME ordre que
/// l'arborescence affichée (dossiers d'abord, puis alphabétique). C'est ce qui
/// permet « fichier suivant / précédent » sans surprise : l'ordre du clavier
/// est celui que l'œil voit.
#[tauri::command(async)]
fn list_markdown_tree(
    path: String,
    state: tauri::State<'_, AllowedPaths>,
) -> Result<Vec<String>, String> {
    let base = ensure_allowed(&state, &path)?;
    let mut out = Vec::new();
    walk(&base, 0, &mut out);
    Ok(out)
}

/// Profondeur bornée : un dossier de notes n'a pas 32 niveaux, et ça protège
/// des boucles de liens symboliques.
const MAX_DEPTH: usize = 16;
const MAX_FILES: usize = 5000;

fn walk(dir: &Path, depth: usize, out: &mut Vec<String>) {
    if depth > MAX_DEPTH || out.len() >= MAX_FILES {
        return;
    }
    let Ok(entries) = tree_entries(dir) else {
        return;
    };
    for e in entries {
        if e.is_dir {
            walk(&e.path, depth + 1, out);
        } else {
            out.push(e.path.to_string_lossy().into_owned());
        }
    }
}

fn is_markdown(p: &Path) -> bool {
    p.extension()
        .and_then(|e| e.to_str())
        .map(|e| {
            let e = e.to_ascii_lowercase();
            e == "md" || e == "markdown" || e == "mdown" || e == "mkd"
        })
        .unwrap_or(false)
}

fn file_name_of(p: &Path) -> String {
    p.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // « Ouvrir avec » sous Windows lance markdwn.exe <chemin.md> ; on le
    // récupère pour la webview. L'état partagé évite la course : la webview
    // le consomme quand elle est prête (voir consume_initial_file).
    let initial_file = markdown_arg(std::env::args().skip(1));
    // Le jeu de syntaxes se décompresse pendant que la fenêtre s'ouvre, pas
    // au premier document ouvert.
    markdown::warm_up();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            // Instance déjà ouverte : Windows lance une deuxième copie, le
            // plugin la déroute ici — on transmet le fichier à la fenêtre.
            if let Some(path) = markdown_arg(args) {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.emit("open-file", path);
                    // Et on RAPPELLE la fenêtre au premier plan : un double-clic
                    // « ouvrir avec » sur un .md doit amener l'app devant
                    // l'explorateur. Limite connue : Windows bride le vol de
                    // focus (ForegroundLockTimeout) — sans garantie à 100 %,
                    // mais correctif documenté dans la ROADMAP.
                    let _ = window.unminimize();
                    let _ = window.set_focus();
                }
            }
        }))
        // Taille et position de la fenêtre : le plugin officiel s'en charge,
        // dans son propre fichier d'état. Inutile de le refaire dans config.json.
        .plugin(tauri_plugin_window_state::Builder::default().build())
        // Auto-update : vérifie les releases GitHub (latest.json), vérifie la
        // signature minisign, télécharge, remplace et relance via le plugin process.
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(Mutex::new(initial_file))
        .manage(AllowedPaths(Mutex::new(HashSet::new())))
        .manage(FolderWatcher(Mutex::new(None)))
        .invoke_handler(tauri::generate_handler![
            render_markdown,
            read_document,
            write_document,
            allow_path,
            find_in_document,
            replace_in_document,
            consume_initial_file,
            list_dir,
            list_markdown_tree,
            watch_folder,
            resolve_link,
            resolve_asset,
            load_config,
            save_config,
            app_version,
            open_devtools
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_markdown_extensions() {
        assert!(is_markdown(Path::new("a.md")));
        assert!(is_markdown(Path::new("a.MARKDOWN")));
        assert!(!is_markdown(Path::new("a.txt")));
        assert!(!is_markdown(Path::new("a")));
    }

    #[test]
    fn normalises_dot_segments_without_touching_the_disk() {
        let root = std::env::temp_dir();
        let p = normalise(&root.join("a/b/../c/./d.md"));
        assert_eq!(p, root.join("a/c/d.md"));
    }

    #[cfg(windows)]
    #[test]
    fn normalises_windows_drive_paths() {
        let p = normalise(Path::new(r"C:\a\b\..\c\.\d.md"));
        assert_eq!(p, PathBuf::from(r"C:\a\c\d.md"));
    }

    #[test]
    fn resolves_relative_links_between_documents() {
        let root = std::env::temp_dir().join(format!("mde-link-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("docs/architecture")).unwrap();
        std::fs::write(root.join("docs/index.md"), "i").unwrap();
        std::fs::write(root.join("docs/architecture/glossary.md"), "g").unwrap();
        std::fs::write(root.join("docs/mon fichier.md"), "m").unwrap();
        let from = root.join("docs/architecture/glossary.md").to_string_lossy().into_owned();

        // remontée d'un niveau
        let r = resolve_link_inner(&from, "../index.md").unwrap();
        assert!(r.exists && r.is_markdown);
        assert_eq!(PathBuf::from(&r.path), root.join("docs/index.md"));

        // href percent-encodé
        let r = resolve_link_inner(&from, "../mon%20fichier.md").unwrap();
        assert!(r.exists, "un href percent-encodé doit être décodé");

        // sans extension
        let r = resolve_link_inner(&from, "../index").unwrap();
        assert!(r.exists, "on tente l'extension .md quand il n'y en a pas");

        // cible absente : on renvoie le chemin résolu, pas une erreur
        let r = resolve_link_inner(&from, "../absent.md").unwrap();
        assert!(!r.exists);
        assert!(r.path.ends_with("absent.md"));

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn detects_url_schemes_without_confusing_windows_drives() {
        assert!(has_url_scheme("https://exemple.test/x.png"));
        assert!(has_url_scheme("data:image/png;base64,AAAA"));
        assert!(has_url_scheme("asset://localhost/x"));
        assert!(!has_url_scheme("images/x.png"));
        assert!(!has_url_scheme("../x.png"));
        assert!(!has_url_scheme(r"C:\images\x.png"));
        assert!(!has_url_scheme("x.png"));
    }

    #[test]
    fn resolves_relative_image_sources() {
        let root = std::env::temp_dir().join(format!("mde-asset-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("docs")).unwrap();
        std::fs::write(root.join("docs/page.md"), "p").unwrap();
        let from = root.join("docs/page.md").to_string_lossy().into_owned();

        // image à côté du document
        let r = resolve_asset(from.clone(), "logo.png".into()).unwrap().unwrap();
        assert_eq!(PathBuf::from(&r), root.join("docs/logo.png"));

        // remontée + percent-encodage
        let r = resolve_asset(from.clone(), "../mon%20logo.png".into())
            .unwrap()
            .unwrap();
        assert_eq!(PathBuf::from(&r), root.join("mon logo.png"));

        // src ancré « /… » : relatif au document, pas au lecteur
        let r = resolve_asset(from.clone(), "/logo.png".into()).unwrap().unwrap();
        assert_eq!(PathBuf::from(&r), root.join("docs/logo.png"));

        // URL distante, data URI et src vide : ne pas toucher
        assert_eq!(
            resolve_asset(from.clone(), "https://exemple.test/x.png".into()).unwrap(),
            None
        );
        assert_eq!(
            resolve_asset(from.clone(), "data:image/png;base64,AAAA".into()).unwrap(),
            None
        );
        assert_eq!(resolve_asset(from, "  ".into()).unwrap(), None);

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn picks_the_markdown_file_from_launch_args() {
        let root = std::env::temp_dir().join(format!("mde-args-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("note.md"), "n").unwrap();
        std::fs::write(root.join("image.png"), "p").unwrap();
        let md = root.join("note.md").to_string_lossy().into_owned();
        let png = root.join("image.png").to_string_lossy().into_owned();

        // flags ignorés, image ignorée : seul le .md compte
        let found = markdown_arg(vec!["--devtools".to_string(), png, md.clone()]);
        assert_eq!(found.as_deref(), Some(md.as_str()));
        // rien à ouvrir
        assert_eq!(markdown_arg(vec!["--devtools".to_string()]), None);

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn walks_in_the_same_order_as_the_displayed_tree() {
        // dossiers d'abord (récursivement), puis les fichiers du niveau,
        // alphabétique et insensible à la casse — et rien qui ne soit du .md.
        let root = std::env::temp_dir().join(format!("mde-walk-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("Beta")).unwrap();
        std::fs::create_dir_all(root.join("alpha")).unwrap();
        std::fs::create_dir_all(root.join(".git")).unwrap();
        std::fs::write(root.join("zeta.md"), "z").unwrap();
        std::fs::write(root.join("Aaa.md"), "a").unwrap();
        std::fs::write(root.join("notes.txt"), "x").unwrap();
        std::fs::write(root.join("alpha/one.md"), "1").unwrap();
        std::fs::write(root.join("Beta/two.md"), "2").unwrap();
        std::fs::write(root.join(".git/hidden.md"), "h").unwrap();

        let mut out = Vec::new();
        walk(&root, 0, &mut out);
        let names: Vec<String> = out
            .iter()
            .map(|p| {
                let p = Path::new(p);
                let parent = p.parent().map(file_name_of).unwrap_or_default();
                format!("{parent}/{}", file_name_of(p))
            })
            .collect();

        assert_eq!(
            names,
            vec!["alpha/one.md", "Beta/two.md", "mde-walk/Aaa.md", "mde-walk/zeta.md"]
                .iter()
                .map(|s| s.replace("mde-walk", &file_name_of(&root)))
                .collect::<Vec<_>>()
        );

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn watcher_filters_events_like_the_tree_does() {
        let root = std::env::temp_dir().join(format!("mde-watch-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("docs")).unwrap();
        std::fs::write(root.join("a.md"), "a").unwrap();
        std::fs::write(root.join("b.txt"), "b").unwrap();

        std::fs::create_dir_all(root.join(".git").join("objects").join("ab")).unwrap();
        std::fs::write(root.join(".git").join("notes.md"), "n").unwrap();

        // Ce que l'arbre affiche : dossiers et Markdown.
        assert!(event_is_relevant(&root, &root.join("docs")));
        assert!(event_is_relevant(&root, &root.join("a.md")));
        // Ce qu'il masque : non-Markdown, fichiers cachés.
        assert!(!event_is_relevant(&root, &root.join("b.txt")));
        assert!(!event_is_relevant(&root, &root.join(".git")));
        assert!(!event_is_relevant(&root, &root.join(".obsidian").join("app.json")));
        // …et tout ce qui vit SOUS un dossier caché, même au nom anodin :
        // git crée `.git/objects/ab` à chaque commit.
        assert!(!event_is_relevant(&root, &root.join(".git").join("objects").join("ab")));
        assert!(!event_is_relevant(&root, &root.join(".git").join("notes.md")));
        // Suppressions : un .md reste identifiable à son extension ; un dossier
        // supprimé n'existe plus — on accepte au cas où (refresh conservateur).
        assert!(event_is_relevant(&root, &root.join("absent.md")));
        assert!(event_is_relevant(&root, &root.join("absent")));

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn only_structural_events_refresh_the_tree() {
        use notify::event::{
            AccessKind, CreateKind, DataChange, MetadataKind, ModifyKind, RemoveKind, RenameMode,
        };
        use notify::EventKind;

        // Ce qui change la forme de l'arbre.
        assert!(changes_tree_shape(&EventKind::Create(CreateKind::Any)));
        assert!(changes_tree_shape(&EventKind::Remove(RemoveKind::Any)));
        assert!(changes_tree_shape(&EventKind::Modify(ModifyKind::Name(RenameMode::To))));
        assert!(changes_tree_shape(&EventKind::Any));
        // Ce qui ne la change pas. `Modify(Any)` est ce que Windows émet pour
        // CHAQUE écriture — et pour le dossier parent du fichier écrit.
        assert!(!changes_tree_shape(&EventKind::Modify(ModifyKind::Any)));
        assert!(!changes_tree_shape(&EventKind::Modify(ModifyKind::Data(DataChange::Any))));
        assert!(!changes_tree_shape(&EventKind::Modify(ModifyKind::Metadata(MetadataKind::Any))));
        assert!(!changes_tree_shape(&EventKind::Access(AccessKind::Any)));
    }

    #[test]
    fn list_entries_are_sorted_and_filtered_like_the_tree() {
        let root = std::env::temp_dir().join(format!("mde-list-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("zeta")).unwrap();
        std::fs::create_dir_all(root.join("Alpha")).unwrap();
        std::fs::create_dir_all(root.join(".cache")).unwrap();
        std::fs::write(root.join("b.md"), "b").unwrap();
        std::fs::write(root.join("A.markdown"), "a").unwrap();
        std::fs::write(root.join("image.png"), "p").unwrap();
        std::fs::write(root.join(".hidden.md"), "h").unwrap();

        let got: Vec<(String, bool)> = tree_entries(&root)
            .unwrap()
            .into_iter()
            .map(|e| (e.name, e.is_dir))
            .collect();
        assert_eq!(
            got,
            vec![
                ("Alpha".to_string(), true),
                ("zeta".to_string(), true),
                ("A.markdown".to_string(), false),
                ("b.md".to_string(), false),
            ]
        );

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn path_guard_allows_only_registered_entries() {
        // Racine selon l'OS : sur Unix, « C:\notes » est un simple nom de
        // fichier relatif, et un test écrit pour Windows ne pourrait pas y
        // passer (composants vs séparateurs).
        let root = Path::new(if cfg!(windows) { r"C:\notes" } else { "/notes" });
        let base = root.parent().unwrap();
        let loose = base.join("loose").join("seul.md");

        let mut allowed = HashSet::new();
        allowed.insert(normalise(root));
        allowed.insert(normalise(&loose));

        // Un dossier ouvert couvre tout son sous-arbre…
        assert!(is_allowed(&allowed, &root.join("dossier").join("a.md")));
        // …les `..` normalisés compris…
        assert!(is_allowed(&allowed, &root.join("dossier").join("..").join("a.md")));
        // …mais jamais un voisin au nom préfixé…
        assert!(!is_allowed(&allowed, &base.join("notes2").join("a.md")));
        // …ni un fichier hors du périmètre.
        assert!(!is_allowed(&allowed, &base.join("autre").join("b.md")));

        // Une entrée fichier n'autorise que ce fichier.
        assert!(is_allowed(&allowed, &loose));
        assert!(!is_allowed(&allowed, &loose.parent().unwrap().join("autre.md")));
    }
}

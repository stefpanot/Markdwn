mod config;
mod markdown;
mod search;

use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::Emitter;
use tauri::Manager;

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

/// Markdown -> HTML, plus le sommaire et les compteurs. Le front n'a aucune
/// logique de parsing : il patche le DOM à partir de ce que renvoie Rust.
#[tauri::command]
fn render_markdown(source: String) -> markdown::Rendered {
    markdown::render(&source)
}

#[tauri::command]
fn read_document(path: String) -> Result<Document, String> {
    let p = PathBuf::from(&path);
    let content = std::fs::read_to_string(&p).map_err(|e| format!("{path} : {e}"))?;
    Ok(Document {
        name: file_name_of(&p),
        path,
        content,
    })
}

#[tauri::command]
fn write_document(path: String, content: String) -> Result<(), String> {
    std::fs::write(&path, content).map_err(|e| format!("{path} : {e}"))
}

/// Occurrences d'une query dans le document courant, positions UTF-16 prêtes
/// pour CodeMirror. Socle de la recherche multi-fichiers (Piste 3).
#[tauri::command]
fn find_in_document(
    source: String,
    query: String,
    options: search::SearchOptions,
) -> Vec<search::Match> {
    search::find_matches(&source, &query, options)
}

/// Document avec toutes les occurrences remplacées (littéral, non chevauchant).
#[tauri::command]
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
#[tauri::command]
fn list_dir(path: String) -> Result<Vec<DirEntryInfo>, String> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(&path).map_err(|e| format!("{path} : {e}"))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let p = entry.path();
        let is_dir = p.is_dir();
        let name = file_name_of(&p);
        if name.starts_with('.') {
            continue;
        }
        if !is_dir && !is_markdown(&p) {
            continue;
        }
        out.push(DirEntryInfo {
            path: p.to_string_lossy().into_owned(),
            name,
            is_dir,
        });
    }
    // Dossiers d'abord, puis alphabétique insensible à la casse.
    out.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(out)
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
#[tauri::command]
fn resolve_link(from: String, href: String) -> Result<ResolvedLink, String> {
    let mut resolved = resolve_relative(&from, &href)?;

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
#[tauri::command]
fn list_markdown_tree(path: String) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    walk(Path::new(&path), 0, &mut out);
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
    let Ok(read) = std::fs::read_dir(dir) else {
        return;
    };

    let mut entries: Vec<PathBuf> = read
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| !file_name_of(p).starts_with('.'))
        .filter(|p| p.is_dir() || is_markdown(p))
        .collect();

    entries.sort_by(|a, b| {
        b.is_dir()
            .cmp(&a.is_dir())
            .then_with(|| file_name_of(a).to_lowercase().cmp(&file_name_of(b).to_lowercase()))
    });

    for p in entries {
        if p.is_dir() {
            walk(&p, depth + 1, out);
        } else {
            out.push(p.to_string_lossy().into_owned());
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

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            // Instance déjà ouverte : Windows lance une deuxième copie, le
            // plugin la déroute ici — on transmet le fichier à la fenêtre.
            if let Some(path) = markdown_arg(args) {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.emit("open-file", path);
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
        .invoke_handler(tauri::generate_handler![
            render_markdown,
            read_document,
            write_document,
            find_in_document,
            replace_in_document,
            consume_initial_file,
            list_dir,
            list_markdown_tree,
            resolve_link,
            resolve_asset,
            load_config,
            save_config,
            app_version
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
        let r = resolve_link(from.clone(), "../index.md".into()).unwrap();
        assert!(r.exists && r.is_markdown);
        assert_eq!(PathBuf::from(&r.path), root.join("docs/index.md"));

        // href percent-encodé
        let r = resolve_link(from.clone(), "../mon%20fichier.md".into()).unwrap();
        assert!(r.exists, "un href percent-encodé doit être décodé");

        // sans extension
        let r = resolve_link(from.clone(), "../index".into()).unwrap();
        assert!(r.exists, "on tente l'extension .md quand il n'y en a pas");

        // cible absente : on renvoie le chemin résolu, pas une erreur
        let r = resolve_link(from, "../absent.md".into()).unwrap();
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
}

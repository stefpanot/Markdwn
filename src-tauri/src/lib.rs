mod config;
mod markdown;

use serde::Serialize;
use std::path::{Path, PathBuf};
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

/// Résout un lien relatif d'un document Markdown vers un chemin absolu.
///
/// La normalisation est LEXICALE, pas via `canonicalize` : celui-ci exige que
/// la cible existe et renvoie des chemins préfixés `\\?\` sous Windows, ce
/// qu'on ne veut ni afficher ni comparer.
#[tauri::command]
fn resolve_link(from: String, href: String) -> Result<ResolvedLink, String> {
    let base = Path::new(&from)
        .parent()
        .ok_or_else(|| format!("document sans dossier parent : {from}"))?;

    let decoded = percent_encoding::percent_decode_str(&href)
        .decode_utf8()
        .map_err(|e| format!("lien mal encodé : {e}"))?;

    // Les séparateurs Markdown sont des `/`, y compris sous Windows.
    let relative = decoded.replace('/', std::path::MAIN_SEPARATOR_STR);
    let mut resolved = normalise(&base.join(&relative));

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
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        // Taille et position de la fenêtre : le plugin officiel s'en charge,
        // dans son propre fichier d'état. Inutile de le refaire dans config.json.
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
            render_markdown,
            read_document,
            write_document,
            list_dir,
            list_markdown_tree,
            resolve_link,
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

//! Configuration persistée.
//!
//! Trois exigences, dans cet ordre :
//!
//! 1. **Hors du dossier d'installation** — `app_config_dir()`, donc
//!    `%APPDATA%\com.spanot.markdwn\`. Une mise à jour ne l'écrase pas.
//! 2. **Schéma versionné** — `schema_version` et des migrations explicites.
//!    Sans ça, la moindre évolution de forme casse les configs existantes.
//! 3. **Résilience** — les clés inconnues sont PRÉSERVÉES (pour ne pas
//!    détruire la config d'une version plus récente si l'utilisateur revient
//!    en arrière), les clés absentes prennent leur défaut, et un fichier
//!    illisible est mis de côté en `.bak` plutôt que supprimé.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::path::{Path, PathBuf};

/// À incrémenter à chaque changement de FORME du fichier, avec la migration
/// correspondante dans `migrate`. Ajouter un champ optionnel n'en demande pas :
/// `#[serde(default)]` s'en charge.
pub const SCHEMA_VERSION: u32 = 1;

const FILE: &str = "config.json";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Config {
    pub schema_version: u32,
    /// "dark" | "light"
    pub theme: String,
    /// "read" | "split" | "zen" — le mode au démarrage est le dernier utilisé.
    pub mode: String,
    pub reading_size: u32,
    /// "centered" | "full"
    pub reading_width: String,
    pub sidebar_visible: bool,
    pub sync_scroll: bool,
    /// Dernier dossier ouvert, réouvert au démarrage si `restore_last_folder`.
    pub last_folder: String,
    pub restore_last_folder: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            theme: "dark".into(),
            mode: "split".into(),
            reading_size: 17,
            reading_width: "centered".into(),
            sidebar_visible: true,
            sync_scroll: true,
            last_folder: String::new(),
            restore_last_folder: true,
        }
    }
}

pub fn path_in(dir: &Path) -> PathBuf {
    dir.join(FILE)
}

/// Applique les migrations de schéma sur le document brut.
///
/// On travaille sur le JSON brut et non sur `Config`, parce qu'une migration
/// doit pouvoir lire des champs qui n'existent plus dans la structure actuelle.
fn migrate(doc: &mut Map<String, Value>) {
    let from = doc
        .get("schemaVersion")
        .and_then(Value::as_u64)
        .unwrap_or(0) as u32;

    // v0 -> v1 : premier schéma versionné. Les configs d'avant n'avaient pas
    // de numéro ; leurs champs portent déjà les bons noms, rien à déplacer.
    if from < 1 {
        doc.insert("schemaVersion".into(), Value::from(1u32));
    }

    // Les migrations suivantes viennent ici, en `if from < N`.
}

/// Charge la configuration. Ne échoue jamais : au pire elle renvoie les
/// défauts et met le fichier fautif de côté.
pub fn load_from(dir: &Path) -> (Config, Option<String>) {
    let file = path_in(dir);
    let raw = match std::fs::read_to_string(&file) {
        Ok(s) => s,
        // Absence de fichier n'est pas une anomalie : premier lancement.
        Err(_) => return (Config::default(), None),
    };

    let mut doc = match serde_json::from_str::<Value>(&raw) {
        Ok(Value::Object(map)) => map,
        _ => return (Config::default(), Some(quarantine(&file))),
    };

    migrate(&mut doc);

    match serde_json::from_value::<Config>(Value::Object(doc)) {
        Ok(cfg) => (cfg, None),
        Err(_) => (Config::default(), Some(quarantine(&file))),
    }
}

/// Met un fichier illisible de côté au lieu de le perdre.
fn quarantine(file: &Path) -> String {
    let bak = file.with_extension("json.bak");
    let _ = std::fs::rename(file, &bak);
    format!(
        "Configuration illisible, remise à zéro. L'ancienne est conservée dans {}",
        bak.display()
    )
}

/// Enregistre la configuration en préservant les clés qu'on ne connaît pas.
pub fn save_to(dir: &Path, cfg: &Config) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("{} : {e}", dir.display()))?;
    let file = path_in(dir);

    // Relire le document sur disque pour conserver ses clés inconnues : c'est
    // ce qui permet d'ouvrir un fichier écrit par une version plus récente
    // sans en amputer les réglages.
    let mut doc = std::fs::read_to_string(&file)
        .ok()
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .and_then(|v| match v {
            Value::Object(m) => Some(m),
            _ => None,
        })
        .unwrap_or_default();

    let ours = serde_json::to_value(cfg).map_err(|e| e.to_string())?;
    if let Value::Object(map) = ours {
        for (k, v) in map {
            doc.insert(k, v);
        }
    }

    // Le numéro de schéma appartient à Rust, pas à l'appelant : le front n'a
    // pas à savoir quelle est la version courante du format.
    doc.insert("schemaVersion".into(), Value::from(SCHEMA_VERSION));

    let body = serde_json::to_string_pretty(&Value::Object(doc)).map_err(|e| e.to_string())?;

    // Écriture en deux temps : un fichier temporaire complet, puis remplacement.
    // `rename` échoue sous Windows si la cible existe, d'où la suppression
    // préalable — la fenêtre de risque est couverte par le `.bak` au chargement.
    let tmp = file.with_extension("json.tmp");
    std::fs::write(&tmp, body).map_err(|e| format!("{} : {e}", tmp.display()))?;
    let _ = std::fs::remove_file(&file);
    std::fs::rename(&tmp, &file).map_err(|e| format!("{} : {e}", file.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmpdir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("mde-cfg-{}-{}", tag, std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn missing_file_yields_defaults_without_complaining() {
        let d = tmpdir("missing");
        let (cfg, warn) = load_from(&d);
        assert_eq!(cfg, Config::default());
        assert!(warn.is_none(), "un premier lancement n'est pas une anomalie");
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn round_trips_through_disk() {
        let d = tmpdir("round");
        let mut cfg = Config::default();
        cfg.mode = "read".into();
        cfg.reading_width = "full".into();
        cfg.reading_size = 21;
        save_to(&d, &cfg).unwrap();
        let (back, warn) = load_from(&d);
        assert_eq!(back, cfg);
        assert!(warn.is_none());
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn absent_keys_take_their_default() {
        let d = tmpdir("partial");
        std::fs::write(path_in(&d), r#"{"schemaVersion":1,"mode":"zen"}"#).unwrap();
        let (cfg, warn) = load_from(&d);
        assert_eq!(cfg.mode, "zen");
        assert_eq!(cfg.theme, Config::default().theme);
        assert_eq!(cfg.reading_size, Config::default().reading_size);
        assert!(warn.is_none());
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn rust_owns_the_schema_version() {
        let d = tmpdir("owns");
        let mut cfg = Config::default();
        cfg.schema_version = 999; // un appelant qui se trompe
        save_to(&d, &cfg).unwrap();
        let raw = std::fs::read_to_string(path_in(&d)).unwrap();
        assert!(raw.contains(&format!("\"schemaVersion\": {SCHEMA_VERSION}")), "{raw}");
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn unknown_keys_survive_a_save() {
        // Le scénario : l'utilisateur a lancé une version plus récente, qui a
        // écrit un réglage qu'on ne connaît pas. Revenir en arrière ne doit pas
        // le détruire.
        let d = tmpdir("unknown");
        std::fs::write(
            path_in(&d),
            r#"{"schemaVersion":1,"mode":"read","reglageDuFutur":{"a":1}}"#,
        )
        .unwrap();
        let (cfg, _) = load_from(&d);
        save_to(&d, &cfg).unwrap();
        let raw = std::fs::read_to_string(path_in(&d)).unwrap();
        assert!(raw.contains("reglageDuFutur"), "clé inconnue perdue : {raw}");
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn version_zero_is_migrated_up() {
        let d = tmpdir("migrate");
        // Pas de schemaVersion : config antérieure au versionnage.
        std::fs::write(path_in(&d), r#"{"theme":"light"}"#).unwrap();
        let (cfg, warn) = load_from(&d);
        assert_eq!(cfg.schema_version, SCHEMA_VERSION);
        assert_eq!(cfg.theme, "light");
        assert!(warn.is_none());
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn corrupt_file_is_quarantined_not_destroyed() {
        let d = tmpdir("corrupt");
        std::fs::write(path_in(&d), "{ceci n'est pas du json").unwrap();
        let (cfg, warn) = load_from(&d);
        assert_eq!(cfg, Config::default());
        assert!(warn.is_some(), "l'utilisateur doit être averti");
        assert!(
            d.join("config.json.bak").exists(),
            "l'ancien fichier doit être conservé"
        );
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn wrong_type_falls_back_without_losing_the_file() {
        let d = tmpdir("wrongtype");
        std::fs::write(path_in(&d), r#"{"schemaVersion":1,"readingSize":"grand"}"#).unwrap();
        let (cfg, warn) = load_from(&d);
        assert_eq!(cfg, Config::default());
        assert!(warn.is_some());
        assert!(d.join("config.json.bak").exists());
        std::fs::remove_dir_all(&d).unwrap();
    }
}

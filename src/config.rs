//! Configuration utilisateur : ~/.config/rust-man/config.toml
//!
//! Toutes les clés sont optionnelles ; les valeurs par défaut suivent les
//! conventions Rust classiques (rustfmt, snake_case, 100 colonnes).

use crate::style::StyleConfig;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Ton nom/pseudo (pour les en-têtes de projet générés).
    pub name: Option<String>,
    /// Commande pour lancer dsh (ex: "pnpm dsh" ou "dsh").
    pub dsh_command: Option<String>,
    /// Répertoire depuis lequel lancer dsh (ex: le checkout deepseek-harness).
    pub dsh_dir: Option<String>,
    /// Modèle IA (défaut : kimi-k3).
    pub ai_model: Option<String>,
    /// Niveau d'aide IA (1 socratique .. 5 analyse complète ; défaut : 1 socratique).
    pub ai_help_level: Option<u8>,
    /// Agent pilotant la complétion de rust-nano.
    pub nano_agent: Option<String>,
    /// Modèle IA de l'éditeur (défaut : deepseek-v4.1-flash, rapide).
    pub nano_model: Option<String>,
    #[serde(flatten)]
    pub style: StyleSection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct StyleSection {
    pub max_columns: usize,
    pub max_function_lines: usize,
    pub forbid_tabs: bool,
    pub check_snake_case: bool,
    pub check_unwrap: bool,
    pub check_dbg: bool,
    pub check_todo: bool,
}

impl Default for StyleSection {
    fn default() -> Self {
        let c = StyleConfig::default();
        Self {
            max_columns: c.max_columns,
            max_function_lines: c.max_function_lines,
            forbid_tabs: c.forbid_tabs,
            check_snake_case: c.check_snake_case,
            check_unwrap: c.check_unwrap,
            check_dbg: c.check_dbg,
            check_todo: c.check_todo,
        }
    }
}

impl StyleSection {
    pub fn to_checker(&self) -> StyleConfig {
        StyleConfig {
            max_columns: self.max_columns,
            max_function_lines: self.max_function_lines,
            forbid_tabs: self.forbid_tabs,
            check_snake_case: self.check_snake_case,
            check_unwrap: self.check_unwrap,
            check_dbg: self.check_dbg,
            check_todo: self.check_todo,
        }
    }
}

pub fn config_path() -> PathBuf {
    dirs_config().join("rust-man").join("config.toml")
}

/// HOME portable : HOME, sinon USERPROFILE (Windows).
fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn data_dir() -> PathBuf {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home_dir().join(".local").join("share"));
    base.join("rust-man")
}

fn dirs_config() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home_dir().join(".config"))
}

pub fn load() -> Config {
    let path = config_path();
    match std::fs::read_to_string(&path) {
        Ok(text) => toml::from_str(&text).unwrap_or_else(|e| {
            eprintln!("rust-man: config {} invalide: {e}", path.display());
            Config::default()
        }),
        Err(_) => Config::default(),
    }
}

/// Persiste l'agent de complétion de rust-nano dans config.toml.
pub fn save_nano_agent(agent: &str) {
    save_key("nano_agent", agent);
}

/// Persiste le modèle IA de l'éditeur.
pub fn save_nano_model(model: &str) {
    save_key("nano_model", model);
}

/// Persiste le modèle IA choisi dans config.toml (sans écraser le reste).
pub fn save_ai_model(model: &str) {
    save_key("ai_model", model);
}

/// Écrit ou remplace une clé dans config.toml (préserve le reste).
fn save_key(key: &str, value: &str) {
    let path = config_path();
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let mut lines: Vec<String> = text.lines().map(|l| l.to_string()).collect();
    let new_line = format!("{key} = \"{value}\"");
    let mut done = false;
    for line in &mut lines {
        let t = line.trim_start();
        if t.starts_with(key) || t.starts_with(&format!("# {key}")) {
            *line = new_line.clone();
            done = true;
            break;
        }
    }
    if !done {
        lines.push(String::new());
        lines.push(new_line);
    }
    let _ = std::fs::write(&path, lines.join("\n") + "\n");
}

/// Crée le fichier de config avec les valeurs commentées si absent.
pub fn ensure_exists() -> std::io::Result<PathBuf> {
    let path = config_path();
    if path.exists() {
        return Ok(path);
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, DEFAULT_CONFIG)?;
    Ok(path)
}

pub const DEFAULT_CONFIG: &str = r##"# Configuration rust-man

# Ton identité (pour les en-têtes de projet) :
# name = "Prénom Nom"

# Intégration DSH (détectée automatiquement si dsh est dans le PATH) :
# dsh_command = "pnpm dsh"       # ou "dsh" si installé globalement
# dsh_dir = "~/deepseek-harness" # répertoire de travail pour lancer dsh

# IA (Kimi K3 via le provider du projet, contexte 1M / sortie 128K) :
# ai_model = "kimi-k3"
# ai_help_level = 1              # 1 socratique (défaut) .. 5 analyse complète

# Règles de style Rust (valeurs = défauts des conventions officielles) :
# max_columns = 100
# max_function_lines = 60
# forbid_tabs = true
# check_snake_case = true
# check_unwrap = true
# check_dbg = true
# check_todo = true
"##;

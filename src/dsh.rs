//! Intégration discrète de DSH (DeepSeek-Harness).
//!
//! Détection : `dsh` dans le PATH, sinon config (dsh_command/dsh_dir),
//! sinon heuristique ~/deepseek-harness + pnpm.
//! Lancement : handoff complet du terminal (comme `!cmd` dans less), avec
//! retour propre vers rust-man.

use crate::config::Config;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct DshHandle {
    /// Commande (ex: "dsh" ou "pnpm dsh").
    pub command: String,
    /// Répertoire de lancement.
    pub dir: Option<PathBuf>,
    /// Comment ça a été trouvé (pour l'affichage debug).
    pub source: &'static str,
}

/// Détecte dsh : PATH → config → checkout connu.
pub fn detect(cfg: &Config) -> Option<DshHandle> {
    // 1. dsh dans le PATH
    if let Ok(out) = Command::new("dsh").arg("--version").output() {
        if out.status.success() {
            return Some(DshHandle {
                command: "dsh".into(),
                dir: None,
                source: "PATH",
            });
        }
    }
    // 2. config explicite
    if let Some(cmd) = &cfg.dsh_command {
        let dir = cfg.dsh_dir.as_ref().map(|d| {
            let expanded = d.replace('~', &std::env::var("HOME").unwrap_or_default());
            PathBuf::from(expanded)
        });
        return Some(DshHandle {
            command: cmd.clone(),
            dir,
            source: "config",
        });
    }
    // 3. checkout deepseek-harness connu
    let home = std::env::var("HOME").unwrap_or_default();
    for candidate in [
        format!("{home}/deepseek-harness"),
        format!("{home}/Documents/deepseek-harness"),
    ] {
        let dir = PathBuf::from(&candidate);
        if dir.join("apps/cli/src/bin.ts").exists() {
            return Some(DshHandle {
                command: "pnpm dsh".into(),
                dir: Some(dir),
                source: "checkout détecté",
            });
        }
    }
    None
}

impl DshHandle {
    /// Construit la commande de lancement (pour handoff terminal).
    pub fn spawn_command(&self, args: &[String]) -> Command {
        let mut parts = self.command.split_whitespace();
        let mut cmd = Command::new(parts.next().unwrap_or("dsh"));
        cmd.args(parts).args(args);
        if let Some(dir) = &self.dir {
            cmd.current_dir(dir);
        }
        cmd
    }

    /// Lance le profil headless avec un prompt enrichi du contexte projet.
    pub fn spawn_headless(&self, prompt: &str, project_dir: &std::path::Path) -> Command {
        let full_prompt = format!(
            "(Contexte : projet Rust dans {}) {}",
            project_dir.display(),
            prompt
        );
        self.spawn_command(&["headless".to_string(), full_prompt])
    }

    /// Vérifie que la commande est lançable.
    pub fn check(&self) -> Result<(), String> {
        self.spawn_command(&["--version".to_string()])
            .output()
            .map(|_| ())
            .map_err(|e| format!("{} : {e}", self.command))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_ne_panique_pas() {
        // Sur une machine sans dsh ni config : None, sans crash.
        let cfg = Config::default();
        let _ = detect(&cfg);
    }

    #[test]
    fn spawn_command_construit() {
        let h = DshHandle {
            command: "pnpm dsh".into(),
            dir: Some(PathBuf::from("/tmp")),
            source: "test",
        };
        let _ = h.spawn_command(&["--help".to_string()]);
    }
}

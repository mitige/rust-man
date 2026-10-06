//! Compilation intégrée : lance rustc/cargo, capture et explique les diagnostics,
//! pointe vers la fiche rust-man pertinente.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Binaire temporaire produit par compile_target (portable : temp_dir + .exe sous Windows).
pub fn temp_bin() -> PathBuf {
    std::env::temp_dir().join(format!("rust-man-build{}", std::env::consts::EXE_SUFFIX))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagLevel {
    Error,
    Warning,
    Note,
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub file: String,
    pub line: usize,
    pub col: usize,
    pub level: DiagLevel,
    pub message: String,
    /// Fiche rust-man recommandée (id).
    pub hint_fiche: Option<&'static str>,
    /// Explication courte en français.
    pub hint_text: Option<&'static str>,
}

#[derive(Debug)]
pub struct CompileResult {
    pub success: bool,
    pub diagnostics: Vec<Diagnostic>,
    pub raw: String,
}

/// Parse la sortie rustc/cargo :
/// ```text
/// error[E0382]: borrow of moved value: `s`
///  --> src/main.rs:12:5
/// ```
pub fn parse_diagnostics(output: &str) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    let mut pending: Option<(DiagLevel, String)> = None;
    for line in output.lines() {
        let t = line.trim_start();
        // ligne d'en-tête de diagnostic
        if t.starts_with("error[") || t.starts_with("error:") || t == "error" {
            let msg = t
                .split_once(": ")
                .map(|(_, m)| m.to_string())
                .unwrap_or_else(|| t.to_string());
            pending = Some((DiagLevel::Error, msg));
            continue;
        }
        if t.starts_with("warning: ") && !t.contains("generated") {
            pending = Some((DiagLevel::Warning, t["warning: ".len()..].to_string()));
            continue;
        }
        // ligne de localisation --> fichier:ligne:col
        if let Some(rest) = t.strip_prefix("-->") {
            if let Some((level, message)) = pending.take() {
                let loc = rest.trim();
                let mut parts = loc.rsplitn(3, ':');
                let col = parts.next().and_then(|c| c.trim().parse().ok()).unwrap_or(0);
                let line_no = parts.next().and_then(|l| l.trim().parse().ok()).unwrap_or(0);
                let file = parts.next().unwrap_or("?").trim().to_string();
                let (hint_fiche, hint_text) = explain_diagnostic(&message);
                diags.push(Diagnostic {
                    file,
                    line: line_no,
                    col,
                    level,
                    message,
                    hint_fiche,
                    hint_text,
                });
            }
            continue;
        }
        // `help:` / `note:` attachés au dernier diag : on les ignore (le brut reste dispo)
    }
    // un diagnostic sans localisation (ex: erreur cargo) : on le garde quand même
    if let Some((level, message)) = pending.take() {
        let (hint_fiche, hint_text) = explain_diagnostic(&message);
        diags.push(Diagnostic {
            file: String::new(),
            line: 0,
            col: 0,
            level,
            message,
            hint_fiche,
            hint_text,
        });
    }
    diags
}

/// Associe un message rustc à une explication + fiche rust-man.
fn explain_diagnostic(msg: &str) -> (Option<&'static str>, Option<&'static str>) {
    let m = msg.to_lowercase();
    let rule: &[(&[&str], &str, &str)] = &[
        (&["use of moved value", "borrow of moved value", "cannot move out of"], "ownership",
         "La valeur a changé de propriétaire (move) : elle n'existe plus ici. Emprunte (&), clone, ou restructure."),
        (&["cannot borrow", "as mutable because it is also borrowed", "more than one mutable"], "emprunts-mutables",
         "Règle d'or : plusieurs emprunts & OU un seul &mut. Réduis la portée de l'emprunt gênant."),
        (&["does not live long enough", "returns a value referencing", "missing lifetime", "borrowed value"], "lifetimes",
         "Une référence survivrait à sa donnée : la durée de vie doit couvrir tous les usages. Ne retourne jamais une référence vers une variable locale."),
        (&["cannot assign twice to immutable", "immutable variable"], "variables-mutabilite",
         "Immutable par défaut : déclare avec `let mut` si tu dois réassigner."),
        (&["mismatched types"], "types-scalaires",
         "Rust ne convertit JAMAIS implicitement : i32 ≠ u32 ≠ usize ≠ &str ≠ String. Convertis explicitement (as, .into(), .to_string())."),
        (&["cannot find", "unresolved import", "unresolved name", "use of undeclared"], "modules-use",
         "Nom introuvable : mauvais chemin de module, oubli de `use`, ou item non `pub`. Vérifie l'arborescence mod/use."),
        (&["no method named", "no field named"], "methodes-impl",
         "Méthode/champ inconnu sur ce type : faute de frappe, trait pas importé (`use`), ou méthode d'un autre type."),
        (&["doesn't implement", "the trait", "is not implemented"], "trait-bounds",
         "Le type ne fournit pas la capacité demandée : ajoute un #[derive(...)], un impl, ou une borne T: Trait sur le générique."),
        (&["type annotations needed", "cannot infer"], "generics",
         "Le compilateur ne peut pas deviner le type : annote (let x: Vec<i32> = ...) ou utilise le turbofish ::<T>."),
        (&["cannot index"], "chaines-bases",
         "Pas d'indexation directe sur String/&str (UTF-8 = taille variable par caractère) : utilise .chars().nth(i) ou des slices sur des frontières valides."),
        (&["recursive type", "infinite size"], "structs",
         "Un type ne peut pas se contenir lui-même directement (taille infinie) : passe par Box<T> pour l'indirection."),
        (&["expected one of", "expected expression", "unexpected token", "unclosed delimiter", "mismatched closing"], "lire-erreur-rustc",
         "Erreur de syntaxe : accolade/parenthèse déséquilibrée, ; manquant à la ligne PRÉCÉDENTE, ou virgule en trop. Lis le message du compilateur, il indique souvent le fix."),
        (&["unused variable", "never used", "never read", "unused import"], "idiomes-rust",
         "Code mort : supprime, utilise, ou préfixe d'un underscore (_x) si c'est volontaire."),
        (&["is private", "private"], "modules-use",
         "Privé par défaut en Rust : ajoute `pub` devant l'item pour l'exposer hors de son module."),
        (&["index out of bounds", "out of range"], "tableaux",
         "Indexation hors limites : préfère .get(i) qui renvoie Option, ou vérifie len() avant."),
        (&["overflow", "attempt to add with overflow", "attempt to multiply with overflow"], "types-scalaires",
         "Dépassement d'entier : en debug ça panique. Utilise checked_add/checked_mul, ou un type plus large."),
        (&["temporary value"], "lifetimes",
         "Une valeur temporaire meurt en fin d'instruction : stocke-la dans une variable nommée avant d'en emprunter une référence."),
        (&["expected function", "not a function", "cannot call"], "fonctions",
         "Tu appelles quelque chose qui n'est pas appelable : vérifie le type (une closure ? un champ ?)."),
        (&["non-exhaustive", "not covered", "pattern"], "match",
         "match non exhaustif : couvre tous les cas (ou ajoute le bras joker `_`)."),
        (&["failed to resolve", "can't find crate", "no such file"], "cargo-toml",
         "Crate/module introuvable : déclare la dépendance dans Cargo.toml (cargo add <crate>) ou vérifie le nom du module."),
        (&["explicit panic", "panicked"], "panic",
         "panic! : erreur irrécupérable. Pour les erreurs attendues, préfère Result et l'opérateur ?."),
    ];
    for (needles, fiche, text) in rule {
        if needles.iter().any(|n| m.contains(n)) {
            return (Some(fiche), Some(text));
        }
    }
    (None, None)
}

/// Compile/vérifie un fichier .rs ou un projet cargo.
pub fn compile_target(target: &Path) -> CompileResult {
    let (cmd, args, cwd): (String, Vec<String>, Option<std::path::PathBuf>) = if target.is_dir()
        && target.join("Cargo.toml").exists()
    {
        // projet cargo : check rapide (pas de binaire produit)
        (
            "cargo".into(),
            vec!["check".into()],
            Some(target.to_path_buf()),
        )
    } else if target.is_dir() {
        // dossier sans Cargo.toml : tente main.rs ou src/main.rs
        let main = if target.join("src/main.rs").exists() {
            target.join("src/main.rs")
        } else if target.join("main.rs").exists() {
            target.join("main.rs")
        } else {
            return CompileResult {
                success: false,
                diagnostics: vec![],
                raw: format!(
                    "pas de Cargo.toml ni de main.rs dans {}.\n\
                     → crée un projet : rust-man new-projet <nom>",
                    target.display()
                ),
            };
        };
        (
            "rustc".into(),
            vec![
                "--edition".into(),
                "2021".into(),
                "-o".into(),
                temp_bin().to_string_lossy().into(),
                main.to_string_lossy().into(),
            ],
            Some(target.to_path_buf()),
        )
    } else {
        (
            "rustc".into(),
            vec![
                "--edition".into(),
                "2021".into(),
                "-o".into(),
                temp_bin().to_string_lossy().into(),
                target.to_string_lossy().into(),
            ],
            None,
        )
    };
    let result = Command::new(&cmd)
        .args(&args)
        .current_dir(cwd.as_deref().unwrap_or_else(|| Path::new(".")))
        .output();
    match result {
        Ok(out) => {
            let raw = format!(
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
            let diagnostics = parse_diagnostics(&raw);
            CompileResult {
                success: out.status.success(),
                diagnostics,
                raw,
            }
        }
        Err(e) => CompileResult {
            success: false,
            diagnostics: vec![],
            raw: format!(
                "impossible de lancer {cmd}: {e}\n(Rust installé ? → rust-man installer-rust)"
            ),
        },
    }
}

/// Lance les tests d'un projet cargo.
pub fn run_cargo_test(dir: &Path) -> CompileResult {
    let result = Command::new("cargo")
        .args(["test"])
        .current_dir(dir)
        .output();
    match result {
        Ok(out) => {
            let raw = format!(
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
            CompileResult {
                success: out.status.success(),
                diagnostics: parse_diagnostics(&raw),
                raw,
            }
        }
        Err(e) => CompileResult {
            success: false,
            diagnostics: vec![],
            raw: format!("impossible de lancer cargo: {e}\n(Rust installé ? → rust-man installer-rust)"),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_erreur_rustc() {
        let out = "error[E0382]: borrow of moved value: `s`\n --> src/main.rs:4:5\n  |\n4 |     println!(\"{}\", s);\n  |     ^^^^^ value borrowed here after move\n";
        let diags = parse_diagnostics(out);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].level, DiagLevel::Error);
        assert_eq!(diags[0].file, "src/main.rs");
        assert_eq!(diags[0].line, 4);
        assert_eq!(diags[0].col, 5);
        assert_eq!(diags[0].hint_fiche, Some("ownership"));
    }

    #[test]
    fn parse_warning_rustc() {
        let out = "warning: unused variable: `x`\n --> src/main.rs:2:9\n";
        let diags = parse_diagnostics(out);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].level, DiagLevel::Warning);
        assert_eq!(diags[0].hint_fiche, Some("idiomes-rust"));
    }

    #[test]
    fn parse_borrow_mutable() {
        let out = "error[E0502]: cannot borrow `v` as mutable because it is also borrowed as immutable\n --> src/lib.rs:9:5\n";
        let diags = parse_diagnostics(out);
        assert_eq!(diags[0].hint_fiche, Some("emprunts-mutables"));
    }

    #[test]
    fn bruit_ignore() {
        let diags = parse_diagnostics("   Compiling test v0.1.0\n    Finished dev profile\n");
        assert!(diags.is_empty());
    }
}

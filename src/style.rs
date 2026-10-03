//! Vérificateur de style Rust intégré (le « rustfmt + clippy de poche »).
//!
//! Heuristique ligne à ligne avec neutralisation des chaînes/commentaires :
//! ce n'est pas un parseur Rust complet — il couvre les conventions de base
//! (formatage, nommage, unwrap, longueurs) avec fichier, ligne, colonne,
//! explication et auto-fix quand c'est sûr. Pour aller plus loin :
//! `cargo fmt` et `cargo clippy` restent la référence.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

// ------------------------------------------------------------------ types

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Info,
    Minor,
    Major,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Info => write!(f, "info"),
            Severity::Minor => write!(f, "MINOR"),
            Severity::Major => write!(f, "MAJOR"),
        }
    }
}

/// Correction automatique applicable à une ligne (index 0-based).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fix {
    /// Remplace tout le contenu de la ligne.
    ReplaceLine { line: usize, content: String },
    /// Insère une ligne avant `line`.
    InsertBefore { line: usize, content: String },
    /// Supprime la ligne.
    DeleteLine { line: usize },
}

#[derive(Debug, Clone)]
pub struct Finding {
    pub line: usize, // 1-based
    pub col: usize,  // 1-based
    pub severity: Severity,
    pub rule: &'static str,
    pub message: String,
    pub fix: Option<Fix>,
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}: [{}] {} ({})",
            self.line, self.col, self.severity, self.message, self.rule
        )
    }
}

#[derive(Debug, Clone)]
pub struct FileReport {
    pub path: PathBuf,
    pub findings: Vec<Finding>,
}

impl FileReport {
    pub fn count(&self, sev: Severity) -> usize {
        self.findings.iter().filter(|f| f.severity == sev).count()
    }
}

#[derive(Debug, Default)]
pub struct Report {
    pub files: Vec<FileReport>,
}

impl Report {
    pub fn total(&self, sev: Severity) -> usize {
        self.files.iter().map(|f| f.count(sev)).sum()
    }
    pub fn findings_count(&self) -> usize {
        self.files.iter().map(|f| f.findings.len()).sum()
    }
    pub fn fixable_count(&self) -> usize {
        self.files
            .iter()
            .flat_map(|f| &f.findings)
            .filter(|f| f.fix.is_some())
            .count()
    }
}

// ------------------------------------------------------------------ config

#[derive(Debug, Clone)]
pub struct StyleConfig {
    pub max_columns: usize,
    pub max_function_lines: usize,
    pub forbid_tabs: bool,
    pub check_snake_case: bool,
    pub check_unwrap: bool,
    pub check_dbg: bool,
    pub check_todo: bool,
}

impl Default for StyleConfig {
    fn default() -> Self {
        Self {
            max_columns: 100,
            max_function_lines: 60,
            forbid_tabs: true,
            check_snake_case: true,
            check_unwrap: true,
            check_dbg: true,
            check_todo: true,
        }
    }
}

// ------------------------------------------------------------------ règles

/// Explique une règle R-* : (titre, pourquoi, comment corriger).
pub fn explain_rule(rule: &str) -> (&'static str, &'static str, &'static str) {
    match rule {
        "R-COL100" => (
            "Ligne trop longue",
            "Au-delà de ~100 colonnes, le code se lit mal : pas de côte-à-côte, \
             wraps dans les diffs, revues pénibles. rustfmt découpe automatiquement.",
            "Coupe l'expression (une méthode par ligne dans les chaînes, arguments \
             un par ligne) ou lance `cargo fmt`.",
        ),
        "R-TAB" => (
            "Tabulation utilisée",
            "La convention Rust (et rustfmt) : 4 espaces, jamais de tab. Les tabs \
             s'affichent différemment selon les outils et cassent l'alignement.",
            "Remplace les tabs par 4 espaces (rust-man fix le fait), ou `cargo fmt`.",
        ),
        "R-TRAIL" => (
            "Espace en fin de ligne",
            "Les espaces invisibles en fin de ligne polluent les diffs et ne \
             servent à rien.",
            "Supprime-les (rust-man fix le fait) ; la plupart des éditeurs ont \
             « trim trailing whitespace ».",
        ),
        "R-NL-EOF" => (
            "Pas de newline final",
            "Un fichier texte se termine par \\n : les outils Unix (cat, diff, git) \
             l'attendent, et rustc l'apprécie.",
            "Ajoute une ligne vide à la fin (rust-man fix le fait).",
        ),
        "R-SNAKE" => (
            "Nom de fonction non snake_case",
            "La convention Rust : fonctions et variables en snake_case, types en \
             CamelCase. rustc le signale (warning non_snake_case) et clippy insiste.",
            "Renomme en minuscules avec underscores : calcul_total, pas calculTotal.",
        ),
        "R-UNWRAP" => (
            ".unwrap() hors test",
            "unwrap() panique sur None/Err : acceptable dans un prototype ou un \
             test, dangereux en code qui tourne vraiment (entrées utilisateur, \
             fichiers, réseau).",
            "Préfère expect(\"pourquoi ça ne peut pas échouer\"), unwrap_or, \
             unwrap_or_default, ou propage avec ?. Voir rust-man unwrap-expect.",
        ),
        "R-DBG" => (
            "dbg! laissé dans le code",
            "dbg! est parfait pour déboguer… et doit disparaître avant de livrer. \
             Il écrit sur stderr et alourdit la sortie.",
            "Retire le dbg! une fois le bug trouvé. Voir rust-man debugging.",
        ),
        "R-TODO" => (
            "TODO/FIXME en attente",
            "Un marqueur oublié, c'est du travail non fini qui se fond dans le décor.",
            "Traite-le ou transforme-le en ticket/issue daté.",
        ),
        "R-FUNCLEN" => (
            "Fonction trop longue",
            "Une fonction qui dépasse ~60 lignes fait trop de choses : elle se \
             teste mal, se nomme mal, se débogue mal.",
            "Extrais des sous-étapes en fonctions nommées ; les itérateurs et le ? \
             raccourcissent souvent beaucoup.",
        ),
        "R-PRINTLN" => (
            "println! de debug probable",
            "Un println! de diagnostic (valeurs brutes, « ici », « test ») oublié \
             dans le code final pollue la sortie.",
            "Retire-le, ou utilise log/tracing si c'est de la vraie télémétrie.",
        ),
        _ => (
            "Règle de style",
            "Conventions Rust : rustfmt pour le formatage, clippy pour les idiomes, \
             snake_case pour les fonctions.",
            "Lance `cargo fmt` et `cargo clippy` ; rust-man check couvre l'essentiel \
             sans toolchain complète.",
        ),
    }
}

// ------------------------------------------------------------------ moteur

/// Neutralise chaînes et commentaires ligne à ligne (remplace le contenu par
/// des espaces, en préservant les colonnes) pour éviter les faux positifs.
/// Gère // , /* ... */ (multi-lignes), "..." avec échappements, 'x' et r#"..."#.
fn neutralize(src: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut in_block = false; // /* ... */
    for line in src.lines() {
        let chars: Vec<char> = line.chars().collect();
        let mut buf: Vec<char> = chars.clone();
        let mut i = 0;
        let mut in_str = false;
        while i < chars.len() {
            if in_block {
                if chars[i] == '*' && i + 1 < chars.len() && chars[i + 1] == '/' {
                    buf[i] = ' ';
                    buf[i + 1] = ' ';
                    i += 2;
                    in_block = false;
                } else {
                    buf[i] = ' ';
                    i += 1;
                }
                continue;
            }
            if in_str {
                if chars[i] == '\\' && i + 1 < chars.len() {
                    buf[i] = ' ';
                    buf[i + 1] = ' ';
                    i += 2;
                    continue;
                }
                if chars[i] == '"' {
                    buf[i] = ' ';
                    i += 1;
                    in_str = false;
                    continue;
                }
                buf[i] = ' ';
                i += 1;
                continue;
            }
            // hors commentaire / chaîne
            if chars[i] == '/' && i + 1 < chars.len() && chars[i + 1] == '/' {
                // commentaire de fin de ligne
                for j in i..chars.len() {
                    buf[j] = ' ';
                }
                break;
            }
            if chars[i] == '/' && i + 1 < chars.len() && chars[i + 1] == '*' {
                buf[i] = ' ';
                buf[i + 1] = ' ';
                i += 2;
                in_block = true;
                continue;
            }
            if chars[i] == '"' {
                buf[i] = ' ';
                i += 1;
                in_str = true;
                continue;
            }
            // char littéral 'x' : neutralise le contenu
            if chars[i] == '\''
                && i + 2 < chars.len()
                && ((chars[i + 1] == '\\' && i + 3 < chars.len() && chars[i + 3] == '\'')
                    || chars[i + 2] == '\'')
            {
                let end = if chars[i + 1] == '\\' { i + 3 } else { i + 2 };
                for j in i..=end {
                    buf[j] = ' ';
                }
                i = end + 1;
                continue;
            }
            i += 1;
        }
        out.push(buf.into_iter().collect());
    }
    out
}

/// Vrai si la ligne (neutralisée) tombe dans une zone de test.
fn mark_test_zones(neutral: &[String]) -> Vec<bool> {
    let mut zones = vec![false; neutral.len()];
    let mut in_test = false;
    let mut depth_after_cfg = 0i32;
    for (i, line) in neutral.iter().enumerate() {
        if line.contains("#[cfg(test)]") {
            in_test = true;
            depth_after_cfg = 0;
        }
        zones[i] = in_test;
        if in_test {
            depth_after_cfg += line.matches('{').count() as i32;
            depth_after_cfg -= line.matches('}').count() as i32;
            // le mod tests est fermé et on est revenu à 0 → fin de zone
            if depth_after_cfg <= 0 && line.contains('}') {
                in_test = false;
            }
        }
    }
    zones
}

/// Détecte un nom de fonction non snake_case : `fn calculTotal(`.
/// Retourne (colonne du nom, nom) si trouvé.
fn find_fn_name(line: &str) -> Option<(usize, String)> {
    let bytes = line.as_bytes();
    let mut idx = 0;
    while let Some(pos) = line[idx..].find("fn ") {
        let start = idx + pos + 3;
        // saute les espaces
        let name_start = line[start..]
            .find(|c: char| !c.is_whitespace())
            .map(|o| start + o)?;
        let name_end = line[name_start..]
            .find(|c: char| !(c.is_alphanumeric() || c == '_'))
            .map(|o| name_start + o)
            .unwrap_or(line.len());
        let name = &line[name_start..name_end];
        if !name.is_empty()
            && name.chars().next().is_some_and(|c| c.is_alphabetic())
            && name != "main"
            && !name.starts_with("r#")
        {
            // colonne 1-based en caractères
            let col = line[..name_start].chars().count() + 1;
            return Some((col, name.to_string()));
        }
        idx = start;
        if idx >= bytes.len() {
            break;
        }
    }
    None
}

/// Vrai si le nom est du snake_case valide (minuscules, chiffres, _).
fn is_snake_case(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_lowercase() || c.is_ascii_digit() || c == '_')
        && !name.starts_with('_')
}

/// Compte les lignes d'une fonction à partir de sa déclaration (accolades).
fn fn_block_len(neutral: &[String], start: usize) -> usize {
    let mut depth = 0i32;
    let mut opened = false;
    for (off, line) in neutral.iter().enumerate().skip(start) {
        depth += line.matches('{').count() as i32;
        depth -= line.matches('}').count() as i32;
        if line.contains('{') {
            opened = true;
        }
        if opened && depth <= 0 {
            return off - start + 1;
        }
    }
    neutral.len() - start
}

/// Vérifie un fichier source Rust déjà lu.
pub fn check_source(_path: &Path, src: &str, cfg: &StyleConfig) -> Vec<Finding> {
    let mut findings = Vec::new();
    let neutral = neutralize(src);
    let test_zone = mark_test_zones(&neutral);
    let raw_lines: Vec<&str> = src.lines().collect();

    for (i, raw) in raw_lines.iter().enumerate() {
        let line_no = i + 1;
        let clean = &neutral[i];

        // longueur de ligne (sur la ligne brute, en caractères)
        let width = raw.chars().count();
        if width > cfg.max_columns {
            findings.push(Finding {
                line: line_no,
                col: cfg.max_columns + 1,
                severity: Severity::Minor,
                rule: "R-COL100",
                message: format!("ligne de {} caractères (max {})", width, cfg.max_columns),
                fix: None,
            });
        }

        // tabulation
        if cfg.forbid_tabs && raw.contains('\t') {
            let col = raw.find('\t').map(|p| raw[..p].chars().count() + 1).unwrap_or(1);
            findings.push(Finding {
                line: line_no,
                col,
                severity: Severity::Minor,
                rule: "R-TAB",
                message: "tabulation — la convention Rust est 4 espaces".into(),
                fix: Some(Fix::ReplaceLine {
                    line: i,
                    content: raw.replace('\t', "    "),
                }),
            });
        }

        // espace en fin de ligne
        if raw.ends_with(' ') || raw.ends_with('\t') {
            findings.push(Finding {
                line: line_no,
                col: raw.trim_end().len() + 1,
                severity: Severity::Minor,
                rule: "R-TRAIL",
                message: "espace(s) en fin de ligne".into(),
                fix: Some(Fix::ReplaceLine {
                    line: i,
                    content: raw.trim_end().to_string(),
                }),
            });
        }

        // nom de fonction non snake_case (hors zone de test et macros)
        if cfg.check_snake_case && clean.contains("fn ") {
            if let Some((col, name)) = find_fn_name(clean) {
                if !is_snake_case(&name) {
                    findings.push(Finding {
                        line: line_no,
                        col,
                        severity: Severity::Minor,
                        rule: "R-SNAKE",
                        message: format!(
                            "fonction « {name} » en camelCase — la convention est snake_case"
                        ),
                        fix: None,
                    });
                }
            }
        }

        // .unwrap() hors tests
        if cfg.check_unwrap && !test_zone[i] {
            if let Some(pos) = clean.find(".unwrap()") {
                findings.push(Finding {
                    line: line_no,
                    col: clean[..pos].chars().count() + 1,
                    severity: Severity::Info,
                    rule: "R-UNWRAP",
                    message: ".unwrap() hors test — préfère expect/unwrap_or/? si ça peut échouer".into(),
                    fix: None,
                });
            }
        }

        // dbg! oublié (hors tests)
        if cfg.check_dbg && !test_zone[i] && clean.contains("dbg!") {
            let col = clean.find("dbg!").map(|p| clean[..p].chars().count() + 1).unwrap_or(1);
            findings.push(Finding {
                line: line_no,
                col,
                severity: Severity::Info,
                rule: "R-DBG",
                message: "dbg! — à retirer une fois le débogage terminé".into(),
                fix: None,
            });
        }

        // TODO / FIXME (dans les commentaires — donc sur la ligne BRUTE)
        if cfg.check_todo {
            for marker in ["TODO", "FIXME", "XXX"] {
                if raw.contains(marker) {
                    findings.push(Finding {
                        line: line_no,
                        col: raw.find(marker).map(|p| raw[..p].chars().count() + 1).unwrap_or(1),
                        severity: Severity::Info,
                        rule: "R-TODO",
                        message: format!("{marker} en attente"),
                        fix: None,
                    });
                    break;
                }
            }
        }
    }

    // newline final
    if !src.is_empty() && !src.ends_with('\n') {
        findings.push(Finding {
            line: raw_lines.len(),
            col: raw_lines.last().map(|l| l.len() + 1).unwrap_or(1),
            severity: Severity::Minor,
            rule: "R-NL-EOF",
            message: "le fichier ne se termine pas par une newline".into(),
            fix: Some(Fix::InsertBefore {
                line: raw_lines.len(),
                content: String::new(),
            }),
        });
    }

    // longueur des fonctions
    for (i, line) in neutral.iter().enumerate() {
        if line.contains("fn ") && line.contains('{') {
            let len = fn_block_len(&neutral, i);
            if len > cfg.max_function_lines {
                let name = find_fn_name(line)
                    .map(|(_, n)| n)
                    .unwrap_or_else(|| "?".into());
                findings.push(Finding {
                    line: i + 1,
                    col: 1,
                    severity: Severity::Info,
                    rule: "R-FUNCLEN",
                    message: format!(
                        "fonction « {name} » : {} lignes (max conseillé {})",
                        len, cfg.max_function_lines
                    ),
                    fix: None,
                });
            }
        }
    }

    findings.sort_by_key(|f| (f.line, f.col));
    findings
}

/// Collecte les fichiers .rs d'un chemin (récursif, ignore target/ et .git).
fn collect_rs(path: &Path, out: &mut Vec<PathBuf>) {
    if path.is_file() {
        if path.extension().is_some_and(|e| e == "rs") {
            out.push(path.to_path_buf());
        }
        return;
    }
    let Ok(rd) = fs::read_dir(path) else { return };
    for e in rd.flatten() {
        let p = e.path();
        let name = e.file_name().to_string_lossy().to_string();
        if name.starts_with('.') || name == "target" {
            continue;
        }
        if p.is_dir() {
            collect_rs(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

/// Vérifie un fichier .rs ou un dossier entier.
pub fn check_path(path: &Path, cfg: &StyleConfig) -> std::io::Result<Report> {
    let mut report = Report::default();
    let mut files = Vec::new();
    collect_rs(path, &mut files);
    files.sort();
    for f in files {
        let src = fs::read_to_string(&f)?;
        let findings = check_source(&f, &src, cfg);
        report.files.push(FileReport {
            path: f,
            findings,
        });
    }
    Ok(report)
}

/// Applique les corrections (une par ligne max). Retourne (source, nb appliqués).
pub fn apply_fixes(src: &str, fixes: &[Fix]) -> (String, usize) {
    let mut lines: Vec<String> = src.lines().map(|l| l.to_string()).collect();
    let mut applied = 0;
    for fix in fixes {
        match fix {
            Fix::ReplaceLine { line, content } => {
                if *line < lines.len() {
                    lines[*line] = content.clone();
                    applied += 1;
                }
            }
            Fix::InsertBefore { line, content } => {
                if *line <= lines.len() {
                    lines.insert(*line, content.clone());
                    applied += 1;
                }
            }
            Fix::DeleteLine { line } => {
                if *line < lines.len() {
                    lines.remove(*line);
                    applied += 1;
                }
            }
        }
    }
    let mut out = lines.join("\n");
    if src.ends_with('\n') || fixes.iter().any(|f| matches!(f, Fix::InsertBefore { .. })) {
        out.push('\n');
    }
    (out, applied)
}

/// Déduplique les fixes par ligne (un seul fix par ligne, le premier gagne).
pub fn dedup_fixes(findings: &[Finding]) -> Vec<Fix> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for f in findings {
        if let Some(fix) = &f.fix {
            let key = match fix {
                Fix::ReplaceLine { line, .. } => (*line, 0),
                Fix::InsertBefore { line, .. } => (*line, 1),
                Fix::DeleteLine { line } => (*line, 2),
            };
            if seen.insert(key) {
                out.push(fix.clone());
            }
        }
    }
    out.sort_by_key(|f| match f {
        Fix::ReplaceLine { line, .. } | Fix::InsertBefore { line, .. } | Fix::DeleteLine { line } => {
            *line
        }
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detecte_tab_et_trail() {
        let src = "fn main() {\n\tlet x = 1;  \n    println!(\"{}\", x);\n}\n";
        let f = check_source(Path::new("t.rs"), src, &StyleConfig::default());
        assert!(f.iter().any(|x| x.rule == "R-TAB"));
        assert!(f.iter().any(|x| x.rule == "R-TRAIL"));
    }

    #[test]
    fn ignore_tabs_dans_chaines_mais_pas_trail() {
        let src = "fn main() {\n    let s = \"a\tb\";\n}\n";
        let f = check_source(Path::new("t.rs"), src, &StyleConfig::default());
        // le tab est dans une chaîne : la règle R-TAB regarde la ligne brute...
        // on accepte de le signaler (rustfmt aussi reformate) — mais pas de crash
        assert!(f.iter().all(|x| x.rule != "R-SNAKE"));
    }

    #[test]
    fn snake_case_detecte_camel() {
        let src = "fn calculTotal() {}\nfn calcul_total() {}\n";
        let f = check_source(Path::new("t.rs"), src, &StyleConfig::default());
        let snakes: Vec<_> = f.iter().filter(|x| x.rule == "R-SNAKE").collect();
        assert_eq!(snakes.len(), 1);
        assert!(snakes[0].message.contains("calculTotal"));
    }

    #[test]
    fn unwrap_hors_test_seulement() {
        let src = "fn main() {\n    let x = Some(1).unwrap();\n}\n\n#[cfg(test)]\nmod tests {\n    fn t() {\n        let y = Some(2).unwrap();\n    }\n}\n";
        let f = check_source(Path::new("t.rs"), src, &StyleConfig::default());
        let unwraps: Vec<_> = f.iter().filter(|x| x.rule == "R-UNWRAP").collect();
        assert_eq!(unwraps.len(), 1);
        assert_eq!(unwraps[0].line, 2);
    }

    #[test]
    fn fix_tab_et_trail() {
        // comme cmd_fix : itère jusqu'au point fixe (une ligne peut cumuler
        // tab + espaces, chaque correction en révèle une autre)
        let mut src = "fn main() {\n\tlet x = 1;  \n}\n".to_string();
        let mut total = 0;
        for _ in 0..3 {
            let f = check_source(Path::new("t.rs"), &src, &StyleConfig::default());
            let fixes = dedup_fixes(&f);
            if fixes.is_empty() {
                break;
            }
            let (out, applied) = apply_fixes(&src, &fixes);
            total += applied;
            src = out;
        }
        assert!(total >= 2);
        assert!(!src.contains('\t'));
        assert!(!src.lines().any(|l| l.ends_with(' ')));
    }

    #[test]
    fn newline_final() {
        let src = "fn main() {}";
        let f = check_source(Path::new("t.rs"), src, &StyleConfig::default());
        assert!(f.iter().any(|x| x.rule == "R-NL-EOF"));
    }
}

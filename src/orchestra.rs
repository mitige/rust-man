//! « Projet Complémentarité de l'Homme » — l'orchestrateur pédagogique.
//!
//! Transforme les tâches du jour de l'étudiant en plan d'action concret dont
//! chaque étape peut invoquer un outil de rust-man (fiche, style, compile, quiz)
//! ou un agent IA spécialisé.

use crate::ai::{system_prompt, AgentRole, AiClient, AiError, HelpLevel};
use crate::progress::Progress;
use serde::Deserialize;

/// Action exécutable depuis le TUI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepAction {
    /// Ouvrir une fiche de documentation (id).
    Fiche(String),
    /// Lancer le checker de style sur un chemin.
    Style(String),
    /// Compiler un fichier/répertoire.
    Compile(String),
    /// Lancer un quiz d'une catégorie.
    Quiz(String),
    /// Discuter avec un agent spécialisé (nom + contexte).
    Agent(String, String),
    /// Étape manuelle (l'étudiant agit).
    Manuel,
}

#[derive(Debug, Clone)]
pub struct PlanStep {
    pub titre: String,
    pub pourquoi: String,
    pub action: StepAction,
    pub done: bool,
}

#[derive(Debug, Clone)]
pub struct Plan {
    pub analyse: String,
    pub steps: Vec<PlanStep>,
}

/// Contexte projet injecté dans la requête de l'orchestrateur.
pub struct ProjectContext {
    pub cwd: String,
    pub rs_files: Vec<String>,
    pub has_cargo_toml: bool,
}

impl ProjectContext {
    pub fn scan(cwd: &std::path::Path) -> Self {
        let mut rs_files = Vec::new();
        let mut has_cargo_toml = false;
        if let Ok(rd) = std::fs::read_dir(cwd) {
            for e in rd.flatten() {
                let p = e.path();
                let name = e.file_name().to_string_lossy().to_string();
                if p.is_file() {
                    if name.ends_with(".rs") {
                        rs_files.push(name);
                    } else if name == "Cargo.toml" {
                        has_cargo_toml = true;
                    }
                } else if p.is_dir() && name == "src" {
                    if let Ok(rd2) = std::fs::read_dir(&p) {
                        for e2 in rd2.flatten() {
                            let n2 = e2.file_name().to_string_lossy().to_string();
                            if n2.ends_with(".rs") {
                                rs_files.push(format!("src/{n2}"));
                            }
                        }
                    }
                }
            }
        }
        rs_files.sort();
        rs_files.truncate(100);
        Self {
            cwd: cwd.to_string_lossy().to_string(),
            rs_files,
            has_cargo_toml,
        }
    }

    fn describe(&self) -> String {
        format!(
            "Répertoire : {}\nCargo.toml : {}\nFichiers Rust : {}",
            self.cwd,
            if self.has_cargo_toml { "oui" } else { "non" },
            if self.rs_files.is_empty() {
                "(aucun)".to_string()
            } else {
                self.rs_files.join(", ")
            }
        )
    }

    /// Version enrichie : liste + contenu borné des fichiers.
    fn describe_with_contents(&self) -> String {
        let snap = crate::project::snapshot(std::path::Path::new(&self.cwd));
        format!("{}\n\n{}", self.describe(), snap.to_prompt())
    }
}

/// Demande un plan à l'orchestrateur (appel bloquant — thread séparé).
pub fn request_plan(
    client: &AiClient,
    mission: &str,
    progress: &Progress,
    ctx: &ProjectContext,
    level: HelpLevel,
) -> Result<Plan, AiError> {
    let system = system_prompt(AgentRole::Orchestrateur, level);
    let user = format!(
        "TÂCHES DU JOUR DE L'ÉTUDIANT :\n{mission}\n\n\
         ÉTAT DU PROJET :\n{}\n\n\
         PROGRESSION ENREGISTRÉE :\n{}\n\n\
         Construis un plan d'action pédagogique. Réponds UNIQUEMENT en JSON strict, \
         sans texte autour, avec cette forme exacte :\n\
         {{\"analyse\": \"2-3 phrases sur la situation\",\n\
         \"etapes\": [{{\"titre\": \"...\", \"pourquoi\": \"...\", \
         \"action\": {{\"type\": \"fiche|style|compile|quiz|agent|manuel\", \"cible\": \"...\"}}}}]}}\n\
         Types d'action : \"fiche\" (cible = id d'une fiche rust-man, ex: ownership, emprunts, traits...), \
         \"style\" (cible = chemin), \"compile\" (cible = chemin), \"quiz\" (cible = \
         demarrer|bases|ownership|erreurs|types|collections|outils|pratique), \"agent\" (cible = nom du rôle), \
         \"manuel\" (l'étudiant code lui-même).\n\
         5 à 9 étapes maximum, courtes, dans l'ordre d'attaque. Alterne apprentissage \
         (fiche/agent) et pratique (manuel/compile). La dernière étape doit consolider \
         (quiz ou reformulation).",
        ctx.describe_with_contents(),
        progress.summary_for_ai()
    );
    // le retry sur réponse vide est géré par le client (chat_long)
    let raw = client.chat_long(&system, &user)?;
    parse_plan(&raw).ok_or_else(|| {
        let preview: String = raw.chars().take(400).collect();
        AiError::InvalidResponse(format!(
            "plan JSON invalide. Début de la réponse :\n{preview}"
        ))
    })
}

#[derive(Deserialize)]
struct RawPlan {
    analyse: String,
    etapes: Vec<RawStep>,
}

#[derive(Deserialize)]
struct RawStep {
    titre: String,
    #[serde(default)]
    pourquoi: String,
    #[serde(default)]
    action: Option<RawAction>,
}

#[derive(Deserialize)]
struct RawAction {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    cible: String,
}

/// Extrait le premier bloc JSON équilibré { ... } en respectant les chaînes.
fn extract_json_block(raw: &str) -> Option<&str> {
    let start = raw.find('{')?;
    let mut depth = 0i32;
    let mut in_str = false;
    let mut escape = false;
    for (i, c) in raw[start..].char_indices() {
        if escape {
            escape = false;
            continue;
        }
        match c {
            '\\' if in_str => escape = true,
            '"' => in_str = !in_str,
            '{' | '[' if !in_str => depth += 1,
            '}' | ']' if !in_str => {
                depth -= 1;
                if depth == 0 {
                    return Some(&raw[start..start + i + 1]);
                }
            }
            _ => {}
        }
    }
    // tronqué : tout le reste, la réparation refermera
    Some(raw[start..].trim_end())
}

/// Répare les défauts JSON fréquents des LLM : virgules traînantes,
/// JSON tronqué (accolades/chaînes non fermées).
fn repair_json(text: &str) -> String {
    // 1. virgules traînantes avant } ou ]
    let mut out = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    let mut in_str = false;
    let mut escape = false;
    while i < chars.len() {
        let c = chars[i];
        if escape {
            out.push(c);
            escape = false;
            i += 1;
            continue;
        }
        if c == '\\' && in_str {
            out.push(c);
            escape = true;
            i += 1;
            continue;
        }
        if c == '"' {
            in_str = !in_str;
            out.push(c);
            i += 1;
            continue;
        }
        if c == ',' && !in_str {
            // regarde le prochain caractère non-blanc
            let mut j = i + 1;
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }
            if j < chars.len() && (chars[j] == '}' || chars[j] == ']') {
                i += 1; // saute la virgule
                continue;
            }
        }
        out.push(c);
        i += 1;
    }
    // 2. tronqué : ferme chaîne ouverte puis crochets/accolades
    if in_str {
        out.push('"');
    }
    let mut stack: Vec<char> = Vec::new();
    let mut ins = false;
    let mut esc = false;
    for c in out.chars() {
        if esc {
            esc = false;
            continue;
        }
        match c {
            '\\' if ins => esc = true,
            '"' => ins = !ins,
            '{' | '[' if !ins => stack.push(c),
            '}' | ']' if !ins => {
                stack.pop();
            }
            _ => {}
        }
    }
    while let Some(open) = stack.pop() {
        out.push(if open == '{' { '}' } else { ']' });
    }
    out
}

/// Construit le Plan depuis un RawPlan parsé.
fn plan_from_raw(plan: RawPlan) -> Plan {
    let steps = plan
        .etapes
        .into_iter()
        .map(|s| PlanStep {
            titre: s.titre,
            pourquoi: s.pourquoi,
            action: match s.action {
                Some(a) => match a.kind.as_str() {
                    "fiche" => StepAction::Fiche(a.cible),
                    "style" => StepAction::Style(a.cible),
                    "compile" => StepAction::Compile(a.cible),
                    "quiz" => StepAction::Quiz(a.cible),
                    "agent" => StepAction::Agent(a.cible, String::new()),
                    _ => StepAction::Manuel,
                },
                None => StepAction::Manuel,
            },
            done: false,
        })
        .collect();
    Plan {
        analyse: plan.analyse,
        steps,
    }
}

/// Repli : le modèle n'a pas produit de JSON — on découpe son texte en étapes
/// numérotées (toutes en action manuelle, l'utilisateur reste guidé).
fn plan_from_text(raw: &str) -> Option<Plan> {
    let mut steps = Vec::new();
    let mut analyse = String::new();
    for line in raw.lines() {
        let t = line.trim();
        // lignes "1. xxx" / "1) xxx" / "- xxx"
        let is_step = {
            let digit_start = t.chars().take_while(|c| c.is_ascii_digit()).count();
            (digit_start > 0 && t[digit_start..].starts_with(['.', ')'])) || t.starts_with("- ")
        };
        if is_step {
            let title = t
                .trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c == ')' || c == '-' || c == ' ')
                .trim()
                .to_string();
            if !title.is_empty() {
                steps.push(PlanStep {
                    titre: title,
                    pourquoi: String::new(),
                    action: StepAction::Manuel,
                    done: false,
                });
            }
        } else if steps.is_empty() && !t.is_empty() && analyse.len() < 300 {
            if !analyse.is_empty() {
                analyse.push(' ');
            }
            analyse.push_str(t);
        }
    }
    if steps.len() >= 2 {
        Some(Plan {
            analyse: if analyse.is_empty() {
                "Plan généré en mode texte.".to_string()
            } else {
                analyse
            },
            steps,
        })
    } else {
        None
    }
}

/// Parse le plan : JSON direct → JSON réparé → repli texte. Ne rate jamais
/// pour une raison de forme si le modèle a produit quelque chose d'exploitable.
pub fn parse_plan(raw: &str) -> Option<Plan> {
    if let Some(json) = extract_json_block(raw) {
        if let Ok(plan) = serde_json::from_str::<RawPlan>(json) {
            return Some(plan_from_raw(plan));
        }
        let repaired = repair_json(json);
        if let Ok(plan) = serde_json::from_str::<RawPlan>(&repaired) {
            return Some(plan_from_raw(plan));
        }
    }
    plan_from_text(raw)
}

/// Mode « Apprends-moi ce qu'on vient de faire » (13.6) : synthèse pédagogique.
pub fn teach_back(
    client: &AiClient,
    travail: &str,
    level: HelpLevel,
) -> Result<String, AiError> {
    let system = system_prompt(AgentRole::Orchestrateur, level);
    let user = format!(
        "L'étudiant demande : « Explique-moi tout ce qu'on vient de faire comme si je \
         devais le refaire seul demain. »\n\nTRAVAUX RÉALISÉS :\n{travail}\n\n\
         Produis la synthèse pédagogique complète :\n\
         ## Le problème et l'approche choisie\n\
         ## Pourquoi cette approche (et les alternatives écartées)\n\
         ## Les étapes de la solution, dans l'ordre\n\
         ## Les erreurs rencontrées et leurs causes\n\
         ## Le code final expliqué\n\
         ## À mémoriser vs à comprendre\n\
         ## Comment reconnaître un problème similaire\n\
         ## Erreurs qu'un étudiant refera (pièges)\n\
         Sois complet mais structuré en sections courtes."
    );
    client.chat(&system, &user)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_plan_json_strict() {
        let raw = r#"{"analyse":"Il faut comprendre ownership.","etapes":[
            {"titre":"Lire la fiche ownership","pourquoi":"base","action":{"type":"fiche","cible":"ownership"}},
            {"titre":"Coder","action":{"type":"manuel"}}
        ]}"#;
        let plan = parse_plan(raw).unwrap();
        assert_eq!(plan.steps.len(), 2);
        assert_eq!(plan.steps[0].action, StepAction::Fiche("ownership".into()));
        assert_eq!(plan.steps[1].action, StepAction::Manuel);
    }

    #[test]
    fn parse_plan_json_dans_texte() {
        let raw = "Voici le plan :\n```json\n{\"analyse\":\"a\",\"etapes\":[{\"titre\":\"t\",\"action\":{\"type\":\"quiz\",\"cible\":\"ownership\"}}]}\n```";
        let plan = parse_plan(raw).unwrap();
        assert_eq!(plan.steps[0].action, StepAction::Quiz("ownership".into()));
    }

    #[test]
    fn parse_plan_invalide() {
        assert!(parse_plan("").is_none());
    }

    #[test]
    fn parse_plan_virgules_trainantes() {
        let raw = r#"{"analyse":"a","etapes":[{"titre":"t","action":{"type":"fiche","cible":"result"},},],}"#;
        let plan = parse_plan(raw).unwrap();
        assert_eq!(plan.steps.len(), 1);
        assert_eq!(plan.steps[0].action, StepAction::Fiche("result".into()));
    }

    #[test]
    fn parse_plan_tronque() {
        // réponse coupée au milieu (max_tokens atteint)
        let raw = r#"{"analyse":"a","etapes":[{"titre":"Lire","action":{"type":"fiche","cible":"result"}},{"titre":"Coder","action":{"type":"man"#;
        let plan = parse_plan(raw).unwrap();
        assert!(plan.steps.len() >= 1);
    }

    #[test]
    fn parse_plan_repli_texte() {
        let raw = "Voici le plan pour ta journée :\n\n1. Lire la fiche ownership\n2. Coder une pile générique\n3. Compiler et vérifier le style";
        let plan = parse_plan(raw).unwrap();
        assert_eq!(plan.steps.len(), 3);
        assert!(plan.steps[0].titre.contains("ownership"));
        assert_eq!(plan.steps[0].action, StepAction::Manuel);
    }
}


//! Couche IA centralisée : UN SEUL client pour tous les agents de rust-man.
//!
//! Provider : Kimi K3 via l'endpoint OpenAI-compatible du projet DSH.
//! Clé API : env `OPP_API_KEY`, sinon `~/.dsh/.credentials.yaml` (jamais affichée).

use crate::config::Config;
use std::fmt;
use std::path::PathBuf;

const DEFAULT_BASE_URL: &str = "https://dashscope.aliyuncs.com/compatible-mode/v1";
const DEFAULT_MODEL: &str = "kimi-k3";

/// Fenêtre de contexte des modèles du provider : 1M tokens.
pub const CONTEXT_WINDOW_TOKENS: u32 = 1_048_576; // 1M
/// Sortie maximale des modèles du provider : 128K tokens.
pub const MAX_OUTPUT_TOKENS: u32 = 131_072; // 128K

/// Modèles servis par le provider du projet
/// (contexte 1M, sortie max 128K — cf. constantes ci-dessus).
pub const AVAILABLE_MODELS: &[&str] = &[
    "kimi-k3",
    "deepseek-v4.1-flash",
    "glm-5.3-prime",
    "qwen3.8-max-0902",
];

// ------------------------------------------------------------------ erreurs

#[derive(Debug)]
pub enum AiError {
    NoKey,
    Network(String),
    Http(u16, String),
    InvalidResponse(String),
}

impl fmt::Display for AiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AiError::NoKey => write!(
                f,
                "clé API introuvable. Définis OPP_API_KEY ou ajoute-la à ~/.dsh/.credentials.yaml"
            ),
            AiError::Network(e) => write!(f, "erreur réseau : {e}"),
            AiError::Http(code, body) => write!(f, "HTTP {code} : {}", &body[..body.len().min(200)]),
            AiError::InvalidResponse(e) => write!(f, "réponse IA invalide : {e}"),
        }
    }
}

// ------------------------------------------------------------------ client

#[derive(Clone)]
pub struct AiClient {
    base_url: String,
    api_key: String,
    pub model: String,
}

impl AiClient {
    pub fn from_env(cfg: &Config) -> Result<Self, AiError> {
        let api_key = std::env::var("OPP_API_KEY")
            .ok()
            .filter(|k| !k.is_empty())
            .or_else(read_dsh_credentials_key)
            .ok_or(AiError::NoKey)?;
        let base_url = std::env::var("RMAN_AI_BASE")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
        let model = std::env::var("RMAN_AI_MODEL")
            .ok()
            .filter(|s| !s.is_empty())
            .or_else(|| cfg.ai_model.clone())
            .unwrap_or_else(|| DEFAULT_MODEL.to_string());
        Ok(Self {
            base_url,
            api_key,
            model,
        })
    }

    /// Appel chat completions (non-streaming). Bloquant — à lancer dans un thread.
    /// Budget de sortie = plafond du provider (128K). Retries automatiques :
    /// réponse vide (raisonnement a tout mangé) → budget ×4 borné à 128K ;
    /// erreur transitoire (429/5xx/réseau) → nouvelle tentative.
    pub fn chat(&self, system: &str, user: &str) -> Result<String, AiError> {
        self.chat_resilient(system, user, MAX_OUTPUT_TOKENS)
    }

    /// Appel rapide pour l'interactif (complétion) : petit budget, timeout court.
    pub fn chat_fast(&self, system: &str, user: &str) -> Result<String, AiError> {
        self.chat_with_budget(system, user, 1024)
    }

    /// Appel avec le budget de sortie maximal (plans, synthèses longues) :
    /// 128K tokens, de quoi ne jamais couper un plan en plein milieu.
    pub fn chat_long(&self, system: &str, user: &str) -> Result<String, AiError> {
        self.chat_resilient(system, user, MAX_OUTPUT_TOKENS)
    }

    /// Change le modèle utilisé par ce client.
    pub fn set_model(&mut self, model: &str) {
        self.model = model.to_string();
    }

    /// Modèle suivant dans la liste des modèles disponibles.
    pub fn next_model_name(&self) -> &'static str {
        let pos = AVAILABLE_MODELS
            .iter()
            .position(|m| *m == self.model)
            .unwrap_or(0);
        AVAILABLE_MODELS[(pos + 1) % AVAILABLE_MODELS.len()]
    }

    /// Une tentative + retries ciblés selon la cause de l'échec.
    fn chat_resilient(&self, system: &str, user: &str, max_tokens: u32) -> Result<String, AiError> {
        let mut budget = max_tokens;
        let mut last_err = None;
        for attempt in 0..3 {
            match self.chat_with_budget(system, user, budget) {
                Ok(r) => return Ok(r),
                Err(AiError::InvalidResponse(e)) if e.contains("réponse vide") => {
                    // le raisonnement a épuisé le budget → on quadruple,
                    // borné au plafond du provider (128K)
                    budget = budget.saturating_mul(4).min(MAX_OUTPUT_TOKENS);
                    last_err = Some(AiError::InvalidResponse(e));
                }
                Err(AiError::Http(400, _)) if budget > 8192 => {
                    // ce modèle refuse ce budget de sortie → on divise par 2
                    // et on retente avec un plafond réaliste
                    budget /= 2;
                    std::thread::sleep(std::time::Duration::from_secs(2 + attempt * 3));
                    last_err = Some(AiError::Http(400, "budget de sortie refusé".into()));
                }
                Err(e @ AiError::Http(_, _)) => {
                    // 429/5xx transitoires ; les 400 sont fréquents sur ce
                    // provider (load-balancing entre backends) → on retente aussi
                    std::thread::sleep(std::time::Duration::from_secs(2 + attempt * 3));
                    last_err = Some(e);
                }
                Err(e @ AiError::Network(_)) => {
                    std::thread::sleep(std::time::Duration::from_secs(2));
                    last_err = Some(e);
                }
                Err(e) => return Err(e),
            }
        }
        Err(last_err.unwrap_or(AiError::InvalidResponse("échec après retries".into())))
    }

    fn chat_with_budget(&self, system: &str, user: &str, max_tokens: u32) -> Result<String, AiError> {
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let body = serde_json::json!({
            "model": self.model,
            "messages": [
                {"role": "system", "content": system},
                {"role": "user", "content": user}
            ],
            "max_tokens": max_tokens,
        });
        // timeout : 90s de marge réseau + génération à ~30 tok/s, plafond 1 h
        // (un budget 128K laisse le temps aux modèles à raisonnement)
        let timeout_secs = (90 + u64::from(max_tokens) / 30).min(3600);
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(std::time::Duration::from_secs(timeout_secs)))
            .build()
            .into();
        let response = agent
            .post(&url)
            .header("Authorization", &format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .send_json(&body);
        match response {
            Ok(mut resp) => {
                let value: serde_json::Value = resp
                    .body_mut()
                    .read_json()
                    .map_err(|e| AiError::InvalidResponse(e.to_string()))?;
                let content = value
                    .pointer("/choices/0/message/content")
                    .and_then(|c| c.as_str())
                    .map(|s| s.to_string());
                match content {
                    // kimi-k3 est un modèle à raisonnement : si le budget a été
                    // mangé par le reasoning, content revient vide → on signale
                    // pour retenter avec un budget plus large
                    Some(c) if c.trim().is_empty() => Err(AiError::InvalidResponse(
                        "réponse vide du modèle (budget de raisonnement épuisé)".into(),
                    )),
                    Some(c) => Ok(c),
                    None => Err(AiError::InvalidResponse(
                        "pas de choices[0].message.content".into(),
                    )),
                }
            }
            Err(ureq::Error::StatusCode(code)) => {
                Err(AiError::Http(code, format!("le provider a rejeté la requête ({code})")))
            }
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("timeout") {
                    Err(AiError::Network(
                        "l'IA a mis trop de temps à répondre. Réessaie — si ça persiste, \
                         le réseau est lent ou le contexte projet est gros (lance rust-man \
                         depuis un dossier plus petit).".into(),
                    ))
                } else {
                    Err(AiError::Network(msg))
                }
            }
        }
    }
}

/// Lit OPP_API_KEY dans ~/.dsh/.credentials.yaml sans l'afficher.
fn read_dsh_credentials_key() -> Option<String> {
    let home = std::env::var("DSH_HOME")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".dsh")
        });
    let text = std::fs::read_to_string(home.join(".credentials.yaml")).ok()?;
    for line in text.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("OPP_API_KEY:") {
            let key = rest.trim().trim_matches('"').trim_matches('\'').to_string();
            if !key.is_empty() {
                return Some(key);
            }
        }
    }
    None
}

// ------------------------------------------------------------------ niveaux d'aide

/// Niveau d'assistance. Par défaut : Socratique — rust-man guide par questions,
/// il ne donne pas la réponse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HelpLevel {
    #[default]
    Socratique = 1,
    Guidage = 2,
    Assistance = 3,
    Intervention = 4,
    Analyse = 5,
}

impl HelpLevel {
    pub fn from_u8(n: u8) -> Self {
        match n {
            1 => Self::Socratique,
            3 => Self::Assistance,
            4 => Self::Intervention,
            5 => Self::Analyse,
            _ => Self::Guidage,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Socratique => "1·Socratique",
            Self::Guidage => "2·Guidage",
            Self::Assistance => "3·Assistance",
            Self::Intervention => "4·Intervention",
            Self::Analyse => "5·Analyse",
        }
    }

    pub fn next(&self) -> Self {
        Self::from_u8((*self as u8 % 5) + 1)
    }

    pub fn directive(&self) -> &'static str {
        match self {
            Self::Socratique =>
                "NIVEAU SOCRATIQUE : ne donne JAMAIS la solution. Pose uniquement des questions \
                 qui guident, donne des indices minimaux, fais raisonner l'étudiant.",
            Self::Guidage =>
                "NIVEAU GUIDAGE : décompose le problème en étapes, propose du pseudo-code et des \
                 pistes détaillées, mais laisse l'étudiant écrire le code.",
            Self::Assistance =>
                "NIVEAU ASSISTANCE : tu peux montrer des exemples courts (PAS la solution de \
                 l'exercice), corriger des lignes précises et expliquer en détail.",
            Self::Intervention =>
                "NIVEAU INTERVENTION : tu peux écrire ou corriger du code directement quand \
                 c'est nécessaire, en expliquant chaque choix pour que l'étudiant puisse le refaire.",
            Self::Analyse =>
                "NIVEAU ANALYSE COMPLÈTE : analyse approfondie, explication exhaustive, synthèse \
                 pédagogique et exercices de consolidation.",
        }
    }
}

// ------------------------------------------------------------------ agents

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentRole {
    // ── Pôle humain : les élèves & seniors ─────────────────────────────
    Orchestrateur,
    Intervenant,
    Aine,
    BinomeSocratique,
    BinomeBasNiveau,
    BinomeAlgo,
    BinomeClean,
    BinomeDebug,
    BinomeMotivation,
    GuideExercice,
    CoachOral,
    BinomeIntuition,
    // ── Pôle technique : les spécialistes ──────────────────────────────
    CodeReviewer,
    StyleChecker,
    CompilerAssistant,
    CargoAssistant,
    DocAssistant,
    ExamCoach,
    CodeExplainer,
}

pub const AGENTS: &[AgentRole] = &[
    AgentRole::Orchestrateur,
    AgentRole::Intervenant,
    AgentRole::Aine,
    AgentRole::BinomeSocratique,
    AgentRole::BinomeBasNiveau,
    AgentRole::BinomeAlgo,
    AgentRole::BinomeClean,
    AgentRole::BinomeDebug,
    AgentRole::BinomeMotivation,
    AgentRole::GuideExercice,
    AgentRole::CoachOral,
    AgentRole::BinomeIntuition,
    AgentRole::CodeReviewer,
    AgentRole::StyleChecker,
    AgentRole::CompilerAssistant,
    AgentRole::CargoAssistant,
    AgentRole::DocAssistant,
    AgentRole::ExamCoach,
    AgentRole::CodeExplainer,
];

impl AgentRole {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Orchestrateur => "Complémentarité de l'Homme",
            Self::Intervenant => "L'Intervenant",
            Self::Aine => "L'Aîné",
            Self::BinomeSocratique => "Binôme Socratique",
            Self::BinomeBasNiveau => "Binôme Bas-Niveau",
            Self::BinomeAlgo => "Binôme Algo",
            Self::BinomeClean => "Binôme Clean",
            Self::BinomeDebug => "Binôme Debug",
            Self::BinomeMotivation => "Binôme Motivation",
            Self::GuideExercice => "Guide d'Exercice",
            Self::CoachOral => "Coach d'Oral",
            Self::BinomeIntuition => "Binôme Intuition",
            Self::CodeReviewer => "Code Reviewer",
            Self::StyleChecker => "Style Checker",
            Self::CompilerAssistant => "Compiler Assistant",
            Self::CargoAssistant => "Cargo Assistant",
            Self::DocAssistant => "Documentation",
            Self::ExamCoach => "Exam Coach",
            Self::CodeExplainer => "Code Explainer",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Orchestrateur => "donne tes tâches du jour, il coordonne tout le reste",
            Self::Intervenant => "expert Rust et généraliste, explique avec une clarté rare",
            Self::Aine => "a appris Rust avant toi — franc, exigeant, te pousse à te dépasser",
            Self::BinomeSocratique => "n'explique qu'en questions — la réponse, c'est toi qui la trouves",
            Self::BinomeBasNiveau => "mémoire, pile/tas, ownership — intuition machine en schémas",
            Self::BinomeAlgo => "construit le raisonnement avant le code, pseudo-code d'abord",
            Self::BinomeClean => "lisibilité, idiomes, design — du code qu'on aime relire",
            Self::BinomeDebug => "traque les bugs avec toi, une hypothèse à la fois",
            Self::BinomeMotivation => "momentum, micro-objectifs, zéro surcharge mentale",
            Self::GuideExercice => "te construit un exo et te guide étape par étape, sans la solution",
            Self::BinomeIntuition => "le ressenti derrière chaque concept — l'intuition d'abord",
            Self::CoachOral => "oral blanc : t'entraîne à EXPLIQUER ton code comme un examinateur",
            Self::CodeReviewer => "relit ton code et signale les problèmes, ligne par ligne",
            Self::StyleChecker => "idiomes Rust, rustfmt, clippy — la convention officielle",
            Self::CompilerAssistant => "traduit les erreurs rustc (borrow checker inclus) en français clair",
            Self::CargoAssistant => "Cargo.toml, dépendances, features, workspaces",
            Self::DocAssistant => "trouve et explique fonctions et concepts de la std",
            Self::ExamCoach => "entraînement : exercices et questions de vérification",
            Self::CodeExplainer => "détaille le fonctionnement d'un programme",
        }
    }

    /// Groupe d'affichage dans le sélecteur d'agents.
    pub fn group(&self) -> &'static str {
        match self {
            Self::Orchestrateur => "orchestration",
            Self::Intervenant | Self::Aine | Self::BinomeSocratique | Self::BinomeBasNiveau
            | Self::BinomeIntuition
            | Self::BinomeAlgo | Self::BinomeClean | Self::BinomeDebug | Self::BinomeMotivation => "élèves & seniors",
            _ => "spécialistes",
        }
    }
}

/// Prompt système de chaque agent. `level` ajuste l'autonomie donnée à l'IA.
pub fn system_prompt(role: AgentRole, level: HelpLevel) -> String {
    let base = "Tu es un assistant intégré à rust-man, l'outil d'apprentissage du langage \
                Rust d'un étudiant qui suit des cours de Rust en parallèle. Contexte : Rust \
                stable, édition 2021, cargo, rustfmt, clippy. L'étudiant sait déjà programmer \
                (bases acquises) mais DÉBUTE en Rust : le borrow checker, l'ownership et les \
                traits sont ses zones de combat. Il a un profil TDAH : explications COURTES \
                et progressives, étapes numérotées, une seule action à la fois, pas de pavé \
                indigeste. Réponds en français, tutoie, va droit au but. Quand tu montres du \
                code, qu'il soit minimal, idiomatique et conforme rustfmt. Règle d'or : \
                l'étudiant doit APPRENDRE à refaire seul — ne fais jamais son exercice à \
                sa place sans qu'il ait d'abord essayé.\n\n\
                IMPORTANT : le message de l'étudiant commence par une section « PROJET DE \
                L'ÉTUDIANT » contenant le contenu réel de ses fichiers (répertoire où rust-man \
                a été lancé). Tu as donc DÉJÀ accès à son code : réfère-toi directement à \
                son contenu (cite les lignes), ne demande JAMAIS de coller un fichier. Si la \
                section dit « aucun fichier Rust détecté », dis qu'il faut lancer rust-man \
                depuis le dossier du projet.";
    let role_part = match role {
        AgentRole::Orchestrateur =>
            "Tu es « Complémentarité de l'Homme », l'orchestrateur pédagogique. Tu coordonnes \
             les outils de rust-man (fiches de doc, checker de style, compilateur, quiz, agents) \
             pour faire réussir l'étudiant. Tu décomposes les objectifs en étapes concrètes, \
             tu désignes l'outil ou l'agent pertinent à chaque étape, et tu vérifies la \
             compréhension avant d'avancer.",
        AgentRole::Intervenant =>
            "Tu es l'Intervenant : un expert qui maîtrise Rust en profondeur (ownership, \
             lifetimes, traits, modèle mémoire, monomorphisation) autant que l'informatique \
             générale. Tu expliques avec une précision et une clarté rares : d'abord \
             l'intuition, puis le détail exact, puis le piège. Tu relies toujours le concept \
             à ce que fait réellement le compilateur ou la machine.",
        AgentRole::Aine =>
            "Tu es l'Aîné : tu as appris Rust avant lui et tu connais les murs contre \
             lesquels on se cogne (le borrow checker, les lifetimes, &str vs String). \
             Ton style est franc et exigeant. Tu ne ménages pas les excuses, mais tu crois \
             dur comme fer en l'étudiant. Tu partages tes erreurs d'avant, tu dis les choses \
             comme elles sont, et tu le pousses à te dépasser. Jamais méchant, toujours direct.",
        AgentRole::BinomeSocratique =>
            "Tu es le Binôme Socratique. Tu ne donnes JAMAIS la réponse : tu poses des \
             questions courtes et précises qui guident l'étudiant vers sa propre solution. \
             S'il bloque, tu réduis la question, tu ne la résous pas. Tu valides chaque étape \
             franchie. Ton but : qu'il puisse refaire seul, demain, sans toi.",
        AgentRole::BinomeBasNiveau =>
            "Tu es le Binôme Bas-Niveau : la machine te passionne. Pile, tas, registres, \
             ownership, drop, monomorphisation, ce que fait VRAIMENT le processeur. Tu \
             expliques avec des schémas ASCII de la mémoire et des analogies concrètes. \
             Chaque concept Rust (move, emprunt, Box, Rc) est relié à ce qui se passe \
             en dessous.",
        AgentRole::BinomeAlgo =>
            "Tu es le Binôme Algo. Avant toute ligne de code : entrées, sorties, cas limites, \
             pseudo-code sur papier, complexité. Tu refuses de parler Rust tant que l'approche \
             n'est pas claire. Tu demandes toujours à l'étudiant sa propre approche d'abord.",
        AgentRole::BinomeClean =>
            "Tu es le Binôme Clean : lisibilité, nommage, découpage, idiomes Rust (itérateurs \
             plutôt qu'index, Option/Result plutôt que sentinelles, &str plutôt que &String). \
             Tu relis comme un pair qui a le souci du détail, et tu montres comment rendre le \
             code évident. Un code propre est un code qu'on débogue moins.",
        AgentRole::BinomeDebug =>
            "Tu es le Binôme Debug. Face à un bug : symptôme précis, hypothèses ordonnées, \
             UNE expérience à la fois (dbg!, test minimal, lecture de l'erreur rustc). Tu \
             apprends à l'étudiant la méthode, pas juste la réponse. « Qu'est-ce que tu \
             observes, qu'est-ce que tu attendais ? »",
        AgentRole::BinomeMotivation =>
            "Tu es le Binôme Motivation. Tu connais la charge des cours et la fatigue. Tu \
             découpes tout en micro-objectifs de 25 minutes, tu célèbres chaque victoire, tu \
             ramènes à l'essentiel quand ça déborde. Ton énergie est calme et constante : \
             jamais de culpabilisation, toujours la prochaine petite action.",
        AgentRole::GuideExercice =>
            "Tu es le Guide d'Exercice. On te donne un sujet (ex: « les itérateurs ») ; tu \
             crées un EXERCICE progressif et tu guides l'étudiant PAS À PAS. \
             Fonctionnement : 1) présente l'exo en 2 lignes et la 1re micro-étape SEULEMENT. \
             2) attends que l'étudiant dise ce qu'il a fait. 3) vérifie (demande son code ou \
             sa sortie), corrige si besoin, puis donne l'étape suivante. JAMAIS la solution \
             complète — chaque étape tient en une action. Termine par une mini-vérification \
             de compréhension (« explique-moi pourquoi… »).",
        AgentRole::BinomeIntuition =>
            "Tu es le Binôme Intuition. Ta mission : faire RESSENTIR le concept, pas le \
             réciter. Pour chaque notion (l'ownership comme prêt d'un livre, les lifetimes \
             comme des contrats de location…), tu donnes d'abord l'intuition viscérale \
             (l'analogie qui colle, le modèle mental juste), puis un micro-exemple qui la \
             rend concrète, puis pourquoi c'est comme ça. Tu veux que l'étudiant puisse dire \
             « je le sens » et l'expliquer sans le cours. Pas de jargon avant l'intuition.",
        AgentRole::CoachOral =>
            "Tu es le Coach d'Oral : tu joues un EXAMINATEUR. Tu demandes à l'étudiant \
             d'EXPLIQUER son code ou un concept (un examinateur ne code pas, il questionne). \
             Tu évalues sa réponse : clarté, exactitude, intuition. Tu creuses (« pourquoi \
             &str ici ? », « que se passe-t-il si… », « montre-moi sur un exemple »). Tu \
             notes les points faibles et tu les retravailles. Ton but : qu'il explique comme \
             s'il ressentait le code — pas réciter. Tu es exigeant mais juste : la note tombe \
             sur la pire explication.",
        AgentRole::CodeReviewer =>
            "Tu es le Code Reviewer. Tu relis le code fourni : bugs potentiels, panics \
             possibles (unwrap, indexation), emprunts inutilement compliqués, clones \
             superflus, lisibilité, idiomes Rust. Tu cites les lignes précises et expliques \
             le pourquoi de chaque remarque.",
        AgentRole::StyleChecker =>
            "Tu es le Style Checker. Tu analyses la conformité aux conventions Rust \
             officielles : nommage (snake_case/CamelCase), formatage rustfmt, lints clippy \
             classiques. Tu cites la règle ou le lint, la ligne, et tu proposes la \
             correction. Strict mais pédagogue.",
        AgentRole::CompilerAssistant =>
            "Tu es le Compiler Assistant. Tu traduis les erreurs et warnings de rustc en \
             français simple : ce que le compilateur a compris, pourquoi ça coince, comment \
             corriger. Tu as une expertise particulière du borrow checker (E0382, E0502, \
             E0597…) : tu expliques TOUJOURS quel emprunt vit encore et où il meurt. \
             Toujours dans l'ordre des messages.",
        AgentRole::CargoAssistant =>
            "Tu es le Cargo Assistant. Tu expliques et corriges tout ce qui touche cargo : \
             Cargo.toml, dépendances et versions semver, features, [dev-dependencies], \
             workspaces, profils dev/release, cargo add/update/tree.",
        AgentRole::DocAssistant =>
            "Tu es l'assistant Documentation. Tu expliques les fonctions et types de la \
             bibliothèque standard : signature exacte, retour, cas d'erreur (Option/Result), \
             exemple minimal. Tu renvoies vers les fiches rust-man quand elles existent.",
        AgentRole::ExamCoach =>
            "Tu es l'Exam Coach. Tu crées des exercices d'entraînement progressifs et des \
             questions de vérification façon partiel. Tu adaptes la difficulté au niveau \
             observé. Tu ne corriges qu'après une tentative.",
        AgentRole::CodeExplainer =>
            "Tu es le Code Explainer. Tu détaille un programme ligne par ligne : ce que fait \
             chaque instruction, qui possède quoi (ownership), où vivent les emprunts, le \
             flot d'exécution. Clair et méthodique.",
    };
    format!("{base}\n\n{role_part}\n\n{}", level.directive())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_cycle() {
        let l = HelpLevel::from_u8(1);
        assert_eq!(l.name(), "1·Socratique");
        assert_eq!(l.next(), HelpLevel::Guidage);
        assert_eq!(HelpLevel::from_u8(99), HelpLevel::Guidage);
    }

    #[test]
    fn niveau_defaut_socratique() {
        assert_eq!(HelpLevel::default(), HelpLevel::Socratique);
    }

    #[test]
    fn enveloppe_cohérente() {
        // sortie 128K doit tenir dans la fenêtre 1M
        assert!(MAX_OUTPUT_TOKENS < CONTEXT_WINDOW_TOKENS);
        assert_eq!(CONTEXT_WINDOW_TOKENS / 1_048_576, 1); // 1M
        assert_eq!(MAX_OUTPUT_TOKENS / 1024, 128); // 128K
    }

    #[test]
    fn prompts_mentionnent_rust() {
        for role in AGENTS {
            let p = system_prompt(*role, HelpLevel::Guidage);
            assert!(p.contains("Rust"), "{} sans contexte Rust", role.name());
            assert!(p.contains("NIVEAU"), "{} sans directive de niveau", role.name());
        }
    }

    #[test]
    fn agents_uniques() {
        let mut names: Vec<_> = AGENTS.iter().map(|a| a.name()).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), AGENTS.len());
    }
}

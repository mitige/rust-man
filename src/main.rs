//! rust-man — environnement d'apprentissage Rust, en complément des cours.

mod tui;

use rust_man::{ai, compile, config, content, dsh, exo, orchestra, progress, project, quiz, render, serve, style};
use clap::{CommandFactory, Parser, Subcommand};
use clap::error::ErrorKind;
use content::{category_label, ENTRIES};
use render::Renderer;
use std::io::IsTerminal;
use std::path::Path;

#[derive(Parser)]
#[command(
    name = "rust-man",
    version,
    about = "Ton compagnon Rust : doc, style, quiz, IA — dans le terminal",
    long_about = "rust-man = documentation Rust + checker de style + compilation expliquée\n\
                  + quiz + exercices vérifiés + assistants IA (Kimi K3), le tout dans le TUI.\n\n\
                  Sans argument : environnement interactif (TUI).\n\
                  Avec un sujet : affiche la fiche directement, comme man."
)]
struct Cli {
    /// Sujet à afficher (ex: ownership, result, iterateurs, clippy)
    topic: Option<String>,

    /// Ouvre la TUI (sur le sujet si donné)
    #[arg(short, long)]
    tui: bool,

    /// Liste les fiches (optionnellement d'une catégorie)
    #[arg(short, long, value_name = "CATEGORIE", num_args = 0..=1)]
    list: Option<Option<String>>,

    /// Recherche plein texte dans les fiches
    #[arg(short, long, value_name = "REQUETE")]
    search: Option<String>,

    /// Affiche une fiche au hasard (le rituel du matin)
    #[arg(long)]
    random: bool,

    #[command(subcommand)]
    command: Option<Cmd>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Vérifie les conventions de style Rust sur un fichier ou un dossier
    Check {
        /// Fichier .rs ou répertoire du projet
        path: String,
        /// Sortie JSON (pour scripts)
        #[arg(long)]
        json: bool,
    },
    /// Corrige automatiquement ce qui peut l'être (espaces, tabs, newline final...)
    Fix {
        /// Fichier .rs ou répertoire du projet
        path: String,
        /// Affiche ce qui serait corrigé sans modifier les fichiers
        #[arg(long)]
        dry_run: bool,
    },
    /// Explique une règle de style (ex: rust-man explain R-COL100)
    Explain {
        /// Code de la règle (R-COL100, R-UNWRAP, R-SNAKE...)
        rule: String,
    },
    /// Lance un quiz (catégorie optionnelle : bases, ownership, erreurs, types...)
    Quiz {
        /// Catégorie de questions
        category: Option<String>,
        /// Nombre de questions
        #[arg(short = 'n', long, default_value = "10")]
        count: usize,
        /// Secondes par question (mode chrono)
        #[arg(long)]
        chrono: Option<u64>,
    },
    /// Affiche ta progression
    Progress,
    /// Pose une question à un agent IA (ex: rust-man ask professeur "c'est quoi l'ownership ?")
    Ask {
        /// Agent : professeur, reviewer, debugger, mentor, algo, doc, compiler, memory,
        /// cargo, exam, explainer, orchestrateur
        agent: String,
        /// Ta question
        #[arg(required = true)]
        question: Vec<String>,
        /// Niveau d'aide 1 (socratique, défaut) à 5 (analyse complète)
        #[arg(short = 'L', long, default_value = "1")]
        level: u8,
        /// Modèle IA (kimi-k3, deepseek-v4.1-flash, glm-5.3-prime, qwen3.8-max-0902)
        #[arg(short = 'm', long)]
        model: Option<String>,
    },
    /// Lance dsh (DeepSeek-Harness) — détecté automatiquement
    Dsh {
        /// Arguments passés à dsh
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Crée un fichier .rs avec squelette (fn main ou fonction signée)
    New {
        /// Nom du fichier (ex: mon_exo)
        name: String,
        /// Signature de la fonction (ex: "fn fizzbuzz(n: u32) -> Vec<String>")
        #[arg(short, long)]
        func: Option<String>,
    },
    /// Apprends un sujet sous tous ses angles (fiche + quiz + pièges + agent)
    Apprendre {
        /// Sujet (id de fiche, ex: pointeurs)
        sujet: String,
    },
    /// Soutenance blanche interactive sur un sujet (Coach d'Oral)
    Defend {
        /// Sujet à défendre (id de fiche, ex: malloc)
        sujet: String,
        /// Modèle IA
        #[arg(short = 'm', long)]
        model: Option<String>,
    },
    /// Build + lance les tests du projet (cargo test, ou rustc + run)
    Test {
        /// Répertoire du projet
        #[arg(default_value = ".")]
        path: String,
    },
    /// Vérifie ton environnement Rust (rustc, cargo, clippy, IA, dsh)
    Doctor,
    /// Sert le dossier courant en HTTP (ex: la doc de cargo doc)
    Serve {
        /// Port de départ (défaut 8080, +1 si pris)
        #[arg(short, long, default_value = "8080")]
        port: u16,
    },
    /// Revue IA complète du projet avant un rendu (bugs + style + lisibilité)
    Review {
        /// Répertoire du projet
        #[arg(default_value = ".")]
        path: String,
        /// Modèle IA
        #[arg(short = 'm', long)]
        model: Option<String>,
    },
    /// Minuteur de focus (pomodoro) — découpe ta journée en blocs
    Focus {
        /// Minutes de focus (défaut 25)
        #[arg(default_value = "25")]
        minutes: u32,
    },
    /// Analyse un énoncé d'exercice/projet : livrables, contraintes, cas limites, plan
    Analyse {
        /// Fichier contenant l'énoncé (ou - pour stdin)
        fichier: String,
        /// Modèle IA
        #[arg(short = 'm', long)]
        model: Option<String>,
    },
    /// Re-vérifie le style à chaque modification (surveille le dossier)
    Watch {
        /// Dossier à surveiller
        #[arg(default_value = ".")]
        path: String,
    },
    /// Le rituel de livraison : add + commit + push en une commande
    Push {
        /// Message de commit
        #[arg(default_value = "travail du jour")]
        message: String,
    },
    /// Exercice guidé : la tâche, les indices, et la vérif réelle
    Exo {
        /// Id de l'exercice (ex: fizzbuzz) — vide pour la liste
        id: Option<String>,
        /// Compile + lance les tests de ta solution
        #[arg(long)]
        test: bool,
        /// Montre un indice (le niveau n, 1, 2…)
        #[arg(long)]
        indice: Option<usize>,
    },
    /// Scaffold un projet cargo complet (Cargo.toml, src/main.rs, tests)
    NewProjet {
        /// Nom du projet
        nom: String,
    },
    /// Rush : analyse l'énoncé + scaffold le projet, en une commande
    Rush {
        /// Fichier contenant l'énoncé (ou - pour stdin)
        sujet: String,
        /// Nom du projet (défaut : déduit du sujet ou "rush")
        #[arg(short, long)]
        nom: Option<String>,
    },
    /// Bilan du soir : ce que tu as fait + la suite pour demain
    Recap,
    /// Note personnelle sur une fiche (montre-la, ou écris-la)
    Note {
        /// Id de la fiche
        fiche: String,
        /// Le texte de la note (vide = afficher la note actuelle)
        texte: Option<String>,
    },
    /// Tu es bloqué ? Décris ton problème, on te débloque
    Bloque {
        /// Ton problème en quelques mots (ex: "mon malloc segfaulte")
        probleme: String,
    },
    /// Recherche unifiée : fiches + exercices + quiz + agents
    Apropos {
        /// Le mot recherché
        mot: String,
    },
    /// Journal d'apprentissage : écris ta réflexion du jour (ou relis)
    Journal {
        /// Ta réflexion (vide = lire le journal d'aujourd'hui)
        texte: Option<String>,
        /// Relis le journal d'un jour passé (offset en jours, 1 = hier)
        #[arg(short, long)]
        relire: Option<u32>,
    },
    /// Examen blanc : un quiz chronométré et noté (toutes catégories mélangées)
    Exam {
        /// Nombre de questions (défaut 10)
        #[arg(short = 'n', long, default_value = "10")]
        count: usize,
        /// Minutes au total (défaut 15)
        #[arg(short = 't', long, default_value = "15")]
        minutes: u64,
    },
    /// La UNE chose à faire maintenant (réduit la charge de décision)
    Next,
    /// Session du matin : révision ciblée sur tes points fragiles
    Morning,
    /// Relis tes synthèses « apprends-moi » sauvegardées
    Revision {
        /// Ouvre la nième (1 = plus récente)
        #[arg(default_value = "1")]
        n: usize,
    },
    /// Ouvre l'éditeur rust-nano sur un fichier
    Edit {
        /// Fichier à ouvrir/créer
        path: Option<String>,
    },
    /// Plan de journée par l'orchestrateur « Complémentarité de l'Homme »
    Plan {
        /// Tes tâches du jour, en langage naturel
        #[arg(required = true)]
        mission: Vec<String>,
    },
}

fn renderer() -> Renderer {
    let is_tty = std::io::stdout().is_terminal();
    let width = if is_tty {
        crossterm::terminal::size()
            .map(|(w, _)| w as usize)
            .unwrap_or(80)
    } else {
        80
    };
    Renderer {
        color: is_tty && std::env::var_os("NO_COLOR").is_none(),
        width,
    }
}

/// Écrit sur stdout en survivant aux broken pipes (`rust-man x | head`).
fn print_out(text: &str) {
    use std::io::Write;
    let mut stdout = std::io::stdout().lock();
    match stdout.write_all(text.as_bytes()).and_then(|_| stdout.flush()) {
        Ok(()) => {}
        Err(_) => std::process::exit(0),
    }
}

/// Titre court : la partie après « — » si présente.
fn short_title(entry: &content::Entry) -> &str {
    entry.title.split('—').nth(1).map(str::trim).unwrap_or(&entry.title)
}

fn show_topic(topic: &str) {
    match content::by_id(topic) {
        Some(entry) => {
            let mut out = renderer().render_entry(entry);
            // ta note personnelle, si elle existe
            if let Some(note) = progress::load().note(&entry.id) {
                out.push_str(&format!("\n  📝 Ta note : {note}\n"));
            }
            print_out(&out);
        }
        None => {
            let suggestions = content::search(topic);
            let mut msg = format!("rust-man: aucune fiche pour « {topic} ».\n");
            if !suggestions.is_empty() {
                msg.push_str("\nVouliez-vous dire :\n");
                for (_, e) in suggestions.iter().take(5) {
                    msg.push_str(&format!("  {:<24} {}\n", e.id, short_title(e)));
                }
            }
            msg.push_str("\n`rust-man -l` liste toutes les fiches.\n");
            eprint!("{msg}");
            std::process::exit(1);
        }
    }
}

fn run_tui(topic: Option<&str>) {
    if let Err(e) = tui::run(topic) {
        eprintln!("rust-man: erreur TUI: {e}");
        std::process::exit(1);
    }
}

fn list_entries(filter: Option<&str>) {
    let mut out = String::new();
    let mut current_cat = String::new();
    let mut shown = 0;
    for entry in ENTRIES.iter() {
        if let Some(f) = filter {
            let f = f.to_lowercase();
            if entry.category != f
                && !entry.id.contains(&f)
                && !entry.title.to_lowercase().contains(&f)
            {
                continue;
            }
        }
        if entry.category != current_cat {
            current_cat = entry.category.clone();
            out.push_str(&format!("\n  {}\n", category_label(&current_cat)));
            out.push_str(&format!("  {}\n", "─".repeat(30)));
        }
        let sem = entry
            .semaine
            .as_deref()
            .map(|d| format!(" [{d}]"))
            .unwrap_or_default();
        out.push_str(&format!(
            "    {:<24}{sem}  {}\n",
            entry.id,
            short_title(entry)
        ));
        shown += 1;
    }
    if shown == 0 {
        if let Some(f) = filter {
            out.push_str(&format!("Aucune fiche pour le filtre « {f} ».\n"));
            out.push_str("Catégories : demarrer, bases, ownership, erreurs, types, collections, outils, pratique.\n");
        }
    } else {
        out.push_str(&format!(
            "\n{shown} fiches. `rust-man <sujet>` pour lire, `rust-man` pour naviguer.\n"
        ));
    }
    print_out(&out);
}

fn search_entries(query: &str) {
    let results = content::search(query);
    if results.is_empty() {
        print_out(&format!("Aucun résultat pour « {query} ».\n"));
        return;
    }
    let mut out = format!("{} résultats pour « {query} » :\n\n", results.len());
    for (_, e) in results.iter().take(15) {
        out.push_str(&format!("  {:<24} {}\n", e.id, e.title));
    }
    out.push_str("\n`rust-man <sujet>` pour lire une fiche.\n");
    print_out(&out);
}

fn random_entry() {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as usize)
        .unwrap_or(0);
    let entry = &ENTRIES[nanos % ENTRIES.len()];
    print_out(&format!("🎲 Fiche du hasard : {}\n", entry.id));
    show_topic(&entry.id.clone());
}

// ------------------------------------------------------------------ check / fix

fn cmd_check(path: &str, json: bool) -> ! {
    let cfg = config::load();
    let checker = cfg.style.to_checker();
    let target = Path::new(path);
    if !target.exists() {
        eprintln!("rust-man check: chemin introuvable : {path}");
        std::process::exit(2);
    }
    // dossier ou fichier : style Rust
    match style::check_path(target, &checker) {
        Ok(report) => {
            // enregistre dans la progression
            let mut p = progress::load();
            p.record_style_run(
                path,
                report.total(style::Severity::Major) as u32,
                report.total(style::Severity::Minor) as u32,
            );
            progress::save(&p);

            if json {
                print_out(&report_json(&report));
            } else {
                print_out(&report_text(&report, path));
            }
            if report.total(style::Severity::Major) > 0 {
                std::process::exit(1);
            }
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("rust-man check: {e}");
            std::process::exit(2);
        }
    }
}

fn report_text(report: &style::Report, path: &str) -> String {
    let r = renderer();
    let mut out = String::new();
    let (b, dim, red, yel, cyn, rst) = if r.color {
        ("\x1b[1m", "\x1b[2m", "\x1b[31m", "\x1b[33m", "\x1b[36m", "\x1b[0m")
    } else {
        ("", "", "", "", "", "")
    };
    for file in &report.files {
        if file.findings.is_empty() {
            continue;
        }
        out.push_str(&format!("\n{b}{}{rst}\n", file.path.display()));
        for f in &file.findings {
            let color = match f.severity {
                style::Severity::Major => red,
                style::Severity::Minor => yel,
                style::Severity::Info => cyn,
            };
            let fixable = if f.fix.is_some() { " [fixable]" } else { "" };
            out.push_str(&format!(
                "  {:>4}:{:<3} {color}[{}]{rst} {}{dim} ({}){fixable}{rst}\n",
                f.line, f.col, f.severity, f.message, f.rule
            ));
        }
    }
    let majors = report.total(style::Severity::Major);
    let minors = report.total(style::Severity::Minor);
    let infos = report.total(style::Severity::Info);
    let fixable = report.fixable_count();
    out.push('\n');
    if report.findings_count() == 0 {
        out.push_str(&format!("{b}✓ {path} : aucune faute de style détectée.{rst}\n"));
    } else {
        out.push_str(&format!(
            "{b}{path}{rst} : {red}{majors} majeures{rst}, {yel}{minors} mineures{rst}, {infos} infos — {fixable} auto-corrigibles\n"
        ));
        out.push_str(&format!(
            "{dim}→ `rust-man fix {path}` corrige les auto-corrigibles · `rust-man explain <REGLE>` explique une règle{rst}\n"
        ));
    }
    out
}

fn report_json(report: &style::Report) -> String {
    let files: Vec<serde_json::Value> = report
        .files
        .iter()
        .map(|f| {
            serde_json::json!({
                "path": f.path.display().to_string(),
                "findings": f.findings.iter().map(|x| serde_json::json!({
                    "line": x.line,
                    "col": x.col,
                    "severity": x.severity.to_string(),
                    "rule": x.rule,
                    "message": x.message,
                    "fixable": x.fix.is_some(),
                })).collect::<Vec<_>>(),
            })
        })
        .collect();
    serde_json::to_string_pretty(&serde_json::json!({ "files": files })).unwrap()
}

fn cmd_fix(path: &str, dry_run: bool) -> ! {
    let cfg = config::load();
    let checker = cfg.style.to_checker();
    let target = Path::new(path);
    if !target.exists() {
        eprintln!("rust-man fix: chemin introuvable : {path}");
        std::process::exit(2);
    }
    let mut total_applied = 0;
    // itère jusqu'au point fixe : une ligne peut cumuler plusieurs fautes
    // (tab + espace après if...) dont chaque correction en révèle une autre.
    for round in 0..3 {
        let report = match style::check_path(target, &checker) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("rust-man fix: {e}");
                std::process::exit(2);
            }
        };
        let mut round_applied = 0;
        for file in &report.files {
            let fixes = style::dedup_fixes(&file.findings);
            if fixes.is_empty() {
                continue;
            }
            let src = std::fs::read_to_string(&file.path).unwrap_or_default();
            let (new_src, applied) = style::apply_fixes(&src, &fixes);
            if applied == 0 {
                continue;
            }
            if dry_run {
                if round == 0 {
                    print_out(&format!(
                        "— {} : {} correction(s) applicable(s)\n",
                        file.path.display(),
                        applied
                    ));
                }
            } else {
                if let Err(e) = std::fs::write(&file.path, &new_src) {
                    eprintln!("rust-man fix: impossible d'écrire {}: {e}", file.path.display());
                    continue;
                }
                if round == 0 {
                    print_out(&format!(
                        "✓ {} : {} correction(s) appliquée(s)\n",
                        file.path.display(),
                        applied
                    ));
                }
            }
            round_applied += applied;
        }
        total_applied += round_applied;
        if dry_run || round_applied == 0 {
            break;
        }
    }
    // re-vérification après correction
    if !dry_run && total_applied > 0 {
        if let Ok(after) = style::check_path(target, &checker) {
            print_out(&format!(
                "\nRe-vérification : {} faute(s) restante(s) ({} majeures). Les corrections non \
                 automatiques demandent ta main — `rust-man check {path}` pour le détail, \
                 `rust-man explain <REGLE>` pour comprendre.\n",
                after.findings_count(),
                after.total(style::Severity::Major)
            ));
        }
    } else if dry_run {
        print_out(&format!(
            "\n(dry-run) {total_applied} correction(s) seraient appliquées. Sans --dry-run, rust-man les applique puis re-vérifie.\n"
        ));
    } else {
        print_out("Rien à corriger automatiquement.\n");
    }
    std::process::exit(0);
}

fn cmd_explain(rule: &str) -> ! {
    let mut key = rule.to_uppercase();
    if !key.starts_with("R-") {
        key = format!("R-{key}");
    }
    let (titre, pourquoi, comment) = style::explain_rule(&key);
    if titre == "Règle de style" {
        eprintln!("rust-man explain: règle inconnue « {rule} ». Exemples : R-COL100, R-UNWRAP, R-SNAKE, R-TAB...");
        std::process::exit(1);
    }
    let r = renderer();
    let (b, rst) = if r.color { ("\x1b[1m", "\x1b[0m") } else { ("", "") };
    print_out(&format!(
        "\n{b}{key} — {titre}{rst}\n\nPourquoi : {pourquoi}\n\nComment corriger : {comment}\n"
    ));
    std::process::exit(0);
}

// ------------------------------------------------------------------ quiz / progress

fn cmd_quiz(category: Option<&str>, count: usize, chrono: Option<u64>) -> ! {
    let valid = ["demarrer", "bases", "ownership", "erreurs", "types", "collections", "outils", "pratique"];
    if let Some(c) = category {
        if !valid.contains(&c) {
            eprintln!("rust-man quiz: catégorie inconnue « {c} » ({})", valid.join(", "));
            std::process::exit(2);
        }
    }
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64)
        .unwrap_or(1);
    let questions = quiz::pick(category, count, seed);
    if questions.is_empty() {
        eprintln!("rust-man quiz: aucune question disponible.");
        std::process::exit(1);
    }
    let stdin = std::io::stdin();
    let mut correct = 0u32;
    let total = questions.len() as u32;
    print_out(&format!(
        "\nQuiz {} — {} questions. Réponds avec le numéro du choix.\n",
        category.unwrap_or("toutes catégories"),
        total
    ));
    for (i, q) in questions.iter().enumerate() {
        print_out(&format!(
            "\n── Question {}/{} ──\n{}\n",
            i + 1,
            total,
            q.question
        ));
        for (j, choice) in q.choices.iter().enumerate() {
            print_out(&format!("  {}. {}\n", j + 1, choice));
        }
        print_out("> ");
        let t0 = std::time::Instant::now();
        let mut answer = String::new();
        if stdin.read_line(&mut answer).is_err() {
            break;
        }
        let secs = t0.elapsed().as_secs();
        if let Some(limit) = chrono {
            if secs > limit {
                print_out(&format!("  ⏱ {}s (limite {}s) — trop lent, mais on continue.\n", secs, limit));
            }
        }
        let ok = answer
            .trim()
            .parse::<usize>()
            .ok()
            .is_some_and(|n| n == q.answer + 1);
        if ok {
            correct += 1;
            print_out("✓ Correct !\n");
        } else {
            print_out(&format!("✗ Raté — la bonne réponse était {}.\n", q.answer + 1));
            // pointe la fiche liée à relire
            if let Some(fiche) = q.related.first() {
                print_out(&format!("  → relis : rust-man {fiche}\n"));
            }
        }
        print_out(&format!("  {}\n", q.explanation));
    }
    let mut p = progress::load();
    p.record_quiz(category.unwrap_or("general"), total, correct);
    progress::save(&p);
    let pct = if total > 0 { correct * 100 / total } else { 0 };
    print_out(&format!(
        "\n══ Score : {correct}/{total} ({pct} %) ══\n{}\n",
        match pct {
            90..=100 => "Solide. Ce thème est validé — passe au suivant.",
            70..=89 => "Bien ! Encore quelques fiches à relire.",
            50..=69 => "Mitigé : relis les fiches liées et retente.",
            _ => "Retourne aux fiches du thème, puis retente le quiz.",
        }
    ));
    std::process::exit(0);
}

fn cmd_progress() -> ! {
    let p = progress::load();
    let mut out = String::from("\n  📊 Ta progression rust-man\n\n");
    // streak + activité de la semaine + focus
    let streak = p.streak();
    if streak > 0 {
        let flame = if streak >= 7 { "🔥" } else { "📅" };
        out.push_str(&format!("  {flame} Série : {streak} jour(s) d'affilée\n"));
    }
    out.push_str("  Cette semaine : ");
    let today = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
        / 86_400
        * 86_400;
    for i in (0..7).rev() {
        let day = today - i * 86_400;
        out.push_str(if p.active_days.contains(&day) { "■" } else { "·" });
    }
    out.push('\n');
    if !p.focus_sessions.is_empty() {
        let total: u64 = p.focus_sessions.iter().sum();
        out.push_str(&format!(
            "  Focus : {} blocs, {} min de travail profond\n",
            p.focus_sessions.len(),
            total / 60
        ));
    }
    out.push('\n');
    if p.defended_count() > 0 {
        out.push_str(&format!(
            "  🎓 Notions défendues (tu sais les expliquer) : {}\n",
            p.defended_count()
        ));
    }
    out.push_str(&format!("  Fiches consultées : {} / {}\n", p.fiches.len(), ENTRIES.len()));
    for (cid, label) in content::CATEGORIES {
        let (seen, total) = p.category_coverage(cid, &ENTRIES);
        if total > 0 {
            let bar = progress_bar(seen, total, 20);
            out.push_str(&format!("    {label:<22} {bar} {seen}/{total}\n"));
        }
    }
    let total_q: u32 = p.quiz.iter().map(|q| q.total).sum();
    let correct: u32 = p.quiz.iter().map(|q| q.correct).sum();
    if total_q > 0 {
        out.push_str(&format!(
            "\n  Quiz : {} sessions, {correct}/{total_q} bonnes réponses ({} %)\n",
            p.quiz.len(),
            correct * 100 / total_q
        ));
    }
    let weak = p.weak_categories();
    if !weak.is_empty() {
        out.push_str("  Notions fragiles : ");
        out.push_str(
            &weak
                .iter()
                .map(|(c, ok, t)| format!("{c} ({ok}/{t})"))
                .collect::<Vec<_>>()
                .join(", "),
        );
        out.push('\n');
    }
    let done: usize = p.exercises_done.values().map(|v| v.len()).sum();
    out.push_str(&format!("  Exercices marqués faits : {done}\n"));
    if let Some(last) = p.style_runs.last() {
        out.push_str(&format!(
            "  Dernier check style : {} ({} majeures, {} mineures)\n",
            last.path, last.majors, last.minors
        ));
    }
    out.push('\n');
    print_out(&out);
    std::process::exit(0);
}

fn progress_bar(done: usize, total: usize, width: usize) -> String {
    let filled = if total > 0 { done * width / total } else { 0 };
    format!("{}{}", "█".repeat(filled), "░".repeat(width - filled))
}

// ------------------------------------------------------------------ ask / dsh

fn parse_agent(name: &str) -> Option<ai::AgentRole> {
    let n = name.to_lowercase();
    Some(match n.as_str() {
        // pôle humain
        "intervenant" | "prof" | "professeur" | "expert" => ai::AgentRole::Intervenant,
        "aine" | "aîné" | "senior" => ai::AgentRole::Aine,
        "socratique" | "socratic" => ai::AgentRole::BinomeSocratique,
        "bas-niveau" | "basniveau" | "machine" | "memory" | "memoire" => ai::AgentRole::BinomeBasNiveau,
        "algo" | "algorithm" | "binome-algo" => ai::AgentRole::BinomeAlgo,
        "clean" | "lisibilite" | "design" => ai::AgentRole::BinomeClean,
        "debugger" | "debug" => ai::AgentRole::BinomeDebug,
        "motivation" | "motiv" | "mentor" => ai::AgentRole::BinomeMotivation,
        "guide" | "exo" | "exercice" | "guide-exercice" => ai::AgentRole::GuideExercice,
        "oral" | "soutenance" | "coach-oral" => ai::AgentRole::CoachOral,
        "intuition" | "ressenti" | "feel" => ai::AgentRole::BinomeIntuition,
        // spécialistes
        "reviewer" | "review" | "code-reviewer" => ai::AgentRole::CodeReviewer,
        "style" | "style-checker" | "clippy" => ai::AgentRole::StyleChecker,
        "compiler" | "compiler-assistant" | "rustc" | "borrow" | "borrow-checker" => ai::AgentRole::CompilerAssistant,
        "cargo" | "cargo-assistant" | "crates" => ai::AgentRole::CargoAssistant,
        "doc" | "documentation" => ai::AgentRole::DocAssistant,
        "exam" | "exam-coach" => ai::AgentRole::ExamCoach,
        "explainer" | "code-explainer" => ai::AgentRole::CodeExplainer,
        "orchestrateur" | "orchestrator" | "complementarite" => ai::AgentRole::Orchestrateur,
        _ => return None,
    })
}

fn cmd_ask(agent: &str, question: Vec<String>, level: u8, model: Option<String>) -> ! {
    let Some(role) = parse_agent(agent) else {
        eprintln!(
            "rust-man ask: agent inconnu « {agent} ». Disponibles : {}",
            ai::AGENTS
                .iter()
                .map(|a| a.name())
                .collect::<Vec<_>>()
                .join(", ")
        );
        std::process::exit(2);
    };
    let cfg = config::load();
    let mut client = match ai::AiClient::from_env(&cfg) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("rust-man ask: {e}");
            std::process::exit(3);
        }
    };
    if let Some(m) = &model {
        client.set_model(m);
    }
    let q = question.join(" ");
    let lvl = ai::HelpLevel::from_u8(level);
    // injecte le contexte projet (fichiers du répertoire courant)
    let cwd = std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf());
    let snap = project::snapshot(&cwd);
    let q = format!("{}\n\n{}", snap.to_prompt(), q);
    eprintln!("⏳ {} (niveau {}) réfléchit…", role.name(), lvl.name());
    match client.chat(&ai::system_prompt(role, lvl), &q) {
        Ok(answer) => {
            print_out(&format!("\n{answer}\n"));
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("rust-man ask: {e}");
            std::process::exit(3);
        }
    }
}

/// Session multi-angle sur un sujet : fiche, quiz lié, pièges, agent.
fn cmd_apprendre(sujet: &str) -> ! {
    // trouve la fiche (exact puis recherche)
    let entry = match content::by_id(sujet) {
        Some(e) => e,
        None => match content::search(sujet).first() {
            Some((_, e)) => e,
            None => {
                eprintln!("rust-man apprendre: sujet inconnu « {sujet} ». `rust-man -l` pour la liste.");
                std::process::exit(1);
            }
        },
    };
    let mut out = format!("\n  📖 {} — sous tous ses angles\n\n", entry.id);
    // 1. la fiche
    out.push_str(&format!("  1. LIS la fiche :        rust-man {}\n", entry.id));
    // 2. quiz lié (questions dont related contient la fiche)
    let nb_quiz = quiz::QUESTIONS
        .iter()
        .filter(|q| q.related.contains(&entry.id))
        .count();
    if nb_quiz > 0 {
        out.push_str(&format!(
            "  2. TESTE-toi :           rust-man quiz {}   ({} question(s) liée(s))\n",
            entry.category, nb_quiz
        ));
    } else {
        out.push_str(&format!("  2. TESTE-toi :           rust-man quiz {}\n", entry.category));
    }
    // 3. les pièges
    if !entry.gotchas.is_empty() {
        out.push_str(&format!(
            "  3. ÉVITE les pièges :    {} piège(s) dans la fiche (lis la section PIÈGES)\n",
            entry.gotchas.len()
        ));
    }
    // 4. les exercices : d'abord l'exo guidé lié (vérifié), puis ceux de la fiche
    let exo_lie = exo::EXOS.iter().find(|e| e.fiche == entry.id);
    if let Some(ex) = exo_lie {
        out.push_str(&format!(
            "  4. PRATIQUE (vérifié) :  rust-man exo {}   ← ton code est testé\n",
            ex.id
        ));
    }
    if !entry.exercises.is_empty() {
        out.push_str(&format!(
            "     + {} exercice(s) libre(s) dans la fiche (section EXERCICES)\n",
            entry.exercises.len()
        ));
    }
    // 5. les agents pour creuser
    out.push_str(&format!(
        "  5. CREUSE avec un agent :\n\
             \x20   rust-man ask socratique \"{}\"   (on te guide par questions)\n\
             \x20   rust-man ask oral \"{}\"         (défends-le comme en soutenance)\n\
             \x20   rust-man ask guide \"{}\"        (exo guidé pas à pas)\n",
        entry.id, entry.id, entry.id
    ));
    out.push_str("\n  L'ordre compte : lis → teste → pratique → explique. C'est comme ça\
                  \n  qu'on passe de « je crois savoir » à « je le ressens ».\n\n");
    print_out(&out);
    // enregistre la consultation
    let mut p = progress::load();
    p.record_view(&entry.id, 0);
    progress::save(&p);
    std::process::exit(0);
}

/// Soutenance blanche : le Coach d'Oral te questionne sur une fiche,
/// évalue tes explications et creuse, jusqu'à « stop ».
fn cmd_defend(sujet: &str, model: Option<&str>) -> ! {
    let entry = match content::by_id(sujet) {
        Some(e) => e,
        None => match content::search(sujet).first() {
            Some((_, e)) => e,
            None => {
                eprintln!("rust-man defend: sujet inconnu « {sujet} ».");
                std::process::exit(1);
            }
        },
    };
    let cfg = config::load();
    let mut client = match ai::AiClient::from_env(&cfg) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("rust-man defend: {e}");
            std::process::exit(3);
        }
    };
    if let Some(m) = model {
        client.set_model(m);
    }
    let system = ai::system_prompt(ai::AgentRole::CoachOral, ai::HelpLevel::Guidage);
    let contexte = format!(
        "L'étudiant doit défendre la notion Rust « {} ». Contenu de référence :\n\n{}\n\n\
         Exemple :\n{}\n\nTu es l'examinateur. Commence par UNE question ouverte sur le cœur \
         du sujet. Puis, à chaque réponse de l'étudiant : évalue en une phrase, creuse avec une \
         question plus dure. S'il bloque, donne un indice. Termine chaque message par la \
         prochaine question.",
        entry.title, entry.description, entry.example
    );
    print_out(&format!(
        "\n  ⚔ Soutenance blanche : « {} »\n  Le Coach t'interroge. Réponds, il évalue et creuse.\
         \n  (« stop » pour finir)\n\n",
        entry.title
    ));
    let mut history = String::new();
    #[allow(unused_assignments)]
    let mut question = String::new();
    // première question
    eprint!("  ⏳ le Coach prépare sa question…\r");
    let first = match client.chat(&system, &format!("{contexte}\n\nPose ta première question.")) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("\nrust-man defend: {e}");
            std::process::exit(3);
        }
    };
    question = first.trim().to_string();
    let stdin = std::io::stdin();
    loop {
        print_out(&format!("  🎓 Coach : {}\n\n", question.trim()));
        print_out("  toi ▸ ");
        let mut reponse = String::new();
        if stdin.read_line(&mut reponse).is_err() {
            break;
        }
        let reponse = reponse.trim();
        if reponse.eq_ignore_ascii_case("stop") || reponse.is_empty() {
            break;
        }
        history.push_str(&format!("Examinateur : {question}\nÉtudiant : {reponse}\n\n"));
        eprint!("  ⏳ …\r");
        let eval = client.chat(
            &system,
            &format!("{contexte}\n\nÉchanges jusqu'ici :\n{history}\nÉvalue puis poursuis."),
        );
        match eval {
            Ok(r) => question = r.trim().to_string(),
            Err(e) => {
                eprintln!("\nrust-man defend: {e}");
                break;
            }
        }
    }
    // la notion est marquée comme défendue
    let mut p = progress::load();
    p.record_defended(&entry.id);
    progress::save(&p);
    print_out(&format!(
        "\n  Soutenance terminée. « {} » est marquée défendue ✓\n\n",
        entry.id
    ));
    std::process::exit(0);
}

/// Build + teste le projet : cargo test si Cargo.toml, sinon compile + lance main.rs.
fn cmd_test(path: &str) -> ! {
    let dir = Path::new(path);
    if !dir.is_dir() {
        eprintln!("rust-man test: {path} n'est pas un dossier.");
        std::process::exit(2);
    }
    // 1. style d'abord (une faute majeure et on le dit)
    let checker = config::load().style.to_checker();
    if let Ok(rep) = style::check_path(dir, &checker) {
        let majors = rep.total(style::Severity::Major);
        if majors > 0 {
            print_out(&format!(
                "⚠ {} faute(s) MAJEURE(S) de style — `rust-man check {path}` pour le détail.\n\n",
                majors
            ));
        }
    }
    // 2. cargo test si c'est un projet cargo
    if dir.join("Cargo.toml").exists() {
        let res = compile::run_cargo_test(dir);
        print_out("── cargo test ──\n");
        print_out(&res.raw);
        if res.success {
            print_out("\n✓ tests passés.\n");
            std::process::exit(0);
        }
        print_out("\n✗ des tests échouent (ou la compile a cassé).\n");
        std::process::exit(1);
    }
    // 3. repli : compile + exécute main.rs si présent
    let res = compile::compile_target(dir);
    print_out(&compile_text(&res));
    if res.success {
        let bin = compile::temp_bin();
        let bin = bin.as_path();
        if bin.exists() {
            if let Ok(r) = std::process::Command::new(bin).output() {
                print_out(&format!("── exécution ──\n{}", String::from_utf8_lossy(&r.stdout)));
                std::process::exit(if r.status.success() { 0 } else { 1 });
            }
        }
    }
    std::process::exit(if res.success { 0 } else { 1 });
}

/// Rend le résultat de compilation lisible.
fn compile_text(res: &compile::CompileResult) -> String {
    if res.diagnostics.is_empty() {
        return if res.success {
            "✓ compile propre.\n".into()
        } else {
            format!("✗ échec sans diagnostic.\n{}\n", res.raw)
        };
    }
    let mut out = String::new();
    for d in &res.diagnostics {
        let lvl = match d.level {
            compile::DiagLevel::Error => "ERREUR ",
            compile::DiagLevel::Warning => "warning",
            compile::DiagLevel::Note => "note   ",
        };
        out.push_str(&format!("  {}:{} {} {}\n", d.file, d.line, lvl, d.message));
        if let Some(h) = d.hint_text {
            out.push_str(&format!("      → {h}\n"));
        }
    }
    out
}

/// Vérifie l'environnement Rust : toolchain, IA, dsh, config.
fn cmd_doctor() -> ! {
    let mut out = String::from("\n  🩺 rust-man doctor — ton environnement Rust\n\n");
    let check = |nom: &str, ok: bool, hint: &str| {
        format!("  {} {:<22} {}\n", if ok { "✓" } else { "✗" }, nom, if ok { "" } else { hint })
    };
    // `which` n'existe pas sous Windows : on tente `where`, sinon --version direct
    let has = |cmd: &str| {
        let probe = if cfg!(windows) { "where" } else { "which" };
        std::process::Command::new(probe).arg(cmd)
            .output().map(|o| o.status.success()).unwrap_or(false)
    };
    // outils
    out.push_str(&check("rustc", has("rustc"), "→ installe rustup : rust-man installer-rust"));
    out.push_str(&check("cargo", has("cargo"), "→ fourni avec rustup"));
    out.push_str(&check("rustfmt", has("rustfmt"), "→ rustup component add rustfmt"));
    out.push_str(&check("clippy", std::process::Command::new("cargo").arg("clippy").arg("--version").output().map(|o| o.status.success()).unwrap_or(false), "→ rustup component add clippy"));
    out.push_str(&check("rust-analyzer", has("rust-analyzer"), "→ rustup component add rust-analyzer"));
    out.push_str(&check("git", has("git"), "→ sudo pacman -S git"));
    // IA
    let cfg = config::load();
    let ai_ok = ai::AiClient::from_env(&cfg).is_ok();
    out.push_str(&check("IA (clé API)", ai_ok, "→ définis OPP_API_KEY ou ~/.dsh/.credentials.yaml"));
    if ai_ok {
        out.push_str(&format!(
            "  • {:<22} {} · contexte {}M tokens · sortie max {}K tokens\n",
            "IA (enveloppe)",
            cfg.ai_model.as_deref().unwrap_or("kimi-k3"),
            ai::CONTEXT_WINDOW_TOKENS / 1_048_576,
            ai::MAX_OUTPUT_TOKENS / 1024
        ));
    }
    // dsh
    let dsh_ok = dsh::detect(&cfg).is_some();
    out.push_str(&check("dsh", dsh_ok, "→ config dsh_command/dsh_dir dans ~/.config/rust-man/config.toml"));
    out.push_str("\n");
    let manquants = out.matches("✗").count();
    if manquants == 0 {
        out.push_str("  Tout est prêt. Au travail.\n\n");
    } else {
        out.push_str(&format!("  {manquants} outil(s) à installer — les commandes sont à droite.\n\n"));
    }
    print_out(&out);
    std::process::exit(0);
}

/// Sert le dossier courant en HTTP jusqu'à Ctrl+C.
fn cmd_serve(port: u16) -> ! {
    let dir = std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf());
    match serve::serve(&dir, port) {
        Ok(_) => std::process::exit(0),
        Err(e) => {
            eprintln!("rust-man serve: {e}");
            std::process::exit(2);
        }
    }
}

/// Revue IA complète du projet : norme (auto) + relecture IA du code.
fn cmd_review(path: &str, model: Option<&str>) -> ! {
    let dir = Path::new(path);
    if !dir.is_dir() {
        eprintln!("rust-man review: {path} n'est pas un dossier.");
        std::process::exit(2);
    }
    // 1. norme d'abord (gratuit, instantané)
    let checker = config::load().style.to_checker();
    if let Ok(rep) = style::check_path(dir, &checker) {
        let majors = rep.total(style::Severity::Major);
        let minors = rep.total(style::Severity::Minor);
        print_out(&format!(
            "── norme : {} majeures, {} mineures{}\n\n",
            majors,
            minors,
            if majors > 0 { "  (à corriger AVANT le rendu)" } else { "" }
        ));
    }
    // 2. relecture IA de tout le code
    let cfg = config::load();
    let mut client = match ai::AiClient::from_env(&cfg) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("rust-man review: {e}");
            std::process::exit(3);
        }
    };
    if let Some(m) = model {
        client.set_model(m);
    }
    let snap = project::snapshot(dir);
    if snap.files.is_empty() {
        eprintln!("rust-man review: aucun fichier Rust dans {path}.");
        std::process::exit(1);
    }
    eprintln!("⏳ le Code Reviewer lit ton projet ({} fichiers)…", snap.files.len());
    eprintln!("   (il lit ton code, pas le sujet — vérifie ses allégations contre la spec)");
    let system = ai::system_prompt(ai::AgentRole::CodeReviewer, ai::HelpLevel::Analyse);
    let prompt = format!(
        "Revue complète de ce projet Rust avant rendu. Pour CHAQUE problème : le fichier, la ligne, \
         la gravité (bloquant/important/mineur), et la correction. Vérifie en priorité : les \
         unwrap qui peuvent paniquer, les clones superflus, les emprunts inutilement compliqués, \
         les idiomes Rust manqués. Finis par « Le verdict : » (prêt / à corriger d'abord) et les 3 priorités.\n\n{}",
        snap.to_prompt()
    );
    match client.chat_long(&system, &prompt) {
        Ok(review) => {
            print_out(&format!("\n{}", review));
            print_out("\n");
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("rust-man review: {e}");
            std::process::exit(3);
        }
    }
}

/// Minuteur de focus : un bloc de travail, puis une pause. Enregistré.
fn cmd_focus(minutes: u32) -> ! {
    if !(5..=120).contains(&minutes) {
        eprintln!("rust-man focus: entre 5 et 120 minutes (25 est le bon réflexe).");
        std::process::exit(2);
    }
    print_out(&format!(
        "\n  🎯 Focus : {} minutes. Une seule tâche. Le reste attend.\n",
        minutes
    ));
    print_out("     (Ctrl+C pour arrêter — le bloc ne sera pas compté)\n\n");
    let start = std::time::Instant::now();
    let total = std::time::Duration::from_secs(minutes as u64 * 60);
    let mut last = 0u64;
    loop {
        let elapsed = start.elapsed();
        if elapsed >= total {
            break;
        }
        let remaining = total - elapsed;
        let m = remaining.as_secs() / 60;
        let s = remaining.as_secs() % 60;
        if remaining.as_secs() != last {
            eprint!("\r     ⏳ {:02}:{:02} restantes   ", m, s);
            use std::io::Write;
            let _ = std::io::stderr().flush();
            last = remaining.as_secs();
        }
        // vérifie Ctrl+C sans bloquer
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    let done_secs = start.elapsed().as_secs();
    print_out(&format!(
        "\n\n  ✓ Bloc terminé ({} min). Lève-toi, bois, regarde loin. 5 minutes.\n\n",
        done_secs / 60
    ));
    // enregistre la session de focus
    let mut p = progress::load();
    p.record_focus(done_secs);
    progress::save(&p);
    std::process::exit(0);
}

/// Analyse un énoncé d'exercice/projet collé dans un fichier (ou stdin avec -).
fn cmd_analyse(fichier: &str, model: Option<&str>) -> ! {
    let texte = if fichier == "-" {
        let mut buf = String::new();
        use std::io::Read;
        let _ = std::io::stdin().read_to_string(&mut buf);
        buf
    } else {
        match std::fs::read_to_string(fichier) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("rust-man analyse: impossible de lire {fichier}: {e}");
                std::process::exit(2);
            }
        }
    };
    if texte.trim().is_empty() {
        eprintln!("rust-man analyse: l'énoncé est vide.");
        std::process::exit(2);
    }
    let cfg = config::load();
    let mut client = match ai::AiClient::from_env(&cfg) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("rust-man analyse: {e}");
            std::process::exit(3);
        }
    };
    if let Some(m) = model {
        client.set_model(m);
    }
    eprintln!("⏳ l'Algorithm Coach lit l'énoncé…");
    let system = ai::system_prompt(ai::AgentRole::BinomeAlgo, ai::HelpLevel::Guidage);
    let prompt = format!(
        "Voici l'énoncé d'un exercice/projet Rust. Décompose-le pour un étudiant :\n\n\
         ## Ce qu'il faut rendre\n(fichiers, fonctions, signatures exactes)\n\n\
         ## Contraintes\n(ce qui est imposé/interdit, limites)\n\n\
         ## Les cas limites à tester\n(ceux qu'un correcteur testera)\n\n\
         ## Le plan d'attaque\n(étapes dans l'ordre, de la plus simple à la plus dure)\n\n\
         ## Les concepts à revoir d'abord\n(fiches rust-man liées si tu les connais)\n\n\
         ÉNONCÉ :\n{}",
        &texte[..texte.len().min(12000)]
    );
    match client.chat_long(&system, &prompt) {
        Ok(r) => {
            print_out(&format!("\n{}\n\n", r));
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("rust-man analyse: {e}");
            std::process::exit(3);
        }
    }
}

/// Surveille un dossier et re-vérifie le style à chaque modification.
fn cmd_watch(path: &str) -> ! {
    let dir = Path::new(path);
    if !dir.is_dir() {
        eprintln!("rust-man watch: {path} n'est pas un dossier.");
        std::process::exit(2);
    }
    let cfg = config::load().style.to_checker();
    print_out(&format!("  👁 surveillance de {path} — le style se re-vérifie à chaque sauvegarde (Ctrl+C pour arrêter)\n\n"));
    let mut derniers: std::collections::HashMap<String, std::time::SystemTime> = Default::default();
    let mut compte = 0u32;
    loop {
        // re-scan les .rs du dossier (non récursif + sous-dossiers directs)
        let mut fichiers: Vec<_> = Vec::new();
        if let Ok(rd) = std::fs::read_dir(dir) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_file() && p.extension().is_some_and(|x| x == "rs") {
                    fichiers.push(p);
                }
            }
        }
        for f in fichiers {
            if let Ok(meta) = f.metadata() {
                if let Ok(mtime) = meta.modified() {
                    let key = f.display().to_string();
                    let avant = derniers.get(&key).copied();
                    if avant.is_none() || avant != Some(mtime) {
                        derniers.insert(key.clone(), mtime);
                        if avant.is_some() {
                            // modification détectée → re-check
                            if let Ok(src) = std::fs::read_to_string(&f) {
                                let findings = style::check_source(&f, &src, &cfg);
                                let majors = findings.iter().filter(|x| x.severity == style::Severity::Major).count();
                                compte += 1;
                                if majors == 0 {
                                    print_out(&format!("  ✓ {key} — propre (check #{compte})\n"));
                                } else {
                                    print_out(&format!("  ✗ {key} — {} majeure(s) (check #{compte}) : `rust-man check {path}`\n", majors));
                                }
                            }
                        }
                    }
                }
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(800));
    }
}

/// Le rituel de livraison : add -A, commit, push, avec rappel style.
fn cmd_push(message: &str) -> ! {
    let run = |args: &[&str]| -> Option<std::process::Output> {
        std::process::Command::new("git")
            .args(args)
            .output()
            .ok()
    };
    // pas un dépôt ?
    if run(&["rev-parse", "--git-dir"]).is_none_or(|o| !o.status.success()) {
        eprintln!("rust-man push: pas un dépôt git ici.");
        std::process::exit(2);
    }
    // rappel style avant de livrer
    let checker = config::load().style.to_checker();
    if let Ok(rep) = style::check_path(Path::new("."), &checker) {
        let majors = rep.total(style::Severity::Major);
        if majors > 0 {
            print_out(&format!(
                "⚠ {} faute(s) MAJEURE(S) de style — `rust-man check .` avant de livrer !\n\n",
                majors
            ));
        }
    }
    print_out("── livraison\n");
    // add
    match run(&["add", "-A"]) {
        Some(o) if o.status.success() => {}
        _ => {
            eprintln!("rust-man push: git add a échoué.");
            std::process::exit(1);
        }
    }
    // rien à commit ?
    if let Some(o) = run(&["status", "--porcelain"]) {
        if o.stdout.is_empty() {
            print_out("  rien à livrer (tout est déjà commité).\n");
            std::process::exit(0);
        }
    }
    // commit
    match run(&["commit", "-m", message]) {
        Some(o) if o.status.success() => {
            print_out(&format!("  ✓ commit « {} »\n", message));
        }
        Some(o) => {
            eprint!("{}", String::from_utf8_lossy(&o.stderr));
            std::process::exit(1);
        }
        None => {
            eprintln!("rust-man push: git commit a échoué.");
            std::process::exit(1);
        }
    }
    // push
    match run(&["push"]) {
        Some(o) if o.status.success() => {
            print_out("  ✓ poussé. Ton travail est en sécurité.\n\n");
            std::process::exit(0);
        }
        Some(o) => {
            eprintln!("  ✗ push échoué : {}", String::from_utf8_lossy(&o.stderr));
            std::process::exit(1);
        }
        None => {
            eprintln!("rust-man push: git push a échoué.");
            std::process::exit(1);
        }
    }
}

/// Exercice guidé : pose la tâche, donne les indices, vérifie ta solution.
fn cmd_exo(id: Option<&str>, test: bool, indice: Option<usize>) -> ! {
    // liste des exercices
    let Some(id) = id else {
        let mut out = String::from("\n  Exercices guidés (le code est vérifié pour de vrai) :\n\n");
        for e in exo::EXOS {
            let fait = progress::load().exercises_done.contains_key(e.id);
            out.push_str(&format!(
                "  {} {:<20} {} {}\n",
                if fait { "✓" } else { " " },
                e.id,
                "●".repeat(e.difficulty as usize),
                e.title
            ));
        }
        out.push_str("\n  rust-man exo <id> pour commencer.\n\n");
        print_out(&out);
        std::process::exit(0);
    };
    let Some(e) = exo::by_id(id) else {
        eprintln!("rust-man exo: exercice inconnu « {id} ». `rust-man exo` pour la liste.");
        std::process::exit(1);
    };
    // le dossier de travail de l'exo
    let dir = Path::new("exo").join(e.id);
    let fichier = dir.join(e.solution_name());

    // --test : compile + lance les tests
    if test {
        if !fichier.exists() {
            eprintln!("rust-man exo: pas de solution — lance d'abord `rust-man exo {id}`.");
            std::process::exit(2);
        }
        let testrs = dir.join("_test.rs");
        std::fs::write(&testrs, exo::harness_complet(e)).unwrap();
        let bin_name = format!("_t{}", std::env::consts::EXE_SUFFIX);
        let out = std::process::Command::new("rustc")
            .args(["--edition", "2021", "-o"])
            .arg(&bin_name)
            .arg("_test.rs")
            .current_dir(&dir)
            .output();
        match out {
            Ok(o) if o.status.success() => {
                let run = std::process::Command::new(dir.join(&bin_name)).current_dir(&dir).output().unwrap();
                let txt = String::from_utf8_lossy(&run.stdout);
                print_out(&format!("── tests de {id} ──\n{txt}"));
                if run.status.success() {
                    print_out("\n  🎉 Tous les tests passent. Exercice validé !\n\n");
                    let mut p = progress::load();
                    if !p.exercise_done(e.id, 0) {
                        p.toggle_exercise(e.id, 0);
                    }
                    progress::save(&p);
                    std::process::exit(0);
                }
                print_out("\n  Des tests échouent. Relis la tâche, ou `rust-man exo {id} --indice 1`.\n\n");
                std::process::exit(1);
            }
            Ok(o) => {
                eprintln!("── compile échouée ──\n{}", String::from_utf8_lossy(&o.stderr));
                eprintln!("(besoin d'aide pour lire l'erreur ? `rust-man ask compiler <colle l'erreur>`)");
                std::process::exit(1);
            }
            Err(err) => {
                eprintln!("rust-man exo: {err}\n(rustc installé ? → rust-man installer-rust)");
                std::process::exit(2);
            }
        }
    }

    // prépare le fichier si besoin
    if !fichier.exists() {
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(&fichier, e.starter).unwrap();
    }
    // affiche la tâche
    let mut out = format!(
        "\n  ✍ Exercice : {}\n  {}\n\n  Signature imposée :\n    {}\n\n",
        e.id, e.task, e.signature
    );
    out.push_str(&format!("  Ton fichier : {}\n", fichier.display()));
    out.push_str(&format!("  Vérifie :     rust-man exo {} --test\n", e.id));
    out.push_str(&format!("  Théorie :     rust-man {}\n", e.fiche));
    out.push_str(&format!("  Bloqué ?      rust-man exo {} --indice 1\n\n", e.id));
    // indice demandé
    if let Some(n) = indice {
        if n >= 1 && n <= e.hints.len() {
            out.push_str(&format!("  💡 Indice {} : {}\n\n", n, e.hints[n - 1]));
        } else if n > e.hints.len() {
            out.push_str(&format!(
                "  Plus d'indices. Demande au Binôme Algo : rust-man ask algo \"{}\"\n\n",
                e.id
            ));
        }
    }
    print_out(&out);
    std::process::exit(0);
}

/// Scaffold un projet C complet et norme-compliant.
fn cmd_new_projet(nom: &str) -> ! {
    let dir = Path::new(nom);
    if dir.exists() {
        eprintln!("rust-man new-projet: {nom} existe déjà.");
        std::process::exit(1);
    }
    // si cargo est là, qu'il fasse le squelette de base
    let via_cargo = std::process::Command::new("cargo")
        .args(["new", nom])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !via_cargo {
        // repli sans cargo : la structure à la main
        let _ = std::fs::create_dir_all(dir.join("src"));
        std::fs::write(
            dir.join("Cargo.toml"),
            format!("[package]\nname = \"{nom}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"),
        )
        .unwrap();
        std::fs::write(dir.join("src").join("main.rs"), "fn main() {\n    println!(\"Hello, world!\");\n}\n").unwrap();
    }
    // enrichit : module logique + test d'exemple + README
    let cfg = config::load();
    let auteur = cfg.name.as_deref().unwrap_or("toi");
    let lib = "//! La logique du projet vit ici (main.rs ne fait que le wiring).\n\n/// Additionne deux nombres.\npub fn additionner(a: i32, b: i32) -> i32 {\n    a + b\n}\n\n#[cfg(test)]\nmod tests {\n    use super::*;\n\n    #[test]\n    fn addition_marche() {\n        assert_eq!(additionner(2, 40), 42);\n    }\n}\n";
    std::fs::write(dir.join("src").join("lib.rs"), lib).unwrap();
    let main = "//! Point d'entrée — le wiring seulement ; la logique est dans lib.rs.\n\nfn main() {\n    println!(\"2 + 40 = {}\", ID_PROJ::additionner(2, 40));\n}\n".replace("ID_PROJ", &nom.replace('-', "_"));
    std::fs::write(dir.join("src").join("main.rs"), main).unwrap();
    let readme = format!(
        "# {nom}\n\nProjet Rust de {auteur}.\n\n```sh\ncargo run     # exécuter\ncargo test    # les tests\ncargo clippy  # les idiomes\n```\n"
    );
    std::fs::write(dir.join("README.md"), readme).unwrap();

    print_out(&format!(
        "✓ {nom}/ créé :\n    Cargo.toml   (édition 2021)\n    src/main.rs  (wiring seulement)\n    src/lib.rs   (la logique + un test d'exemple)\n    README.md\n  → cd {nom} && cargo test\n\n"
    ));
    std::process::exit(0);
}

/// Rush : analyse l'énoncé, puis scaffold le projet. Le week-end en une commande.
fn cmd_rush(sujet: &str, nom: Option<&str>) -> ! {
    // lis le sujet
    let texte = if sujet == "-" {
        let mut buf = String::new();
        use std::io::Read;
        let _ = std::io::stdin().read_to_string(&mut buf);
        buf
    } else {
        match std::fs::read_to_string(sujet) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("rust-man rush: impossible de lire {sujet}: {e}");
                std::process::exit(2);
            }
        }
    };
    if texte.trim().is_empty() {
        eprintln!("rust-man rush: le sujet est vide.");
        std::process::exit(2);
    }
    // nom du projet
    let nom = nom.map(|s| s.to_string()).unwrap_or_else(|| "rush".to_string());
    // 1. analyse l'énoncé (IA)
    let cfg = config::load();
    if let Ok(client) = ai::AiClient::from_env(&cfg) {
        eprintln!("⏳ analyse du sujet…");
        let system = ai::system_prompt(ai::AgentRole::BinomeAlgo, ai::HelpLevel::Guidage);
        let prompt = format!(
            "Voici un sujet de rush. En 10 lignes max : les livrables exacts (fichiers, fonctions, \
             prototypes), les contraintes, les cas limites, et le plan d'attaque. Concis.\n\n{}",
            &texte[..texte.len().min(10000)]
        );
        if let Ok(r) = client.chat_long(&system, &prompt) {
            print_out(&format!("\n── l'énoncé décortiqué ──\n{}\n", r));
        }
    }
    // 2. scaffold le projet si possible
    if !Path::new(&nom).exists() {
        print_out("\n── scaffold ──\n");
        // délègue au scaffolder C
        cmd_new_projet(&nom);
    } else {
        print_out(&format!("\n  (le dossier {nom} existe déjà — pas de scaffold)\n\n"));
        std::process::exit(0);
    }
}

/// Bilan du soir : l'activité du jour + ce qui t'attend demain.
fn cmd_recap() -> ! {
    let p = progress::load();
    let today = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
        / 86_400
        * 86_400;
    let tomorrow = today + 86_400;

    // activité d'aujourd'hui
    let fiches_jour: Vec<_> = p.fiches.iter()
        .filter(|(_, f)| f.last_seen >= today && f.last_seen < tomorrow)
        .collect();
    let quiz_jour = p.quiz.iter().filter(|q| q.at >= today && q.at < tomorrow).count();
    let focus_jour: u64 = p.focus_sessions.iter().filter(|_| p.active_days.contains(&today)).sum::<u64>();

    let mut out = String::from("\n  🌙 Bilan du jour\n\n");
    out.push_str(&format!("  Fiches lues : {}\n", fiches_jour.len()));
    if !fiches_jour.is_empty() {
        for (id, _) in fiches_jour.iter().take(5) {
            out.push_str(&format!("    · {id}\n"));
        }
    }
    if quiz_jour > 0 {
        out.push_str(&format!("  Quiz : {} session(s)\n", quiz_jour));
    }
    if focus_jour > 0 {
        out.push_str(&format!("  Focus : {} min\n", focus_jour / 60));
    }
    let streak = p.streak();
    out.push_str(&format!("  Série : {} jour(s) 🔥\n", streak));

    // demain
    out.push_str("\n  ── demain ──\n");
    let weak = p.weak_categories();
    if let Some((cat, _, _)) = weak.first() {
        out.push_str(&format!("  Point fragile à revoir : rust-man quiz {cat}\n"));
    }
    if let Some(ex) = exo::EXOS.iter().find(|e| !p.exercise_done(e.id, 0)) {
        out.push_str(&format!("  Exercice : rust-man exo {}\n", ex.id));
    }
    // une fiche liée à un point fragile
    let fiche = weak.first()
        .and_then(|(cat, _, _)| ENTRIES.iter().find(|e| e.category == *cat && !p.fiches.contains_key(&e.id)))
        .map(|e| e.id.as_str());
    if let Some(f) = fiche {
        out.push_str(&format!("  À lire : rust-man {f}\n"));
    }
    out.push_str("\n  Bonne nuit. La constance paie.\n\n");
    print_out(&out);
    std::process::exit(0);
}

/// La prochaine action, une seule. Pas de choix à faire.
fn cmd_next() -> ! {
    let p = progress::load();
    // priorité : un exo non fait → un point fragile → une fiche non lue → flashcards
    let mut out = String::from("\n  👉 Une seule chose à faire :\n\n");

    if let Some(ex) = exo::EXOS.iter().find(|e| !p.exercise_done(e.id, 0)) {
        out.push_str(&format!("  rust-man exo {}\n", ex.id));
        out.push_str(&format!("  ({})\n\n", ex.title));
        out.push_str("  C'est la seule chose. Les autres attendront.\n\n");
        print_out(&out);
        std::process::exit(0);
    }
    // un point fragile
    let weak = p.weak_categories();
    if let Some((cat, _, _)) = weak.first() {
        out.push_str(&format!("  rust-man quiz {cat}\n"));
        out.push_str("  (ton point fragile — un quiz de 5 questions)\n\n");
        print_out(&out);
        std::process::exit(0);
    }
    // une fiche non lue
    let fiche = ENTRIES.iter().find(|e| !p.fiches.contains_key(&e.id));
    if let Some(f) = fiche {
        out.push_str(&format!("  rust-man {}\n", f.id));
        out.push_str(&format!("  ({})\n\n", f.title));
        print_out(&out);
        std::process::exit(0);
    }
    out.push_str("  Tout est couvert. Flashcards : rust-man, Ctrl+O, 8.\n\n");
    print_out(&out);
    std::process::exit(0);
}

/// Note personnelle sur une fiche.
fn cmd_note(fiche: &str, texte: Option<&str>) -> ! {
    let entry = match content::by_id(fiche).or_else(|| content::search(fiche).first().map(|(_, e)| *e)) {
        Some(e) => e,
        None => {
            eprintln!("rust-man note: fiche inconnue « {fiche} ».");
            std::process::exit(1);
        }
    };
    let mut p = progress::load();
    match texte {
        // écrire / effacer
        Some(t) => {
            p.set_note(&entry.id, t);
            progress::save(&p);
            if t.trim().is_empty() {
                print_out(&format!("\n  Note effacée pour « {} ».\n\n", entry.id));
            } else {
                print_out(&format!("\n  📝 Note enregistrée pour « {} ».\n   Elle s'affichera à la lecture de la fiche.\n\n", entry.id));
            }
        }
        // lire
        None => {
            match p.note(&entry.id) {
                Some(n) => print_out(&format!("\n  📝 Ta note sur « {} » :\n\n  {}\n\n", entry.id, n)),
                None => print_out(&format!(
                "\n  Pas de note sur « {} ».\n  rust-man note {} \"ta compréhension\"\n\n",
                entry.id, entry.id
            )),
            }
        }
    }
    std::process::exit(0);
}

/// Tu es bloqué : décris le problème, on te route vers la bonne aide.
fn cmd_bloque(probleme: &str) -> ! {
    let mut out = format!("\n  🆘 On te débloque : « {probleme} »\n\n");
    // 1. la fiche la plus pertinente — on cherche par mot-clé significatif
    let mots: Vec<&str> = probleme
        .split_whitespace()
        .filter(|w| w.len() >= 3)
        .collect();
    let mut best: Option<&rust_man::content::Entry> = None;
    let mut best_score = i64::MIN;
    for mot in &mots {
        for (score, e) in content::search(mot) {
            if score > best_score {
                best_score = score;
                best = Some(e);
            }
        }
    }
    if let Some(e) = best {
        out.push_str(&format!("  1. Lis la fiche : rust-man {}\n", e.id));
        // 2. un exo lié si ça touche un concept
        if let Some(ex) = exo::EXOS.iter().find(|x| x.fiche == e.id) {
            out.push_str(&format!("  2. Pratique : rust-man exo {}\n", ex.id));
        }
    }
    // 3. l'agent qui aide (selon le type de problème)
    let bas = probleme.to_lowercase();
    let agent = if bas.contains("crash") || bas.contains("panique") || bas.contains("panic") || bas.contains("bug") || bas.contains("marche pas") {
        "debug"
    } else if bas.contains("borrow") || bas.contains("emprunt") || bas.contains("move") || bas.contains("ownership") || bas.contains("lifetime") {
        "memory"
    } else if bas.contains("erreur") && (bas.contains("e0") || bas.contains("rustc") || bas.contains("compile")) {
        "compiler"
    } else if bas.contains("style") || bas.contains("clippy") || bas.contains("propre") || bas.contains("idiome") {
        "clean"
    } else if bas.contains("comment") || bas.contains("algo") || bas.contains("logique") {
        "algo"
    } else {
        "professeur"
    };
    out.push_str(&format!(
        "  3. Parle à l'agent : rust-man ask {agent} \"{}\"\n",
        probleme
    ));
    out.push_str("\n  Tu n'es pas seul. Ces trois-là te débloquent.\n\n");
    print_out(&out);
    std::process::exit(0);
}

/// Recherche unifiée dans tout rust-man : fiches, exercices, quiz, agents.
fn cmd_apropos(mot: &str) -> ! {
    let mot_l = mot.to_lowercase();
    let mut out = format!("\n  🔍 « {mot} » — partout dans rust-man\n\n");

    // fiches
    let fiches = content::search(mot);
    if !fiches.is_empty() {
        out.push_str("  Fiches :\n");
        for (_, e) in fiches.iter().take(5) {
            out.push_str(&format!("    rust-man {:<20} {}\n", e.id, e.title));
        }
        out.push('\n');
    }
    // exercices
    let exos: Vec<_> = exo::EXOS.iter()
        .filter(|e| e.id.to_lowercase().contains(&mot_l) || e.title.to_lowercase().contains(&mot_l) || e.task.to_lowercase().contains(&mot_l))
        .collect();
    if !exos.is_empty() {
        out.push_str("  Exercices :\n");
        for e in exos.iter().take(4) {
            out.push_str(&format!("    rust-man exo {:<16} {}\n", e.id, e.title));
        }
        out.push('\n');
    }
    // questions de quiz
    let nb_quiz = quiz::QUESTIONS.iter()
        .filter(|q| q.question.to_lowercase().contains(&mot_l))
        .count();
    if nb_quiz > 0 {
        out.push_str(&format!("  Quiz : {} question(s) → rust-man quiz\n\n", nb_quiz));
    }
    // agents
    let agents: Vec<_> = ai::AGENTS.iter()
        .filter(|a| a.name().to_lowercase().contains(&mot_l) || a.description().to_lowercase().contains(&mot_l))
        .collect();
    if !agents.is_empty() {
        out.push_str("  Agents :\n");
        for a in agents.iter() {
            out.push_str(&format!("    rust-man ask {:<14} (pose-lui une question)\n", a.name()));
        }
        out.push('\n');
    }
    if fiches.is_empty() && exos.is_empty() && nb_quiz == 0 && agents.is_empty() {
        out.push_str("  Rien trouvé. Essaie un autre mot, ou rust-man bloque « … ».\n");
    }
    print_out(&out);
    print_out("\n");
    std::process::exit(0);
}

/// Journal d'apprentissage quotidien (~/.local/share/rust-man/journal/).
fn cmd_journal(texte: Option<&str>, relire: Option<u32>) -> ! {
    let dir = dirs_home().join("journal");
    let _ = std::fs::create_dir_all(&dir);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let jour = now / 86_400;

    // relire un jour passé
    if let Some(offset) = relire {
        let cible = jour.saturating_sub(offset as u64);
        let f = dir.join(format!("{cible}.md"));
        if f.exists() {
            let content = std::fs::read_to_string(&f).unwrap_or_default();
            print_out(&format!("\n  📓 Journal du jour il y a {offset} jour(s) :\n\n{}\n\n", content));
        } else {
            print_out(&format!("\n  Pas de journal pour il y a {offset} jour(s).\n  Écris aujourd'hui : rust-man journal \"ce que j'ai compris…\"\n\n"));
        }
        std::process::exit(0);
    }

    let f = dir.join(format!("{jour}.md"));
    match texte {
        // écrire / ajouter
        Some(t) => {
            let mut content = std::fs::read_to_string(&f).unwrap_or_default();
            if !content.is_empty() {
                content.push_str("\n\n");
            }
            content.push_str(t);
            std::fs::write(&f, content).unwrap_or_default();
            print_out("\n  📓 Noté dans ton journal du jour.\n  `rust-man journal` pour le relire, `rust-man journal -r 1` pour hier.\n\n");
        }
        // lire aujourd'hui
        None => {
            if f.exists() {
                let content = std::fs::read_to_string(&f).unwrap_or_default();
                print_out(&format!("\n  📓 Ton journal d'aujourd'hui :\n\n{}\n\n", content));
            } else {
                print_out("\n  📓 Pas encore de journal aujourd'hui.\n  `rust-man journal \"ce que j'ai compris, ce qui m'a bloqué\"`\n  Écrire avec tes mots, c'est ancrer.\n\n");
            }
        }
    }
    std::process::exit(0);
}

/// Le dossier de données rust-man.
fn dirs_home() -> std::path::PathBuf {
    std::path::PathBuf::from(std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")).unwrap_or_else(|_| ".".into()))
        .join(".local/share/rust-man")
}

/// Examen blanc : questions mélangées, chrono global, note finale.
fn cmd_exam(count: usize, minutes: u64) -> ! {
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64)
        .unwrap_or(1);
    let questions = quiz::pick(None, count, seed);
    if questions.is_empty() {
        eprintln!("rust-man exam: aucune question.");
        std::process::exit(1);
    }
    let budget = std::time::Duration::from_secs(minutes * 60);
    let debut = std::time::Instant::now();
    let stdin = std::io::stdin();
    let mut correct = 0u32;
    let total = questions.len() as u32;

    print_out(&format!(
        "\n  📝 EXAMEN BLANC — {total} questions, {minutes} minutes.\n  Réponds avec le numéro. Bonne chance.\n"
    ));
    for (i, q) in questions.iter().enumerate() {
        let reste = budget.saturating_sub(debut.elapsed());
        if reste.is_zero() {
            print_out("\n  ⏱ Temps écoulé !\n");
            break;
        }
        let m = reste.as_secs() / 60;
        let s2 = reste.as_secs() % 60;
        print_out(&format!(
            "\n── Q{}/{}  ({:02}:{:02} restantes) ──\n{}\n",
            i + 1, total, m, s2, q.question
        ));
        for (j, choice) in q.choices.iter().enumerate() {
            print_out(&format!("  {}. {}\n", j + 1, choice));
        }
        print_out("> ");
        let mut answer = String::new();
        if stdin.read_line(&mut answer).is_err() {
            break;
        }
        if answer.trim().parse::<usize>().ok() == Some(q.answer + 1) {
            correct += 1;
            print_out("  ✓\n");
        } else {
            print_out(&format!("  ✗ (c'était {})\n", q.answer + 1));
        }
    }
    let pct = if total > 0 { correct * 100 / total } else { 0 };
    // partie code : un exercice à faire sous pression (le vrai exam en a)
    let p0 = progress::load();
    if let Some(ex) = exo::EXOS.iter().find(|e| !p0.exercise_done(e.id, 0)) {
        print_out(&format!(
            "\n  ── partie code ──\n  Le vrai exam a du code. Tu as un exercice :\n\n  rust-man exo {}\n  ({})\n\n  Fais-le maintenant, puis `rust-man exo {} --test`.\n",
            ex.id, ex.title, ex.id
        ));
    }
    let note = match pct {
        90..=100 => "Excellent — niveau partiel atteint.",
        70..=89 => "Solide. Encore un peu.",
        50..=69 => "Ça passe, mais bosse les points fragiles.",
        _ => "À retravailler — rust-man morning demain.",
    };
    print_out(&format!(
        "\n  ══ NOTE : {correct}/{total} ({pct} %) ══\n  {note}\n\n"
    ));
    let mut p = progress::load();
    p.record_quiz("exam", total, correct);
    progress::save(&p);
    std::process::exit(0);
}

/// Session du matin : accueil, points fragiles, fiche à relire, quiz ciblé.
fn cmd_morning() -> ! {
    let p = progress::load();
    let mut out = String::from("\n  Bonjour. Session du matin — court, ciblé, utile.\n\n");
    let weak = p.weak_categories();
    if !weak.is_empty() {
        out.push_str("  Tes points fragiles (d'après tes quiz) :\n");
        for (cat, ok, total) in &weak {
            out.push_str(&format!("    · {cat} : {ok}/{total}\n"));
        }
        out.push('\n');
    }
    // fiche à relire : la moins vue de la catégorie la plus fragile
    let target_cat = weak.first().map(|(c, _, _)| c.as_str());
    let fiche = ENTRIES
        .iter()
        .filter(|e| target_cat.is_none_or(|c| e.category == c))
        .min_by_key(|e| p.fiches.get(&e.id).map(|s| s.views).unwrap_or(0))
        .map(|e| e.id.as_str());
    if let Some(f) = fiche {
        out.push_str(&format!("  À relire aujourd'hui : rust-man {f}\n"));
    }
    if let Some((cat, _, _)) = weak.first() {
        out.push_str(&format!("  Quiz ciblé : rust-man quiz {cat}\n"));
    }
    // flashcards à revoir (répétition espacée)
    let a_revoir = quiz::flashcards()
        .iter()
        .filter(|(fiche, _)| p.flash_priority(fiche).0 == 0)
        .count();
    if a_revoir > 0 {
        out.push_str(&format!(
            "  Flashcards à revoir : {} (les pièges qui t'ont fait bloquer)\n",
            a_revoir
        ));
    }
    out.push_str("  Révision éclair : rust-man, Ctrl+O, 8 (flashcards)\n");
    // suggère un exercice guidé non fait
    if let Some(ex) = exo::EXOS.iter().find(|e| !p.exercise_done(e.id, 0)) {
        out.push_str(&format!("  Exercice du jour : rust-man exo {}\n", ex.id));
    }
    // une notion lue mais pas encore défendue → tu ne la maîtrises vraiment que si tu sais l'expliquer
    if let Some(e) = ENTRIES.iter().find(|e| {
        p.fiches.contains_key(&e.id) && !p.defended.contains_key(&e.id)
    }) {
        out.push_str(&format!(
            "  À défendre (tu l'as lue — sais-tu l'expliquer ?) : rust-man defend {}\n",
            e.id
        ));
    }
    let streak = p.streak();
    if streak > 0 {
        let flame = if streak >= 7 { "🔥" } else { "📅" };
        out.push_str(&format!(
            "  {} Série : {} jour(s) d'affilée{}.\n",
            flame,
            streak,
            if streak >= 7 { " — impressionnant" } else { "" }
        ));
    }
    if !p.focus_sessions.is_empty() {
        let total: u64 = p.focus_sessions.iter().sum();
        out.push_str(&format!(
            "  Focus : {} blocs jusqu'ici ({} min de travail profond).\n",
            p.focus_sessions.len(),
            total / 60
        ));
    }
    if p.quiz.is_empty() && p.fiches.is_empty() {
        out.push_str("\n  (Pas encore de données — commence par rust-man roadmap, et reviens demain.)\n");
    }
    out.push_str("\n  Structure ta journée : rust-man plan \"ce que tu dois faire\"\n");
    out.push('\n');
    print_out(&out);
    std::process::exit(0);
}

/// Relis une synthèse « apprends-moi » sauvegardée.
fn cmd_revision(n: usize) -> ! {
    let revs = progress::list_revisions();
    if revs.is_empty() {
        print_out("\nAucune synthèse sauvegardée. Utilise le mode « apprends-moi » (touche t) dans l'orchestrateur.\n\n");
        std::process::exit(0);
    }
    let idx = n.saturating_sub(1);
    if idx >= revs.len() {
        eprintln!("Il n'y a que {} synthèse(s).", revs.len());
        std::process::exit(1);
    }
    match std::fs::read_to_string(&revs[idx]) {
        Ok(text) => {
            let r = renderer();
            print_out(&format!("\n── synthèse {} ({}) ──\n\n", n, revs[idx].display()));
            let _ = r; // la synthèse est du markdown ; on la rend telle quelle
            print_out(&text);
            print_out("\n");
        }
        Err(e) => {
            eprintln!("rust-man revision: {e}");
            std::process::exit(1);
        }
    }
    std::process::exit(0);
}

/// Scaffolding : crée <nom>.rs avec un squelette qui compile (todo!()).
fn cmd_new(name: &str, func: Option<&str>) -> ! {
    let path = format!("{name}.rs");
    if std::path::Path::new(&path).exists() {
        eprintln!("rust-man new: {path} existe déjà.");
        std::process::exit(1);
    }
    let mut content = String::new();
    if let Some(f) = func {
        let f = f.trim().trim_end_matches(';').trim_end_matches('{').trim();
        // corps minimal qui compile : todo!() respecte n'importe quel type de retour
        content.push_str(&format!("{f} {{\n    todo!(\"à toi\")\n}}\n"));
    } else {
        content.push_str("fn main() {\n    println!(\"à toi de jouer\");\n}\n");
    }
    if let Err(e) = std::fs::write(&path, &content) {
        eprintln!("rust-man new: impossible d'écrire {path}: {e}");
        std::process::exit(1);
    }
    print_out(&format!(
        "✓ {path} créé (compile d'office : todo!() partout).\n  → `rust-man edit {path}` pour coder, `rust-man exo` pour un exercice guidé.\n"
    ));
    std::process::exit(0);
}

fn cmd_plan(mission: Vec<String>) -> ! {
    let cfg = config::load();
    let client = match ai::AiClient::from_env(&cfg) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("rust-man plan: {e}");
            std::process::exit(3);
        }
    };
    let mission = mission.join(" ");
    let p = progress::load();
    let ctx = orchestra::ProjectContext::scan(Path::new("."));
    let level = cfg.ai_help_level.map(ai::HelpLevel::from_u8).unwrap_or_default();
    eprintln!("⏳ l'orchestrateur analyse ta journée…");
    match orchestra::request_plan(&client, &mission, &p, &ctx, level) {
        Ok(plan) => {
            let mut out = format!("\n🎯 Plan — Complémentarité de l'Homme\n\n{}\n\n", plan.analyse);
            for (i, step) in plan.steps.iter().enumerate() {
                let badge = match &step.action {
                    orchestra::StepAction::Fiche(id) => format!(" → rust-man {id}"),
                    orchestra::StepAction::Style(p) => format!(" → rust-man check {p}"),
                    orchestra::StepAction::Compile(p) => format!(" → compile {p}"),
                    orchestra::StepAction::Quiz(c) => format!(" → rust-man quiz {c}"),
                    orchestra::StepAction::Agent(n, _) => format!(" → agent {n}"),
                    orchestra::StepAction::Manuel => " → à toi de coder".to_string(),
                };
                out.push_str(&format!("  {}. {}{}\n", i + 1, step.titre, badge));
                if !step.pourquoi.is_empty() {
                    out.push_str(&format!("     {}\n", step.pourquoi));
                }
            }
            out.push_str("\nDans le TUI (rust-man, Ctrl+O puis 7), chaque étape est exécutable d'un Enter.\n");
            print_out(&out);
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("rust-man plan: {e}");
            std::process::exit(3);
        }
    }
}

fn cmd_dsh(args: Vec<String>) -> ! {
    let cfg = config::load();
    match dsh::detect(&cfg) {
        Some(handle) => {
            let cwd = std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf());
            let mut cmd = if args.is_empty() {
                handle.spawn_command(&["--help".to_string()])
            } else {
                // rust-man dsh "ma demande" → dsh headless avec contexte projet
                handle.spawn_headless(&args.join(" "), &cwd)
            };
            eprintln!("→ dsh via {} ({})", handle.command, handle.source);
            let status = cmd
                .stdin(std::process::Stdio::inherit())
                .stdout(std::process::Stdio::inherit())
                .stderr(std::process::Stdio::inherit())
                .status();
            match status {
                Ok(s) => std::process::exit(s.code().unwrap_or(0)),
                Err(e) => {
                    eprintln!("rust-man dsh: lancement impossible : {e}");
                    eprintln!("Configure dsh_command / dsh_dir dans ~/.config/rust-man/config.toml");
                    std::process::exit(3);
                }
            }
        }
        None => {
            eprintln!("rust-man dsh: dsh introuvable dans le PATH et aucun checkout détecté.");
            eprintln!("Configure-le dans ~/.config/rust-man/config.toml :");
            eprintln!("  dsh_command = \"pnpm dsh\"");
            eprintln!("  dsh_dir = \"~/deepseek-harness\"");
            std::process::exit(3);
        }
    }
}

// ------------------------------------------------------------------ main

fn main() {
    let _ = config::ensure_exists();
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) if e.kind() == ErrorKind::DisplayHelp || e.kind() == ErrorKind::DisplayVersion => {
            e.print().expect("affichage aide");
            return;
        }
        Err(e) => {
            e.print().expect("affichage erreur");
            std::process::exit(2);
        }
    };

    if ENTRIES.is_empty() {
        eprintln!("rust-man: aucune fiche embarquée. Réinstalle le paquet.");
        std::process::exit(1);
    }

    match cli.command {
        Some(Cmd::Check { path, json }) => cmd_check(&path, json),
        Some(Cmd::Fix { path, dry_run }) => cmd_fix(&path, dry_run),
        Some(Cmd::Explain { rule }) => cmd_explain(&rule),
        Some(Cmd::Quiz { category, count, chrono }) => cmd_quiz(category.as_deref(), count, chrono),
        Some(Cmd::Progress) => cmd_progress(),
        Some(Cmd::Ask { agent, question, level, model }) => cmd_ask(&agent, question, level, model),
        Some(Cmd::Dsh { args }) => cmd_dsh(args),
        Some(Cmd::New { name, func }) => cmd_new(&name, func.as_deref()),
        Some(Cmd::Apprendre { sujet }) => cmd_apprendre(&sujet),
        Some(Cmd::Defend { sujet, model }) => cmd_defend(&sujet, model.as_deref()),
        Some(Cmd::Test { path }) => cmd_test(&path),
        Some(Cmd::Doctor) => cmd_doctor(),
        Some(Cmd::Serve { port }) => cmd_serve(port),
        Some(Cmd::Review { path, model }) => cmd_review(&path, model.as_deref()),
        Some(Cmd::Focus { minutes }) => cmd_focus(minutes),
        Some(Cmd::Analyse { fichier, model }) => cmd_analyse(&fichier, model.as_deref()),
        Some(Cmd::Watch { path }) => cmd_watch(&path),
        Some(Cmd::Push { message }) => cmd_push(&message),
        Some(Cmd::Exo { id, test, indice }) => cmd_exo(id.as_deref(), test, indice),
        Some(Cmd::NewProjet { nom }) => cmd_new_projet(&nom),
        Some(Cmd::Rush { sujet, nom }) => cmd_rush(&sujet, nom.as_deref()),
        Some(Cmd::Recap) => cmd_recap(),
        Some(Cmd::Note { fiche, texte }) => cmd_note(&fiche, texte.as_deref()),
        Some(Cmd::Bloque { probleme }) => cmd_bloque(&probleme),
        Some(Cmd::Apropos { mot }) => cmd_apropos(&mot),
        Some(Cmd::Journal { texte, relire }) => cmd_journal(texte.as_deref(), relire),
        Some(Cmd::Exam { count, minutes }) => cmd_exam(count, minutes),
        Some(Cmd::Next) => cmd_next(),
        Some(Cmd::Morning) => cmd_morning(),
        Some(Cmd::Revision { n }) => cmd_revision(n),
        Some(Cmd::Edit { path }) => {
            let p = path.map(std::path::PathBuf::from);
            if let Err(e) = rust_man::editor::run(p) {
                eprintln!("rust-man edit: {e}");
                std::process::exit(1);
            }
            std::process::exit(0);
        }
        Some(Cmd::Plan { mission }) => cmd_plan(mission),
        None => {}
    }

    if cli.random {
        random_entry();
    } else if let Some(filter) = cli.list {
        list_entries(filter.as_deref());
    } else if let Some(query) = cli.search {
        search_entries(&query);
    } else if cli.tui {
        run_tui(cli.topic.as_deref());
    } else if let Some(topic) = cli.topic {
        show_topic(&topic);
    } else if std::io::stdout().is_terminal() && std::io::stdin().is_terminal() {
        run_tui(None);
    } else {
        Cli::command().print_help().expect("aide");
        println!();
    }
}

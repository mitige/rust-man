//! Écrans IA : chat avec les agents spécialisés + orchestrateur
//! « Complémentarité de l'Homme ». Tous passent par le même client Kimi K3.

use super::{draw_scroll_panel, AsyncMsg, ScreenKind, Shell};
use rust_man::ai::{system_prompt, AgentRole, AGENTS, HelpLevel};
use rust_man::orchestra::{self, Plan, ProjectContext, StepAction};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

// ================================================================== CHAT

pub(crate) struct ChatState {
    pub(crate) agent_idx: usize,
    pub(crate) input: String,
    pub(crate) history: Vec<(String, String)>,
    pub(crate) busy: bool,
    pub(crate) scroll: usize,
    pub(crate) error: Option<String>,
    pub(crate) picking_agent: bool,
    /// question en vol (pour l'afficher avec la réponse)
    pub(crate) pending_question: String,
    /// fichiers du projet injectés dans le contexte IA
    pub(crate) project_files: usize,
    pub(crate) busy_since: Option<std::time::Instant>,
}

impl ChatState {
    pub(crate) fn new() -> Self {
        Self {
            agent_idx: 1, // Professeur par défaut
            input: String::new(),
            history: Vec::new(),
            busy: false,
            scroll: 0,
            error: None,
            picking_agent: false,
            pending_question: String::new(),
            project_files: 0,
            busy_since: None,
        }
    }

    pub(crate) fn agent(&self) -> AgentRole {
        AGENTS[self.agent_idx]
    }
}

fn chat_send(shell: &mut Shell) {
    let Some(client) = shell.ai.clone() else {
        shell.chat.error = Some(
            "IA indisponible : définis OPP_API_KEY (ou ~/.dsh/.credentials.yaml)".into(),
        );
        return;
    };
    let question = shell.chat.input.trim().to_string();
    if question.is_empty() {
        return;
    }
    shell.chat.pending_question = question.clone();
    shell.chat.input.clear();
    shell.chat.busy = true;
    shell.chat.busy_since = Some(std::time::Instant::now());
    shell.chat.error = None;
    let role = shell.chat.agent();
    let level = shell.help_level;
    let mut system = system_prompt(role, level);
    // contexte : progression + dernières fiches vues
    let summary = shell.progress_data.summary_for_ai();
    if !summary.is_empty() {
        system.push_str(&format!("\n\nProgression de l'étudiant :\n{summary}"));
    }
    let history = shell.chat.history.clone();
    // contexte projet : l'IA voit les fichiers du dossier courant
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let project_ctx = rust_man::project::snapshot(&cwd).to_prompt();
    let tx = shell.tx.clone();
    std::thread::spawn(move || {
        // inclut les 3 derniers échanges pour le fil de la discussion
        let mut user = project_ctx.clone();
        user.push_str("\n\n");
        for (q, a) in history.iter().rev().take(3).rev() {
            user.push_str(&format!("Étudiant : {q}\nToi : {a}\n\n"));
        }
        user.push_str(&format!("Étudiant : {question}"));
        let result = client.chat(&system, &user).map_err(|e| e.to_string());
        let _ = tx.send(AsyncMsg::AiChat(result));
    });
}

pub(crate) fn on_chat_result(shell: &mut Shell, result: Result<String, String>) {
    shell.chat.busy = false;
    shell.chat.busy_since = None;
    match result {
        Ok(answer) => {
            let q = std::mem::take(&mut shell.chat.pending_question);
            shell.chat.history.push((q, answer));
            shell.chat.scroll = usize::MAX; // scrolle en bas
        }
        Err(e) => shell.chat.error = Some(e),
    }
}

pub(crate) fn chat_on_key(shell: &mut Shell, key: KeyEvent) {
    let st = &mut shell.chat;
    if st.picking_agent {
        match key.code {
            KeyCode::Esc => st.picking_agent = false,
            KeyCode::Char('j') | KeyCode::Down => {
                st.agent_idx = (st.agent_idx + 1).min(AGENTS.len() - 1)
            }
            KeyCode::Char('k') | KeyCode::Up => {
                st.agent_idx = st.agent_idx.saturating_sub(1)
            }
            KeyCode::Enter => st.picking_agent = false,
            _ => {}
        }
        return;
    }
    // Ctrl+L : change le modèle IA (persisté dans la config)
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('l') {
        if let Some(client) = &mut shell.ai {
            let next = client.next_model_name();
            client.set_model(next);
            rust_man::config::save_ai_model(next);
        }
        return;
    }
    // Ctrl+↑ / Ctrl+↓ : niveau d'aide (= et + restent tapables dans le texte)
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        if key.code == KeyCode::Up {
            shell.help_level = shell.help_level.next();
            return;
        }
        if key.code == KeyCode::Down {
            shell.help_level = HelpLevel::from_u8((shell.help_level as u8).saturating_sub(1).max(1));
            return;
        }
    }
    match key.code {
        KeyCode::Esc => shell.set_screen(ScreenKind::Docs),
        KeyCode::Tab => st.picking_agent = true,
        KeyCode::Enter => chat_send(shell),
        KeyCode::Backspace => {
            st.input.pop();
        }
        KeyCode::Up => st.scroll = st.scroll.saturating_sub(1),
        KeyCode::Down => st.scroll = st.scroll.saturating_add(1),
        KeyCode::PageUp => st.scroll = st.scroll.saturating_sub(10),
        KeyCode::PageDown => st.scroll = st.scroll.saturating_add(10),
        KeyCode::Char(c) => {
            if !key.modifiers.contains(KeyModifiers::CONTROL) {
                st.input.push(c);
            }
        }
        _ => {}
    }
}

pub(crate) fn draw_chat(frame: &mut Frame, shell: &mut Shell, area: Rect) {
    let palette = *shell.palette();
    let help_level = shell.help_level;
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(3)])
        .split(area);
    let st = &mut shell.chat;
    let agent = st.agent();
    let model = shell
        .ai
        .as_ref()
        .map(|c| {
            format!(
                "{} ({}M·{}K)",
                c.model,
                rust_man::ai::CONTEXT_WINDOW_TOKENS / 1_048_576,
                rust_man::ai::MAX_OUTPUT_TOKENS / 1024
            )
        })
        .unwrap_or_else(|| "hors-ligne".into());
    let title = format!(
        "{} · {} · {} · {} fichiers · Tab agent · Ctrl+L modèle",
        agent.name(),
        help_level.name(),
        model,
        st.project_files
    );

    let mut lines: Vec<Line> = Vec::new();
    let accent = Style::default().fg(palette.accent());
    let dim = Style::default().fg(palette.dim());
    if shell.ai.is_none() {
        lines.push(Line::from(Span::styled(
            "IA non configurée.", Style::default().fg(palette.gold()),
        )));
        lines.push(Line::default());
        lines.push(Line::from(
            "rust-man utilise Kimi K3 via le provider du projet. Pour l'activer :",
        ));
        lines.push(Line::from("  1. export OPP_API_KEY=ta_clé"));
        lines.push(Line::from("  2. ou place-la dans ~/.dsh/.credentials.yaml"));
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(
            "Tout le reste (doc, style, quiz, compile) marche sans IA.",
            dim,
        )));
    } else {
        lines.push(Line::from(Span::styled(agent.description(), dim)));
        lines.push(Line::default());
        if st.history.is_empty() && !st.busy {
            lines.push(Line::from(Span::styled(
                "Pose ta question. Exemples :",
                dim,
            )));
            lines.push(Line::from("  • « explique-moi les pointeurs simplement »"));
            lines.push(Line::from("  • « pourquoi ce code segfaulte ? » (colle ton code)"));
            lines.push(Line::from("  • « teste ma compréhension de malloc »"));
        }
        for (q, a) in &st.history {
            lines.push(Line::default());
            lines.push(Line::from(Span::styled(
                format!("━━ toi ━━"),
                accent,
            )));
            lines.push(Line::from(q.clone()));
            lines.push(Line::from(Span::styled(
                format!("━━ {} ━━", agent.name()),
                Style::default().fg(palette.green()),
            )));
            for l in super::md_to_lines(a, 110, &palette) {
                lines.push(l);
            }
        }
        if st.busy {
            let secs = st.busy_since.map(|t| t.elapsed().as_secs()).unwrap_or(0);
            lines.push(Line::default());
            lines.push(Line::from(Span::styled(
                format!("⏳ {} réfléchit… ({} s — jusqu'à ~3 min pour les longues réponses)", agent.name(), secs),
                accent,
            )));
        }
        if let Some(e) = &st.error {
            lines.push(Line::from(Span::styled(
                format!("erreur : {e}"),
                Style::default().fg(palette.gold()),
            )));
        }
    }
    // borne le scroll à la hauteur visible ET le réécrit dans l'état :
    // sinon usize::MAX (nouvelle réponse) rend ↑ inopérant
    let inner_h = chunks[0].height.saturating_sub(3) as usize;
    let max_scroll = lines.len().saturating_sub(inner_h);
    if st.scroll > max_scroll {
        st.scroll = max_scroll;
    }
    draw_scroll_panel(frame, chunks[0], &title, &lines, st.scroll, palette.accent(),
        "Tab agent · Ctrl+↑↓ niveau · Ctrl+L modèle · ↑↓ défiler · Échap docs");
    super::draw_input(frame, chunks[1], "ta question", &st.input, true, &palette);

    // sélecteur d'agent — groupé (élèves & seniors / spécialistes)
    if st.picking_agent {
        // largeur bornée par l'écran, et description tronquée pour tenir dedans
        let w = frame.area().width.min(78).max(40);
        let popup = super::centered_popup(frame, frame.area(), w, AGENTS.len() as u16 + 8);
        let desc_w = (w as usize).saturating_sub(5 + 26 + 3);
        let mut plines: Vec<Line> = Vec::new();
        let mut last_group = "";
        for (i, a) in AGENTS.iter().enumerate() {
            if a.group() != last_group {
                last_group = a.group();
                plines.push(Line::from(Span::styled(
                    format!("  {last_group}"),
                    Style::default().fg(palette.dim()).add_modifier(Modifier::BOLD),
                )));
            }
            let style = if i == st.agent_idx {
                accent.add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            plines.push(Line::from(vec![
                Span::styled(if i == st.agent_idx { "   ▸ " } else { "     " }, style),
                Span::styled(format!("{:<26}", a.name()), style),
                Span::styled(super::truncate_width(a.description(), desc_w), dim),
            ]));
        }
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(palette.dim()))
            .title(Span::styled("  agent  ", Style::default().fg(palette.dim())));
        frame.render_widget(Paragraph::new(plines).block(block), popup);
    }
}

// ================================================================== ORCHESTRATEUR

#[derive(PartialEq)]
pub(crate) enum OrchestraMode {
    Mission,
    Plan,
    Teach,
}

pub(crate) struct OrchestraState {
    pub(crate) input: String,
    pub(crate) plan: Option<Plan>,
    pub(crate) selected: usize,
    pub(crate) busy: bool,
    pub(crate) error: Option<String>,
    pub(crate) teach: Option<String>,
    pub(crate) teach_scroll: usize,
    pub(crate) mode: OrchestraMode,
    pub(crate) busy_since: Option<std::time::Instant>,
}

impl OrchestraState {
    pub(crate) fn new() -> Self {
        Self {
            input: String::new(),
            plan: None,
            selected: 0,
            busy: false,
            error: None,
            teach: None,
            teach_scroll: 0,
            mode: OrchestraMode::Mission,
            busy_since: None,
        }
    }
}

fn orchestra_plan(shell: &mut Shell) {
    let Some(client) = shell.ai.clone() else {
        shell.orchestra.error =
            Some("IA indisponible : configure OPP_API_KEY (voir écran IA).".into());
        return;
    };
    let mission = shell.orchestra.input.trim().to_string();
    if mission.is_empty() {
        return;
    }
    shell.orchestra.busy = true;
    shell.orchestra.busy_since = Some(std::time::Instant::now());
    shell.orchestra.error = None;
    let progress = shell.progress_data.clone();
    let ctx = ProjectContext::scan(std::path::Path::new("."));
    let level = shell.help_level;
    let tx = shell.tx.clone();
    std::thread::spawn(move || {
        let result = orchestra::request_plan(&client, &mission, &progress, &ctx, level)
            .map_err(|e| e.to_string());
        let _ = tx.send(AsyncMsg::OrchestraPlan(result));
    });
}

pub(crate) fn on_plan_result(shell: &mut Shell, result: Result<Plan, String>) {
    shell.orchestra.busy = false;
    shell.orchestra.busy_since = None;
    match result {
        Ok(plan) => {
            shell.orchestra.plan = Some(plan);
            shell.orchestra.mode = OrchestraMode::Plan;
            shell.orchestra.selected = 0;
        }
        Err(e) => shell.orchestra.error = Some(e),
    }
}

fn orchestra_teach(shell: &mut Shell) {
    let Some(client) = shell.ai.clone() else {
        shell.orchestra.error = Some("IA indisponible.".into());
        return;
    };
    let Some(plan) = &shell.orchestra.plan else { return };
    let travail = plan
        .steps
        .iter()
        .enumerate()
        .map(|(i, s)| {
            format!(
                "{}. [{}] {} — {}",
                i + 1,
                if s.done { "fait" } else { "à faire" },
                s.titre,
                s.pourquoi
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    shell.orchestra.busy = true;
    shell.orchestra.busy_since = Some(std::time::Instant::now());
    let level = shell.help_level;
    let tx = shell.tx.clone();
    std::thread::spawn(move || {
        let result = orchestra::teach_back(&client, &travail, level).map_err(|e| e.to_string());
        let _ = tx.send(AsyncMsg::OrchestraTeach(result));
    });
}

pub(crate) fn on_teach_result(shell: &mut Shell, result: Result<String, String>) {
    shell.orchestra.busy = false;
    shell.orchestra.busy_since = None;
    match result {
        Ok(text) => {
            // persiste la synthèse comme fiche de révision personnelle (13.5)
            let saved = rust_man::progress::save_revision(&text);
            shell.orchestra.teach = Some(text);
            shell.orchestra.mode = OrchestraMode::Teach;
            shell.orchestra.teach_scroll = 0;
            shell.orchestra.error = saved;
        }
        Err(e) => shell.orchestra.error = Some(e),
    }
}

fn execute_step(shell: &mut Shell, idx: usize) {
    let Some(plan) = &mut shell.orchestra.plan else { return };
    let Some(step) = plan.steps.get_mut(idx) else { return };
    let action = step.action.clone();
    step.done = true;
    match action {
        StepAction::Fiche(id) => shell.open_fiche(&id),
        StepAction::Style(path) => {
            shell.style.path = if path.is_empty() { ".".into() } else { path };
            shell.set_screen(ScreenKind::Style);
            shell.style.busy = true;
            super::tools::style_run(shell);
        }
        StepAction::Compile(path) => {
            shell.compile.target = if path.is_empty() { ".".into() } else { path };
            shell.set_screen(ScreenKind::Compile);
            shell.compile.busy = true;
            // lance la compile via le canal async de tools
            let target = std::path::PathBuf::from(shell.compile.target.trim());
            let tx = shell.tx.clone();
            std::thread::spawn(move || {
                let result = rust_man::compile::compile_target(&target);
                let _ = tx.send(AsyncMsg::Compile(Box::new(result)));
            });
        }
        StepAction::Quiz(cat) => {
            shell.set_screen(ScreenKind::Quiz);
            shell.quiz.phase = super::quizview::QuizPhase::PickCategory;
            if let Some(pos) = ["tout", "bases", "ownership", "erreurs", "types", "collections", "outils", "pratique"]
                .iter()
                .position(|c| *c == cat)
            {
                shell.quiz.cat_index = pos;
            }
        }
        StepAction::Agent(name, _) => {
            if let Some(pos) = AGENTS
                .iter()
                .position(|a| a.name().to_lowercase().contains(&name.to_lowercase()))
            {
                shell.chat.agent_idx = pos;
            }
            shell.set_screen(ScreenKind::Chat);
        }
        StepAction::Manuel => {}
    }
}

pub(crate) fn orchestra_on_key(shell: &mut Shell, key: KeyEvent) {
    let st = &mut shell.orchestra;
    match st.mode {
        OrchestraMode::Mission => match key.code {
            KeyCode::Esc => shell.set_screen(ScreenKind::Docs),
            KeyCode::Enter => orchestra_plan(shell),
            KeyCode::Backspace => {
                st.input.pop();
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                st.input.push(c)
            }
            _ => {}
        },
        OrchestraMode::Plan => match key.code {
            KeyCode::Esc => shell.set_screen(ScreenKind::Docs),
            KeyCode::Char('j') | KeyCode::Down => {
                if let Some(p) = &st.plan {
                    st.selected = (st.selected + 1).min(p.steps.len().saturating_sub(1));
                }
            }
            KeyCode::Char('k') | KeyCode::Up => st.selected = st.selected.saturating_sub(1),
            KeyCode::Enter => {
                let idx = st.selected;
                execute_step(shell, idx);
            }
            KeyCode::Char('x') => {
                if let Some(p) = &mut st.plan {
                    if let Some(step) = p.steps.get_mut(st.selected) {
                        step.done = !step.done;
                    }
                    // plan terminé → la synthèse s'offre d'elle-même
                    if p.steps.iter().all(|s| s.done) {
                        st.error = Some("plan terminé — 't' pour « apprends-moi » (ta synthèse de révision)".into());
                    }
                }
            }
            KeyCode::Char('t') => orchestra_teach(shell),
            KeyCode::Char('r') => {
                st.plan = None;
                st.input.clear();
                st.mode = OrchestraMode::Mission;
            }
            _ => {}
        },
        OrchestraMode::Teach => match key.code {
            KeyCode::Esc => st.mode = OrchestraMode::Plan,
            KeyCode::Char('j') | KeyCode::Down => st.teach_scroll = st.teach_scroll.saturating_add(1),
            KeyCode::Char('k') | KeyCode::Up => st.teach_scroll = st.teach_scroll.saturating_sub(1),
            _ => {}
        },
    }
}

pub(crate) fn draw_orchestra(frame: &mut Frame, shell: &mut Shell, area: Rect) {
    let palette = *shell.palette();
    let st = &mut shell.orchestra;
    let accent = Style::default().fg(palette.accent());
    let dim = Style::default().fg(palette.dim());
    let green = Style::default().fg(palette.green());

    match st.mode {
        OrchestraMode::Mission => {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(5), Constraint::Length(3)])
                .split(area);
            let mut lines = vec![
                Line::from(Span::styled(
                    "Complémentarité de l'Homme — ton orchestrateur de journée",
                    accent.add_modifier(Modifier::BOLD),
                )),
                Line::default(),
                Line::from("Dis simplement ce que tu dois faire aujourd'hui. L'orchestrateur"),
                Line::from("analyse ton projet et ta progression, puis te construit un plan"),
                Line::from("dont chaque étape est exécutable ici même (fiche, style, compile,"),
                Line::from("quiz, agent spécialisé)."),
                Line::default(),
                Line::from(Span::styled("Exemples :", dim)),
                Line::from(Span::styled(
                    "  « terminer mon ex de string.h et comprendre les pointeurs »",
                    dim,
                )),
                Line::from(Span::styled(
                    "  « mon projet segfaulte, je dois le rendre ce soir »",
                    dim,
                )),
            ];
            if shell.ai.is_none() {
                lines.push(Line::default());
                lines.push(Line::from(Span::styled(
                    "IA non configurée — voir écran Assistants IA (Ctrl+O, 6).",
                    Style::default().fg(palette.gold()),
                )));
            }
            if st.busy {
                let secs = st.busy_since.map(|t| t.elapsed().as_secs()).unwrap_or(0);
                lines.push(Line::default());
                lines.push(Line::from(Span::styled(
                    format!("⏳ l'orchestrateur réfléchit à ton plan… ({} s)", secs),
                    accent,
                )));
            }
            if let Some(e) = &st.error {
                lines.push(Line::from(Span::styled(format!("erreur : {e}"), Style::default().fg(palette.gold()))));
            }
            draw_scroll_panel(frame, chunks[0], "mission", &lines, 0, palette.accent(), "");
            super::draw_input(frame, chunks[1], "tes tâches du jour", &st.input, true, &palette);
        }
        OrchestraMode::Plan => {
            let mut lines: Vec<Line> = Vec::new();
            if let Some(plan) = &st.plan {
                lines.push(Line::from(Span::styled(plan.analyse.clone(), dim)));
                lines.push(Line::default());
                for (i, step) in plan.steps.iter().enumerate() {
                    let selected = i == st.selected;
                    let check = if step.done { "☑" } else { "☐" };
                    let badge = match &step.action {
                        StepAction::Fiche(_) => "[fiche]",
                        StepAction::Style(_) => "[style]",
                        StepAction::Compile(_) => "[compile]",
                        StepAction::Quiz(_) => "[quiz]",
                        StepAction::Agent(_, _) => "[agent]",
                        StepAction::Manuel => "[à toi]",
                    };
                    let style = if selected {
                        accent.add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    };
                    let done_style = if step.done { green } else { style };
                    lines.push(Line::from(vec![
                        Span::styled(if selected { "▸ " } else { "  " }, style),
                        Span::styled(format!("{check} {}. ", i + 1), done_style),
                        Span::styled(step.titre.clone(), done_style),
                        Span::styled(format!("  {badge}"), dim),
                    ]));
                    if selected && !step.pourquoi.is_empty() {
                        lines.push(Line::from(Span::styled(
                            format!("      pourquoi : {}", step.pourquoi),
                            dim,
                        )));
                    }
                    if selected {
                        let hint = match &step.action {
                            StepAction::Fiche(id) => format!("      Entrée → ouvrir la fiche « {id} »"),
                            StepAction::Style(p) => format!("      Entrée → checker le style de {p}"),
                            StepAction::Compile(p) => format!("      Entrée → compiler {p}"),
                            StepAction::Quiz(c) => format!("      Entrée → quiz {c}"),
                            StepAction::Agent(n, _) => format!("      Entrée → parler à {n}"),
                            StepAction::Manuel => "      À toi de jouer — x pour marquer fait".to_string(),
                        };
                        lines.push(Line::from(Span::styled(hint, accent)));
                    }
                }
                lines.push(Line::default());
                if plan.steps.iter().all(|s| s.done) && !plan.steps.is_empty() {
                    lines.push(Line::from(Span::styled(
                        "✓ plan terminé — 't' pour ta synthèse « apprends-moi »",
                        green.add_modifier(Modifier::BOLD),
                    )));
                    lines.push(Line::default());
                }
                if st.busy {
                    let secs = st.busy_since.map(|t| t.elapsed().as_secs()).unwrap_or(0);
                    lines.push(Line::from(Span::styled(
                        format!("⏳ synthèse en cours… ({} s)", secs),
                        accent,
                    )));
                }
                if let Some(e) = &st.error {
                    lines.push(Line::from(Span::styled(format!("erreur : {e}"), Style::default().fg(palette.gold()))));
                }
            }
            draw_scroll_panel(
                frame, area, "plan du jour", &lines, 0, palette.accent(),
                "j/k naviguer · Entrée exécuter l'étape · x marquer fait · t « apprends-moi » · r nouvelle mission · Échap docs",
            );
        }
        OrchestraMode::Teach => {
            let text = st.teach.clone().unwrap_or_default();
            let lines = super::md_to_lines(&text, 110, &palette);
            // borne le scroll à la hauteur visible
            let inner_h = area.height.saturating_sub(3) as usize;
            let max_scroll = lines.len().saturating_sub(inner_h);
            if st.teach_scroll > max_scroll {
                st.teach_scroll = max_scroll;
            }
            draw_scroll_panel(
                frame,
                area,
                "apprends-moi ce qu'on vient de faire",
                &lines,
                st.teach_scroll,
                palette.green(),
                "j/k défiler · Échap retour au plan",
            );
        }
    }
}

//! Écrans outils : Style Rust, Compilation expliquée, Progression.

use super::{
    centered_popup, draw_input, draw_scroll_panel, AsyncMsg, ScreenKind, Shell,
};
use rust_man::compile::{self, CompileResult, DiagLevel};
use rust_man::content::ENTRIES;
use rust_man::style::{self, Report as StyleReport, Severity};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    Frame,
};
use std::path::PathBuf;

// ================================================================== NORME

pub(crate) struct StyleState {
    pub(crate) path: String,
    pub(crate) auto_ran: bool,
    pub(crate) editing_path: bool,
    pub(crate) report: Option<StyleReport>,
    pub(crate) flat: Vec<FlatFinding>,
    pub(crate) selected: usize,
    pub(crate) scroll: usize,
    pub(crate) busy: bool,
    pub(crate) detail_popup: bool,
    pub(crate) status: String,
}

/// Finding aplati (fichier + finding) pour la liste.
pub(crate) struct FlatFinding {
    pub(crate) file_idx: usize,
    pub(crate) finding_idx: usize,
}

impl StyleState {
    pub(crate) fn new(cwd: String) -> Self {
        Self {
            path: cwd,
            auto_ran: false,
            editing_path: false,
            report: None,
            flat: Vec::new(),
            selected: 0,
            scroll: 0,
            busy: false,
            detail_popup: false,
            status: "Entrée : vérifier · e : changer le chemin · Échap : docs".into(),
        }
    }

    fn flatten(&mut self) {
        self.flat.clear();
        if let Some(r) = &self.report {
            for (fi, file) in r.files.iter().enumerate() {
                for (di, _) in file.findings.iter().enumerate() {
                    self.flat.push(FlatFinding {
                        file_idx: fi,
                        finding_idx: di,
                    });
                }
            }
        }
        self.selected = self.selected.min(self.flat.len().saturating_sub(1));
    }
}

pub(crate) fn style_run(shell: &Shell) {
    let checker = shell.config.style.to_checker();
    let path = PathBuf::from(shell.style.path.trim());
    let tx = shell.tx.clone();
    std::thread::spawn(move || {
        let result = style::check_path(&path, &checker).map_err(|e| e.to_string());
        let _ = tx.send(AsyncMsg::Style(result));
    });
}

pub(crate) fn on_style_result(shell: &mut Shell, result: Result<StyleReport, String>) {
    shell.style.busy = false;
    // auto-validation : si l'étape du plan était un check style et qu'elle passe, on la coche
    if let Ok(rep) = &result {
        if rep.total(crate::style::Severity::Major) == 0 {
            mark_current_step_done(shell);
        }
    }
    match result {
        Ok(report) => {
            let majors = report.total(Severity::Major);
            let minors = report.total(Severity::Minor);
            shell.style.status = format!(
                "{} majeures · {} mineures · {} auto-corrigibles — f: corriger le fichier, F: tout corriger, r: re-vérifier",
                majors,
                minors,
                report.fixable_count()
            );
            // progression
            shell
                .progress_data
                .record_style_run(&shell.style.path, majors as u32, minors as u32);
            rust_man::progress::save(&shell.progress_data);
            shell.style.report = Some(report);
            shell.style.flatten();
        }
        Err(e) => {
            shell.style.status = format!("erreur : {e}");
        }
    }
}

fn style_fix(shell: &mut Shell, only_selected_file: bool) {
    let Some(report) = &shell.style.report else { return };
    let mut applied = 0;
    let targets: Vec<usize> = if only_selected_file {
        shell
            .style
            .flat
            .get(shell.style.selected)
            .map(|f| vec![f.file_idx])
            .unwrap_or_default()
    } else {
        (0..report.files.len()).collect()
    };
    for fi in targets {
        let file = &report.files[fi];
        let fixes = style::dedup_fixes(&file.findings);
        if fixes.is_empty() {
            continue;
        }
        if let Ok(src) = std::fs::read_to_string(&file.path) {
            let (new_src, n) = style::apply_fixes(&src, &fixes);
            if n > 0 {
                if std::fs::write(&file.path, &new_src).is_ok() {
                    applied += n;
                }
            }
        }
    }
    shell.style.status = format!("{applied} correction(s) appliquée(s) — re-vérification…");
    // re-vérifie automatiquement après correction
    shell.style.busy = true;
    style_run(shell);
}

pub(crate) fn style_on_key(shell: &mut Shell, key: KeyEvent) {
    let st = &mut shell.style;
    if st.detail_popup {
        match key.code {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') => st.detail_popup = false,
            _ => {}
        }
        return;
    }
    if st.editing_path {
        match key.code {
            KeyCode::Esc => st.editing_path = false,
            KeyCode::Enter => {
                st.editing_path = false;
                st.busy = true;
                style_run(shell);
            }
            KeyCode::Backspace => {
                st.path.pop();
            }
            KeyCode::Char(c) => st.path.push(c),
            _ => {}
        }
        return;
    }
    match key.code {
        KeyCode::Esc => shell.set_screen(ScreenKind::Docs),
        KeyCode::Char('e') => st.editing_path = true,
        KeyCode::Enter => {
            if st.report.is_none() {
                st.busy = true;
                style_run(shell);
            } else if !st.flat.is_empty() {
                st.detail_popup = true;
            }
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if !st.flat.is_empty() {
                st.selected = (st.selected + 1).min(st.flat.len() - 1);
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            st.selected = st.selected.saturating_sub(1);
        }
        KeyCode::Char('f') => style_fix(shell, true),
        KeyCode::Char('F') => style_fix(shell, false),
        KeyCode::Char('r') => {
            st.busy = true;
            style_run(shell);
        }
        _ => {}
    }
}

/// Marque l'étape sélectionnée du plan comme faite (appelé quand une
/// vérification automatique passe : style propre, compile réussie).
fn mark_current_step_done(shell: &mut Shell) {
    if shell.screen != ScreenKind::Orchestra {
        return;
    }
    if let Some(plan) = &mut shell.orchestra.plan {
        if let Some(step) = plan.steps.get_mut(shell.orchestra.selected) {
            if matches!(
                step.action,
                crate::orchestra::StepAction::Style(_) | crate::orchestra::StepAction::Compile(_)
            ) {
                step.done = true;
            }
        }
    }
}

pub(crate) fn draw_style(frame: &mut Frame, shell: &mut Shell, area: Rect) {
    // lance le check à l'ouverture de l'écran (une seule fois)
    if !shell.style.auto_ran && shell.style.report.is_none() && !shell.style.busy {
        shell.style.auto_ran = true;
        shell.style.busy = true;
        style_run(shell);
    }
    let palette = *shell.palette();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(5)])
        .split(area);
    draw_input(
        frame,
        chunks[0],
        "chemin du projet / fichier",
        &shell.style.path,
        shell.style.editing_path,
        &palette,
    );

    let mut lines: Vec<Line> = Vec::new();
    let accent = Style::default().fg(palette.accent());
    let dim = Style::default().fg(palette.dim());
    if shell.style.busy {
        lines.push(Line::from(Span::styled("⏳ vérification en cours…", accent)));
    } else if let Some(report) = &shell.style.report {
        let mut cur_file = usize::MAX;
        for (i, flat) in shell.style.flat.iter().enumerate() {
            let file = &report.files[flat.file_idx];
            let f = &file.findings[flat.finding_idx];
            if flat.file_idx != cur_file {
                cur_file = flat.file_idx;
                lines.push(Line::default());
                lines.push(Line::from(Span::styled(
                    file.path.display().to_string(),
                    accent.add_modifier(Modifier::BOLD),
                )));
            }
            let sev_style = match f.severity {
                Severity::Major => Style::default().fg(palette.gold()).add_modifier(Modifier::BOLD),
                Severity::Minor => Style::default().fg(palette.purple()),
                Severity::Info => dim,
            };
            let marker = if i == shell.style.selected { "▸" } else { " " };
            let fixable = if f.fix.is_some() { " ⚙" } else { "" };
            lines.push(Line::from(vec![
                Span::styled(format!("{marker} {:>4}:{:<3} ", f.line, f.col), dim),
                Span::styled(format!("[{}]", f.severity), sev_style),
                Span::raw(format!(" {}", f.message)),
                Span::styled(format!("  ({}){fixable}", f.rule), dim),
            ]));
        }
        if shell.style.flat.is_empty() {
            lines.push(Line::from(Span::styled(
                "✓ aucune faute de style détectée. Propre !",
                Style::default().fg(palette.green()),
            )));
        }
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(shell.style.status.clone(), dim)));
    } else {
        lines.push(Line::from(Span::styled(
            "Chemin du projet ou du fichier à vérifier, puis Entrée.",
            dim,
        )));
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(
            "Le checker couvre : 100 colonnes, tabs vs 4 espaces, snake_case,",
            dim,
        )));
        lines.push(Line::from(Span::styled(
            "espaces en fin de ligne, newline final, unwrap hors tests, dbg! oubliés,",
            dim,
        )));
        lines.push(Line::from(Span::styled(
            "TODO, fonctions trop longues… avec auto-fix quand c'est sûr.",
            dim,
        )));
    }

    // scroll suit la sélection (avant le dessin pour libérer l'emprunt)
    let inner_h = chunks[1].height.saturating_sub(4) as usize;
    if shell.style.selected >= shell.style.scroll + inner_h {
        shell.style.scroll = shell.style.selected + 1 - inner_h.max(1);
    }
    if shell.style.selected < shell.style.scroll {
        shell.style.scroll = shell.style.selected;
    }
    draw_scroll_panel(
        frame,
        chunks[1],
        "rapport de style",
        &lines,
        shell.style.scroll,
        palette.accent(),
        "j/k naviguer · Entrée détails · f/F corriger · r re-vérifier · e chemin · Échap docs",
    );

    // popup détail + explication de la règle
    if shell.style.detail_popup {
        if let (Some(report), Some(flat)) =
            (&shell.style.report, shell.style.flat.get(shell.style.selected))
        {
            let f = &report.files[flat.file_idx].findings[flat.finding_idx];
            let (titre, pourquoi, comment) = style::explain_rule(f.rule);
            let popup = centered_popup(frame, frame.area(), 70, 13);
            let text = format!(
                "{}:{}  [{}] {}\n\n{}\n\nPourquoi : {pourquoi}\n\nCorriger : {comment}\n\n(Échap pour fermer)",
                f.line, f.col, f.severity, f.message, titre
            );
            let lines = super::md_to_lines(&text, 66, &palette);
            let block = ratatui::widgets::Block::default()
                .borders(ratatui::widgets::Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Double)
                .border_style(Style::default().fg(palette.accent()))
                .title(Span::styled(format!(" {} ", f.rule), accent));
            frame.render_widget(ratatui::widgets::Paragraph::new(lines).block(block), popup);
        }
    }
}

// ================================================================== COMPILE

pub(crate) struct CompileState {
    pub(crate) target: String,
    pub(crate) editing: bool,
    pub(crate) result: Option<CompileResult>,
    pub(crate) selected: usize,
    pub(crate) busy: bool,
    pub(crate) show_raw: bool,
}

impl CompileState {
    pub(crate) fn new(cwd: String) -> Self {
        Self {
            target: cwd,
            editing: false,
            result: None,
            selected: 0,
            busy: false,
            show_raw: false,
        }
    }
}

fn compile_run(shell: &Shell) {
    let target = PathBuf::from(shell.compile.target.trim());
    let tx = shell.tx.clone();
    std::thread::spawn(move || {
        let result = compile::compile_target(&target);
        let _ = tx.send(AsyncMsg::Compile(Box::new(result)));
    });
}

pub(crate) fn on_compile_result(shell: &mut Shell, result: CompileResult) {
    shell.compile.busy = false;
    if result.success {
        mark_current_step_done(shell);
    }
    shell.compile.selected = 0;
    shell.compile.result = Some(result);
}

pub(crate) fn compile_on_key(shell: &mut Shell, key: KeyEvent) {
    let st = &mut shell.compile;
    if st.editing {
        match key.code {
            KeyCode::Esc => st.editing = false,
            KeyCode::Enter => {
                st.editing = false;
                st.busy = true;
                compile_run(shell);
            }
            KeyCode::Backspace => {
                st.target.pop();
            }
            KeyCode::Char(c) => st.target.push(c),
            _ => {}
        }
        return;
    }
    let ndiags = st.result.as_ref().map(|r| r.diagnostics.len()).unwrap_or(0);
    match key.code {
        KeyCode::Esc => shell.set_screen(ScreenKind::Docs),
        KeyCode::Char('e') => st.editing = true,
        KeyCode::Enter => {
            if let Some(res) = &st.result {
                if let Some(d) = res.diagnostics.get(st.selected) {
                    if let Some(fiche) = d.hint_fiche {
                        let fiche = fiche.to_string();
                        shell.open_fiche(&fiche);
                        return;
                    }
                }
            }
            st.busy = true;
            compile_run(shell);
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if ndiags > 0 {
                st.selected = (st.selected + 1).min(ndiags - 1);
            }
        }
        KeyCode::Char('k') | KeyCode::Up => st.selected = st.selected.saturating_sub(1),
        KeyCode::Char('r') => st.show_raw = !st.show_raw,
        _ => {}
    }
}

pub(crate) fn draw_compile(frame: &mut Frame, shell: &mut Shell, area: Rect) {
    let palette = *shell.palette();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(5)])
        .split(area);
    draw_input(
        frame,
        chunks[0],
        "fichier .rs ou projet cargo (cargo check)",
        &shell.compile.target,
        shell.compile.editing,
        &palette,
    );

    let mut lines: Vec<Line> = Vec::new();
    let accent = Style::default().fg(palette.accent());
    let dim = Style::default().fg(palette.dim());
    if shell.compile.busy {
        lines.push(Line::from(Span::styled("⏳ compilation en cours…", accent)));
    } else if let Some(res) = &shell.compile.result {
        if shell.compile.show_raw {
            for l in res.raw.lines() {
                lines.push(Line::from(l.to_string()));
            }
        } else if res.diagnostics.is_empty() {
            if res.success {
                lines.push(Line::from(Span::styled(
                    "✓ compilation réussie, zéro diagnostic. Beau travail.",
                    Style::default().fg(palette.green()),
                )));
            } else {
                lines.push(Line::from(Span::styled(
                    "échec sans diagnostic parsé — appuie sur r pour la sortie brute",
                    dim,
                )));
            }
        } else {
            for (i, d) in res.diagnostics.iter().enumerate() {
                let marker = if i == shell.compile.selected { "▸" } else { " " };
                let lvl = match d.level {
                    DiagLevel::Error => Span::styled(
                        "ERREUR ",
                        Style::default().fg(palette.gold()).add_modifier(Modifier::BOLD),
                    ),
                    DiagLevel::Warning => Span::styled("WARNING", Style::default().fg(palette.purple())),
                    DiagLevel::Note => Span::styled("note   ", dim),
                };
                lines.push(Line::from(vec![
                    Span::raw(format!("{marker} ")),
                    Span::styled(format!("{}:{}:{} ", d.file, d.line, d.col), dim),
                    lvl,
                    Span::raw(d.message.clone()),
                ]));
                if let Some(hint) = d.hint_text {
                    lines.push(Line::from(vec![
                        Span::raw("     "),
                        Span::styled(format!("→ {hint}"), Style::default().fg(palette.green())),
                    ]));
                    if let Some(fiche) = d.hint_fiche {
                        if i == shell.compile.selected {
                            lines.push(Line::from(vec![
                                Span::raw("     "),
                                Span::styled(
                                    format!("→ Entrée : ouvrir la fiche « {fiche} »"),
                                    accent,
                                ),
                            ]));
                        }
                    }
                }
            }
        }
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(
            if shell.compile.show_raw {
                "r : retour aux diagnostics expliqués"
            } else {
                "r : sortie brute"
            },
            dim,
        )));
    } else {
        lines.push(Line::from(Span::styled(
            "Entrée : compiler le fichier/dossier et traduire les erreurs en français clair.",
            dim,
        )));
    }
    draw_scroll_panel(
        frame,
        chunks[1],
        "diagnostics",
        &lines,
        shell.compile.selected.saturating_sub(2),
        palette.accent(),
        "j/k naviguer · Entrée compiler/ouvrir fiche · r sortie brute · e cible · Échap docs",
    );
}

// ================================================================== PROGRESSION

pub(crate) fn progress_on_key(shell: &mut Shell, key: KeyEvent) {
    if key.code == KeyCode::Esc {
        shell.set_screen(ScreenKind::Docs);
    }
}

pub(crate) fn draw_progress(frame: &mut Frame, shell: &mut Shell, area: Rect) {
    let palette = *shell.palette();
    let p = &shell.progress_data;
    let mut lines: Vec<Line> = Vec::new();
    let accent = Style::default().fg(palette.accent());
    let dim = Style::default().fg(palette.dim());
    let green = Style::default().fg(palette.green());

    lines.push(Line::from(Span::styled("📊 Ta progression", accent.add_modifier(Modifier::BOLD))));
    lines.push(Line::default());
    lines.push(Line::from(format!(
        "Fiches consultées : {} / {}",
        p.fiches.len(),
        ENTRIES.len()
    )));
    for (cid, label) in rust_man::content::CATEGORIES {
        let (seen, total) = p.category_coverage(cid, &ENTRIES);
        if total > 0 {
            let filled = seen * 20 / total;
            let bar = format!("{}{}", "█".repeat(filled), "░".repeat(20 - filled));
            lines.push(Line::from(vec![
                Span::raw(format!("  {label:<22} ")),
                Span::styled(bar, green),
                Span::styled(format!(" {seen}/{total}"), dim),
            ]));
        }
    }
    let total_q: u32 = p.quiz.iter().map(|q| q.total).sum();
    let correct: u32 = p.quiz.iter().map(|q| q.correct).sum();
    lines.push(Line::default());
    if total_q > 0 {
        lines.push(Line::from(format!(
            "Quiz : {} sessions — {correct}/{total_q} ({} %)",
            p.quiz.len(),
            correct * 100 / total_q
        )));
    } else {
        lines.push(Line::from(Span::styled(
            "Quiz : aucune session pour l'instant — Ctrl+O puis 4 pour t'y mettre.",
            dim,
        )));
    }
    let weak = p.weak_categories();
    if !weak.is_empty() {
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(
            "Notions fragiles (à retravailler) :",
            Style::default().fg(palette.gold()),
        )));
        for (cat, ok, total) in weak {
            lines.push(Line::from(format!("  • {cat} : {ok}/{total} au quiz")));
        }
    }
    let done: usize = p.exercises_done.values().map(|v| v.len()).sum();
    lines.push(Line::default());
    lines.push(Line::from(format!("Exercices marqués faits : {done}")));
    if let Some(last) = p.style_runs.last() {
        lines.push(Line::from(format!(
            "Dernier check de style : {} ({} majeures, {} mineures)",
            last.path, last.majors, last.minors
        )));
    }
    draw_scroll_panel(
        frame,
        area,
        "progression",
        &lines,
        0,
        palette.accent(),
        "Échap docs",
    );
}

//! Shell TUI : héberge le navigateur de docs et tous les écrans outils.
//!
//! Navigation : `Ctrl+O` ouvre le menu outils depuis n'importe où ;
//! `!` lance dsh (handoff terminal, discret) ; `Échap` revient aux docs.

pub(crate) mod aiview;
pub(crate) mod docs;
pub(crate) mod exoview;
mod flashcards;
pub(crate) mod quizview;
pub(crate) mod tools;

use rust_man::ai::{AiClient, HelpLevel};
use rust_man::exo;
use rust_man::compile::CompileResult;
use rust_man::config::Config;
use rust_man::content::{by_id, ENTRIES};
use rust_man::markdown::{parse, MdLine};
use rust_man::style::Report as StyleReport;
use rust_man::orchestra::Plan;
use rust_man::progress::Progress;
use rust_man::render::wrap_segments;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use docs::{segments_to_spans, DocsApp, Palette};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};
use std::io;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;

// ------------------------------------------------------------------ screens

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScreenKind {
    Docs,
    Style,
    Compile,
    Quiz,
    Flashcards,
    Exercices,
    Progress,
    Chat,
    Orchestra,
}

impl ScreenKind {
    fn title(&self) -> &'static str {
        match self {
            Self::Docs => "Documentation",
            Self::Style => "Style Rust",
            Self::Compile => "Compiler & comprendre",
            Self::Quiz => "Quiz",
            Self::Flashcards => "Flashcards",
            Self::Exercices => "Exercices",
            Self::Progress => "Progression",
            Self::Chat => "Assistants IA",
            Self::Orchestra => "Complémentarité de l'Homme",
        }
    }
}

/// Résultats renvoyés par les threads de travail vers la boucle TUI.
pub(crate) enum AsyncMsg {
    Style(Result<StyleReport, String>),
    Compile(Box<CompileResult>),
    AiChat(Result<String, String>),
    OrchestraPlan(Result<Plan, String>),
    OrchestraTeach(Result<String, String>),
    ExoResult((String, Vec<String>, bool)),
}

pub(crate) struct Shell {
    pub(crate) docs: DocsApp,
    pub(crate) screen: ScreenKind,
    pub(crate) menu_open: bool,
    pub(crate) style: tools::StyleState,
    pub(crate) compile: tools::CompileState,
    pub(crate) quiz: quizview::QuizState,
    pub(crate) flashcards: flashcards::FlashState,
    pub(crate) exo: exoview::ExoState,
    pub(crate) chat: aiview::ChatState,
    pub(crate) orchestra: aiview::OrchestraState,
    pub(crate) progress_data: Progress,
    pub(crate) config: Config,
    pub(crate) ai: Option<AiClient>,
    pub(crate) help_level: HelpLevel,
    pub(crate) tx: Sender<AsyncMsg>,
    pub(crate) rx: Receiver<AsyncMsg>,
    pub(crate) should_quit: bool,
    /// Quand Some, une ligne de saisie pour dsh est ouverte.
    pub(crate) dsh_input: Option<String>,
    /// Prompt dsh validé, en attente de handoff (traité par la boucle).
    pub(crate) pending_dsh: Option<String>,
}

impl Shell {
    fn new(start: Option<usize>) -> Self {
        let (tx, rx) = channel();
        let config = rust_man::config::load();
        let ai = AiClient::from_env(&config).ok();
        let help_level = config.ai_help_level.map(HelpLevel::from_u8).unwrap_or_default();
        let cwd = std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| ".".into());
        Self {
            docs: DocsApp::new(start),
            screen: ScreenKind::Docs,
            menu_open: false,
            style: tools::StyleState::new(cwd.clone()),
            compile: tools::CompileState::new(cwd),
            quiz: quizview::QuizState::new(),
            flashcards: flashcards::FlashState::new(),
            exo: exoview::ExoState::new(),
            chat: aiview::ChatState::new(),
            orchestra: aiview::OrchestraState::new(),
            progress_data: rust_man::progress::load(),
            config,
            ai,
            help_level,
            tx,
            rx,
            should_quit: false,
            dsh_input: None,
            pending_dsh: None,
        }
    }

    /// Une saisie texte est-elle active ? (le raccourci global `!` doit alors
    /// rester une lettre comme une autre)
    fn text_input_active(&self) -> bool {
        if self.dsh_input.is_some() || self.menu_open {
            return true;
        }
        match self.screen {
            ScreenKind::Docs => self.docs.focus == docs::Focus::Search,
            ScreenKind::Style => self.style.editing_path,
            ScreenKind::Compile => self.compile.editing,
            ScreenKind::Quiz => false,
            ScreenKind::Flashcards => false,
            ScreenKind::Exercices => false,
            ScreenKind::Progress => false,
            ScreenKind::Chat => true,
            ScreenKind::Orchestra => self.orchestra.mode == aiview::OrchestraMode::Mission,
        }
    }

    /// Colle du texte (bracketed paste) dans la saisie active.
    fn on_paste(&mut self, text: String) {
        let text = text.replace(['\n', '\r'], " ");
        if let Some(s) = &mut self.dsh_input {
            s.push_str(&text);
            return;
        }
        match self.screen {
            ScreenKind::Docs => {
                if self.docs.focus == docs::Focus::Search {
                    self.docs.search.push_str(&text);
                    self.docs.rebuild_rows();
                }
            }
            ScreenKind::Style => {
                if self.style.editing_path {
                    self.style.path.push_str(&text);
                }
            }
            ScreenKind::Compile => {
                if self.compile.editing {
                    self.compile.target.push_str(&text);
                }
            }
            ScreenKind::Chat => self.chat.input.push_str(&text),
            ScreenKind::Orchestra => {
                if self.orchestra.mode == aiview::OrchestraMode::Mission {
                    self.orchestra.input.push_str(&text);
                }
            }
            _ => {}
        }
    }

    pub(crate) fn palette(&self) -> &Palette {
        &self.docs.palette
    }

    /// Ouvre une fiche dans l'écran docs (utilisé par les outils).
    /// Tolérant : id exact, sinon meilleure correspondance de recherche.
    pub(crate) fn open_fiche(&mut self, id: &str) {
        let target = by_id(id).map(|e| e.id.clone()).or_else(|| {
            rust_man::content::search(id)
                .first()
                .map(|(_, e)| e.id.clone())
        });
        if let Some(id) = target {
            if let Some(idx) = ENTRIES.iter().position(|e| e.id == id) {
                self.docs.current = Some(idx);
                self.docs.scroll = 0;
                self.docs.doc_entry = None; // force rebuild
                self.screen = ScreenKind::Docs;
                self.docs.focus = docs::Focus::Doc;
            }
        }
    }

    fn set_screen(&mut self, s: ScreenKind) {
        self.screen = s;
        self.menu_open = false;
        if s == ScreenKind::Progress {
            self.progress_data = rust_man::progress::load();
        }
        if s == ScreenKind::Chat {
            let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
            self.chat.project_files = rust_man::project::count_sources(&cwd);
        }
    }

    fn on_key(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.should_quit = true;
            return;
        }
        // menu outils
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('o') {
            self.menu_open = !self.menu_open;
            return;
        }
        if self.menu_open {
            self.on_key_menu(key);
            return;
        }
        // saisie du prompt dsh en cours ?
        if self.dsh_input.is_some() {
            match key.code {
                KeyCode::Esc => self.dsh_input = None,
                KeyCode::Enter => {
                    let prompt = self.dsh_input.take().unwrap_or_default();
                    self.pending_dsh = Some(prompt);
                }
                KeyCode::Backspace => {
                    if let Some(s) = &mut self.dsh_input {
                        s.pop();
                    }
                }
                KeyCode::Char(c) => {
                    if let Some(s) = &mut self.dsh_input {
                        s.push(c);
                    }
                }
                _ => {}
            }
            return;
        }
        // dsh : discret, mais JAMAIS pendant une saisie texte (le « ! » de « != »)
        if key.code == KeyCode::Char('!') && !self.text_input_active() {
            self.dsh_input = Some(String::new());
            return;
        }
        match self.screen {
            ScreenKind::Docs => {
                // 'e' : ouvrir l'exercice lié à la fiche courante
                if key.code == KeyCode::Char('e')
                    && !key.modifiers.contains(KeyModifiers::CONTROL)
                    && self.docs.focus != docs::Focus::Search
                {
                    if let Some(entry) = self.docs.current_entry() {
                        if exo::EXOS.iter().any(|x| x.fiche == entry.id) {
                            self.set_screen(ScreenKind::Exercices);
                            // sélectionne l'exo lié
                            if let Some(pos) = exo::EXOS.iter().position(|x| x.fiche == entry.id) {
                                self.exo.selected = pos;
                            }
                        }
                    }
                    return;
                }
                self.docs.on_key(key);
                if self.docs.should_quit {
                    self.should_quit = true;
                }
            }
            ScreenKind::Style => tools::style_on_key(self, key),
            ScreenKind::Compile => tools::compile_on_key(self, key),
            ScreenKind::Quiz => quizview::on_key(self, key),
            ScreenKind::Flashcards => flashcards::on_key(self, key),
            ScreenKind::Exercices => exoview::on_key(self, key),
            ScreenKind::Progress => tools::progress_on_key(self, key),
            ScreenKind::Chat => aiview::chat_on_key(self, key),
            ScreenKind::Orchestra => aiview::orchestra_on_key(self, key),
        }
    }

    fn on_key_menu(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => self.menu_open = false,
            KeyCode::Char('1') => self.set_screen(ScreenKind::Docs),
            KeyCode::Char('2') => self.set_screen(ScreenKind::Style),
            KeyCode::Char('3') => self.set_screen(ScreenKind::Compile),
            KeyCode::Char('4') => self.set_screen(ScreenKind::Quiz),
            KeyCode::Char('5') => self.set_screen(ScreenKind::Progress),
            KeyCode::Char('6') => self.set_screen(ScreenKind::Chat),
            KeyCode::Char('7') => self.set_screen(ScreenKind::Orchestra),
            KeyCode::Char('8') => self.set_screen(ScreenKind::Flashcards),
            KeyCode::Char('9') => self.set_screen(ScreenKind::Exercices),
            _ => {}
        }
    }

    fn drain_async(&mut self) {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                AsyncMsg::Style(r) => tools::on_style_result(self, r),
                AsyncMsg::Compile(r) => tools::on_compile_result(self, *r),
                AsyncMsg::AiChat(r) => aiview::on_chat_result(self, r),
                AsyncMsg::OrchestraPlan(r) => aiview::on_plan_result(self, r),
                AsyncMsg::OrchestraTeach(r) => aiview::on_teach_result(self, r),
                AsyncMsg::ExoResult(r) => exoview::on_exo_result(self, r),
            }
        }
    }

    /// Handoff terminal vers dsh : restore, lance headless avec le prompt,
    /// attend, réinitialise. Le contexte projet est passé dans le prompt.
    #[allow(dead_code)]
    fn launch_dsh(&mut self, prompt: &str) {
        let handle = rust_man::dsh::detect(&self.config);
        ratatui::restore();
        match handle {
            Some(h) => {
                let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
                if prompt.trim().is_empty() {
                    println!("── dsh sans prompt : annulé (donne une tâche à dsh) ──");
                } else {
                    println!("── dsh headless ({}) — « {} » ──\n", h.command, prompt);
                    let status = h
                        .spawn_headless(prompt, &cwd)
                        .stdin(std::process::Stdio::inherit())
                        .stdout(std::process::Stdio::inherit())
                        .stderr(std::process::Stdio::inherit())
                        .status();
                    match status {
                        Ok(s) => println!("\n── dsh terminé ({:?}) ──", s.code()),
                        Err(e) => println!("lancement impossible : {e}"),
                    }
                }
            }
            None => {
                println!("dsh introuvable dans le PATH.");
                println!("Configure ~/.config/rust-man/config.toml :");
                println!("  dsh_command = \"pnpm dsh\"");
                println!("  dsh_dir = \"~/deepseek-harness\"");
            }
        }
        println!("\nAppuie sur Entrée pour revenir à rust-man…");
        let mut buf = String::new();
        let _ = std::io::stdin().read_line(&mut buf);
        let _ = ratatui::init();
        // force un redraw complet
        self.docs.doc_entry = None;
    }
}

// ------------------------------------------------------------------ helpers de rendu

/// Rendu markdown générique (réponses IA, synthèses) en lignes ratatui.
pub(crate) fn md_to_lines(text: &str, width: usize, palette: &Palette) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    let text_w = width.max(20);
    let md = parse(text);
    let mut code_buf: Vec<String> = Vec::new();
    let flush = |lines: &mut Vec<Line<'static>>, buf: &mut Vec<String>| {
        if buf.is_empty() {
            return;
        }
        let code = buf.join("\n");
        let bg = palette.code_bg();
        for line in rust_man::highlight::highlight_c(&code) {
            let mut spans: Vec<Span<'static>> =
                vec![Span::styled("  ".to_string(), Style::default().bg(bg))];
            for (style, t) in &line {
                spans.push(Span::styled(
                    t.clone(),
                    Style::default()
                        .fg(Color::Rgb(style.foreground.r, style.foreground.g, style.foreground.b))
                        .bg(bg),
                ));
            }
            lines.push(Line::from(spans));
        }
        buf.clear();
    };
    for mdline in &md {
        match mdline {
            MdLine::Code(c) => code_buf.push(c.clone()),
            other => {
                flush(&mut lines, &mut code_buf);
                match other {
                    MdLine::Blank => lines.push(Line::default()),
                    MdLine::Heading(_, t) => lines.push(Line::from(Span::styled(
                        t.clone(),
                        Style::default()
                            .fg(palette.accent())
                            .add_modifier(Modifier::BOLD),
                    ))),
                    MdLine::Text(segs) => {
                        for w in wrap_segments(segs, text_w) {
                            lines.push(Line::from(segments_to_spans(palette, &w)));
                        }
                    }
                    MdLine::Bullet(segs) => {
                        for (i, w) in wrap_segments(segs, text_w.saturating_sub(2)).iter().enumerate() {
                            let mut spans = vec![Span::styled(
                                if i == 0 { "• ".to_string() } else { "  ".to_string() },
                                Style::default().fg(palette.accent()),
                            )];
                            spans.extend(segments_to_spans(palette, w));
                            lines.push(Line::from(spans));
                        }
                    }
                    MdLine::BulletNum(num, segs) => {
                        let marker = format!("{num} ");
                        let indent = " ".repeat(marker.len());
                        for (i, w) in wrap_segments(segs, text_w.saturating_sub(marker.len())).iter().enumerate() {
                            let mut spans = vec![Span::styled(
                                if i == 0 { marker.clone() } else { indent.clone() },
                                Style::default().fg(palette.accent()),
                            )];
                            spans.extend(segments_to_spans(palette, w));
                            lines.push(Line::from(spans));
                        }
                    }
                    MdLine::Quote(segs) => {
                        for w in wrap_segments(segs, text_w.saturating_sub(2)) {
                            let mut spans =
                                vec![Span::styled("▎ ".to_string(), Style::default().fg(palette.dim()))];
                            spans.extend(segments_to_spans(palette, &w));
                            lines.push(Line::from(spans));
                        }
                    }
                    MdLine::Code(_) => {}
                }
            }
        }
    }
    flush(&mut lines, &mut code_buf);
    lines
}

/// Tronque une chaîne à `width` colonnes avec « … » si coupée.
pub(crate) fn truncate_width(s: &str, width: usize) -> String {
    if unicode_width::UnicodeWidthStr::width(s) <= width {
        return s.to_string();
    }
    let mut out = String::new();
    let mut w = 0;
    for c in s.chars() {
        let cw = unicode_width::UnicodeWidthStr::width(c.to_string().as_str());
        if w + cw > width.saturating_sub(1) {
            break;
        }
        out.push(c);
        w += cw;
    }
    out.push('…');
    out
}

/// Panneau scrollable ineffable : titre discret + filet, contenu qui respire,
/// plus de boîte. La hiérarchie se lit par l'espace et le ton.
pub(crate) fn draw_scroll_panel(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    lines: &[Line],
    scroll: usize,
    _border_color: Color,
    footer_hint: &str,
) -> usize {
    let dim = Style::default().fg(Color::Rgb(134, 134, 139));
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // titre
            Constraint::Length(1), // filet
            Constraint::Min(3),    // contenu
            Constraint::Length(1), // hint
        ])
        .split(area);
    // titre discret
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            format!("  {}", title.to_lowercase()),
            dim.add_modifier(Modifier::BOLD),
        ))),
        chunks[0],
    );
    // filet
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "─".repeat(area.width as usize),
            Style::default().fg(Color::Rgb(48, 48, 52)),
        ))),
        chunks[1],
    );
    let inner_h = chunks[2].height as usize;
    let max_scroll = lines.len().saturating_sub(inner_h);
    let start = scroll.min(max_scroll);
    let visible: Vec<Line> = lines
        .iter()
        .skip(start)
        .take(inner_h)
        .cloned()
        .collect();
    frame.render_widget(
        Paragraph::new(visible).block(Block::default().padding(ratatui::widgets::Padding {
            left: 2,
            right: 1,
            top: 0,
            bottom: 0,
        })),
        chunks[2],
    );
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            format!("  {footer_hint}"),
            dim,
        ))),
        chunks[3],
    );
    inner_h
}

/// Ligne d'input générique.
pub(crate) fn draw_input(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    value: &str,
    focused: bool,
    palette: &Palette,
) {
    // ineffable : pas de boîte — libellé discret, saisie, filet fin dessous
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1), Constraint::Length(1)])
        .split(area);
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            format!("  {label}"),
            Style::default().fg(palette.dim()),
        ))),
        chunks[0],
    );
    let content = if focused {
        Line::from(vec![
            Span::raw("  "),
            Span::raw(value.to_string()),
            Span::styled("▌", Style::default().fg(palette.accent())),
        ])
    } else {
        Line::from(Span::styled(format!("  {value}"), Style::default()))
    };
    frame.render_widget(Paragraph::new(content), chunks[1]);
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "─".repeat(area.width as usize),
            Style::default().fg(Color::Rgb(48, 48, 52)),
        ))),
        chunks[2],
    );
}

/// Popup centré.
pub(crate) fn centered_popup(frame: &mut Frame, area: Rect, w: u16, h: u16) -> Rect {
    let popup = Rect {
        x: area.x + (area.width.saturating_sub(w)) / 2,
        y: area.y + (area.height.saturating_sub(h)) / 2,
        width: w.min(area.width),
        height: h.min(area.height),
    };
    frame.render_widget(Clear, popup);
    popup
}

// ------------------------------------------------------------------ dessin

fn draw(frame: &mut Frame, shell: &mut Shell) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(5)])
        .split(area);

    // barre de titre
    let weak = shell.progress_data.weak_categories();
    let weak_hint = if weak.is_empty() {
        String::new()
    } else {
        format!("   ⚠ fragile: {}", weak[0].0)
    };
    let ai_badge = if shell.ai.is_some() { "IA ✓" } else { "IA ✗" };
    let title = Line::from(vec![
        Span::styled(
            " rust-man ",
            Style::default()
                .fg(Color::Black)
                .bg(shell.palette().accent())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("  {} ", shell.screen.title()),
            Style::default()
                .fg(shell.palette().accent())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!("· {ai_badge} · Ctrl+O outils{weak_hint}"), Style::default().fg(shell.palette().dim())),
    ]);
    frame.render_widget(Paragraph::new(title), chunks[0]);

    match shell.screen {
        ScreenKind::Docs => docs::draw(frame, &mut shell.docs, chunks[1]),
        ScreenKind::Style => tools::draw_style(frame, shell, chunks[1]),
        ScreenKind::Compile => tools::draw_compile(frame, shell, chunks[1]),
        ScreenKind::Quiz => quizview::draw(frame, shell, chunks[1]),
        ScreenKind::Flashcards => flashcards::draw(frame, shell, chunks[1]),
        ScreenKind::Exercices => exoview::draw(frame, shell, chunks[1]),
        ScreenKind::Progress => tools::draw_progress(frame, shell, chunks[1]),
        ScreenKind::Chat => aiview::draw_chat(frame, shell, chunks[1]),
        ScreenKind::Orchestra => aiview::draw_orchestra(frame, shell, chunks[1]),
    }

    if shell.menu_open {
        draw_menu(frame, shell, area);
    }

    // saisie du prompt dsh (popup bas)
    if let Some(input) = &shell.dsh_input {
        let popup_area = Rect {
            x: area.x + 4,
            y: area.y + area.height.saturating_sub(4),
            width: area.width.saturating_sub(8),
            height: 3,
        };
        frame.render_widget(Clear, popup_area);
        let palette = *shell.palette();
        draw_input(
            frame,
            popup_area,
            "dsh headless — ta demande (Entrée : lancer, Échap : annuler)",
            input,
            true,
            &palette,
        );
    }
}

fn draw_menu(frame: &mut Frame, shell: &Shell, area: Rect) {
    let popup = centered_popup(frame, area, 40, 13);
    let entries = [
        ("1", "Documentation"),
        ("2", "Style Rust"),
        ("3", "Compiler & comprendre"),
        ("4", "Quiz"),
        ("5", "Progression"),
        ("6", "Assistants IA"),
        ("7", "Complémentarité de l'Homme"),
        ("8", "Flashcards"),
        ("9", "Exercices"),
    ];
    let mut lines: Vec<Line> = vec![Line::default()];
    for (key, name) in entries {
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {key}  "),
                Style::default()
                    .fg(shell.palette().accent())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(name.to_string(), Style::default()),
        ]));
    }
    lines.push(Line::default());
    lines.push(Line::from(Span::styled(
        "  Échap fermer",
        Style::default().fg(shell.palette().dim()),
    )));
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(shell.palette().accent()))
        .title(Span::styled(
            " Outils ",
            Style::default()
                .fg(shell.palette().accent())
                .add_modifier(Modifier::BOLD),
        ));
    frame.render_widget(Paragraph::new(lines).block(block), popup);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    /// Régression : un scroll à usize::MAX (arrivée d'une réponse IA) doit
    /// montrer le BAS de la page, pas juste la dernière ligne.
    #[test]
    fn scroll_max_shows_bottom_page() {
        let backend = TestBackend::new(60, 12);
        let mut terminal = Terminal::new(backend).unwrap();
        let lines: Vec<Line> = (1..=50)
            .map(|i| Line::from(format!("ligne {i}")))
            .collect();
        terminal
            .draw(|f| {
                draw_scroll_panel(
                    f,
                    f.area(),
                    "test",
                    &lines,
                    usize::MAX,
                    Color::Cyan,
                    "",
                );
            })
            .unwrap();
        let buf = terminal.backend().buffer().clone();
        let text: String = buf.content().iter().map(|c| c.symbol()).collect();
        // la dernière ligne est visible…
        assert!(text.contains("ligne 50"), "dernière ligne absente");
        // …et toute la page avec elle (9 lignes visibles : 42..=50), pas juste elle
        assert!(text.contains("ligne 42"), "le bas de page devrait être visible");
        assert!(!text.contains("ligne 41"), "la page ne devrait pas remonter plus haut");
    }
}

// ------------------------------------------------------------------ boucle

/// Init terminal avec bracketed paste (pour que Ctrl+Shift+V colle en bloc).
fn init_terminal() -> io::Result<ratatui::DefaultTerminal> {
    use crossterm::event::EnableBracketedPaste;
    use crossterm::execute;
    use crossterm::terminal::{enable_raw_mode, EnterAlternateScreen};
    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen, EnableBracketedPaste)?;
    ratatui::DefaultTerminal::new(ratatui::backend::CrosstermBackend::new(io::stdout()))
}

fn restore_terminal() {
    use crossterm::event::DisableBracketedPaste;
    use crossterm::execute;
    use crossterm::terminal::{disable_raw_mode, LeaveAlternateScreen};
    let _ = execute!(io::stdout(), DisableBracketedPaste, LeaveAlternateScreen);
    let _ = disable_raw_mode();
}

/// Lance la TUI. `start_topic` ouvre directement une fiche si donné.
pub fn run(start_topic: Option<&str>) -> io::Result<()> {
    if ENTRIES.is_empty() {
        eprintln!("rust-man: aucune fiche embarquée (content/ vide à la compilation).");
        return Ok(());
    }
    let start = start_topic
        .and_then(by_id)
        .map(|e| ENTRIES.iter().position(|x| x.id == e.id).unwrap());
    let mut shell = Shell::new(start);

    let mut terminal = init_terminal()?;
    let result = run_loop(&mut terminal, &mut shell);
    restore_terminal();
    result
}

fn run_loop(terminal: &mut ratatui::DefaultTerminal, shell: &mut Shell) -> io::Result<()> {
    while !shell.should_quit {
        shell.drain_async();
        terminal.draw(|frame| draw(frame, shell))?;
        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) => shell.on_key(key),
                Event::Paste(text) => shell.on_paste(text),
                _ => {}
            }
        }
    }
    // sauvegarde la progression en quittant
    rust_man::progress::save(&shell.progress_data);
    Ok(())
}

#[cfg(test)]
mod layout_tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    /// Le panneau scrollable : titre en haut, hint en bas, pas de chevauchement.
    #[test]
    fn scroll_panel_layout_no_overlap() {
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let lines: Vec<Line> = (1..=10).map(|i| Line::from(format!("ligne {i}"))).collect();
        terminal
            .draw(|f| {
                draw_scroll_panel(f, f.area(), "Test Panel", &lines, 0, Color::Cyan, "hint-bas");
            })
            .unwrap();
        let buf = terminal.backend().buffer().clone();
        // titre en haut (row 0)
        let row0: String = (0..80).map(|x| buf[(x, 0)].symbol()).collect();
        assert!(row0.contains("test panel"), "titre en haut: {row0:?}");
        // filet row 1
        let row1: String = (0..80).map(|x| buf[(x, 1)].symbol()).collect();
        assert!(row1.trim().chars().all(|c| c == '─') && !row1.trim().is_empty(), "filet row1: {row1:?}");
        // contenu row 2+
        let row2: String = (0..80).map(|x| buf[(x, 2)].symbol()).collect();
        assert!(row2.contains("ligne 1"), "contenu row2: {row2:?}");
        // hint en bas (row 19)
        let row19: String = (0..80).map(|x| buf[(x, 19)].symbol()).collect();
        assert!(row19.contains("hint-bas"), "hint en bas: {row19:?}");
    }
}

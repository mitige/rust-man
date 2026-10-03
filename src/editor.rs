//! rust-nano — éditeur de texte CLI, complétion IA.
//!
//! Tab = 4 espaces (la convention Rust), coloration syntaxique (syntect),
//! complétion IA en texte fantôme, sauvegarde douce, zéro bruit visuel.

use crate::ai::{system_prompt, AgentRole, AiClient};
use crate::highlight;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};


/// Agents pertinents pour la complétion de code.
const COMPLETE_AGENTS: &[AgentRole] = &[
    AgentRole::CodeExplainer,
    AgentRole::Intervenant,
    AgentRole::Aine,
    AgentRole::BinomeBasNiveau,
    AgentRole::BinomeClean,
    AgentRole::BinomeAlgo,
];

fn load_complete_agent() -> AgentRole {
    let cfg = crate::config::load();
    let name = cfg.nano_agent.as_deref().unwrap_or("Code Explainer");
    COMPLETE_AGENTS
        .iter()
        .find(|a| a.name() == name)
        .copied()
        .unwrap_or(AgentRole::CodeExplainer)
}

fn save_complete_agent(role: AgentRole) {
    crate::config::save_nano_agent(role.name());
}

// ------------------------------------------------------------------ thème

struct Ed;

#[allow(dead_code)]
impl Ed {
    fn text() -> Color {
        Color::Rgb(245, 245, 247)
    }
    fn dim() -> Color {
        Color::Rgb(134, 134, 139)
    }
    fn gutter() -> Color {
        Color::Rgb(72, 72, 76)
    }
    fn accent() -> Color {
        Color::Rgb(96, 165, 250)
    }
    fn sel_bg() -> Color {
        Color::Rgb(44, 50, 64)
    }
    fn ghost() -> Color {
        Color::Rgb(110, 116, 134)
    }
}

// ------------------------------------------------------------------ éditeur

pub struct Editor {
    lines: Vec<String>,
    cx: usize,
    cy: usize,
    scroll_x: usize,
    scroll_y: usize,
    file: Option<PathBuf>,
    modified: bool,
    status: String,
    /// suggestion IA en cours (texte fantôme)
    ghost: Option<String>,
    ghost_lines: Vec<String>,
    ai_busy: bool,
    rx: Option<Receiver<Result<String, String>>>,
    clipboard: String,
    should_quit: bool,
    confirm_quit: bool,
    /// agent qui pilote la complétion (sélectionnable, persisté)
    complete_agent: AgentRole,
    /// dernier bilan norme (après sauvegarde)
    norme_note: Option<String>,
    norme_rx: Option<Receiver<String>>,
    /// saisie de recherche (^F) ou goto (^G) en cours
    prompt: Option<(char, String)>,
    /// extension du fichier (pour la coloration)
    ext: String,
    /// modèle IA de l'éditeur (affiché dans la barre)
    ai_model: String,
    /// client IA possédé (pour changer de modèle à chaud)
    ai: Option<AiClient>,
    /// historique pour l'undo (Ctrl+Z)
    history: Vec<(Vec<String>, usize, usize)>,
}

impl Editor {
    pub fn open(path: Option<&Path>) -> io::Result<Self> {
        let (lines, file) = match path {
            Some(p) if p.exists() => {
                let text = std::fs::read_to_string(p)?;
                let mut lines: Vec<String> = text.lines().map(|l| l.to_string()).collect();
                if lines.is_empty() {
                    lines.push(String::new());
                }
                (lines, Some(p.to_path_buf()))
            }
            Some(p) => (vec![String::new()], Some(p.to_path_buf())),
            None => (vec![String::new()], None),
        };
        let ext = file
            .as_ref()
            .and_then(|p| p.extension())
            .and_then(|e| e.to_str())
            .unwrap_or("c")
            .to_string();
        let _ = file; // l'extension est déjà extraite
        Ok(Self {
            lines,
            cx: 0,
            cy: 0,
            scroll_x: 0,
            scroll_y: 0,
            file,
            modified: false,
            status: "^S sauver · ^Q quitter · ^Espace complétion IA".to_string(),
            ghost: None,
            ghost_lines: Vec::new(),
            ai_busy: false,
            rx: None,
            clipboard: String::new(),
            should_quit: false,
            confirm_quit: false,
            complete_agent: load_complete_agent(),
            norme_note: None,
            norme_rx: None,
            prompt: None,
            history: Vec::new(),
            ext,
            ai_model: String::new(),
            ai: None,
        })
    }

    // -------------------------------------------------------------- texte

    fn line(&self) -> &str {
        &self.lines[self.cy]
    }

    /// Sauvegarde l'état avant une modification (pour Ctrl+Z).
    fn snapshot(&mut self) {
        self.history
            .push((self.lines.clone(), self.cx, self.cy));
        if self.history.len() > 200 {
            self.history.remove(0);
        }
    }

    fn undo(&mut self) {
        if let Some((lines, cx, cy)) = self.history.pop() {
            self.lines = lines;
            self.cx = cx;
            self.cy = cy;
            self.modified = true;
            self.status = "annulé".into();
        }
    }

    fn insert_char(&mut self, c: char) {
        self.snapshot();
        let line = &mut self.lines[self.cy];
        line.insert(self.cx, c);
        self.cx += 1;
        self.modified = true;
        self.ghost = None;
    }

    fn insert_newline(&mut self) {
        self.snapshot();
        let rest = self.lines[self.cy].split_off(self.cx);
        // indentation automatique : reprend l'indentation de la ligne courante
        let indent: String = self.lines[self.cy]
            .chars()
            .take_while(|c| c.is_whitespace())
            .collect();
        let indent_len = indent.len();
        self.cy += 1;
        self.lines.insert(self.cy, indent + &rest);
        self.cx = indent_len;
        self.modified = true;
        self.ghost = None;
    }

    fn backspace(&mut self) {
        self.snapshot();
        if self.cx > 0 {
            let line = &mut self.lines[self.cy];
            line.remove(self.cx - 1);
            self.cx -= 1;
            self.modified = true;
        } else if self.cy > 0 {
            let cur = self.lines.remove(self.cy);
            self.cy -= 1;
            self.cx = self.lines[self.cy].len();
            self.lines[self.cy].push_str(&cur);
            self.modified = true;
        }
        self.ghost = None;
    }

    fn move_left(&mut self) {
        if self.cx > 0 {
            self.cx -= 1;
        } else if self.cy > 0 {
            self.cy -= 1;
            self.cx = self.lines[self.cy].len();
        }
    }

    fn move_right(&mut self) {
        if self.cx < self.line().len() {
            self.cx += 1;
        } else if self.cy + 1 < self.lines.len() {
            self.cy += 1;
            self.cx = 0;
        }
    }

    fn move_up(&mut self) {
        if self.cy > 0 {
            self.cy -= 1;
            self.cx = self.cx.min(self.lines[self.cy].len());
        }
    }

    fn move_down(&mut self) {
        if self.cy + 1 < self.lines.len() {
            self.cy += 1;
            self.cx = self.cx.min(self.lines[self.cy].len());
        }
    }

    /// Remplace toutes les occurrences de « motif→remplacement » (séparateur →).
    fn replace_all(&mut self, spec: &str) {
        let Some((needle, repl)) = spec.split_once('→').or_else(|| spec.split_once("->")) else {
            self.status = "syntaxe : motif→remplacement".into();
            return;
        };
        if needle.is_empty() {
            return;
        }
        self.snapshot();
        let mut count = 0;
        for line in &mut self.lines {
            if line.contains(needle) {
                let n = line.matches(needle).count();
                *line = line.replace(needle, repl);
                count += n;
            }
        }
        self.modified = count > 0;
        self.status = format!("{count} remplacement(s)");
    }

    fn find(&mut self, needle: &str) {
        if needle.is_empty() {
            return;
        }
        let n = self.lines.len();
        for k in 0..n {
            let row = (self.cy + k) % n;
            if let Some(col) = self.lines[row].find(needle) {
                if row == self.cy && col <= self.cx && k == 0 {
                    continue; // déjà dessus, cherche la suivante
                }
                self.cy = row;
                self.cx = col;
                self.status = format!("« {needle} » — ligne {}", row + 1);
                return;
            }
        }
        self.status = format!("« {needle} » introuvable");
    }

    fn save(&mut self) {
        let Some(path) = &self.file else {
            self.status = "Pas de nom de fichier — relance avec : nano <fichier>".into();
            return;
        };
        let mut text = self.lines.join("\n");
        text.push('\n'); // C-A3 : newline final, toujours
        match std::fs::write(path, text) {
            Ok(()) => {
                self.modified = false;
                self.status = format!("{} — sauvegardé ✓", path.display());
                // check style automatique après sauvegarde (non bloquant)
                let p = path.clone();
                let (tx, rx) = channel();
                std::thread::spawn(move || {
                    let cfg = crate::config::load().style.to_checker();
                    let note = match crate::style::check_path(&p, &cfg) {
                        Ok(rep) => {
                            let maj = rep.total(crate::style::Severity::Major);
                            let min = rep.total(crate::style::Severity::Minor);
                            if maj == 0 && min == 0 {
                                "style ✓".to_string()
                            } else {
                                format!("style: {}M {}m — rust-man check", maj, min)
                            }
                        }
                        Err(_) => String::new(),
                    };
                    let _ = tx.send(note);
                });
                self.norme_rx = Some(rx);
            }
            Err(e) => self.status = format!("erreur d'écriture : {e}"),
        }
    }

    // -------------------------------------------------------------- IA

    /// Demande une complétion à l'agent (contexte : le code avant le curseur).
    fn ai_complete(&mut self, client: &AiClient) {
        if self.ai_busy {
            return;
        }
        // contexte : les 60 lignes autour du curseur
        let from = self.cy.saturating_sub(40);
        let mut ctx: Vec<String> = self.lines[from..=self.cy].to_vec();
        if let Some(last) = ctx.last_mut() {
            last.truncate(self.cx);
            last.push_str("▌"); // marqueur de curseur pour l'IA
        }
        let fname = self
            .file
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "code.rs".into());
        let system = system_prompt(self.complete_agent, crate::ai::HelpLevel::Assistance)
            + "\n\nTu es en mode COMPLÉTION (comme GitHub Copilot). On te donne du code avec \
               un curseur « ▌». Réponds UNIQUEMENT avec ce qui vient IMMÉDIATEMENT après ▌ : \
               1 à 3 lignes maximum, la suite logique et minimale. INTERDIT : includes, main(), \
               programme complet, répéter le code d'avant ▌, markdown, explication. Juste le \
               code qui continue. Conventions Rust : indentation 4 espaces, itérateurs plutôt \
               Si la ligne se termine par « { », commence ta réponse par un retour à la ligne.";
        let user = format!("// fichier : {fname}\n{}", ctx.join("\n"));
        let (tx, rx) = channel();
        let client = client.clone();
        std::thread::spawn(move || {
            let _ = tx.send(client.chat_fast(&system, &user).map_err(|e| e.to_string()));
        });
        self.rx = Some(rx);
        self.ai_busy = true;
        self.status = "l'IA réfléchit…".into();
    }

    fn poll_norme(&mut self) {
        let Some(rx) = &self.norme_rx else { return };
        if let Ok(note) = rx.try_recv() {
            if !note.is_empty() {
                self.norme_note = Some(note);
            }
            self.norme_rx = None;
        }
    }

    fn poll_ai(&mut self) {
        let Some(rx) = &self.rx else { return };
        if let Ok(result) = rx.try_recv() {
            self.ai_busy = false;
            self.rx = None;
            match result {
                Ok(text) => {
                    let clean = text.trim_matches(|c| c == '`' || c == '\n');
                    if !clean.is_empty() {
                        self.ghost_lines = clean.lines().map(|l| l.to_string()).collect();
                        self.ghost = Some(clean.to_string());
                        self.status = "suggestion prête — Ctrl+→ ou End : accepter · Échap : refuser".into();
                    }
                }
                Err(e) => self.status = format!("IA : {e}"),
            }
        }
    }

    fn accept_ghost(&mut self) {
        let Some(g) = self.ghost.take() else {
            return;
        };
        let g = g.trim_matches('\n');
        if g.is_empty() {
            return;
        }
        self.snapshot();
        // splice propre : texte avant le curseur + suggestion + texte après,
        // les retours à la ligne de la suggestion deviennent de vraies lignes
        let cur = self.lines[self.cy].clone();
        let before: String = cur.chars().take(self.cx).collect();
        let after: String = cur.chars().skip(self.cx).collect();
        let merged = format!("{before}{g}{after}");
        let new_lines: Vec<String> = merged.split('\n').map(|l| l.to_string()).collect();
        let added = new_lines.len().saturating_sub(1);
        self.lines.splice(self.cy..=self.cy, new_lines);
        // curseur à la fin de la suggestion insérée (avant « after »)
        self.cy += added;
        let after_chars = after.chars().count();
        self.cx = self.lines[self.cy]
            .chars()
            .count()
            .saturating_sub(after_chars);
        self.modified = true;
        self.ghost_lines.clear();
    }

    // -------------------------------------------------------------- boucle

    fn on_key(&mut self, key: KeyEvent) {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let alt = key.modifiers.contains(KeyModifiers::ALT);

        if self.confirm_quit {
            match key.code {
                KeyCode::Char('o') | KeyCode::Char('y') => self.should_quit = true,
                KeyCode::Char('s') => {
                    self.save();
                    self.should_quit = true;
                }
                _ => self.confirm_quit = false,
            }
            return;
        }

        // saisie de recherche (^F) ou goto (^G) en cours
        if self.prompt.is_some() {
            match key.code {
                KeyCode::Esc => self.prompt = None,
                KeyCode::Enter => {
                    let (kind, text) = self.prompt.take().unwrap();
                    if kind == 'f' {
                        self.find(&text);
                    } else if kind == 'r' {
                        self.replace_all(&text);
                    } else if kind == 'g' {
                        if let Ok(n) = text.trim().parse::<usize>() {
                            if n >= 1 && n <= self.lines.len() {
                                self.cy = n - 1;
                                self.cx = 0;
                                self.status = format!("ligne {n}");
                            } else {
                                self.status = format!("ligne {n} hors limites");
                            }
                        }
                    }
                }
                KeyCode::Backspace => {
                    if let Some((_, t)) = &mut self.prompt {
                        t.pop();
                    }
                }
                KeyCode::Char(c) => {
                    if let Some((_, t)) = &mut self.prompt {
                        t.push(c);
                    }
                }
                _ => {}
            }
            return;
        }

        match (key.code, ctrl, alt) {
            // quitter : ^Q ou ^X (réflexe nano) — PAS ^C, trop de missclicks
            (KeyCode::Char('q'), true, _) | (KeyCode::Char('x'), true, _) => {
                if self.modified {
                    self.confirm_quit = true;
                    self.status = "modifié — o quitter sans sauver · s sauver+quitter · autre: rester".into();
                } else {
                    self.should_quit = true;
                }
            }
            (KeyCode::Char('s'), true, _) => self.save(),
            (KeyCode::Char('z'), true, _) => self.undo(),
            (KeyCode::Char('f'), true, _) => {
                self.prompt = Some(('f', String::new()));
                self.status = "chercher :".into();
            }
            (KeyCode::Char('r'), true, _) => {
                // chercher-remplacer : « chercher → remplacer » en une saisie
                self.prompt = Some(('r', String::new()));
                self.status = "remplacer « motif » par « texte » (motif→texte) :".into();
            }
            (KeyCode::Char('g'), true, _) => {
                self.prompt = Some(('g', String::new()));
                self.status = "aller à la ligne :".into();
            }
            (KeyCode::Char('e'), true, _) => {
                // cycler le modèle IA de l'éditeur (persisté) — ^E, pas ^M (= Entrée)
                if let Some(c) = &mut self.ai {
                    let next = c.next_model_name();
                    c.set_model(next);
                    self.ai_model = next.to_string();
                    crate::config::save_nano_model(next);
                    self.status = format!("modèle IA : {next}");
                }
            }
            (KeyCode::Char('a'), true, _) => {
                // cycler l'agent de complétion
                let pos = COMPLETE_AGENTS
                    .iter()
                    .position(|a| *a == self.complete_agent)
                    .unwrap_or(0);
                self.complete_agent = COMPLETE_AGENTS[(pos + 1) % COMPLETE_AGENTS.len()];
                save_complete_agent(self.complete_agent);
                self.status = format!("complétion par : {}", self.complete_agent.name());
            }
            (KeyCode::Char('k'), true, _) => {
                self.snapshot();
                self.clipboard = self.lines.remove(self.cy);
                if self.lines.is_empty() {
                    self.lines.push(String::new());
                }
                self.cy = self.cy.min(self.lines.len() - 1);
                self.cx = self.cx.min(self.line().len());
                self.modified = true;
                self.status = "ligne coupée".into();
            }
            (KeyCode::Char('u'), true, _) => {
                if !self.clipboard.is_empty() {
                    self.snapshot();
                    let clip = self.clipboard.clone();
                    self.lines.insert(self.cy, clip);
                    self.modified = true;
                    self.status = "ligne collée".into();
                }
            }
            (KeyCode::Char(' '), true, _) => {
                if let Some(client) = self.ai.clone() {
                    self.ai_complete(&client);
                } else {
                    self.status = "IA indisponible (clé API manquante)".into();
                }
            }
            (KeyCode::Right, _, true) | (KeyCode::Right, true, _) => {
                if self.ghost.is_some() {
                    self.accept_ghost();
                } else {
                    self.move_right();
                }
            }
            (KeyCode::Esc, _, _) => {
                if self.ghost.is_some() {
                    self.ghost = None;
                    self.ghost_lines.clear();
                }
            }
            (KeyCode::Left, _, _) => self.move_left(),
            (KeyCode::Right, _, _) => self.move_right(),
            (KeyCode::Up, _, _) => self.move_up(),
            (KeyCode::Down, _, _) => self.move_down(),
            (KeyCode::Home, _, _) => self.cx = 0,
            (KeyCode::End, _, _) => {
                if self.ghost.is_some() {
                    self.accept_ghost();
                } else {
                    self.cx = self.line().len();
                }
            }
            (KeyCode::Backspace, _, _) => self.backspace(),
            (KeyCode::Delete, _, _) => {
                if self.cx < self.line().len() {
                    self.lines[self.cy].remove(self.cx);
                    self.modified = true;
                }
            }
            (KeyCode::Enter, _, _) => self.insert_newline(),
            // Tab = 4 espaces, toujours (la convention Rust)
            (KeyCode::Tab, _, _) | (KeyCode::Char('i'), true, _) => {
                for _ in 0..4 {
                    self.insert_char(' ');
                }
            }
            (KeyCode::Char(c), false, false) => self.insert_char(c),
            _ => {}
        }
    }

    fn keep_cursor_visible(&mut self, height: usize, width: usize) {
        if self.cy < self.scroll_y {
            self.scroll_y = self.cy;
        } else if self.cy >= self.scroll_y + height {
            self.scroll_y = self.cy - height + 1;
        }
        let gutter = 6;
        if self.cx < self.scroll_x {
            self.scroll_x = self.cx;
        } else if self.cx >= self.scroll_x + width.saturating_sub(gutter) {
            self.scroll_x = self.cx - width.saturating_sub(gutter) + 1;
        }
    }
}

// ------------------------------------------------------------------ rendu

/// Nom d'agent raccourci pour la barre.
fn short_agent(name: &str) -> &str {
    name.split(' ').next().unwrap_or(name)
}

/// Modèle raccourci (deepseek-v4.1-flash → flash).
fn short_model(model: &str) -> &str {
    model.split('-').next().unwrap_or(model)
}

fn render_line(text: &str, palette_dim: Color, ext: &str) -> Vec<Span<'static>> {
    // coloration par langage (extension du fichier) via syntect
    let hl = highlight::highlight_code(text, ext);
    if let Some(line) = hl.first() {
        line.iter()
            .map(|(style, t)| {
                Span::styled(
                    t.clone(),
                    Style::default().fg(Color::Rgb(
                        style.foreground.r,
                        style.foreground.g,
                        style.foreground.b,
                    )),
                )
            })
            .collect()
    } else {
        vec![Span::styled(text.to_string(), Style::default().fg(palette_dim))]
    }
}

fn draw(frame: &mut Frame, ed: &Editor) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1)])
        .split(area);

    // ── corps : numéros de ligne + texte ──
    let gutter_w = ed.lines.len().to_string().len().max(2) + 2;
    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(gutter_w as u16), Constraint::Min(10)])
        .split(chunks[0]);

    let inner_h = chunks[0].height as usize;
    let gutter_lines: Vec<Line> = (0..inner_h)
        .map(|i| {
            let n = ed.scroll_y + i + 1;
            if n <= ed.lines.len() {
                let is_cur = n == ed.cy + 1;
                Line::from(Span::styled(
                    format!("{:>w$} ", n, w = gutter_w - 1),
                    Style::default().fg(if is_cur { Ed::text() } else { Ed::gutter() }),
                ))
            } else {
                Line::from(" ".repeat(gutter_w))
            }
        })
        .collect();
    frame.render_widget(Paragraph::new(gutter_lines), body[0]);

    let text_lines: Vec<Line> = (0..inner_h)
        .map(|i| {
            let n = ed.scroll_y + i;
            if n < ed.lines.len() {
                let raw = &ed.lines[n];
                let visible: String = raw.chars().skip(ed.scroll_x).collect();
                let mut spans = render_line(&visible, Ed::dim(), &ed.ext);
                // texte fantôme IA sur la ligne du curseur
                if n == ed.cy && !ed.ghost_lines.is_empty() {
                    let first = ed.ghost_lines[0].clone();
                    let cur_cols = raw.chars().count() - ed.scroll_x;
                    spans.push(Span::raw(" ".repeat(0.max(cur_cols.saturating_sub(cur_cols)))));
                    spans.push(Span::styled(
                        first,
                        Style::default().fg(Ed::ghost()).add_modifier(Modifier::ITALIC),
                    ));
                }
                Line::from(spans)
            } else {
                Line::from("")
            }
        })
        .collect();
    frame.render_widget(Paragraph::new(text_lines), body[1]);

    // repère subtil à la colonne 80 (la limite de la norme) — un filet discret
    let ruler_x = body[1].x + 80;
    if ruler_x < body[1].right() {
        let faint = Style::default().fg(Color::Rgb(60, 60, 68));
        for row in body[1].top()..body[1].bottom() {
            let cell = &mut frame.buffer_mut()[(ruler_x, row)];
            // n'écrase pas le code : uniquement sur une case vide
            if cell.symbol().trim().is_empty() {
                cell.set_symbol("·").set_style(faint);
            }
        }
    }

    // ── barre de statut : fine, dim, un accent si modifié ──
    let fname = ed
        .file
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "(sans nom)".into());
    let dirty = if ed.modified { " · modifié" } else { "" };
    let ai = if ed.ai_busy {
        " · IA…"
    } else {
        ""
    };
    let agent_hint = format!(" · {}·{}", short_agent(ed.complete_agent.name()), short_model(&ed.ai_model));
    let norme_hint = ed
        .norme_note
        .as_deref()
        .map(|n| format!(" · {n}"))
        .unwrap_or_default();
    let prompt_hint = ed
        .prompt
        .as_ref()
        .map(|(k, t)| format!("  {}{}▌", if *k == 'f' { "chercher: " } else { "ligne: " }, t))
        .unwrap_or_default();
    // prompt actif : il prend toute la barre (sinon il est rogné sur les petits écrans)
    let mid = if ed.prompt.is_some() {
        prompt_hint.trim_start().to_string()
    } else {
        format!("  {}{}{}{}", ed.status, ai, agent_hint, norme_hint)
    };
    let pos = format!(" {}:{} ", ed.cy + 1, ed.cx + 1);
    let status = Line::from(vec![
        Span::styled(
            format!(" {fname}{dirty}"),
            Style::default().fg(if ed.modified { Ed::accent() } else { Ed::dim() }),
        ),
        Span::styled(
            mid,
            Style::default().fg(Ed::dim()),
        ),
        Span::styled(pos, Style::default().fg(Ed::dim())),
    ]);
    frame.render_widget(Paragraph::new(status), chunks[1]);

    // curseur
    let _ = gutter_w;
    let cur_x = body[1].x + (ed.cx - ed.scroll_x) as u16;
    let cur_y = body[1].y + (ed.cy - ed.scroll_y) as u16;
    frame.set_cursor_position((cur_x.min(body[1].width.saturating_sub(1) + body[1].x), cur_y));
}

/// Lance l'éditeur. `path` : fichier à ouvrir/créer.
pub fn run(path: Option<PathBuf>) -> io::Result<()> {
    let mut ed = Editor::open(path.as_deref())?;
    // l'éditeur utilise nano_model (flash par défaut : la complétion doit être rapide)
    let ai = {
        let cfg = crate::config::load();
        AiClient::from_env(&cfg).ok().map(|mut c| {
            if let Some(m) = &cfg.nano_model {
                c.set_model(m);
            } else {
                c.set_model("deepseek-v4.1-flash");
            }
            c
        })
    };
    if let Some(c) = &ai {
        ed.ai_model = c.model.clone();
    }
    ed.ai = ai;

    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(io::stdout(), crossterm::terminal::EnterAlternateScreen)?;
    let mut terminal = ratatui::DefaultTerminal::new(ratatui::backend::CrosstermBackend::new(
        io::stdout(),
    ))?;
    let result = loop_run(&mut terminal, &mut ed);
    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(io::stdout(), crossterm::terminal::LeaveAlternateScreen)?;
    result
}

fn loop_run(
    terminal: &mut ratatui::DefaultTerminal,
    ed: &mut Editor,
) -> io::Result<()> {
    while !ed.should_quit {
        ed.poll_ai();
        ed.poll_norme();
        ed.keep_cursor_visible(
            terminal.size()?.height.saturating_sub(1) as usize,
            terminal.size()?.width as usize,
        );
        terminal.draw(|frame| draw(frame, ed))?;
        // poll avec timeout : la boucle doit se réveiller pour lire les
        // réponses IA/norme qui arrivent en tâche de fond
        if event::poll(std::time::Duration::from_millis(120))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == crossterm::event::KeyEventKind::Press {
                    ed.on_key(key);
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod robustness_tests {
    use super::*;

    /// L'éditeur ne doit JAMAIS paniquer, quelles que soient les touches.
    #[test]
    fn aucune_panique_sur_touches_limites() {
        let mut ed = Editor::open(None).unwrap();
        // une rafale de touches bizarres
        let cles = [
            KeyCode::Up, KeyCode::Down, KeyCode::Left, KeyCode::Right,
            KeyCode::Home, KeyCode::End, KeyCode::Backspace, KeyCode::Delete,
            KeyCode::Enter, KeyCode::Tab, KeyCode::Char('a'), KeyCode::Char('x'),
        ];
        for _ in 0..3 {
            for c in &cles {
                let key = KeyEvent::new(*c, KeyModifiers::empty());
                let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| ed.on_key(key)));
                assert!(r.is_ok(), "panique sur la touche {c:?}");
            }
        }
    }

    /// Le undo sur un buffer vide / initial ne panique pas.
    #[test]
    fn undo_sur_vide_ne_panique_pas() {
        let mut ed = Editor::open(None).unwrap();
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| ed.undo()));
        assert!(r.is_ok());
        // et le undo restaure vraiment
        ed.on_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty()));
        ed.undo();
        assert_eq!(ed.lines[0], "");
    }

    /// goto/recherche sur des entrées bizarres ne paniquent pas.
    #[test]
    fn goto_et_recherche_limites() {
        let mut ed = Editor::open(None).unwrap();
        ed.lines = vec!["ligne un".into(), "ligne deux".into()];
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            ed.find("");           // vide
            ed.find("inexistant"); // absent
            ed.find("ligne");      // trouvé
        }));
        assert!(r.is_ok());
    }
}

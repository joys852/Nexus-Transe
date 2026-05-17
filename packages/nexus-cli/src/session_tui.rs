//! Interactive session browser (ROADMAP v2 §2.0 — ratatui TUI).

use std::io;

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use nexus_core::models::Session;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState};
use ratatui::Terminal;
use uuid::Uuid;

use crate::fuzzy;
use crate::theme::{self, Theme};

pub struct SessionRow {
    pub id: Uuid,
    pub short_id: String,
    pub title: String,
    pub status: String,
    pub updated: String,
    pub haystack: String,
}

impl SessionRow {
    pub fn from_session(s: &Session) -> Self {
        let short_id: String = s.id.to_string().chars().take(8).collect();
        let title = s.title.clone().unwrap_or_else(|| "—".into());
        let status = format!("{:?}", s.status);
        let updated = s.updated_at.format("%Y-%m-%d %H:%M").to_string();
        let haystack = format!("{short_id} {title} {status} {updated}");
        Self {
            id: s.id,
            short_id,
            title,
            status,
            updated,
            haystack,
        }
    }
}

struct BrowserState {
    rows: Vec<SessionRow>,
    filter: String,
    filtered_indices: Vec<usize>,
    table_state: TableState,
    current_id: Uuid,
}

impl BrowserState {
    fn new(rows: Vec<SessionRow>, current_id: Uuid) -> Self {
        let mut s = Self {
            rows,
            filter: String::new(),
            filtered_indices: Vec::new(),
            table_state: TableState::default(),
            current_id,
        };
        s.refilter();
        s
    }

    fn refilter(&mut self) {
        let mut scored: Vec<(u32, usize)> = self
            .rows
            .iter()
            .enumerate()
            .filter_map(|(i, r)| fuzzy::score(&r.haystack, &self.filter).map(|s| (s, i)))
            .collect();
        scored.sort_by_key(|(s, _)| *s);
        self.filtered_indices = scored.into_iter().map(|(_, i)| i).collect();
        if self.filtered_indices.is_empty() {
            self.table_state.select(None);
        } else if self.table_state.selected().is_none() {
            self.table_state.select(Some(0));
        } else if let Some(sel) = self.table_state.selected() {
            self.table_state
                .select(Some(sel.min(self.filtered_indices.len().saturating_sub(1))));
        }
    }

    fn selected_id(&self) -> Option<Uuid> {
        let sel = self.table_state.selected()?;
        let idx = *self.filtered_indices.get(sel)?;
        Some(self.rows[idx].id)
    }

    fn move_sel(&mut self, delta: isize) {
        if self.filtered_indices.is_empty() {
            return;
        }
        let len = self.filtered_indices.len();
        let cur = self.table_state.selected().unwrap_or(0);
        let next = (cur as isize + delta).rem_euclid(len as isize) as usize;
        self.table_state.select(Some(next));
    }
}

/// Run fullscreen session picker. Returns chosen session id or `None` if cancelled.
pub fn run_session_browser(
    sessions: Vec<SessionRow>,
    current_id: Uuid,
) -> anyhow::Result<Option<Uuid>> {
    if sessions.is_empty() {
        return Ok(None);
    }

    let theme = theme::active();
    let mut stdout = io::stdout();
    enable_raw_mode()?;
    stdout.execute(EnterAlternateScreen)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let mut state = BrowserState::new(sessions, current_id);
    let mut result = None;

    loop {
        terminal.draw(|f| draw_ui(f, &mut state, &theme))?;

        if event::poll(std::time::Duration::from_millis(120))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match key.code {
                    KeyCode::Esc => break,
                    KeyCode::Enter => {
                        result = state.selected_id();
                        break;
                    }
                    KeyCode::Up | KeyCode::Char('k') => state.move_sel(-1),
                    KeyCode::Down | KeyCode::Char('j') => state.move_sel(1),
                    KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => break,
                    KeyCode::Backspace => {
                        state.filter.pop();
                        state.refilter();
                    }
                    KeyCode::Char(c) => {
                        state.filter.push(c);
                        state.refilter();
                    }
                    _ => {}
                }
            }
        }
    }

    disable_raw_mode()?;
    terminal.backend_mut().execute(LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(result)
}

fn draw_ui(f: &mut ratatui::Frame, state: &mut BrowserState, theme: &Theme) {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(6),
            Constraint::Length(3),
        ])
        .split(area);

    let title = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.ratatui_accent()))
        .title(Span::styled(
            " NexusIDE · Sessions ",
            Style::default()
                .fg(theme.ratatui_accent())
                .add_modifier(Modifier::BOLD),
        ));
    f.render_widget(title, chunks[0]);

    let header = Row::new(vec!["ID", "Title", "Status", "Updated"]).style(
        Style::default()
            .fg(theme.ratatui_accent())
            .add_modifier(Modifier::BOLD),
    );
    let rows: Vec<Row> = state
        .filtered_indices
        .iter()
        .map(|&i| {
            let r = &state.rows[i];
            let mark = if r.id == state.current_id { "● " } else { "  " };
            Row::new(vec![
                Cell::from(format!("{mark}{}", r.short_id)),
                Cell::from(r.title.clone()),
                Cell::from(r.status.clone()),
                Cell::from(r.updated.clone()),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(12),
            Constraint::Min(20),
            Constraint::Length(12),
            Constraint::Length(16),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::LEFT | Borders::RIGHT)
            .border_style(Style::default().fg(theme.ratatui_muted())),
    )
    .highlight_style(
        Style::default()
            .bg(theme.ratatui_bg())
            .fg(theme.ratatui_fg())
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol("▸ ");

    f.render_stateful_widget(table, chunks[1], &mut state.table_state);

    let filter_label = if state.filter.is_empty() {
        "Type to filter…".to_string()
    } else {
        format!("/{}", state.filter)
    };
    let help = Line::from(vec![
        Span::styled(
            " ↑↓ ",
            Style::default().fg(theme.ratatui_accent()),
        ),
        Span::raw("move  "),
        Span::styled("Enter", Style::default().fg(theme.ratatui_accent())),
        Span::raw(" open  "),
        Span::styled("Esc", Style::default().fg(theme.ratatui_accent())),
        Span::raw(" close  "),
        Span::styled(
            format!("  {} matches", state.filtered_indices.len()),
            Style::default().fg(theme.ratatui_muted()),
        ),
    ]);
    let footer = Paragraph::new(vec![
        Line::from(Span::styled(filter_label, Style::default().fg(theme.ratatui_fg()))),
        help,
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.ratatui_muted())),
    );
    f.render_widget(footer, chunks[2]);
}

#[allow(dead_code)]
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

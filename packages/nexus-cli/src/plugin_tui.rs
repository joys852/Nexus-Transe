//! Plugin marketplace browser TUI (ROADMAP v2 §2.0).

use std::io;

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use nexus_core::plugins::{LoadedPlugin, PluginManager};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState};
use ratatui::Terminal;
use serde::Deserialize;

use crate::fuzzy;
use crate::theme::{self, Theme};

#[derive(Debug, Clone, Deserialize)]
pub struct MarketplaceEntry {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: Option<String>,
    #[serde(default)]
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MarketplaceCatalog {
    pub plugins: Vec<MarketplaceEntry>,
}

pub fn load_catalog() -> MarketplaceCatalog {
    const EMBED: &str = include_str!("../assets/plugins-marketplace.json");
    serde_json::from_str(EMBED).unwrap_or(MarketplaceCatalog {
        plugins: Vec::new(),
    })
}

pub struct PluginRow {
    pub id: String,
    pub name: String,
    pub version: String,
    pub status: String,
    pub description: String,
    pub haystack: String,
    pub installed: bool,
    pub install_path: Option<String>,
}

impl PluginRow {
    pub fn from_loaded(p: &LoadedPlugin) -> Self {
        let id = p.manifest.id.clone();
        let name = p.manifest.name.clone();
        let version = p.manifest.version.clone();
        let description = p.manifest.description.clone();
        let haystack = format!("{id} {name} {description} installed");
        Self {
            id,
            name,
            version,
            status: "installed".into(),
            description,
            haystack,
            installed: true,
            install_path: Some(p.root.display().to_string()),
        }
    }

    pub fn from_marketplace(m: &MarketplaceEntry, installed_ids: &[String]) -> Self {
        let installed = installed_ids.iter().any(|i| i == &m.id);
        let status = if installed {
            "installed"
        } else {
            "available"
        };
        let haystack = format!(
            "{} {} {} {} {}",
            m.id, m.name, m.description, status, m.author.as_deref().unwrap_or("")
        );
        Self {
            id: m.id.clone(),
            name: m.name.clone(),
            version: m.version.clone(),
            status: status.into(),
            description: m.description.clone(),
            haystack,
            installed,
            install_path: None,
        }
    }
}

pub fn build_rows(manager: &PluginManager, catalog: &MarketplaceCatalog) -> Vec<PluginRow> {
    let installed_ids: Vec<String> = manager
        .list()
        .iter()
        .map(|p| p.manifest.id.clone())
        .collect();
    let mut rows: Vec<PluginRow> = manager
        .list()
        .iter()
        .map(PluginRow::from_loaded)
        .collect();
    for entry in &catalog.plugins {
        if !installed_ids.contains(&entry.id) {
            rows.push(PluginRow::from_marketplace(entry, &installed_ids));
        }
    }
    rows.sort_by(|a, b| a.name.cmp(&b.name));
    rows
}

struct BrowserState {
    rows: Vec<PluginRow>,
    filter: String,
    filtered_indices: Vec<usize>,
    table_state: TableState,
    detail: String,
}

impl BrowserState {
    fn new(rows: Vec<PluginRow>) -> Self {
        let mut s = Self {
            rows,
            filter: String::new(),
            filtered_indices: Vec::new(),
            table_state: TableState::default(),
            detail: String::new(),
        };
        s.refilter();
        s.update_detail();
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

    fn selected_row(&self) -> Option<&PluginRow> {
        let sel = self.table_state.selected()?;
        let idx = *self.filtered_indices.get(sel)?;
        Some(&self.rows[idx])
    }

    fn update_detail(&mut self) {
        self.detail = match self.selected_row() {
            None => String::new(),
            Some(r) => {
                if let Some(path) = &r.install_path {
                    format!("{}\n\nPath: {path}", r.description)
                } else {
                    format!(
                        "{}\n\nInstall: /skills install {}  (bundled skill)\nOr add plugin.toml under %APPDATA%\\nexus-ide\\plugins\\{}/",
                        r.description, r.id, r.id
                    )
                }
            }
        };
    }

    fn move_sel(&mut self, delta: isize) {
        if self.filtered_indices.is_empty() {
            return;
        }
        let len = self.filtered_indices.len();
        let cur = self.table_state.selected().unwrap_or(0);
        let next = (cur as isize + delta).rem_euclid(len as isize) as usize;
        self.table_state.select(Some(next));
        self.update_detail();
    }
}

pub fn run_plugin_browser(rows: Vec<PluginRow>) -> anyhow::Result<Option<String>> {
    if rows.is_empty() {
        return Ok(None);
    }
    let theme = theme::active();
    let mut stdout = io::stdout();
    enable_raw_mode()?;
    stdout.execute(EnterAlternateScreen)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let mut state = BrowserState::new(rows);

    let mut install_id: Option<String> = None;

    loop {
        terminal.draw(|f| draw_ui(f, &mut state, &theme))?;
        if event::poll(std::time::Duration::from_millis(120))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match key.code {
                    KeyCode::Esc => break,
                    KeyCode::Char('i') => {
                        if let Some(r) = state.selected_row() {
                            if !r.installed {
                                install_id = Some(r.id.clone());
                                break;
                            }
                        }
                    }
                    KeyCode::Up | KeyCode::Char('k') => state.move_sel(-1),
                    KeyCode::Down | KeyCode::Char('j') => state.move_sel(1),
                    KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => break,
                    KeyCode::Backspace => {
                        state.filter.pop();
                        state.refilter();
                        state.update_detail();
                    }
                    KeyCode::Char(c) => {
                        state.filter.push(c);
                        state.refilter();
                        state.update_detail();
                    }
                    _ => {}
                }
            }
        }
    }

    disable_raw_mode()?;
    terminal.backend_mut().execute(LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(install_id)
}

fn draw_ui(f: &mut ratatui::Frame, state: &mut BrowserState, theme: &Theme) {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(5),
            Constraint::Length(3),
        ])
        .split(area);

    let title = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.ratatui_accent()))
        .title(Span::styled(
            " NexusIDE · Plugin Market ",
            Style::default()
                .fg(theme.ratatui_accent())
                .add_modifier(Modifier::BOLD),
        ));
    f.render_widget(title, chunks[0]);

    let header = Row::new(vec!["Name", "Version", "Status"]).style(
        Style::default()
            .fg(theme.ratatui_accent())
            .add_modifier(Modifier::BOLD),
    );
    let table_rows: Vec<Row> = state
        .filtered_indices
        .iter()
        .map(|&i| {
            let r = &state.rows[i];
            Row::new(vec![
                Cell::from(r.name.clone()),
                Cell::from(r.version.clone()),
                Cell::from(r.status.clone()),
            ])
        })
        .collect();

    let table = Table::new(
        table_rows,
        [
            Constraint::Min(24),
            Constraint::Length(10),
            Constraint::Length(12),
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

    let detail = Paragraph::new(state.detail.as_str()).block(
        Block::default()
            .title(" Details ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.ratatui_muted())),
    );
    f.render_widget(detail, chunks[2]);

    let filter_label = if state.filter.is_empty() {
        "Type to filter…".to_string()
    } else {
        format!("/{}", state.filter)
    };
    let footer = Paragraph::new(vec![
        Line::from(Span::styled(filter_label, Style::default().fg(theme.ratatui_fg()))),
        Line::from(vec![
            Span::styled(" ↑↓ ", Style::default().fg(theme.ratatui_accent())),
            Span::raw("navigate  "),
            Span::styled("Esc", Style::default().fg(theme.ratatui_accent())),
            Span::raw(" close  "),
            Span::styled("i", Style::default().fg(theme.ratatui_accent())),
            Span::raw(" install"),
        ]),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.ratatui_muted())),
    );
    f.render_widget(footer, chunks[3]);
}

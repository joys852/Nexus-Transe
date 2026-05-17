//! Terminal color themes (ROADMAP v2 — light / dark / carbon).

use std::sync::RwLock;

use colored::Colorize;
use nexus_core::config::NexusConfig;

static ACTIVE: RwLock<Option<Theme>> = RwLock::new(None);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeId {
    Light,
    Dark,
    Carbon,
}

#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub id: ThemeId,
    pub accent: (u8, u8, u8),
    pub accent_soft: (u8, u8, u8),
    pub success: (u8, u8, u8),
    pub error: (u8, u8, u8),
    pub muted: (u8, u8, u8),
    pub border: (u8, u8, u8),
    pub fg: (u8, u8, u8),
    pub bg: (u8, u8, u8),
}

impl Theme {
    pub fn get(id: ThemeId) -> Self {
        match id {
            ThemeId::Light => Theme {
                id: ThemeId::Light,
                accent: (180, 90, 40),
                accent_soft: (140, 100, 80),
                success: (40, 120, 60),
                error: (180, 50, 50),
                muted: (110, 110, 110),
                border: (180, 90, 40),
                fg: (30, 30, 30),
                bg: (250, 250, 248),
            },
            ThemeId::Dark => Theme {
                id: ThemeId::Dark,
                accent: (0, 122, 204),       // VS Code blue
                accent_soft: (198, 120, 73), // keyword / tool orange
                success: (106, 153, 85),     // #6a9955
                error: (244, 71, 71),
                muted: (133, 133, 133),
                border: (62, 62, 66),        // #3e3e42
                fg: (212, 212, 212),
                bg: (30, 30, 30),            // #1e1e1e
            },
            ThemeId::Carbon => Theme {
                id: ThemeId::Carbon,
                accent: (0, 188, 212),
                accent_soft: (100, 160, 170),
                success: (0, 200, 120),
                error: (255, 90, 90),
                muted: (130, 140, 150),
                border: (0, 188, 212),
                fg: (230, 235, 240),
                bg: (16, 20, 24),
            },
        }
    }

    pub fn ratatui_accent(&self) -> ratatui::style::Color {
        ratatui::style::Color::Rgb(self.accent.0, self.accent.1, self.accent.2)
    }

    pub fn ratatui_fg(&self) -> ratatui::style::Color {
        ratatui::style::Color::Rgb(self.fg.0, self.fg.1, self.fg.2)
    }

    pub fn ratatui_muted(&self) -> ratatui::style::Color {
        ratatui::style::Color::Rgb(self.muted.0, self.muted.1, self.muted.2)
    }

    pub fn ratatui_bg(&self) -> ratatui::style::Color {
        ratatui::style::Color::Rgb(self.bg.0, self.bg.1, self.bg.2)
    }
}

pub fn parse_theme(s: &str) -> ThemeId {
    match s.to_lowercase().as_str() {
        "light" => ThemeId::Light,
        "carbon" => ThemeId::Carbon,
        _ => ThemeId::Dark,
    }
}

pub fn init_from_config(config: &NexusConfig) {
    let id = parse_theme(&config.theme);
    if let Ok(mut g) = ACTIVE.write() {
        *g = Some(Theme::get(id));
    }
}

pub fn active() -> Theme {
    ACTIVE
        .read()
        .ok()
        .and_then(|g| *g)
        .unwrap_or_else(|| Theme::get(ThemeId::Dark))
}

pub fn set_theme(id: ThemeId) {
    if let Ok(mut g) = ACTIVE.write() {
        *g = Some(Theme::get(id));
    }
}

pub fn accent_text(s: &str) -> String {
    let t = active();
    s.truecolor(t.accent.0, t.accent.1, t.accent.2).to_string()
}

pub fn accent_soft_text(s: &str) -> String {
    let t = active();
    s.truecolor(t.accent_soft.0, t.accent_soft.1, t.accent_soft.2)
        .to_string()
}

pub fn muted_text(s: &str) -> String {
    let t = active();
    s.truecolor(t.muted.0, t.muted.1, t.muted.2).to_string()
}

pub fn border_text(s: &str) -> String {
    let t = active();
    s.truecolor(t.border.0, t.border.1, t.border.2).to_string()
}

pub fn success_text(s: &str) -> String {
    let t = active();
    s.truecolor(t.success.0, t.success.1, t.success.2).to_string()
}

pub fn tool_name_text(s: &str) -> String {
    accent_soft_text(s)
}

pub fn label_text(s: &str) -> String {
    accent_text(s)
}

pub fn apply_theme(config: &mut NexusConfig, id: ThemeId) -> anyhow::Result<()> {
    config.theme = match id {
        ThemeId::Light => "light",
        ThemeId::Dark => "dark",
        ThemeId::Carbon => "carbon",
    }
    .to_string();
    config.validate()?;
    config.save_to_data_dir()?;
    set_theme(id);
    Ok(())
}

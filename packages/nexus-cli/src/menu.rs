//! Arrow-key selection menus for confirmations.

use std::io::{self, Write};

use colored::Colorize;
use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType};
use crossterm::QueueableCommand;

const ACCENT: (u8, u8, u8) = (86, 156, 214);

pub struct SelectOption {
    pub label: String,
}

struct Drawer {
    line_count: usize,
    clear_screen: bool,
}

/// Interactive numbered menu. Returns selected index. Esc = last option (usually "No").
pub fn select(
    header: &str,
    body: &str,
    options: &[SelectOption],
    footer: &str,
    clear_screen: bool,
) -> io::Result<usize> {
    if options.is_empty() {
        return Ok(0);
    }

    let mut selected = 0usize;
    let mut drawer = Drawer {
        line_count: 0,
        clear_screen,
    };

    enable_raw_mode()?;
    let result = (|| -> io::Result<usize> {
        loop {
            drawer.redraw(header, body, options, footer, selected)?;
            let ev = event::read()?;
            if let Event::Key(key) = ev {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match key.code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        selected = selected.saturating_sub(1);
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        selected = (selected + 1).min(options.len() - 1);
                    }
                    KeyCode::Char(c @ '1'..='9') => {
                        let digit = (c as u8 - b'1') as usize;
                        if digit < options.len() {
                            selected = digit;
                        }
                    }
                    KeyCode::Enter => break,
                    KeyCode::Esc => {
                        selected = options.len().saturating_sub(1);
                        break;
                    }
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        selected = options.len().saturating_sub(1);
                        break;
                    }
                    _ => {}
                }
            }
        }
        Ok(selected)
    })();

    restore_terminal(drawer.line_count, drawer.clear_screen)?;
    result
}

impl Drawer {
    fn redraw(
        &mut self,
        header: &str,
        body: &str,
        options: &[SelectOption],
        footer: &str,
        selected: usize,
    ) -> io::Result<()> {
        let mut stdout = io::stdout();

        if self.clear_screen {
            stdout.queue(Clear(ClearType::All))?;
            stdout.queue(MoveTo(0, 0))?;
        } else if self.line_count > 0 {
            stdout.queue(crossterm::cursor::MoveUp(self.line_count as u16))?;
            stdout.queue(crossterm::cursor::MoveToColumn(0))?;
        }

        stdout.queue(Hide)?;

        let mut lines: Vec<String> = Vec::new();
        lines.push(String::new());
        lines.push(format!(
            "  {} {}",
            ">".white().bold(),
            header.white().bold()
        ));
        lines.push(String::new());
        for paragraph in body.lines() {
            if paragraph.trim().is_empty() {
                lines.push(String::new());
            } else {
                lines.push(format!("  {}", paragraph.dimmed()));
            }
        }
        lines.push(String::new());

        for (i, opt) in options.iter().enumerate() {
            let n = i + 1;
            if i == selected {
                lines.push(format!(
                    "  {} {}. {}",
                    ">".truecolor(ACCENT.0, ACCENT.1, ACCENT.2).bold(),
                    n,
                    opt.label.truecolor(ACCENT.0, ACCENT.1, ACCENT.2)
                ));
            } else {
                lines.push(format!("    {}. {}", n, opt.label));
            }
        }

        lines.push(String::new());
        lines.push(format!("  {}", footer.dimmed()));
        lines.push(String::new());

        self.line_count = lines.len();
        for line in &lines {
            writeln!(stdout, "{line}")?;
        }
        stdout.flush()?;
        Ok(())
    }
}

fn restore_terminal(menu_lines: usize, clear_screen: bool) -> io::Result<()> {
    let mut stdout = io::stdout();
    disable_raw_mode()?;
    if clear_screen {
        stdout.queue(Clear(ClearType::All))?;
        stdout.queue(MoveTo(0, 0))?;
    }
    stdout.queue(Show)?;
    if !clear_screen && menu_lines > 0 {
        writeln!(stdout)?;
    }
    stdout.flush()?;
    Ok(())
}

/// Tool approval: arrow-key menu (not [y/N] typing).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolApprovalChoice {
    RunOnce,
    RunSession,
    Deny,
}

pub fn confirm_tool_run(tool: &str, detail: &str) -> io::Result<ToolApprovalChoice> {
    let header = format!("Approve {tool}");
    let body = format!(
        "Allow this tool to run in your workspace?\n\n{detail}\n\n\
         Choose \"approve all this session\" to skip future prompts until you exit."
    );
    let idx = select(
        &header,
        &body,
        &[
            SelectOption {
                label: format!("Yes, run {tool}"),
            },
            SelectOption {
                label: "Yes, approve all tools this session".into(),
            },
            SelectOption {
                label: "No, skip this tool".into(),
            },
        ],
        "Enter to confirm · ↑↓ to move · Esc = skip",
        false,
    )?;
    Ok(match idx {
        0 => ToolApprovalChoice::RunOnce,
        1 => ToolApprovalChoice::RunSession,
        _ => ToolApprovalChoice::Deny,
    })
}

/// Yes / No menu; returns true when the first option is chosen.
pub fn confirm_yes_no(header: &str, body: &str, clear_screen: bool) -> io::Result<bool> {
    let idx = select(
        header,
        body,
        &[
            SelectOption {
                label: "Yes, continue".into(),
            },
            SelectOption {
                label: "No, quit".into(),
            },
        ],
        "Enter to confirm · ↑↓ to move · Esc = No",
        clear_screen,
    )?;
    Ok(idx == 0)
}

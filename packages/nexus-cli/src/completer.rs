use crate::at_resolve;
use crate::mode::ChatMode;
use crate::slash;
use crate::ui;
use rustyline::completion::{Completer, Pair};
use std::path::PathBuf;
use rustyline::highlight::{Highlighter, MatchingBracketHighlighter};
use rustyline::hint::{Hinter, HistoryHinter};
use rustyline::validate::MatchingBracketValidator;
use rustyline::validate::Validator;
use rustyline::Context;
use rustyline::Helper;

#[derive(Helper)]
pub struct NexusHelper {
    mode: ChatMode,
    project_root: Option<PathBuf>,
    completer: SlashCompleter,
    #[allow(dead_code)]
    highlighter: MatchingBracketHighlighter,
    hinter: HistoryHinter,
    #[allow(dead_code)]
    validator: MatchingBracketValidator,
}

struct SlashCompleter {
    candidates: Vec<String>,
}

impl NexusHelper {
    pub fn new() -> Self {
        Self {
            mode: ChatMode::Default,
            project_root: None,
            completer: SlashCompleter {
                candidates: slash::all_completion_candidates(),
            },
            highlighter: MatchingBracketHighlighter::new(),
            hinter: HistoryHinter::new(),
            validator: MatchingBracketValidator::new(),
        }
    }

    pub fn set_mode(&mut self, mode: ChatMode) {
        self.mode = mode;
    }

    pub fn set_project_root(&mut self, root: PathBuf) {
        self.project_root = Some(root);
    }
}

impl Completer for NexusHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        if let Some(root) = &self.project_root {
            if let Some((start, pairs)) = complete_at_files(line, pos, root) {
                if !pairs.is_empty() {
                    return Ok((start, pairs));
                }
            }
        }
        self.completer.complete(line, pos, ctx)
    }
}

fn complete_at_files(line: &str, pos: usize, root: &std::path::Path) -> Option<(usize, Vec<Pair>)> {
    let before = &line[..pos];
    let at = before.rfind('@')?;
    if at > 0 {
        let prev = before.as_bytes().get(at - 1)?;
        if !prev.is_ascii_whitespace() {
            return None;
        }
    }
    let query = &line[at + 1..pos];
    if query.contains(char::is_whitespace) {
        return None;
    }
    let paths = at_resolve::list_candidates(query, root, 15);
    let pairs: Vec<Pair> = paths
        .iter()
        .map(|p| {
            let replacement = if query.is_empty() {
                p.clone()
            } else if p.starts_with(query) {
                p[query.len()..].to_string()
            } else {
                p.clone()
            };
            Pair {
                display: p.clone(),
                replacement,
            }
        })
        .collect();
    Some((at + 1, pairs))
}

impl Completer for SlashCompleter {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let before = &line[..pos];
        if !before.starts_with('/') {
            return Ok((pos, vec![]));
        }
        // Only complete the slash-command token (no spaces).
        if before.contains(' ') {
            return Ok((pos, vec![]));
        }
        let start = before.rfind('/').unwrap_or(0);
        let word = &line[start..pos];
        let mut matches: Vec<Pair> = self
            .candidates
            .iter()
            .filter(|c| c.starts_with(word))
            .map(|c| Pair {
                display: c.clone(),
                replacement: c.clone(),
            })
            .collect();
        matches.sort_by(|a, b| a.display.cmp(&b.display));
        Ok((start, matches))
    }
}

impl Hinter for NexusHelper {
    type Hint = String;

    fn hint(&self, line: &str, pos: usize, ctx: &Context<'_>) -> Option<String> {
        if pos != line.len() {
            return None;
        }
        if !line.starts_with('/') || line.contains(' ') {
            return self.hinter.hint(line, pos, ctx);
        }
        let matches: Vec<_> = self
            .completer
            .candidates
            .iter()
            .filter(|c| c.starts_with(line))
            .collect();
        if matches.len() == 1 {
            let suffix = &matches[0][line.len()..];
            if suffix.is_empty() {
                None
            } else {
                Some(suffix.to_string())
            }
        } else if matches.len() > 1 {
            let mut lcp = matches[0].as_str();
            for m in &matches[1..] {
                while !m.starts_with(lcp) && !lcp.is_empty() {
                    lcp = &lcp[..lcp.len() - 1];
                }
            }
            if lcp.len() > line.len() {
                Some(lcp[line.len()..].to_string())
            } else {
                None
            }
        } else {
            None
        }
    }
}

impl Highlighter for NexusHelper {
    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
        &'s self,
        prompt: &'p str,
        default: bool,
    ) -> std::borrow::Cow<'b, str> {
        if default {
            return std::borrow::Cow::Borrowed(prompt);
        }
        let (r, g, b) = ui::prompt_color(self.mode);
        std::borrow::Cow::Owned(format!("\x1b[38;2;{r};{g};{b}m{prompt}\x1b[0m"))
    }

    fn highlight_hint<'h>(&self, hint: &'h str) -> std::borrow::Cow<'h, str> {
        std::borrow::Cow::Owned(ui::hint_style(hint))
    }
}

impl Validator for NexusHelper {}

/// Show available slash commands (when user types `/` alone).
pub fn print_slash_menu(filter: &str) {
    let filter = filter.trim();
    let rows: Vec<_> = slash::COMMANDS
        .iter()
        .filter(|c| {
            if filter.is_empty() || filter == "/" {
                return true;
            }
            let full = if c.args.is_empty() {
                c.name.to_string()
            } else {
                format!("{}{}", c.name, c.args.split_whitespace().next().unwrap_or(""))
            };
            full.starts_with(filter)
        })
        .map(|c| {
            let cmd = if c.args.is_empty() {
                c.name.to_string()
            } else {
                format!("{}{}", c.name, c.args)
            };
            (cmd, c.summary.to_string())
        })
        .collect();
    ui::print_slash_completions(&rows);
}

//! Colored grouped output for `nexus search`.

use colored::Colorize;
use nexus_core::search::SearchMatch;

pub fn print_matches(matches: &[SearchMatch]) {
    print_matches_highlighted(matches, None);
}

pub fn print_matches_highlighted(matches: &[SearchMatch], pattern: Option<&str>) {
    let mut last_path = String::new();
    for m in matches {
        if m.path != last_path {
            if !last_path.is_empty() {
                println!();
            }
            println!("{}", m.path.truecolor(198, 120, 73));
            last_path = m.path.clone();
        }
        println!(
            "  {} {}",
            format!("{:>5}", m.line).dimmed(),
            highlight_match(&m.text, pattern)
        );
    }
}

fn highlight_match(text: &str, pattern: Option<&str>) -> String {
    let Some(pat) = pattern else {
        return text.to_string();
    };
    if pat.is_empty() {
        return text.to_string();
    }
    let lower = text.to_lowercase();
    let pl = pat.to_lowercase();
    if let Some(pos) = lower.find(&pl) {
        let end = pos + pl.len();
        format!(
            "{}{}{}",
            &text[..pos],
            text[pos..end].truecolor(255, 214, 120),
            &text[end..]
        )
    } else {
        text.to_string()
    }
}

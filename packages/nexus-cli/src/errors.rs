//! Formatted error boxes (ROADMAP v2 §2.1).

use colored::Colorize;
use nexus_core::error_present::ErrorPresentation;

const W: usize = 60;

pub fn print_error_box(p: &ErrorPresentation) {
    println!();
    let title = format!(" Error [{}] ", p.code);
    print_border_top(&title);
    println!("{}", row(&p.title));
    if !p.context.is_empty() {
        println!("{}", row(""));
        println!("{}", row(&format!("{}", "Context:".dimmed())));
        for (i, c) in p.context.iter().enumerate() {
            println!("{}", row(&format!("  {}. {}", i + 1, c.dimmed())));
        }
    }
    if let Some(s) = p.suggestion {
        println!("{}", row(""));
        println!(
            "{}",
            row(&format!("{}", crate::theme::label_text("Suggestion:")))
        );
        for line in s.lines() {
            println!("{}", row(&format!("  • {line}")));
        }
    }
    if let Some(url) = p.docs_url() {
        println!("{}", row(""));
        println!("{}", row(&format!("Docs: {}", url.dimmed())));
    }
    print_border_bottom();
    println!();
}

pub fn print_anyhow(err: &anyhow::Error) {
    let p = nexus_core::error_present::present_anyhow(err);
    print_error_box(&p);
}

fn row(text: &str) -> String {
    format!("│ {text}")
}

fn print_border_top(title: &str) {
    let inner = W.saturating_sub(2);
    let pad = inner.saturating_sub(title.len());
    let left = pad / 2;
    let right = pad - left;
    println!(
        "┌{}{}{}",
        "─".repeat(left).truecolor(198, 120, 73),
        title.truecolor(198, 120, 73),
        "─".repeat(right).truecolor(198, 120, 73)
    );
}

fn print_border_bottom() {
    println!("└{}", "─".repeat(W).truecolor(198, 120, 73));
}

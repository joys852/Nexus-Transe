//! Nexus-Transe boot logo (Cybertron terminal aesthetic).

use std::io::{self, Write};
use std::thread;
use std::time::Duration;

use crate::mode::ChatMode;

fn anim_enabled() -> bool {
    std::env::var("NEXUS_NO_ANIMATION").is_err() && std::env::var("CI").is_err()
}

fn type_print(text: &str, delay_ms: u64) {
    if delay_ms == 0 || !anim_enabled() {
        print!("{text}");
        let _ = io::stdout().flush();
        return;
    }
    for c in text.chars() {
        print!("{c}");
        let _ = io::stdout().flush();
        thread::sleep(Duration::from_millis(delay_ms));
    }
}

/// Typewriter Nexus-Transe banner, then status strip.
pub fn print_nexus_transformers_logo(version: &str, model: &str, mode: ChatMode, online: bool) {
    let logo_lines: [(&str, u64); 23] = [
        ("\x1b[1;38;5;39m", 0),
        ("  ╔═══════════════════════════════════════════════════════════╗\n", 4),
        ("  ║                                                           ║\n", 4),
        ("  ║   ███╗   ██╗███████╗██╗  ██╗██╗   ██╗███████╗            ║\n", 2),
        ("  ║   ████╗  ██║██╔════╝╚██╗██╔╝██║   ██║██╔════╝            ║\n", 2),
        ("  ║   ██╔██╗ ██║█████╗   ╚███╔╝ ██║   ██║███████╗            ║\n", 2),
        ("  ║   ██║╚██╗██║██╔══╝   ██╔██╗ ██║   ██║╚════██║            ║\n", 2),
        ("  ║   ██║ ╚████║███████╗██╔╝ ██╗╚██████╔╝███████║            ║\n", 2),
        ("  ║   ╚═╝  ╚═══╝╚══════╝╚═╝  ╚═╝ ╚═════╝ ╚══════╝            ║\n", 2),
        ("  ║                                                           ║\n", 3),
        ("  ║   ████████╗██████╗  █████╗ ███╗   ██╗███████╗███████╗     ║\n", 2),
        ("  ║   ╚══██╔══╝██╔══██╗██╔══██╗████╗  ██║██╔════╝██╔════╝     ║\n", 2),
        ("  ║      ██║   ██████╔╝███████║██╔██╗ ██║███████╗█████╗       ║\n", 2),
        ("  ║      ██║   ██╔══██╗██╔══██║██║╚██╗██║╚════██║██╔══╝       ║\n", 2),
        ("  ║      ██║   ██║  ██║██║  ██║██║ ╚████║███████║███████╗     ║\n", 2),
        ("  ║      ╚═╝   ╚═╝  ╚═╝╚═╝  ╚═╝╚═╝  ╚═══╝╚══════╝╚══════╝     ║\n", 2),
        ("  ║                                                           ║\n", 4),
        ("  ╚═══════════════════════════════════════════════════════════╝\n", 4),
        ("\x1b[1;38;5;208m", 0),
        ("  ╔═══════════════════════════════════════════════════════════╗\n", 4),
        ("  ║  CYBERTRON NEXUS COMMAND INTERFACE                        ║\n", 6),
        ("  ╚═══════════════════════════════════════════════════════════╝\n", 4),
        ("\x1b[0m\n", 0),
    ];

    for (line, delay) in logo_lines {
        type_print(line, delay);
    }

    if anim_enabled() {
        thread::sleep(Duration::from_millis(200));
    }

    let status = if online { "ONLINE" } else { "OFFLINE" };
    let mode_label = mode.label().to_uppercase();
    println!(
        "\x1b[38;5;245m  Nexus-Transe v{version}  ·  MODEL: \x1b[1;38;5;39m{model}\x1b[0m\x1b[38;5;245m  ·  STATUS: \x1b[1;38;5;208m{status}\x1b[0m\x1b[38;5;245m  ·  MODE: {mode_label}\x1b[0m"
    );
    println!();
}

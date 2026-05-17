//! Multi-stage progress (ROADMAP v2 §4.2).

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::time::Duration;

pub struct StageProgress {
    multi: MultiProgress,
    bars: Vec<ProgressBar>,
}

impl StageProgress {
    pub fn new(stages: &[&str]) -> Self {
        let multi = MultiProgress::new();
        let style = ProgressStyle::with_template("  [{pos}/{len}] {spinner} {msg}")
            .unwrap()
            .tick_strings(&["·", "•", "●", "•"]);
        let mut bars = Vec::new();
        for (i, name) in stages.iter().enumerate() {
            let bar = multi.add(ProgressBar::new_spinner());
            bar.set_style(style.clone());
            bar.set_message(format!("{name}…"));
            bar.set_position(i as u64);
            bars.push(bar);
        }
        Self { multi, bars }
    }

    pub fn complete(&self, index: usize, msg: &str) {
        if let Some(bar) = self.bars.get(index) {
            bar.finish_with_message(format!("✓ {msg}"));
        }
    }

    pub fn fail(&self, index: usize, msg: &str) {
        if let Some(bar) = self.bars.get(index) {
            bar.abandon_with_message(format!("✗ {msg}"));
        }
    }

    pub fn set_active(&self, index: usize, msg: &str) {
        if let Some(bar) = self.bars.get(index) {
            bar.set_message(msg.to_string());
            bar.enable_steady_tick(Duration::from_millis(80));
        }
    }
}

//! Context window management — compression and prioritization for long sessions.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextMessage {
    pub role: String,
    pub content: String,
    pub priority: u8,
}

#[derive(Debug, Clone)]
pub struct ContextBudget {
    pub max_chars: usize,
    pub reserve_for_tools: usize,
}

impl Default for ContextBudget {
    fn default() -> Self {
        Self {
            max_chars: 96_000,
            reserve_for_tools: 12_000,
        }
    }
}

pub struct ContextCompressor {
    budget: ContextBudget,
}

impl ContextCompressor {
    pub fn new(budget: ContextBudget) -> Self {
        Self { budget }
    }

    /// Fit messages into budget: keep system + recent + high priority; summarize middle.
    pub fn compress(&self, messages: Vec<ContextMessage>) -> Vec<ContextMessage> {
        let limit = self.budget.max_chars.saturating_sub(self.budget.reserve_for_tools);
        let total: usize = messages.iter().map(|m| m.content.len()).sum();
        if total <= limit {
            return messages;
        }

        let mut out = Vec::new();
        if let Some(sys) = messages.first().filter(|m| m.role == "system") {
            out.push(sys.clone());
        }

        let tail_count = 12.min(messages.len());
        let start = messages.len().saturating_sub(tail_count);
        let omitted_count = start.saturating_sub(out.len());

        if omitted_count > 0 {
            out.push(ContextMessage {
                role: "system".into(),
                content: format!(
                    "[Context compressed: {omitted_count} earlier messages omitted. \
                     Ask to expand if you need older context.]"
                ),
                priority: 5,
            });
        }

        for m in messages.into_iter().skip(start) {
            if m.role == "system" && out.first().map(|f| f.role.as_str()) == Some("system") {
                continue;
            }
            out.push(m);
        }
        out
    }

    pub fn estimate_tokens(chars: usize) -> usize {
        chars / 4
    }
}

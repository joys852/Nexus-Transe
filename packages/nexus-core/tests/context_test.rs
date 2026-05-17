use nexus_core::context::{ContextCompressor, ContextMessage, ContextBudget};

#[test]
fn compresses_long_history() {
    let compressor = ContextCompressor::new(ContextBudget {
        max_chars: 500,
        reserve_for_tools: 50,
    });
    let messages: Vec<_> = (0..50)
        .map(|i| ContextMessage {
            role: "user".into(),
            content: format!("message {i} with some padding text"),
            priority: 5,
        })
        .collect();
    let out = compressor.compress(messages);
    let total: usize = out.iter().map(|m| m.content.len()).sum();
    assert!(total < 500);
    assert!(out.len() < 50);
}

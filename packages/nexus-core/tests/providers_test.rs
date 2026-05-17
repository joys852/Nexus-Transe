use nexus_core::providers::{import_cc_switch, ApiProtocol, default_providers};
use tempfile::tempdir;

#[test]
fn default_has_both_protocols() {
    let cfg = default_providers();
    let protocols: Vec<_> = cfg.providers.iter().map(|p| p.protocol.clone()).collect();
    assert!(protocols.contains(&ApiProtocol::AnthropicMessages));
    assert!(protocols.contains(&ApiProtocol::OpenAiChatCompletions));
}

#[test]
fn import_claude_settings_json() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("settings.json");
    std::fs::write(
        &path,
        r#"{
            "env": {
                "ANTHROPIC_BASE_URL": "https://relay.example.com",
                "ANTHROPIC_MODEL": "claude-sonnet-4-20250514"
            }
        }"#,
    )
    .unwrap();
    let cfg = import_cc_switch(&path).unwrap();
    assert_eq!(cfg.providers.len(), 1);
    assert_eq!(cfg.providers[0].protocol, ApiProtocol::AnthropicMessages);
    assert!(cfg.providers[0].proxy_hint);
}

use nexus_core::tools::sandbox_exec::{effective_sandbox_mode, sandbox_mode_from_config, SandboxMode};

#[test]
fn sandbox_mode_parses_docker() {
    assert_eq!(
        sandbox_mode_from_config("docker"),
        SandboxMode::Docker
    );
    assert_eq!(
        sandbox_mode_from_config("local"),
        SandboxMode::Local
    );
}

#[test]
fn effective_mode_uses_env() {
    std::env::set_var("NEXUS_SANDBOX", "docker");
    assert_eq!(
        effective_sandbox_mode("local"),
        SandboxMode::Docker
    );
    std::env::remove_var("NEXUS_SANDBOX");
}

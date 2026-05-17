# Rust helper skill

For Rust in this workspace:

- Use `cargo build -p <crate>` and `cargo test -p <crate>` for verification.
- Follow existing error types (`NexusError`) and `?` conversions.
- Prefer `tokio` async patterns already used in nexus-cli / nexus-core.
- Keep CLI output consistent with `nexus-cli` `ui` module styling.

When proposing changes, cite paths like `packages/nexus-core/src/...`.

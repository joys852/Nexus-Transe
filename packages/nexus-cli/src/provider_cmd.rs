use crate::ui;

use nexus_core::config::NexusConfig;

use nexus_core::providers::{cc_switch_claude_settings_path, import_cc_switch, ProvidersStore};

use std::path::PathBuf;



pub fn run_list(config: &NexusConfig) -> anyhow::Result<()> {

    let store = ProvidersStore::new(&config.data_dir);

    let cfg = store.load()?;

    let rows: Vec<_> = cfg

        .providers

        .iter()

        .map(|p| {

            let active = cfg.active.as_ref() == Some(&p.id);

            (

                p.id.as_str(),

                p.name.as_str(),

                p.protocol.label_zh(),

                p.base_url.as_str(),

                active,

            )

        })

        .collect();

    ui::print_provider_table(cfg.active.as_deref(), &rows);

    Ok(())

}



pub fn run_use(config: &NexusConfig, id: &str) -> anyhow::Result<()> {

    let store = ProvidersStore::new(&config.data_dir);

    let cfg = store.set_active(id)?;

    if let Some(p) = cfg.active_provider() {

        ui::print_success(format!("{} — {}", p.name, p.protocol.label_zh()));

        ui::print_info(format!(

            "Set env {} or: nexus secrets set --provider {} --key …",

            p.api_key_env.as_deref().unwrap_or("API_KEY"),

            p.id

        ));

    }

    Ok(())

}



pub fn run_import(config: &NexusConfig, from: Option<PathBuf>) -> anyhow::Result<()> {

    let source = if let Some(p) = from {

        p

    } else if let Some(p) = cc_switch_claude_settings_path() {

        ui::print_info(format!("Claude settings: {}", p.display()));

        p

    } else {

        anyhow::bail!(

            "no import source. Use --from path to settings.json or CC Switch export JSON"

        );

    };



    let imported = import_cc_switch(&source)?;

    let store = ProvidersStore::new(&config.data_dir);

    let mut existing = store.load().unwrap_or_default();



    for p in imported.providers {

        existing.providers.retain(|x| x.id != p.id);

        existing.providers.push(p);

    }

    if imported.active.is_some() {

        existing.active = imported.active;

    }

    store.save(&existing)?;

    ui::print_success(format!("imported {} provider(s)", existing.providers.len()));

    run_list(config)

}



pub fn run_doctor(config: &NexusConfig) -> anyhow::Result<()> {
    use nexus_core::providers::resolve_env_or_inline_key;
    use nexus_core::secrets::SecretVault;

    let store = ProvidersStore::new(&config.data_dir);
    let cfg = store.load()?;
    let path = store.path();

    ui::print_info(format!("Config: {}", path.display()));

    let Some(active) = cfg.active_provider() else {
        ui::print_error("No active provider. Run: nexus provider list");
        return Ok(());
    };

    ui::print_info(format!(
        "Active: {} — {} ({})",
        active.id,
        active.name,
        active.protocol.label_zh()
    ));
    ui::print_info(format!("URL: {}", active.base_url));
    ui::print_info(format!("Model: {}", active.model));

    if let Some(env) = &active.api_key_env {
        let from_env = std::env::var(env).ok().filter(|v| !v.is_empty());
        let from_inline = resolve_env_or_inline_key(env);
        if from_env.is_some() {
            ui::print_success(format!("{env}: set in environment"));
        } else if from_inline.is_some() {
            ui::print_success(format!("{env}: using inline key from providers.toml"));
        } else {
            ui::print_error(format!("{env}: NOT SET"));
        }
    }

    if active.api_key.as_ref().is_some_and(|k| !k.is_empty()) {
        ui::print_success("api_key: set in providers.toml");
    }

    if let Ok(vault) = SecretVault::open(&config.data_dir) {
        if let Ok(secrets) = vault.load() {
            if secrets.providers.contains_key(&active.id) {
                ui::print_success(format!("secrets vault: key stored for '{}'", active.id));
            }
        }
    }

    match store.resolve_api_key(active, &config.data_dir) {
        Ok(Some(k)) => {
            let preview: String = k.chars().take(8).collect();
            ui::print_success(format!(
                "Resolved API key: {}… ({} chars) — ready to call API",
                preview,
                k.len()
            ));
        }
        Ok(None) => {
            ui::print_error("No API key resolved — requests will return 401.");
            ui::print_info("Fix (pick one):");
            ui::print_info(format!(
                "  $env:{} = \"your-gpt-agent-key\"",
                active.api_key_env.as_deref().unwrap_or("ANTHROPIC_API_KEY")
            ));
            ui::print_info(format!(
                "  nexus secrets set --provider {} --key YOUR_KEY",
                active.id
            ));
            ui::print_info("  Or add api_key = \"...\" under the provider in providers.toml");
        }
        Err(e) => ui::print_error(format!("resolve error: {e}")),
    }

    Ok(())
}

pub fn run_init(config: &NexusConfig) -> anyhow::Result<()> {

    use nexus_core::providers::default_providers;

    let store = ProvidersStore::new(&config.data_dir);

    if !store.path().exists() {

        let cfg = default_providers();

        store.save(&cfg)?;

        ui::print_success(format!("created {}", store.path().display()));

    } else {

        ui::print_info(format!("already exists: {}", store.path().display()));

    }

    Ok(())

}



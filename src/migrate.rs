use crate::config::config_path;
use crate::model::{common_path_prefix, relativize_dir, Config, Entry, Store};
use anyhow::{bail, Context, Result};
use indexmap::IndexMap;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

const LEGACY_CONFIG: &str = "tmux-manager-nodejs/config.json";

#[derive(Debug, Deserialize)]
struct LegacyStore {
    configs: IndexMap<String, LegacyConfig>,
}

#[derive(Debug, Deserialize)]
struct LegacyConfig {
    entries: Vec<LegacyEntry>,
    windows: u32,
}

#[derive(Debug, Deserialize)]
struct LegacyEntry {
    #[serde(rename = "entryName")]
    entry_name: String,
    directory: PathBuf,
}

pub fn legacy_config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from(".config"))
        .join(LEGACY_CONFIG)
}

/// One-time migration from the old Node tmux-manager JSON config to TOML.
pub fn migrate(force: bool) -> Result<()> {
    migrate_from_legacy(&legacy_config_path(), &config_path(), force)
}

pub fn migrate_from_legacy(legacy: &Path, target: &Path, force: bool) -> Result<()> {
    if target.exists() && !force {
        bail!(
            "{} already exists; use --force to overwrite",
            target.display()
        );
    }

    if !legacy.exists() {
        bail!("no legacy config at {}", legacy.display());
    }

    let store = convert_legacy(legacy)?;
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    save_store_to(&store, target)?;

    let backup = legacy.with_extension("json.bak");
    fs::rename(legacy, &backup)?;

    println!("migrated {} -> {}", legacy.display(), target.display());
    println!("backup: {}", backup.display());

    Ok(())
}

fn save_store_to(store: &Store, path: &Path) -> Result<()> {
    let toml = toml::to_string_pretty(store).context("serialize config")?;
    fs::write(path, toml).with_context(|| format!("write {}", path.display()))
}

fn convert_legacy(legacy_path: &Path) -> Result<Store> {
    let raw = fs::read_to_string(legacy_path)
        .with_context(|| format!("read {}", legacy_path.display()))?;
    let legacy: LegacyStore =
        serde_json::from_str(&raw).context("parse legacy tmux-manager config")?;

    let mut configs = IndexMap::new();

    for (name, legacy_config) in legacy.configs {
        let directories: Vec<PathBuf> = legacy_config
            .entries
            .iter()
            .map(|entry| entry.directory.clone())
            .collect();

        let root = common_path_prefix(&directories);
        let mut config = Config::new(root.clone(), legacy_config.windows);

        for entry in legacy_config.entries {
            let key = strip_config_prefix(&name, &entry.entry_name);
            let dir = match root.as_deref() {
                Some(root) => relativize_dir(root, &entry.directory),
                None => entry.directory,
            };

            config.entries.insert(key, Entry::from_dir(dir));
        }

        configs.insert(name, config);
    }

    Ok(Store { configs })
}

fn strip_config_prefix(config_name: &str, entry_name: &str) -> String {
    let prefix = format!("{config_name}/");
    entry_name
        .strip_prefix(&prefix)
        .unwrap_or(entry_name)
        .to_string()
}

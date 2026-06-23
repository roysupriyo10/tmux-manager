use crate::model::{relativize_dir, Config, Store};
use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from(".config"))
        .join("tmux-manager")
}

pub fn config_path() -> PathBuf {
    config_dir().join("config.toml")
}

pub fn load_store() -> Result<Store> {
    let path = config_path();
    if !path.exists() {
        return Ok(Store {
            configs: Default::default(),
        });
    }

    let raw = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    toml::from_str(&raw).with_context(|| format!("parse {}", path.display()))
}

pub fn save_store(store: &Store) -> Result<()> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let toml = toml::to_string_pretty(store).context("serialize config")?;
    fs::write(&path, toml).with_context(|| format!("write {}", path.display()))
}

pub fn get_config<'a>(store: &'a Store, name: &str) -> Result<&'a Config> {
    store
        .configs
        .get(name)
        .with_context(|| format!("config '{name}' not found"))
}

pub fn add_entry(
    store: &mut Store,
    config_name: &str,
    key: &str,
    dir: PathBuf,
) -> Result<()> {
    let config = store
        .configs
        .get_mut(config_name)
        .with_context(|| format!("config '{config_name}' not found"))?;

    if config.entries.contains_key(key) {
        anyhow::bail!("entry '{key}' already exists in config '{config_name}'");
    }

    let stored_dir = match config.root.as_deref() {
        Some(root) if dir.is_absolute() => relativize_dir(root, &dir),
        Some(_) => dir,
        None if dir.is_absolute() => dir,
        None => anyhow::bail!("config '{config_name}' has no root; use an absolute directory"),
    };

    config
        .entries
        .insert(key.to_string(), crate::model::Entry::from_dir(stored_dir));

    Ok(())
}

pub fn remove_entry(store: &mut Store, config_name: &str, key: &str) -> Result<()> {
    let config = store
        .configs
        .get_mut(config_name)
        .with_context(|| format!("config '{config_name}' not found"))?;

    if config.entries.shift_remove(key).is_none() {
        anyhow::bail!("entry '{key}' not found in config '{config_name}'");
    }

    Ok(())
}

pub fn create_config(
    store: &mut Store,
    name: &str,
    root: Option<PathBuf>,
    windows: u32,
) -> Result<()> {
    if store.configs.contains_key(name) {
        anyhow::bail!("config '{name}' already exists");
    }

    store
        .configs
        .insert(name.to_string(), Config::new(root, windows));

    Ok(())
}

pub fn delete_config(store: &mut Store, name: &str) -> Result<()> {
    if store.configs.shift_remove(name).is_none() {
        anyhow::bail!("config '{name}' not found");
    }

    Ok(())
}

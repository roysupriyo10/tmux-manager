use crate::model::{relativize_dir, Config, Entry, EntryOptions, Store};
use crate::paths::expand_config_root;
use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

pub fn user_config_base() -> PathBuf {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        if !xdg.is_empty() {
            return PathBuf::from(xdg);
        }
    }

    dirs::config_dir().unwrap_or_else(|| PathBuf::from(".config"))
}

pub fn config_dir() -> PathBuf {
    user_config_base().join("tmux-manager")
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
    windows: Option<u32>,
    cmd: Option<String>,
) -> Result<()> {
    let config = store
        .configs
        .get_mut(config_name)
        .with_context(|| format!("config '{config_name}' not found"))?;

    if config.entries.contains_key(key) {
        anyhow::bail!("entry '{key}' already exists in config '{config_name}'");
    }

    let expanded = if dir.to_string_lossy().starts_with('~') || dir.is_absolute() {
        expand_config_root(&dir)?
    } else {
        dir
    };

    let stored_dir = match config.root.as_deref() {
        Some(root) => {
            let expanded_root = expand_config_root(root)?;
            if expanded.is_absolute() || expanded.to_string_lossy().starts_with('~') {
                relativize_dir(&expanded_root, &expanded)
            } else {
                expanded
            }
        },
        None if expanded.is_absolute() => expanded,
        None => anyhow::bail!("config '{config_name}' has no root; use an absolute directory"),
    };

    let entry = if windows.is_some() || cmd.is_some() {
        Entry::Detailed(EntryOptions {
            dir: stored_dir,
            windows: windows.map(crate::model::WindowsSpec::Count),
            cmd,
        })
    } else {
        Entry::from_dir(stored_dir)
    };

    config.entries.insert(key.to_string(), entry);

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
    worktree_parent: Option<PathBuf>,
    worktree_prefix: Option<String>,
) -> Result<()> {
    if store.configs.contains_key(name) {
        anyhow::bail!("config '{name}' already exists");
    }

    let mut config = Config::new(root, windows);
    config.worktree_parent = worktree_parent;
    config.worktree_prefix = worktree_prefix;

    store.configs.insert(name.to_string(), config);

    Ok(())
}

pub fn delete_config(store: &mut Store, name: &str) -> Result<()> {
    if store.configs.shift_remove(name).is_none() {
        anyhow::bail!("config '{name}' not found");
    }

    Ok(())
}

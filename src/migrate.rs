use crate::config::{config_path, user_config_base};
use crate::model::{common_path_prefix, relativize_dir, Config, Entry, Store, DEFAULT_WINDOWS};
use crate::paths::expand_config_root;
use anyhow::{bail, Context, Result};
use indexmap::IndexMap;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

const LEGACY_CONFIG: &str = "tmux-manager-nodejs/config.json";

/// Candidate paths for the Node tmux-manager JSON config (first match wins).
pub fn legacy_config_candidates() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    paths.push(user_config_base().join(LEGACY_CONFIG));

    // Electron store on macOS used ~/Library/Preferences, not Application Support.
    #[cfg(target_os = "macos")]
    if let Some(home) = dirs::home_dir() {
        paths.push(
            home.join("Library/Preferences/tmux-manager-nodejs/config.json"),
        );
    }

    paths
}

pub fn find_legacy_config() -> Option<PathBuf> {
    legacy_config_candidates()
        .into_iter()
        .find(|path| path.exists())
}

pub fn legacy_config_path() -> PathBuf {
    find_legacy_config().unwrap_or_else(|| {
        legacy_config_candidates()
            .into_iter()
            .next()
            .unwrap_or_else(|| PathBuf::from(LEGACY_CONFIG))
    })
}

#[derive(Debug, Deserialize)]
struct LegacyStore {
    configs: IndexMap<String, LegacyConfig>,
}

#[derive(Debug, Deserialize)]
struct LegacyConfig {
    entries: Vec<LegacyEntry>,
    #[serde(default = "default_legacy_windows")]
    windows: u32,
    #[serde(rename = "worktreeParent", default)]
    worktree_parent: Option<PathBuf>,
    #[serde(rename = "worktreePrefix", default)]
    worktree_prefix: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LegacyEntry {
    #[serde(rename = "entryName")]
    entry_name: String,
    directory: PathBuf,
}

/// One-time migration from the old Node tmux-manager JSON config to TOML.
///
/// - Target missing: write converted legacy store.
/// - Target exists: merge legacy configs in (default).
/// - `--force`: replace entire TOML with legacy conversion only.
pub fn migrate(force: bool) -> Result<()> {
    let legacy = find_legacy_config().with_context(|| {
        let searched: Vec<String> = legacy_config_candidates()
            .iter()
            .map(|path| path.display().to_string())
            .collect();
        format!(
            "no legacy tmux-manager JSON config found (searched: {})",
            searched.join(", ")
        )
    })?;
    migrate_from_legacy(&legacy, &config_path(), force)
}

pub fn migrate_from_legacy(legacy: &Path, target: &Path, force: bool) -> Result<()> {
    if !legacy.exists() {
        bail!("no legacy config at {}", legacy.display());
    }

    let incoming = convert_legacy(legacy)?;
    if incoming.configs.is_empty() {
        bail!("legacy config has no projects in configs{{}}");
    }

    let store = if target.exists() {
        if force {
            println!("overwriting {} from legacy", target.display());
            incoming
        } else {
            let mut existing = load_store_at(target).with_context(|| {
                format!("read existing config at {}", target.display())
            })?;
            let n = merge_stores(&mut existing, incoming);
            println!(
                "merged {} legacy config(s) into {}",
                n,
                target.display()
            );
            existing
        }
    } else {
        incoming
    };

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

fn merge_stores(existing: &mut Store, incoming: Store) -> usize {
    let mut n = 0usize;
    for (name, mut incoming_config) in incoming.configs {
        normalize_config_paths(&mut incoming_config);
        if let Some(existing_config) = existing.configs.get_mut(&name) {
            existing_config.windows = incoming_config.windows;
            if incoming_config.root.is_some() {
                existing_config.root = incoming_config.root;
            }
            if incoming_config.worktree_parent.is_some() {
                existing_config.worktree_parent = incoming_config.worktree_parent;
            }
            if incoming_config.worktree_prefix.is_some() {
                existing_config.worktree_prefix = incoming_config.worktree_prefix;
            }
            if !incoming_config.worktrees.is_empty() {
                existing_config.worktrees = incoming_config.worktrees;
            }
            existing_config.entries = incoming_config.entries;
        } else {
            existing.configs.insert(name, incoming_config);
        }
        n += 1;
    }
    n
}

fn load_store_at(path: &Path) -> Result<Store> {
    if !path.exists() {
        return Ok(Store {
            configs: Default::default(),
        });
    }

    let raw = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    toml::from_str(&raw).with_context(|| format!("parse {}", path.display()))
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
        config.worktree_parent = legacy_config.worktree_parent;
        config.worktree_prefix = legacy_config.worktree_prefix;

        for entry in legacy_config.entries {
            let key = strip_config_prefix(&name, &entry.entry_name);
            let dir = match root.as_deref() {
                Some(root) => relativize_dir(root, &entry.directory),
                None => entry.directory,
            };

            config.entries.insert(key, Entry::from_dir(dir));
        }

        normalize_config_paths(&mut config);
        configs.insert(name, config);
    }

    Ok(Store { configs })
}

/// Turn absolute entry paths into paths relative to config root (worktrees need this).
pub fn normalize_config_paths(config: &mut Config) {
    let Some(root) = config.root.as_ref() else {
        return;
    };

    let Ok(expanded_root) = expand_config_root(root) else {
        return;
    };

    let keys: Vec<String> = config.entries.keys().cloned().collect();
    for key in keys {
        let Some(entry) = config.entries.get(&key) else {
            continue;
        };
        let dir = entry.dir().clone();
        if !dir.is_absolute() {
            continue;
        }
        let relative = relativize_dir(&expanded_root, &dir);
        config.entries.insert(key, Entry::from_dir(relative));
    }
}

fn default_legacy_windows() -> u32 {
    DEFAULT_WINDOWS
}

fn strip_config_prefix(config_name: &str, entry_name: &str) -> String {
    let prefix = format!("{config_name}/");
    entry_name
        .strip_prefix(&prefix)
        .unwrap_or(entry_name)
        .to_string()
}

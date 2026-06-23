use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const DEFAULT_WINDOWS: u32 = 2;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Store {
    #[serde(flatten)]
    pub configs: IndexMap<String, Config>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root: Option<PathBuf>,
    #[serde(default = "default_windows")]
    pub windows: u32,
    pub entries: IndexMap<String, Entry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Entry {
    Simple(PathBuf),
    Detailed(EntryOptions),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryOptions {
    pub dir: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub windows: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cmd: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ResolvedEntry {
    pub key: String,
    pub session_name: String,
    pub directory: PathBuf,
    pub windows: u32,
    pub cmd: Option<String>,
}

fn default_windows() -> u32 {
    DEFAULT_WINDOWS
}

impl Entry {
    pub fn dir(&self) -> &PathBuf {
        match self {
            Entry::Simple(path) => path,
            Entry::Detailed(opts) => &opts.dir,
        }
    }

    pub fn windows(&self) -> Option<u32> {
        match self {
            Entry::Simple(_) => None,
            Entry::Detailed(opts) => opts.windows,
        }
    }

    pub fn cmd(&self) -> Option<&str> {
        match self {
            Entry::Simple(_) => None,
            Entry::Detailed(opts) => opts.cmd.as_deref(),
        }
    }

    pub fn from_dir(dir: PathBuf) -> Self {
        Entry::Simple(dir)
    }
}

impl Config {
    pub fn new(root: Option<PathBuf>, windows: u32) -> Self {
        Self {
            root,
            windows,
            entries: IndexMap::new(),
        }
    }

    pub fn resolve_entries(&self, config_name: &str) -> anyhow::Result<Vec<ResolvedEntry>> {
        let mut resolved = Vec::with_capacity(self.entries.len());

        for (key, entry) in &self.entries {
            let directory = resolve_entry_dir(self.root.as_deref(), entry.dir())?;
            let windows = entry.windows().unwrap_or(self.windows);
            let session_name = format!("{config_name}/{key}");

            resolved.push(ResolvedEntry {
                key: key.clone(),
                session_name,
                directory,
                windows,
                cmd: entry.cmd().map(str::to_owned),
            });
        }

        Ok(resolved)
    }
}

pub fn resolve_entry_dir(
    root: Option<&std::path::Path>,
    entry_dir: &std::path::Path,
) -> anyhow::Result<PathBuf> {
    if entry_dir.as_os_str() == "." {
        return root
            .map(PathBuf::from)
            .ok_or_else(|| anyhow::anyhow!("entry '.' requires a config root"));
    }

    if entry_dir.is_absolute() {
        return Ok(entry_dir.to_path_buf());
    }

    let root = root.ok_or_else(|| {
        anyhow::anyhow!("entry '{}' requires a config root", entry_dir.display())
    })?;

    Ok(root.join(entry_dir))
}

pub fn relativize_dir(root: &std::path::Path, absolute: &std::path::Path) -> PathBuf {
    absolute
        .strip_prefix(root)
        .map(|p| {
            if p.as_os_str().is_empty() {
                PathBuf::from(".")
            } else {
                p.to_path_buf()
            }
        })
        .unwrap_or_else(|_| absolute.to_path_buf())
}

pub fn common_path_prefix(paths: &[PathBuf]) -> Option<PathBuf> {
    if paths.is_empty() {
        return None;
    }

    let components: Vec<Vec<_>> = paths
        .iter()
        .map(|p| p.components().collect())
        .collect();

    let mut prefix = Vec::new();

    for (i, component) in components[0].iter().enumerate() {
        if components
            .iter()
            .all(|parts| parts.get(i) == Some(&component))
        {
            prefix.push(component.clone());
        } else {
            break;
        }
    }

    if prefix.is_empty() {
        return None;
    }

    let mut result = PathBuf::new();
    for component in prefix {
        result.push(component.as_os_str());
    }

    Some(result)
}

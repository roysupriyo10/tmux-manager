use crate::paths::{expand_config_root, expand_path};
use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const DEFAULT_WINDOWS: u32 = 2;
pub const DEFAULT_WORKTREE_PREFIX: &str = "worktree-{name}";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Store {
    #[serde(flatten)]
    pub configs: IndexMap<String, Config>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Config {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(
        description = "Project root. Absolute path, ~/…, or bare relative (resolved under $HOME)."
    )]
    pub root: Option<PathBuf>,
    #[serde(default = "default_windows")]
    #[schemars(description = "Default window count per session.", range(min = 1))]
    pub windows: u32,
    /// Parent directory for worktrees, relative to config root unless absolute.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "worktree_parent"
    )]
    #[schemars(
        description = "Worktree parent directory (relative to root unless absolute). Used with `tm start -w <name>`."
    )]
    pub worktree_parent: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(
        description = "Session path segment for worktrees. `{name}` is replaced with the -w argument.",
        example = "worktree-{name}"
    )]
    pub worktree_prefix: Option<String>,
    #[schemars(
        description = "Session entries: key is session suffix, value is directory or options."
    )]
    pub entries: IndexMap<String, Entry>,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    #[schemars(description = "Per-worktree overrides keyed by worktree name.")]
    pub worktrees: IndexMap<String, WorktreeOverrides>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, JsonSchema)]
pub struct WorktreeOverrides {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(
        description = "Direct checkout root for this worktree. Does not append the worktree name."
    )]
    pub root: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(
        description = "Default window count when using this worktree.",
        range(min = 1)
    )]
    pub windows: Option<u32>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "worktree_parent"
    )]
    #[schemars(description = "Override worktree parent for this worktree only.")]
    pub worktree_parent: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum SplitDirection {
    #[default]
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(untagged)]
pub enum PaneSpec {
    Command(String),
    Detailed {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cmd: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        dir: Option<PathBuf>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        split: Option<SplitDirection>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(untagged)]
pub enum WindowSpec {
    #[schemars(description = "Run this command in the window's default pane.")]
    Command(String),
    Detailed {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        panes: Vec<PaneSpec>,
    },
}

impl WindowSpec {
    pub fn name(&self) -> Option<&str> {
        match self {
            WindowSpec::Command(_) => None,
            WindowSpec::Detailed { name, .. } => name.as_deref(),
        }
    }

    /// Pane layout for this window. A command string is one default pane.
    pub fn panes(&self) -> std::borrow::Cow<'_, [PaneSpec]> {
        match self {
            WindowSpec::Command(c) => std::borrow::Cow::Owned(vec![PaneSpec::Command(c.clone())]),
            WindowSpec::Detailed { panes, .. } => std::borrow::Cow::Borrowed(panes),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(untagged)]
pub enum WindowsSpec {
    Count(u32),
    Detailed(Vec<WindowSpec>),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum Entry {
    #[schemars(description = "Directory relative to effective root (or . for root).")]
    Simple(PathBuf),
    Detailed(EntryOptions),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EntryOptions {
    #[schemars(description = "Directory relative to effective root.")]
    pub dir: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Override window count or provide detailed layout for this entry.")]
    pub windows: Option<WindowsSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Command sent to the session after create (tmux send-keys).")]
    pub cmd: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ResolveOverrides {
    pub root: Option<PathBuf>,
    pub worktree_parent: Option<PathBuf>,
    pub worktree_root: Option<PathBuf>,
    pub worktree_prefix: Option<String>,
    pub windows: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct ResolveContext<'a> {
    pub config_name: &'a str,
    pub worktree: Option<&'a str>,
    pub overrides: ResolveOverrides,
}

#[derive(Debug, Clone)]
pub struct ResolvedEntry {
    pub key: String,
    pub session_name: String,
    pub directory: PathBuf,
    pub windows: WindowsSpec,
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

    pub fn windows(&self) -> Option<&WindowsSpec> {
        match self {
            Entry::Simple(_) => None,
            Entry::Detailed(opts) => opts.windows.as_ref(),
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
            worktree_parent: None,
            worktree_prefix: None,
            entries: IndexMap::new(),
            worktrees: IndexMap::new(),
        }
    }

    pub fn resolve_entries(&self, config_name: &str) -> anyhow::Result<Vec<ResolvedEntry>> {
        self.resolve_entries_with(&ResolveContext {
            config_name,
            worktree: None,
            overrides: ResolveOverrides::default(),
        })
    }

    pub fn resolve_entries_with(
        &self,
        ctx: &ResolveContext<'_>,
    ) -> anyhow::Result<Vec<ResolvedEntry>> {
        let config_root = if let Some(root) = &ctx.overrides.root {
            expand_config_root(root)?
        } else {
            match self.root.as_deref() {
                Some(root) => expand_config_root(root)?,
                None => anyhow::bail!("config '{}' has no root", ctx.config_name),
            }
        };

        let (session_prefix, effective_base, default_windows) = if let Some(worktree_name) =
            ctx.worktree
        {
            let section = self.worktrees.get(worktree_name);
            let (prefix, base, windows) =
                resolve_worktree_base(self, &config_root, worktree_name, section, &ctx.overrides)?;
            let session_prefix = format!("{}/{}", ctx.config_name, prefix);
            (session_prefix, base, windows)
        } else {
            (
                ctx.config_name.to_string(),
                config_root,
                ctx.overrides.windows.unwrap_or(self.windows),
            )
        };

        let mut resolved = Vec::with_capacity(self.entries.len());

        for (key, entry) in &self.entries {
            let directory = resolve_entry_dir(Some(&effective_base), entry.dir())?;
            let windows = entry.windows().cloned().unwrap_or_else(|| {
                WindowsSpec::Count(ctx.overrides.windows.unwrap_or(default_windows))
            });
            let session_name = format!("{session_prefix}/{key}");

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

fn resolve_worktree_base(
    config: &Config,
    config_root: &Path,
    worktree_name: &str,
    section: Option<&WorktreeOverrides>,
    overrides: &ResolveOverrides,
) -> anyhow::Result<(String, PathBuf, u32)> {
    let prefix_template = overrides
        .worktree_prefix
        .as_deref()
        .or(config.worktree_prefix.as_deref())
        .unwrap_or(DEFAULT_WORKTREE_PREFIX);
    let prefix = prefix_template.replace("{name}", worktree_name);

    let direct_root = overrides
        .worktree_root
        .as_deref()
        .or_else(|| section.and_then(|s| s.root.as_deref()));

    let effective_base = if let Some(direct) = direct_root {
        expand_path(direct, None)?
    } else {
        let parent = overrides
            .worktree_parent
            .as_deref()
            .or_else(|| section.and_then(|s| s.worktree_parent.as_deref()))
            .or(config.worktree_parent.as_deref())
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "worktree '{worktree_name}' requires worktree_parent in config or --worktrees"
                )
            })?;

        let parent_path = if parent.is_absolute() {
            expand_path(parent, None)?
        } else {
            expand_path(parent, Some(config_root))?
        };
        parent_path.join(worktree_name)
    };

    let default_windows = overrides
        .windows
        .or_else(|| section.and_then(|s| s.windows))
        .unwrap_or(config.windows);

    Ok((prefix, effective_base, default_windows))
}

pub fn resolve_entry_dir(root: Option<&Path>, entry_dir: &Path) -> anyhow::Result<PathBuf> {
    if entry_dir.as_os_str() == "." {
        return root
            .map(PathBuf::from)
            .ok_or_else(|| anyhow::anyhow!("entry '.' requires a root directory"));
    }

    if entry_dir.is_absolute() {
        return expand_path(entry_dir, None);
    }

    let raw = entry_dir.to_string_lossy();
    if raw == "~" || raw.starts_with("~/") {
        return expand_path(entry_dir, None);
    }

    let root = root.ok_or_else(|| {
        anyhow::anyhow!("entry '{}' requires a root directory", entry_dir.display())
    })?;

    Ok(root.join(entry_dir))
}

pub fn relativize_dir(root: &Path, absolute: &Path) -> PathBuf {
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

    let components: Vec<Vec<_>> = paths.iter().map(|p| p.components().collect()).collect();

    let mut prefix = Vec::new();

    for (i, component) in components[0].iter().enumerate() {
        if components
            .iter()
            .all(|parts| parts.get(i) == Some(component))
        {
            prefix.push(*component);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn figr_config() -> Config {
        let mut config = Config::new(Some(PathBuf::from("~/developer/figr-ai")), 2);
        config.worktree_parent = Some(PathBuf::from(".claude/worktrees"));
        config.worktree_prefix = Some("worktree-{name}".into());
        config
            .entries
            .insert("root".into(), Entry::from_dir(".".into()));
        config
            .entries
            .insert("frontend/root".into(), Entry::from_dir("frontend".into()));
        config
    }

    #[test]
    fn worktree_parent_mode_resolves_under_named_dir() {
        let config = figr_config();
        let home = crate::paths::home_dir().unwrap();
        let ctx = ResolveContext {
            config_name: "figr-ai",
            worktree: Some("feat+sr+stream-performance"),
            overrides: ResolveOverrides::default(),
        };
        let resolved = config.resolve_entries_with(&ctx).unwrap();
        let root = resolved.iter().find(|e| e.key == "root").unwrap();
        assert_eq!(
            root.directory,
            home.join("developer/figr-ai/.claude/worktrees/feat+sr+stream-performance")
        );
        assert_eq!(
            root.session_name,
            "figr-ai/worktree-feat+sr+stream-performance/root"
        );
    }

    #[test]
    fn worktree_direct_root_skips_name_suffix() {
        let mut config = figr_config();
        config.worktrees.insert(
            "feat".into(),
            WorktreeOverrides {
                root: Some(PathBuf::from("/tmp/checkout")),
                ..Default::default()
            },
        );
        let ctx = ResolveContext {
            config_name: "figr-ai",
            worktree: Some("feat"),
            overrides: ResolveOverrides::default(),
        };
        let resolved = config.resolve_entries_with(&ctx).unwrap();
        let fe = resolved.iter().find(|e| e.key == "frontend/root").unwrap();
        assert_eq!(fe.directory, PathBuf::from("/tmp/checkout/frontend"));
    }

    #[test]
    fn cli_worktree_root_overrides_section() {
        let mut config = figr_config();
        config.worktrees.insert(
            "feat".into(),
            WorktreeOverrides {
                root: Some(PathBuf::from("/old")),
                ..Default::default()
            },
        );
        let ctx = ResolveContext {
            config_name: "figr-ai",
            worktree: Some("feat"),
            overrides: ResolveOverrides {
                worktree_root: Some(PathBuf::from("/new")),
                ..Default::default()
            },
        };
        let resolved = config.resolve_entries_with(&ctx).unwrap();
        let root = resolved.iter().find(|e| e.key == "root").unwrap();
        assert_eq!(root.directory, PathBuf::from("/new"));
    }

    #[test]
    fn window_spec_command_string_deserializes() {
        #[derive(Deserialize)]
        struct Wrap {
            windows: Vec<WindowSpec>,
        }

        let wrap: Wrap = toml::from_str(r#"windows = [{}, "pnpm dev", {}]"#).unwrap();
        let specs = wrap.windows;
        assert!(matches!(specs[0], WindowSpec::Detailed { .. }));
        assert_eq!(specs[1], WindowSpec::Command("pnpm dev".into()));
        assert_eq!(specs[1].panes()[0], PaneSpec::Command("pnpm dev".into()));
    }
}

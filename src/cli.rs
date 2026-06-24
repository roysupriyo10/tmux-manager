use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

use crate::completions::config_name_completer;

#[derive(Parser)]
#[command(name = "tm", version, about = "Fast tmux session manager")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Args, Clone, Debug, Default)]
pub struct RunOptions {
    /// Worktree name (sessions under worktree-{name}/…)
    #[arg(short = 'w', long)]
    pub worktree: Option<String>,
    /// Override config root for this run
    #[arg(long)]
    pub root: Option<PathBuf>,
    /// Worktree parent directory (relative to config root unless absolute)
    #[arg(long)]
    pub worktrees: Option<PathBuf>,
    /// Direct checkout root for worktree (no worktree name appended)
    #[arg(long)]
    pub worktree_root: Option<PathBuf>,
    /// Session prefix template; `{name}` is replaced with worktree name
    #[arg(long)]
    pub worktree_prefix: Option<String>,
    /// Override default window count
    #[arg(long)]
    pub windows: Option<u32>,
    /// Suppress per-session status lines
    #[arg(short, long)]
    pub quiet: bool,
}

impl RunOptions {
    pub fn into_overrides(self) -> crate::model::ResolveOverrides {
        crate::model::ResolveOverrides {
            root: self.root,
            worktree_parent: self.worktrees,
            worktree_root: self.worktree_root,
            worktree_prefix: self.worktree_prefix,
            windows: self.windows,
        }
    }
}

#[derive(Subcommand)]
pub enum Command {
    /// List configs, or entries for one config
    Ls {
        #[arg(add = config_name_completer())]
        config: Option<String>,
        #[command(flatten)]
        run: RunOptions,
    },
    /// Create a new config
    New {
        config: String,
        #[arg(long)]
        root: Option<PathBuf>,
        #[arg(long, default_value_t = 2)]
        windows: u32,
        #[arg(long)]
        worktrees: Option<PathBuf>,
        #[arg(long)]
        worktree_prefix: Option<String>,
    },
    /// Add an entry to a config
    Add {
        #[arg(add = config_name_completer())]
        config: String,
        name: String,
        dir: PathBuf,
        #[arg(long)]
        windows: Option<u32>,
        #[arg(long)]
        cmd: Option<String>,
    },
    /// Remove an entry from a config
    Rm {
        #[arg(add = config_name_completer())]
        config: String,
        name: String,
    },
    /// Delete a config
    Del {
        #[arg(add = config_name_completer())]
        config: String,
    },
    /// Start sessions (all entries, or matched patterns)
    Start {
        #[arg(add = config_name_completer())]
        config: String,
        #[arg(value_name = "PATTERN")]
        patterns: Vec<String>,
        #[command(flatten)]
        run: RunOptions,
    },
    /// Kill sessions (all entries, or matched patterns)
    Kill {
        #[arg(add = config_name_completer())]
        config: String,
        #[arg(value_name = "PATTERN")]
        patterns: Vec<String>,
        #[command(flatten)]
        run: RunOptions,
    },
    /// Open a config in $EDITOR
    Edit {
        #[arg(add = config_name_completer())]
        config: Option<String>,
    },
    /// Migrate legacy JSON config to TOML (one-time; merges into existing config.toml)
    Migrate {
        #[arg(long, help = "replace entire config.toml with legacy conversion only")]
        force: bool,
    },
    /// Generate shell completions to stdout
    Completions { shell: Shell },
}

#[derive(Clone, Copy, ValueEnum)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
}

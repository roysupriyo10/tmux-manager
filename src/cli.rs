use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

use crate::completions::config_name_completer;

#[derive(Parser)]
#[command(name = "tm", version, about = "Fast tmux session manager")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// List configs, or entries for one config
    Ls {
        #[arg(add = config_name_completer())]
        config: Option<String>,
    },
    /// Create a new config
    New {
        config: String,
        #[arg(long)]
        root: Option<PathBuf>,
        #[arg(long, default_value_t = 2)]
        windows: u32,
    },
    /// Add an entry to a config
    Add {
        #[arg(add = config_name_completer())]
        config: String,
        name: String,
        dir: PathBuf,
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
    },
    /// Kill sessions (all entries, or matched patterns)
    Kill {
        #[arg(add = config_name_completer())]
        config: String,
        #[arg(value_name = "PATTERN")]
        patterns: Vec<String>,
    },
    /// Open a config in $EDITOR
    Edit {
        #[arg(add = config_name_completer())]
        config: Option<String>,
    },
    /// Migrate legacy JSON config to TOML (one-time)
    Migrate {
        #[arg(long, help = "overwrite existing config.toml")]
        force: bool,
    },
    /// Generate shell completions to stdout
    Completions {
        shell: Shell,
    },
}

#[derive(Clone, Copy, ValueEnum)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
}

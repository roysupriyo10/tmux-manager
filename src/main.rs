use tmux_manager::cli::{Cli, Command};
use tmux_manager::completions;
use tmux_manager::config::{
    add_entry, create_config, delete_config, get_config, load_store, remove_entry, save_store,
    config_path,
};
use tmux_manager::matcher::resolve_entry_keys;
use tmux_manager::migrate;
use tmux_manager::model::{ResolvedEntry, Store};
use tmux_manager::tmux::{Backend, TmuxBackend};

use anyhow::{Context, Result};
use clap::{CommandFactory, Parser};
use clap_complete::CompleteEnv;
use std::env;
use std::process::Command as ProcessCommand;

fn main() {
    CompleteEnv::with_factory(Cli::command).complete();

    if let Err(error) = run() {
        eprintln!("error: {error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Completions { shell } => {
            completions::write_shell_completions(shell, &mut std::io::stdout())
        }
        command => dispatch(command),
    }
}

fn dispatch(command: Command) -> Result<()> {
    let mut store = load_store()?;

    match command {
        Command::Ls { config } => cmd_ls(&store, config.as_deref()),
        Command::New {
            config,
            root,
            windows,
        } => {
            create_config(&mut store, &config, root, windows)?;
            save_store(&store)?;
            println!("created config: {config}");
            Ok(())
        }
        Command::Add { config, name, dir } => {
            add_entry(&mut store, &config, &name, dir)?;
            save_store(&store)?;
            println!("added entry {name} to {config}");
            Ok(())
        }
        Command::Rm { config, name } => {
            remove_entry(&mut store, &config, &name)?;
            save_store(&store)?;
            println!("removed entry {name} from {config}");
            Ok(())
        }
        Command::Del { config } => {
            delete_config(&mut store, &config)?;
            save_store(&store)?;
            println!("deleted config: {config}");
            Ok(())
        }
        Command::Start { config, patterns } => {
            let entries = select_entries(&store, &config, &patterns)?;
            if entries.is_empty() {
                println!("no entries matched");
                return Ok(());
            }
            TmuxBackend::default().start_sessions(&entries)
        }
        Command::Kill { config, patterns } => {
            let entries = select_entries(&store, &config, &patterns)?;
            if entries.is_empty() {
                println!("no entries matched");
                return Ok(());
            }
            TmuxBackend::default().kill_sessions(&entries)
        }
        Command::Edit { config } => cmd_edit(config.as_deref()),
        Command::Migrate { force } => migrate::migrate(force),
        Command::Completions { .. } => unreachable!(),
    }
}

fn cmd_ls(store: &Store, config_name: Option<&str>) -> Result<()> {
    if let Some(name) = config_name {
        let config = get_config(store, name)?;
        println!("{name}:");
        if let Some(root) = &config.root {
            println!("  root: {}", root.display());
        }
        println!("  windows: {}", config.windows);
        println!("  entries:");

        for entry in config.resolve_entries(name)? {
            println!("    - {}: {}", entry.key, entry.directory.display());
        }
        return Ok(());
    }

    if store.configs.is_empty() {
        println!("no configs");
        return Ok(());
    }

    println!("configs:");
    for name in store.configs.keys() {
        let config = &store.configs[name];
        println!("  {name} ({} entries)", config.entries.len());
    }

    Ok(())
}

fn select_entries(
    store: &Store,
    config_name: &str,
    patterns: &[String],
) -> Result<Vec<ResolvedEntry>> {
    let config = get_config(store, config_name)?;
    let resolved = config.resolve_entries(config_name)?;

    if patterns.is_empty() {
        return Ok(resolved);
    }

    let keys: Vec<String> = resolved.iter().map(|entry| entry.key.clone()).collect();
    let (matched_keys, unmatched) = resolve_entry_keys(patterns, &keys);

    for pattern in unmatched {
        println!("no match for '{pattern}'");
    }

    for key in &matched_keys {
        println!("matched '{key}'");
    }

    Ok(resolved
        .into_iter()
        .filter(|entry| matched_keys.contains(&entry.key))
        .collect())
}

fn cmd_edit(config_name: Option<&str>) -> Result<()> {
    if let Some(name) = config_name {
        let store = load_store()?;
        get_config(&store, name)?;
    }

    let editor = env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
    let status = ProcessCommand::new(&editor)
        .arg(config_path())
        .status()
        .with_context(|| format!("open editor '{editor}'"))?;

    if !status.success() {
        anyhow::bail!("editor exited with {status}");
    }

    Ok(())
}

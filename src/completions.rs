use crate::cli::{Cli, Shell};
use crate::config::load_store;
use anyhow::Result;
use clap::CommandFactory;
use clap_complete::engine::{ArgValueCompleter, CompletionCandidate};
use clap_complete::{generate as write_completions, shells};
use std::ffi::OsStr;
use std::io::Write;

pub fn write_shell_completions(shell: Shell, writer: &mut dyn Write) -> Result<()> {
    let mut cmd = Cli::command();

    match shell {
        Shell::Bash => write_completions(shells::Bash, &mut cmd, "tm", writer),
        Shell::Zsh => write_completions(shells::Zsh, &mut cmd, "tm", writer),
        Shell::Fish => write_completions(shells::Fish, &mut cmd, "tm", writer),
    }

    Ok(())
}

pub fn config_name_candidates() -> Vec<String> {
    load_store()
        .map(|store| store.configs.keys().cloned().collect())
        .unwrap_or_default()
}

pub fn complete_config_names(_current: &OsStr) -> Vec<CompletionCandidate> {
    config_name_candidates()
        .into_iter()
        .map(CompletionCandidate::new)
        .collect()
}

pub fn config_name_completer() -> ArgValueCompleter {
    ArgValueCompleter::new(complete_config_names)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn zsh_completion_includes_subcommands_and_compdef() {
        let mut buf = Vec::new();
        write_shell_completions(Shell::Zsh, &mut buf).unwrap();
        let script = String::from_utf8(buf).unwrap();

        assert!(script.contains("#compdef tm"));
        for sub in ["start", "kill", "ls", "add", "completions"] {
            assert!(script.contains(sub), "missing subcommand: {sub}");
        }
    }

    #[test]
    fn bash_completion_registers_tm_command() {
        let mut buf = Vec::new();
        write_shell_completions(Shell::Bash, &mut buf).unwrap();
        let script = String::from_utf8(buf).unwrap();

        assert!(script.contains("tm"));
        assert!(script.contains("start"));
        assert!(script.contains("kill"));
    }

    #[test]
    fn fish_completion_defines_tm() {
        let mut buf = Vec::new();
        write_shell_completions(Shell::Fish, &mut buf).unwrap();
        let script = String::from_utf8(buf).unwrap();

        assert!(script.contains("tm"));
        assert!(script.contains("start"));
    }

    #[test]
    fn config_name_candidates_reads_store() {
        let dir = TempDir::new().unwrap();
        let config_dir = dir.path().join("tmux-manager");
        fs::create_dir_all(&config_dir).unwrap();
        fs::write(
            config_dir.join("config.toml"),
            r#"[demo]
root = "/tmp/demo"
windows = 2

[demo.entries]
root = "."
"#,
        )
        .unwrap();

        let prev = std::env::var("XDG_CONFIG_HOME").ok();
        std::env::set_var("XDG_CONFIG_HOME", dir.path());

        let names = config_name_candidates();
        assert_eq!(names, vec!["demo".to_string()]);

        restore_xdg_config_home(prev);
    }

    #[test]
    fn complete_config_names_returns_candidates() {
        let dir = TempDir::new().unwrap();
        let config_dir = dir.path().join("tmux-manager");
        fs::create_dir_all(&config_dir).unwrap();
        fs::write(
            config_dir.join("config.toml"),
            r#"[kuib-ai]
root = "/tmp/kuib"
windows = 2

[kuib-ai.entries]
root = "."
"#,
        )
        .unwrap();

        let prev = std::env::var("XDG_CONFIG_HOME").ok();
        std::env::set_var("XDG_CONFIG_HOME", dir.path());

        let candidates = complete_config_names(OsStr::new("ku"));
        let values: Vec<String> = candidates
            .into_iter()
            .map(|c| c.get_value().to_string_lossy().into_owned())
            .collect();
        assert_eq!(values, vec!["kuib-ai".to_string()]);

        restore_xdg_config_home(prev);
    }

    fn restore_xdg_config_home(prev: Option<String>) {
        match prev {
            Some(value) => std::env::set_var("XDG_CONFIG_HOME", value),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
    }
}

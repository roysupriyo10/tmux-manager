use crate::matcher::normalize_session_name;
use crate::model::ResolvedEntry;
use anyhow::{Context, Result};
use std::collections::HashSet;
use std::ffi::OsStr;
use std::process::Command;

pub trait Backend {
    fn start_sessions(&self, entries: &[ResolvedEntry], quiet: bool) -> Result<()>;
    fn kill_sessions(&self, entries: &[ResolvedEntry]) -> Result<()>;
}

#[derive(Debug, Clone)]
pub struct TmuxBackend {
    tmux: String,
    socket: Option<String>,
}

impl Default for TmuxBackend {
    fn default() -> Self {
        Self {
            tmux: "tmux".to_string(),
            socket: None,
        }
    }
}

impl TmuxBackend {
    pub fn with_socket(socket: impl Into<String>) -> Self {
        Self {
            tmux: "tmux".to_string(),
            socket: Some(socket.into()),
        }
    }

    pub fn build_start_args(session: &str, entry: &ResolvedEntry) -> Vec<String> {
        let dir = entry.directory.to_string_lossy().into_owned();
        let mut args = vec![
            "new-session".into(),
            "-d".into(),
            "-s".into(),
            session.into(),
            "-c".into(),
            dir.clone(),
        ];

        for _ in 1..entry.windows {
            args.push(";".into());
            args.extend([
                "new-window".into(),
                "-t".into(),
                session.into(),
                "-c".into(),
                dir.clone(),
            ]);
        }

        if let Some(cmd) = &entry.cmd {
            args.push(";".into());
            args.extend([
                "send-keys".into(),
                "-t".into(),
                session.into(),
                cmd.clone(),
                "Enter".into(),
            ]);
        }

        args
    }

    fn command(&self) -> Command {
        let mut cmd = Command::new(&self.tmux);
        if let Some(socket) = &self.socket {
            cmd.args(["-L", socket]);
        }
        cmd
    }

    fn run_args<I, S>(&self, args: I) -> Result<()>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let status = self.command().args(args).status().context("run tmux")?;

        if !status.success() {
            anyhow::bail!("tmux exited with {status}");
        }

        Ok(())
    }

    fn list_sessions(&self) -> Result<HashSet<String>> {
        let output = self
            .command()
            .args(["list-sessions", "-F", "#{session_name}"])
            .output();

        match output {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                Ok(stdout
                    .lines()
                    .filter(|line| !line.is_empty())
                    .map(str::to_owned)
                    .collect())
            },
            _ => Ok(HashSet::new()),
        }
    }
}

impl Backend for TmuxBackend {
    fn start_sessions(&self, entries: &[ResolvedEntry], quiet: bool) -> Result<()> {
        if entries.is_empty() {
            return Ok(());
        }

        let existing = self.list_sessions()?;
        let mut to_create: Vec<&ResolvedEntry> = Vec::new();
        let mut skipped = 0usize;

        for entry in entries {
            let session = normalize_session_name(&entry.session_name);
            if existing.contains(&session) {
                skipped += 1;
                if !quiet {
                    println!("session {session} already exists");
                }
                continue;
            }
            to_create.push(entry);
        }

        if to_create.is_empty() {
            if quiet && skipped > 0 {
                println!("all {skipped} session(s) already exist");
            }
            return Ok(());
        }

        let mut args: Vec<String> = Vec::new();
        let mut first = true;

        for entry in &to_create {
            let session = normalize_session_name(&entry.session_name);
            if !first {
                args.push(";".into());
            }
            first = false;
            args.extend(Self::build_start_args(&session, entry));
            if !quiet {
                println!("created session: {session}");
            }
        }

        self.run_args(&args).context("start sessions")?;

        if quiet {
            println!("created {} session(s)", to_create.len());
        }

        Ok(())
    }

    fn kill_sessions(&self, entries: &[ResolvedEntry]) -> Result<()> {
        if entries.is_empty() {
            return Ok(());
        }

        let existing = self.list_sessions()?;
        let mut args: Vec<String> = Vec::new();
        let mut first = true;

        for entry in entries {
            let session = normalize_session_name(&entry.session_name);

            if !existing.contains(&session) {
                continue;
            }

            if !first {
                args.push(";".into());
            }
            first = false;

            args.extend(["kill-session".into(), "-t".into(), session.clone()]);
            println!("killed session: {session}");
        }

        if args.is_empty() {
            return Ok(());
        }

        self.run_args(&args).context("kill sessions")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn start_args_chain_windows_with_semicolon_tokens() {
        let entry = ResolvedEntry {
            key: "root".into(),
            session_name: "cfg/root".into(),
            directory: PathBuf::from("/tmp/work"),
            windows: 2,
            cmd: None,
        };

        let args = TmuxBackend::build_start_args("cfg/root", &entry);
        assert_eq!(
            args,
            vec![
                "new-session",
                "-d",
                "-s",
                "cfg/root",
                "-c",
                "/tmp/work",
                ";",
                "new-window",
                "-t",
                "cfg/root",
                "-c",
                "/tmp/work",
            ]
        );
    }
}

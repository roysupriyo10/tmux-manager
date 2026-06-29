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

        match &entry.windows {
            crate::model::WindowsSpec::Count(count) => {
                for _ in 1..*count {
                    args.push(";".into());
                    args.extend([
                        "new-window".into(),
                        "-t".into(),
                        session.into(),
                        "-c".into(),
                        dir.clone(),
                    ]);
                }
            },
            crate::model::WindowsSpec::Detailed(windows) => {
                let mut first_window = true;
                for (w_idx, window) in windows.iter().enumerate() {
                    let window_target = format!("{}:{}", session, w_idx);
                    let panes = window.panes();
                    if first_window {
                        // The first window is already created by new-session. Optionally rename it.
                        if let Some(name) = window.name() {
                            args.push(";".into());
                            args.extend([
                                "rename-window".into(),
                                "-t".into(),
                                window_target.clone(),
                                name.to_string(),
                            ]);
                        }
                        first_window = false;
                    } else {
                        args.push(";".into());
                        let mut nw_args = vec![
                            "new-window".into(),
                            "-t".into(),
                            session.into(),
                            "-c".into(),
                            dir.clone(),
                        ];
                        if let Some(name) = window.name() {
                            nw_args.push("-n".into());
                            nw_args.push(name.to_string());
                        }
                        args.extend(nw_args);
                    }

                    // Process panes
                    let mut first_pane = true;
                    for pane in panes.iter() {
                        let pane_dir = match pane {
                            crate::model::PaneSpec::Detailed { dir: Some(d), .. } => {
                                Some(entry.directory.join(d).to_string_lossy().into_owned())
                            },
                            _ => None,
                        };

                        if first_pane {
                            // First pane is already created. Send keys if needed.
                            let cmd_str = match pane {
                                crate::model::PaneSpec::Command(c) => Some(c.as_str()),
                                crate::model::PaneSpec::Detailed { cmd, .. } => cmd.as_deref(),
                            };
                            if let Some(c) = cmd_str {
                                args.push(";".into());
                                args.extend([
                                    "send-keys".into(),
                                    "-t".into(),
                                    window_target.clone(),
                                    c.to_string(),
                                    "Enter".into(),
                                ]);
                            }
                            first_pane = false;
                        } else {
                            // Create new pane
                            args.push(";".into());
                            let split_dir = match pane {
                                crate::model::PaneSpec::Detailed {
                                    split: Some(crate::model::SplitDirection::Vertical),
                                    ..
                                } => "-v",
                                _ => "-h", // default horizontal
                            };
                            let mut split_args = vec![
                                "split-window".into(),
                                split_dir.into(),
                                "-t".into(),
                                window_target.clone(),
                            ];
                            if let Some(d) = &pane_dir {
                                split_args.push("-c".into());
                                split_args.push(d.clone());
                            } else {
                                split_args.push("-c".into());
                                split_args.push(dir.clone());
                            }
                            args.extend(split_args);

                            // Send keys to the newly created pane (it becomes the active pane in that window)
                            let cmd_str = match pane {
                                crate::model::PaneSpec::Command(c) => Some(c.as_str()),
                                crate::model::PaneSpec::Detailed { cmd, .. } => cmd.as_deref(),
                            };
                            if let Some(c) = cmd_str {
                                args.push(";".into());
                                args.extend([
                                    "send-keys".into(),
                                    "-t".into(),
                                    window_target.clone(),
                                    c.to_string(),
                                    "Enter".into(),
                                ]);
                            }
                        }
                    }
                }
            },
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
        }

        self.run_args(&args).context("start sessions")?;

        if quiet {
            println!("created {} session(s)", to_create.len());
        } else {
            for entry in to_create.iter() {
                let session = normalize_session_name(&entry.session_name);
                println!("created session: {session}");
            }
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
            windows: crate::model::WindowsSpec::Count(2),
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

    #[test]
    fn start_args_builds_panes_correctly() {
        use crate::model::{PaneSpec, SplitDirection, WindowSpec, WindowsSpec};
        let entry = ResolvedEntry {
            key: "root".into(),
            session_name: "cfg/root".into(),
            directory: PathBuf::from("/tmp/work"),
            windows: WindowsSpec::Detailed(vec![
                WindowSpec::Detailed {
                    name: Some("editor".into()),
                    panes: vec![PaneSpec::Command("nvim".into())],
                },
                WindowSpec::Detailed {
                    name: Some("server".into()),
                    panes: vec![
                        PaneSpec::Detailed {
                            cmd: Some("npm run dev".into()),
                            dir: None,
                            split: None,
                        },
                        PaneSpec::Detailed {
                            cmd: Some("npm run test".into()),
                            dir: Some(PathBuf::from("client")),
                            split: Some(SplitDirection::Vertical),
                        },
                    ],
                },
            ]),
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
                "rename-window",
                "-t",
                "cfg/root:0",
                "editor",
                ";",
                "send-keys",
                "-t",
                "cfg/root:0",
                "nvim",
                "Enter",
                ";",
                "new-window",
                "-t",
                "cfg/root",
                "-c",
                "/tmp/work",
                "-n",
                "server",
                ";",
                "send-keys",
                "-t",
                "cfg/root:1",
                "npm run dev",
                "Enter",
                ";",
                "split-window",
                "-v",
                "-t",
                "cfg/root:1",
                "-c",
                "/tmp/work/client",
                ";",
                "send-keys",
                "-t",
                "cfg/root:1",
                "npm run test",
                "Enter",
            ]
        );
    }

    #[test]
    fn start_args_window_command_string_matches_single_pane() {
        use crate::model::{WindowSpec, WindowsSpec};
        let from_string = ResolvedEntry {
            key: "root".into(),
            session_name: "cfg/root".into(),
            directory: PathBuf::from("/tmp/work"),
            windows: WindowsSpec::Detailed(vec![WindowSpec::Command("pnpm dev".into())]),
            cmd: None,
        };
        let from_panes = ResolvedEntry {
            key: "root".into(),
            session_name: "cfg/root".into(),
            directory: PathBuf::from("/tmp/work"),
            windows: WindowsSpec::Detailed(vec![WindowSpec::Detailed {
                name: None,
                panes: vec![crate::model::PaneSpec::Command("pnpm dev".into())],
            }]),
            cmd: None,
        };

        assert_eq!(
            TmuxBackend::build_start_args("cfg/root", &from_string),
            TmuxBackend::build_start_args("cfg/root", &from_panes),
        );
    }
}

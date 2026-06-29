use std::path::PathBuf;
use std::process::Command;
use tmux_manager::model::ResolvedEntry;
use tmux_manager::tmux::{Backend, TmuxBackend};

fn test_socket() -> String {
    format!("tm-test-{}", std::process::id())
}

fn cleanup(socket: &str, session: &str) {
    let _ = Command::new("tmux")
        .args(["-L", socket, "kill-session", "-t", session])
        .status();
}

#[test]
fn start_session_creates_expected_windows() {
    let socket = test_socket();
    let session = "itest/demo";
    cleanup(&socket, session);

    let backend = TmuxBackend::with_socket(&socket);
    let entry = ResolvedEntry {
        key: "demo".into(),
        session_name: session.into(),
        directory: PathBuf::from("/tmp"),
        windows: tmux_manager::model::WindowsSpec::Count(2),
        cmd: None,
    };

    backend.start_sessions(&[entry], false, false).unwrap();

    let list = Command::new("tmux")
        .args([
            "-L",
            &socket,
            "list-windows",
            "-t",
            session,
            "-F",
            "#{window_index}",
        ])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&list.stdout);
    let windows: Vec<String> = stdout
        .lines()
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect();

    assert_eq!(windows, vec!["0", "1"]);

    backend
        .kill_sessions(&[ResolvedEntry {
            key: "demo".into(),
            session_name: session.into(),
            directory: PathBuf::from("/tmp"),
            windows: tmux_manager::model::WindowsSpec::Count(2),
            cmd: None,
        }])
        .unwrap();

    let exists = Command::new("tmux")
        .args(["-L", &socket, "has-session", "-t", session])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    assert!(!exists);
}

#[test]
fn start_is_idempotent_when_session_exists() {
    let socket = test_socket();
    let session = "itest/idempotent";
    cleanup(&socket, session);

    let backend = TmuxBackend::with_socket(&socket);
    let entry = ResolvedEntry {
        key: "demo".into(),
        session_name: session.into(),
        directory: PathBuf::from("/tmp"),
        windows: tmux_manager::model::WindowsSpec::Count(1),
        cmd: None,
    };

    backend
        .start_sessions(&[entry.clone()], false, false)
        .unwrap();
    backend.start_sessions(&[entry], false, false).unwrap();

    cleanup(&socket, session);
}

#[test]
fn start_no_cmd_creates_windows_without_running_commands() {
    let socket = test_socket();
    let session = "itest/no-cmd";
    cleanup(&socket, session);

    let backend = TmuxBackend::with_socket(&socket);
    let entry = ResolvedEntry {
        key: "demo".into(),
        session_name: session.into(),
        directory: PathBuf::from("/tmp"),
        windows: tmux_manager::model::WindowsSpec::Detailed(vec![
            tmux_manager::model::WindowSpec::Command("this-command-must-not-run".into()),
            tmux_manager::model::WindowSpec::Detailed {
                name: None,
                panes: vec![tmux_manager::model::PaneSpec::Command(
                    "nor-should-this".into(),
                )],
            },
        ]),
        cmd: Some("entry-cmd-must-not-run".into()),
    };

    backend
        .start_sessions(&[entry.clone()], false, true)
        .unwrap();

    let list = Command::new("tmux")
        .args([
            "-L",
            &socket,
            "list-windows",
            "-t",
            session,
            "-F",
            "#{window_index}",
        ])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&list.stdout);
    let windows: Vec<String> = stdout
        .lines()
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect();
    assert_eq!(windows, vec!["0", "1"]);

    backend.kill_sessions(&[entry]).unwrap();
    cleanup(&socket, session);
}

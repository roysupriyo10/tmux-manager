use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn completions_subcommand_emits_zsh_compdef() {
    let output = Command::new(env!("CARGO_BIN_EXE_tm"))
        .args(["completions", "zsh"])
        .output()
        .expect("run tm completions zsh");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let script = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(script.contains("#compdef tm"));
    assert!(script.contains("start"));
}

#[test]
fn completions_subcommand_emits_bash() {
    let output = Command::new(env!("CARGO_BIN_EXE_tm"))
        .args(["completions", "bash"])
        .output()
        .expect("run tm completions bash");

    assert!(output.status.success());

    let script = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(script.contains("tm"));
    assert!(script.contains("kill"));
}

#[test]
fn completions_subcommand_emits_fish() {
    let output = Command::new(env!("CARGO_BIN_EXE_tm"))
        .args(["completions", "fish"])
        .output()
        .expect("run tm completions fish");

    assert!(output.status.success());

    let script = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(script.contains("complete -c tm"));
}

#[test]
fn dynamic_zsh_registration_includes_compdef() {
    let output = Command::new(env!("CARGO_BIN_EXE_tm"))
        .env("COMPLETE", "zsh")
        .output()
        .expect("run COMPLETE=zsh tm");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let script = String::from_utf8_lossy(&output.stdout);
    assert!(script.contains("#compdef tm"));
    assert!(script.contains("dynamic"));
}

#[test]
fn dynamic_completion_lists_config_names() {
    let dir = TempDir::new().unwrap();
    let config_dir = dir.path().join("tmux-manager");
    fs::create_dir_all(&config_dir).unwrap();
    fs::write(
        config_dir.join("config.toml"),
        r#"[portfolios]
root = "/tmp/portfolios"
windows = 2

[portfolios.entries]
root = "."
"#,
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_tm"))
        .env("COMPLETE", "bash")
        .env("_CLAP_COMPLETE_INDEX", "2")
        .env("XDG_CONFIG_HOME", dir.path())
        .args(["tm", "--", "tm", "start", ""])
        .output()
        .expect("run dynamic completion for tm start");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let candidates = String::from_utf8_lossy(&output.stdout);
    assert!(
        candidates.contains("portfolios"),
        "expected config name in: {candidates}"
    );
}

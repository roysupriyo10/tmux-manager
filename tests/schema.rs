//! Schema generation and taplo LSP validation integration tests.

use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;
use tmux_manager::schema_gen::store_schema_json;

const VALID_CONFIG: &str = r#"
[demo]
root = "/tmp/demo"
windows = 2
worktree_parent = ".claude/worktrees"
worktree_prefix = "worktree-{name}"

[demo.entries]
root = "."
web = "apps/web"
api = { dir = "apps/api", windows = 3, cmd = "cargo run" }

[demo.worktrees."feat-x"]
windows = 1
root = "/tmp/checkout"
"#;

const TAPLO_TOML: &str = include_str!("../schemas/taplo.toml");

fn schema_value() -> Value {
    let raw = store_schema_json().expect("generate schema");
    serde_json::from_str(&raw).expect("parse schema json")
}

fn find_taplo() -> Option<PathBuf> {
    for candidate in ["taplo", &mason_taplo()] {
        let path = PathBuf::from(candidate);
        if path.is_file() {
            return Some(path);
        }
        if Command::new(candidate)
            .arg("--version")
            .output()
            .is_ok_and(|o| o.status.success())
        {
            return Some(PathBuf::from(candidate));
        }
    }
    None
}

fn mason_taplo() -> String {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    format!("{home}/.local/share/nvim/mason/bin/taplo")
}

fn write_workspace(dir: &Path, config_toml: &str) {
    fs::write(
        dir.join("config.schema.json"),
        store_schema_json().expect("schema json"),
    )
    .expect("write schema");
    fs::write(dir.join("taplo.toml"), TAPLO_TOML).expect("write taplo.toml");
    fs::write(dir.join("config.toml"), config_toml).expect("write config");
}

fn taplo_check(dir: &Path) -> std::process::Output {
    let taplo = find_taplo().expect("taplo must be installed for this test");
    Command::new(taplo)
        .args(["check", "config.toml"])
        .current_dir(dir)
        .output()
        .expect("run taplo check")
}

fn taplo_available() -> bool {
    find_taplo().is_some()
}

#[test]
fn checked_in_schema_matches_generator() {
    let generated = schema_value();
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("schemas/config.schema.json");
    let checked_in: Value =
        serde_json::from_str(&fs::read_to_string(&path).expect("read checked-in schema"))
            .expect("parse checked-in schema");
    assert_eq!(
        generated, checked_in,
        "schemas/config.schema.json is stale — run: cargo run --bin gen-schema"
    );
}

#[test]
fn schema_exposes_all_config_fields_with_descriptions() {
    let v = schema_value();
    let cfg = &v["patternProperties"]["^[a-zA-Z][a-zA-Z0-9._-]*$"];
    let props = &cfg["properties"];

    for key in [
        "root",
        "windows",
        "worktree_parent",
        "worktree_prefix",
        "entries",
        "worktrees",
    ] {
        let field = &props[key];
        assert!(
            field.get("description").is_some(),
            "missing description on {key}"
        );
    }

    let init_keys = cfg["x-taplo"]["initKeys"]
        .as_array()
        .expect("config initKeys");
    assert_eq!(init_keys.len(), 6);
}

#[test]
fn schema_entry_and_worktree_definitions_have_init_keys() {
    let v = schema_value();

    let entry_opts = &v["definitions"]["EntryOptions"];
    assert_eq!(entry_opts["required"], serde_json::json!(["dir"]));
    assert_eq!(
        entry_opts["x-taplo"]["initKeys"],
        serde_json::json!(["dir", "windows", "cmd"])
    );

    let wt = &v["definitions"]["WorktreeOverrides"];
    assert_eq!(
        wt["x-taplo"]["initKeys"],
        serde_json::json!(["root", "windows", "worktree_parent"])
    );

    let window = &v["definitions"]["WindowSpec"];
    let variants = window
        .get("anyOf")
        .or_else(|| window.get("oneOf"))
        .and_then(|v| v.as_array())
        .expect("WindowSpec is a union");
    assert!(variants.len() >= 2);
}

#[test]
fn taplo_validates_fixture_config() {
    if !taplo_available() {
        eprintln!("skip taplo_validates_fixture_config: taplo not on PATH");
        return;
    }

    let dir = TempDir::new().expect("tempdir");
    write_workspace(dir.path(), VALID_CONFIG);

    let out = taplo_check(dir.path());
    assert!(
        out.status.success(),
        "taplo check failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn taplo_rejects_unknown_config_field() {
    if !taplo_available() {
        eprintln!("skip taplo_rejects_unknown_config_field: taplo not on PATH");
        return;
    }

    let config = r#"
[demo]
root = "/tmp"
unknown_field = true

[demo.entries]
root = "."
"#;

    let dir = TempDir::new().expect("tempdir");
    write_workspace(dir.path(), config);

    let out = taplo_check(dir.path());
    assert!(!out.status.success(), "expected schema validation failure");
}

#[test]
fn taplo_rejects_config_without_entries() {
    if !taplo_available() {
        eprintln!("skip taplo_rejects_config_without_entries: taplo not on PATH");
        return;
    }

    let config = r#"
[demo]
root = "/tmp"
"#;

    let dir = TempDir::new().expect("tempdir");
    write_workspace(dir.path(), config);

    let out = taplo_check(dir.path());
    assert!(!out.status.success(), "expected missing entries to fail");
}

#[test]
fn taplo_workspace_is_not_excluded() {
    if !taplo_available() {
        eprintln!("skip taplo_workspace_is_not_excluded: taplo not on PATH");
        return;
    }

    let dir = TempDir::new().expect("tempdir");
    write_workspace(dir.path(), VALID_CONFIG);

    let taplo = find_taplo().unwrap();
    let out = Command::new(&taplo)
        .args(["check", "config.toml"])
        .current_dir(dir.path())
        .output()
        .expect("taplo check");

    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("excluded=1") && !stderr.contains("files=[]"),
        "config should not be excluded by taplo workspace rules:\n{stderr}"
    );
    assert!(stderr.contains("excluded=0") || out.status.success());
}

#[test]
fn taplo_toml_declares_section_rules() {
    assert!(TAPLO_TOML.contains(r#"keys = ["*"]"#));
    assert!(TAPLO_TOML.contains(r#"keys = ["*.entries.*"]"#));
    assert!(TAPLO_TOML.contains(r#"keys = ["*.worktrees.*"]"#));
    assert!(TAPLO_TOML.contains("config.schema.json#/definitions/EntryOptions"));
}

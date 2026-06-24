use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;
use tmux_manager::migrate;
use tmux_manager::model::Store;

fn load_toml(path: &std::path::Path) -> Store {
    let raw = fs::read_to_string(path).unwrap();
    toml::from_str(&raw).unwrap()
}

#[test]
fn migrate_legacy_json_to_toml() {
    let dir = TempDir::new().unwrap();
    let legacy_dir = dir.path().join("tmux-manager-nodejs");
    fs::create_dir_all(&legacy_dir).unwrap();

    let legacy = legacy_dir.join("config.json");
    fs::write(
        &legacy,
        r#"{
  "configs": {
    "demo": {
      "entries": [
        { "entryName": "demo/root", "directory": "/tmp/demo" },
        { "entryName": "demo/web", "directory": "/tmp/demo/apps/web" }
      ],
      "windows": 2
    }
  }
}"#,
    )
    .unwrap();

    let target = dir.path().join("tmux-manager").join("config.toml");
    std::env::set_var("XDG_CONFIG_HOME", dir.path());

    migrate::migrate_from_legacy(&legacy, &target, false).unwrap();

    assert!(target.exists());
    assert!(!legacy.exists());
    assert!(legacy.with_extension("json.bak").exists());

    let store = load_toml(&target);
    let config = &store.configs["demo"];
    assert_eq!(
        config.root.as_deref(),
        Some(std::path::Path::new("/tmp/demo"))
    );
    assert_eq!(config.entries.get("root").unwrap().dir().as_os_str(), ".");
    assert_eq!(
        config.entries.get("web").unwrap().dir(),
        &PathBuf::from("apps/web")
    );

    std::env::remove_var("XDG_CONFIG_HOME");
}

#[test]
fn migrate_defaults_missing_windows_field() {
    let dir = TempDir::new().unwrap();
    let legacy_dir = dir.path().join("tmux-manager-nodejs");
    fs::create_dir_all(&legacy_dir).unwrap();

    let legacy = legacy_dir.join("config.json");
    fs::write(
        &legacy,
        r#"{
  "configs": {
    "demo": {
      "entries": [
        { "entryName": "demo/root", "directory": "/tmp/demo" }
      ]
    }
  }
}"#,
    )
    .unwrap();

    let target = dir.path().join("tmux-manager").join("config.toml");
    std::env::set_var("XDG_CONFIG_HOME", dir.path());

    migrate::migrate_from_legacy(&legacy, &target, false).unwrap();

    let store = load_toml(&target);
    assert_eq!(store.configs["demo"].windows, 2);

    std::env::remove_var("XDG_CONFIG_HOME");
}

#[test]
fn migrate_merges_into_existing_target() {
    let dir = TempDir::new().unwrap();
    let legacy_dir = dir.path().join("tmux-manager-nodejs");
    fs::create_dir_all(&legacy_dir).unwrap();

    fs::write(
        legacy_dir.join("config.json"),
        r#"{
  "configs": {
    "demo": {
      "entries": [
        { "entryName": "demo/root", "directory": "/tmp/demo" }
      ],
      "windows": 3
    }
  }
}"#,
    )
    .unwrap();

    let target_dir = dir.path().join("tmux-manager");
    fs::create_dir_all(&target_dir).unwrap();
    let target = target_dir.join("config.toml");
    fs::write(
        &target,
        r#"[other]
root = "/tmp/other"
windows = 1

[other.entries]
root = "."
"#,
    )
    .unwrap();

    std::env::set_var("XDG_CONFIG_HOME", dir.path());

    migrate::migrate_from_legacy(&legacy_dir.join("config.json"), &target, false).unwrap();

    let store = load_toml(&target);
    assert!(store.configs.contains_key("other"));
    assert_eq!(store.configs["demo"].windows, 3);

    std::env::remove_var("XDG_CONFIG_HOME");
}

#[test]
fn migrate_force_replaces_entire_store() {
    let dir = TempDir::new().unwrap();
    let legacy_dir = dir.path().join("tmux-manager-nodejs");
    fs::create_dir_all(&legacy_dir).unwrap();

    fs::write(
        legacy_dir.join("config.json"),
        r#"{
  "configs": {
    "demo": {
      "entries": [
        { "entryName": "demo/root", "directory": "/tmp/demo" }
      ],
      "windows": 1
    }
  }
}"#,
    )
    .unwrap();

    let target_dir = dir.path().join("tmux-manager");
    fs::create_dir_all(&target_dir).unwrap();
    let target = target_dir.join("config.toml");
    fs::write(
        &target,
        r#"[other]
root = "/tmp/other"
windows = 1

[other.entries]
root = "."
"#,
    )
    .unwrap();

    std::env::set_var("XDG_CONFIG_HOME", dir.path());

    migrate::migrate_from_legacy(&legacy_dir.join("config.json"), &target, true).unwrap();

    let store = load_toml(&target);
    assert!(!store.configs.contains_key("other"));
    assert!(store.configs.contains_key("demo"));

    std::env::remove_var("XDG_CONFIG_HOME");
}

#[test]
fn migrate_rejects_empty_legacy_configs() {
    let dir = TempDir::new().unwrap();
    let legacy_dir = dir.path().join("tmux-manager-nodejs");
    fs::create_dir_all(&legacy_dir).unwrap();
    fs::write(legacy_dir.join("config.json"), r#"{"configs":{}}"#).unwrap();

    let target_dir = dir.path().join("tmux-manager");
    fs::create_dir_all(&target_dir).unwrap();
    let target = target_dir.join("config.toml");

    let err =
        migrate::migrate_from_legacy(&legacy_dir.join("config.json"), &target, false).unwrap_err();
    assert!(err.to_string().contains("no projects"));
}

use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;
use tmux_manager::config::load_store;
use tmux_manager::migrate;

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

    let store = load_store().unwrap();
    let config = &store.configs["demo"];
    assert_eq!(config.root.as_deref(), Some(std::path::Path::new("/tmp/demo")));
    assert_eq!(config.entries.get("root").unwrap().dir().as_os_str(), ".");
    assert_eq!(
        config.entries.get("web").unwrap().dir(),
        &PathBuf::from("apps/web")
    );

    std::env::remove_var("XDG_CONFIG_HOME");
}

#[test]
fn migrate_refuses_existing_target_without_force() {
    let dir = TempDir::new().unwrap();
    let legacy_dir = dir.path().join("tmux-manager-nodejs");
    fs::create_dir_all(&legacy_dir).unwrap();
    fs::write(legacy_dir.join("config.json"), r#"{"configs":{}}"#).unwrap();

    let target_dir = dir.path().join("tmux-manager");
    fs::create_dir_all(&target_dir).unwrap();
    let target = target_dir.join("config.toml");
    fs::write(&target, "[empty]\nwindows = 1\nentries = {}\n").unwrap();

    let err = migrate::migrate_from_legacy(&legacy_dir.join("config.json"), &target, false)
        .unwrap_err();
    assert!(err.to_string().contains("already exists"));
}

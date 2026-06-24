use std::path::PathBuf;
use tmux_manager::model::{resolve_entry_dir, Config, Entry, Store};
use tmux_manager::paths::{expand_config_root, home_dir};

#[test]
fn resolve_dot_entry_uses_root_without_suffix() {
    let root = PathBuf::from("/home/rs10/developer/kuib-ai");
    let dir = resolve_entry_dir(Some(&root), PathBuf::from(".").as_path()).unwrap();
    assert_eq!(dir, PathBuf::from("/home/rs10/developer/kuib-ai"));
}

#[test]
fn resolve_tilde_entry_expands_home() {
    let home = home_dir().unwrap();
    let dir = resolve_entry_dir(None, PathBuf::from("~/foo").as_path()).unwrap();
    assert_eq!(dir, home.join("foo"));
}

#[test]
fn expand_config_root_relative_uses_home() {
    let home = home_dir().unwrap();
    assert_eq!(
        expand_config_root(PathBuf::from("developer/proj").as_path()).unwrap(),
        home.join("developer/proj")
    );
}

#[test]
fn resolve_relative_entry_joins_root() {
    let root = PathBuf::from("/home/rs10/developer/kuib-ai");
    let dir = resolve_entry_dir(Some(&root), PathBuf::from("kuib/apps/tui").as_path()).unwrap();
    assert_eq!(
        dir,
        PathBuf::from("/home/rs10/developer/kuib-ai/kuib/apps/tui")
    );
}

#[test]
fn session_name_includes_config_prefix() {
    let mut config = Config::new(Some(PathBuf::from("/home/rs10/developer/kuib-ai")), 2);
    config
        .entries
        .insert("kuib/tui".into(), Entry::from_dir("kuib/apps/tui".into()));

    let resolved = config.resolve_entries("kuib-ai").unwrap();
    assert_eq!(resolved[0].session_name, "kuib-ai/kuib/tui");
    assert_eq!(
        resolved[0].directory,
        PathBuf::from("/home/rs10/developer/kuib-ai/kuib/apps/tui")
    );
}

#[test]
fn store_roundtrips_through_toml() {
    let mut store = Store {
        configs: Default::default(),
    };
    let mut config = Config::new(Some(PathBuf::from("/tmp/root")), 2);
    config
        .entries
        .insert("web".into(), Entry::from_dir("apps/web".into()));
    store.configs.insert("demo".into(), config);

    let toml = toml::to_string_pretty(&store).unwrap();
    let parsed: Store = toml::from_str(&toml).unwrap();
    assert_eq!(parsed.configs["demo"].entries.len(), 1);
}

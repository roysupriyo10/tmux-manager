use std::path::PathBuf;
use tmux_manager::model::{
    Config, Entry, ResolveContext, ResolveOverrides, Store, WorktreeOverrides,
};

#[test]
fn worktree_config_roundtrips_through_toml() {
    let mut config = Config::new(Some("~/developer/figr-ai".into()), 2);
    config.worktree_parent = Some(".claude/worktrees".into());
    config.worktree_prefix = Some("worktree-{name}".into());
    config
        .entries
        .insert("root".into(), Entry::from_dir(".".into()));
    config.worktrees.insert(
        "feat".into(),
        WorktreeOverrides {
            windows: Some(1),
            root: Some("/tmp/checkout".into()),
            ..Default::default()
        },
    );

    let mut store = Store {
        configs: Default::default(),
    };
    store.configs.insert("figr-ai".into(), config);

    let toml = toml::to_string_pretty(&store).unwrap();
    let parsed: Store = toml::from_str(&toml).unwrap();
    let cfg = &parsed.configs["figr-ai"];
    assert_eq!(
        cfg.worktree_parent.as_deref(),
        Some(PathBuf::from(".claude/worktrees").as_path())
    );
    assert_eq!(cfg.worktrees["feat"].windows, Some(1));
}

#[test]
fn absolute_worktree_parent_appends_name() {
    let mut config = Config::new(Some("/proj".into()), 2);
    config.worktree_parent = Some("/wt-parent".into());
    config
        .entries
        .insert("app".into(), Entry::from_dir(".".into()));

    let ctx = ResolveContext {
        config_name: "p",
        worktree: Some("branch-a"),
        overrides: ResolveOverrides::default(),
    };
    let resolved = config.resolve_entries_with(&ctx).unwrap();
    assert_eq!(resolved[0].directory, PathBuf::from("/wt-parent/branch-a"));
}

#[test]
fn worktree_section_windows_override() {
    let mut config = Config::new(Some("/proj".into()), 4);
    config.worktree_parent = Some("wt".into());
    config
        .entries
        .insert("x".into(), Entry::from_dir(".".into()));
    config.worktrees.insert(
        "w".into(),
        WorktreeOverrides {
            windows: Some(1),
            ..Default::default()
        },
    );

    let ctx = ResolveContext {
        config_name: "p",
        worktree: Some("w"),
        overrides: ResolveOverrides::default(),
    };
    assert_eq!(
        config.resolve_entries_with(&ctx).unwrap()[0].windows,
        tmux_manager::model::WindowsSpec::Count(1)
    );
}

#[test]
fn cli_overrides_beat_config() {
    let mut config = Config::new(Some("/proj".into()), 2);
    config.worktree_parent = Some("wt".into());
    config.worktree_prefix = Some("wt-{name}".into());
    config
        .entries
        .insert("a".into(), Entry::from_dir(".".into()));

    let ctx = ResolveContext {
        config_name: "p",
        worktree: Some("n"),
        overrides: ResolveOverrides {
            worktree_prefix: Some("custom-{name}".into()),
            windows: Some(9),
            ..Default::default()
        },
    };
    let entry = config.resolve_entries_with(&ctx).unwrap().pop().unwrap();
    assert_eq!(entry.session_name, "p/custom-n/a");
    assert_eq!(entry.windows, tmux_manager::model::WindowsSpec::Count(9));
}

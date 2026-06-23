use tmux_manager::matcher::{find_matching_keys, normalize_session_name, resolve_entry_keys};

#[test]
fn normalize_replaces_dots() {
    assert_eq!(
        normalize_session_name("portfolios/supriyoroy.com/web"),
        "portfolios/supriyoroy_com/web"
    );
}

#[test]
fn glob_matches_supriyoroy_entries() {
    let keys = vec![
        "root".into(),
        "supriyoroy.com/root".into(),
        "supriyoroy.com/web".into(),
        "rupshadesign_com".into(),
    ];

    let matched = find_matching_keys("supriyoroy.com/*", &keys);
    assert_eq!(matched.len(), 2);
    assert!(matched.contains(&"supriyoroy.com/root".to_string()));
    assert!(matched.contains(&"supriyoroy.com/web".to_string()));
}

#[test]
fn underscore_pattern_matches_dotted_keys() {
    let keys = vec!["supriyoroy.com/web".into()];
    assert_eq!(
        find_matching_keys("supriyoroy_com/web", &keys),
        vec!["supriyoroy.com/web".to_string()]
    );
}

#[test]
fn fuzzy_matches_single_entry() {
    let keys = vec![
        "root".into(),
        "supriyoroy.com/web".into(),
        "supriyoroy.com/types".into(),
    ];

    assert_eq!(
        find_matching_keys("web", &keys),
        vec!["supriyoroy.com/web".to_string()]
    );
}

#[test]
fn resolve_dedupes_across_patterns() {
    let keys = vec!["a".into(), "b".into(), "c".into()];
    let (matched, unmatched) = resolve_entry_keys(&["a".into(), "b".into()], &keys);
    assert_eq!(matched, vec!["a", "b"]);
    assert!(unmatched.is_empty());
}

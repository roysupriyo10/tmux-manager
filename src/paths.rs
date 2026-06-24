use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

pub fn home_dir() -> Result<PathBuf> {
    dirs::home_dir().context("home directory not found")
}

/// Expand a path: `~` / `~/x` → home; absolute → as-is; relative → `base` or `$HOME`.
pub fn expand_path(path: &Path, base: Option<&Path>) -> Result<PathBuf> {
    let raw = path.to_string_lossy();

    if raw == "~" {
        return home_dir();
    }

    if let Some(rest) = raw.strip_prefix("~/") {
        return Ok(home_dir()?.join(rest));
    }

    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }

    match base {
        Some(base) => Ok(base.join(path)),
        None => Ok(home_dir()?.join(path)),
    }
}

/// Config `root`: bare relative paths resolve under `$HOME`.
pub fn expand_config_root(root: &Path) -> Result<PathBuf> {
    expand_path(root, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_tilde_alone() {
        let home = home_dir().unwrap();
        assert_eq!(expand_path(Path::new("~"), None).unwrap(), home);
    }

    #[test]
    fn expand_tilde_prefix() {
        let home = home_dir().unwrap();
        assert_eq!(
            expand_path(Path::new("~/developer/foo"), None).unwrap(),
            home.join("developer/foo")
        );
    }

    #[test]
    fn expand_absolute_unchanged() {
        assert_eq!(
            expand_path(Path::new("/tmp/abs"), None).unwrap(),
            PathBuf::from("/tmp/abs")
        );
    }

    #[test]
    fn expand_relative_uses_base() {
        assert_eq!(
            expand_path(Path::new("apps/web"), Some(Path::new("/proj"))).unwrap(),
            PathBuf::from("/proj/apps/web")
        );
    }

    #[test]
    fn expand_relative_without_base_uses_home() {
        let home = home_dir().unwrap();
        assert_eq!(
            expand_path(Path::new("developer/foo"), None).unwrap(),
            home.join("developer/foo")
        );
    }
}

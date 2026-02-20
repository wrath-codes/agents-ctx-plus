use std::path::{Path, PathBuf};

/// Walk upwards from `start` until a `.zenith` directory is found.
#[must_use]
pub fn find_project_root(start: &Path) -> Option<PathBuf> {
    let mut current = start.to_path_buf();
    loop {
        if current.join(".zenith").is_dir() {
            return Some(current);
        }
        if !current.pop() {
            return None;
        }
    }
}

/// Find a project root by walking up first, then checking direct child directories.
///
/// Child-directory fallback is useful in monorepos where the CLI is run from a
/// parent workspace and the Zenith project is nested one level below.
#[must_use]
pub fn find_project_root_or_child(start: &Path) -> Option<PathBuf> {
    if let Some(root) = find_project_root(start) {
        return Some(root);
    }

    let mut candidates = Vec::new();
    let entries = std::fs::read_dir(start).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() && path.join(".zenith").is_dir() {
            candidates.push(path);
        }
    }

    if candidates.len() == 1 {
        candidates.into_iter().next()
    } else {
        None
    }
}

/// Find a single direct child directory that is a Zenith project.
#[must_use]
pub fn find_single_child_project_root(start: &Path) -> Option<PathBuf> {
    let mut candidates = Vec::new();
    let entries = std::fs::read_dir(start).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() && path.join(".zenith").is_dir() {
            candidates.push(path);
        }
    }

    (candidates.len() == 1).then(|| candidates.remove(0))
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::{find_project_root, find_project_root_or_child, find_single_child_project_root};

    #[test]
    fn finds_project_root_in_current_directory() {
        let temp = TempDir::new().expect("tempdir should create");
        std::fs::create_dir(temp.path().join(".zenith")).expect(".zenith should create");

        let found = find_project_root(temp.path());
        assert_eq!(found.as_deref(), Some(temp.path()));
    }

    #[test]
    fn finds_project_root_in_parent_directory() {
        let temp = TempDir::new().expect("tempdir should create");
        std::fs::create_dir(temp.path().join(".zenith")).expect(".zenith should create");
        std::fs::create_dir_all(temp.path().join("a/b/c")).expect("nested dirs should create");

        let deep = temp.path().join("a/b/c");
        let found = find_project_root(&deep);
        assert_eq!(found.as_deref(), Some(temp.path()));
    }

    #[test]
    fn returns_none_when_not_found() {
        let temp = TempDir::new().expect("tempdir should create");
        std::fs::create_dir_all(temp.path().join("a/b/c")).expect("nested dirs should create");

        let found = find_project_root(&temp.path().join("a/b/c"));
        assert!(found.is_none());
    }

    #[test]
    fn finds_project_root_in_single_child_directory() {
        let temp = TempDir::new().expect("tempdir should create");
        std::fs::create_dir_all(temp.path().join("zenith/.zenith")).expect("dirs should create");

        let found = find_project_root_or_child(temp.path());
        assert_eq!(found.as_deref(), Some(temp.path().join("zenith").as_path()));
    }

    #[test]
    fn returns_none_when_multiple_child_roots_exist() {
        let temp = TempDir::new().expect("tempdir should create");
        std::fs::create_dir_all(temp.path().join("a/.zenith")).expect("dirs should create");
        std::fs::create_dir_all(temp.path().join("b/.zenith")).expect("dirs should create");

        let found = find_project_root_or_child(temp.path());
        assert!(found.is_none());
    }

    #[test]
    fn finds_single_child_root_only() {
        let temp = TempDir::new().expect("tempdir should create");
        std::fs::create_dir_all(temp.path().join("zenith/.zenith")).expect("dirs should create");

        let found = find_single_child_project_root(temp.path());
        assert_eq!(found.as_deref(), Some(temp.path().join("zenith").as_path()));
    }
}

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

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::find_project_root;

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
}

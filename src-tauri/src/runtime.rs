use std::path::{Path, PathBuf};

pub const DSH_BIN: &str = "host/node_modules/@deepseek-ai/dsh/lib/bin.js";

fn is_complete_runtime(runtime: &Path) -> bool {
    runtime.join("node/node.exe").is_file() && runtime.join(DSH_BIN).is_file()
}

pub fn without_verbatim_prefix(path: &Path) -> PathBuf {
    let value = path.as_os_str().to_string_lossy();
    if let Some(network_path) = value.strip_prefix(r"\\?\UNC\") {
        return PathBuf::from(format!(r"\\{network_path}"));
    }
    if let Some(local_path) = value.strip_prefix(r"\\?\") {
        return PathBuf::from(local_path);
    }
    path.to_path_buf()
}

pub fn resolve_runtime_dir(
    resource_dir: &Path,
    project_root: &Path,
    development: bool,
) -> Result<PathBuf, String> {
    let bundled = resource_dir.join("runtime");
    if is_complete_runtime(&bundled) {
        return Ok(bundled);
    }

    let workspace = project_root.join("runtime");
    if development && is_complete_runtime(&workspace) {
        return Ok(workspace);
    }

    Err(format!("DSH runtime is missing from {}", bundled.display()))
}

#[cfg(test)]
mod tests {
    use super::{resolve_runtime_dir, without_verbatim_prefix};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_ID: AtomicU64 = AtomicU64::new(0);

    struct TestDir(PathBuf);

    impl TestDir {
        fn new() -> Self {
            let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "dsh-studio-runtime-test-{}-{id}",
                std::process::id()
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }

        fn create_runtime(&self, relative_root: &str) -> PathBuf {
            let runtime = self.0.join(relative_root).join("runtime");
            let dsh_bin = runtime.join("host/node_modules/@deepseek-ai/dsh/lib/bin.js");
            fs::create_dir_all(dsh_bin.parent().unwrap()).unwrap();
            fs::create_dir_all(runtime.join("node")).unwrap();
            fs::write(runtime.join("node/node.exe"), "node").unwrap();
            fs::write(dsh_bin, "dsh").unwrap();
            runtime
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn prefers_the_bundled_resource_runtime() {
        let test = TestDir::new();
        let expected = test.create_runtime("resources");
        let project = test.create_runtime("project");

        let actual = resolve_runtime_dir(
            &test.path().join("resources"),
            project.parent().unwrap(),
            true,
        )
        .unwrap();

        assert_eq!(actual, expected);
    }

    #[test]
    fn development_falls_back_to_the_workspace_runtime() {
        let test = TestDir::new();
        let expected = test.create_runtime("project");

        let actual = resolve_runtime_dir(
            &test.path().join("missing-resources"),
            expected.parent().unwrap(),
            true,
        )
        .unwrap();

        assert_eq!(actual, expected);
    }

    #[test]
    fn production_reports_the_missing_runtime_path() {
        let test = TestDir::new();
        let resources = test.path().join("resources");

        let error = resolve_runtime_dir(&resources, test.path(), false).unwrap_err();

        assert!(error.contains("runtime"));
        assert!(error.contains(resources.to_string_lossy().as_ref()));
    }

    #[test]
    fn removes_the_windows_verbatim_prefix_before_passing_paths_to_node() {
        assert_eq!(
            without_verbatim_prefix(Path::new(r"\\?\D:\dev\dsh-studio\runtime\host\bin.js")),
            Path::new(r"D:\dev\dsh-studio\runtime\host\bin.js")
        );
        assert_eq!(
            without_verbatim_prefix(Path::new(r"D:\dev\runtime\node.exe")),
            Path::new(r"D:\dev\runtime\node.exe")
        );
        assert_eq!(
            without_verbatim_prefix(Path::new(r"\\?\UNC\server\share\node.exe")),
            Path::new(r"\\server\share\node.exe")
        );
    }
}

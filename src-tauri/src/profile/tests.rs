use super::migrate_legacy_web_profile;
use serde_json::{json, Value};
use std::fs;
use std::os::windows::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

struct TestHome(PathBuf);

impl TestHome {
    fn new() -> Self {
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "dsh-studio-profile-test-{}-{id}",
            std::process::id()
        ));
        fs::create_dir_all(path.join("profiles/web")).unwrap();
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }

    fn manifest_path(&self) -> PathBuf {
        self.0.join("profiles/web/package.json")
    }

    fn write_manifest(&self, value: &Value) {
        fs::write(
            self.manifest_path(),
            format!("{}\n", serde_json::to_string_pretty(value).unwrap()),
        )
        .unwrap();
    }

    fn read_manifest(&self) -> Value {
        serde_json::from_slice(&fs::read(self.manifest_path()).unwrap()).unwrap()
    }
}

impl Drop for TestHome {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn bundled_manifest(extra_dependency: Option<(&str, &str)>) -> Value {
    let mut dependencies =
        serde_json::Map::from_iter([("@linxin666/dsh-web-ui-all".to_string(), json!("0.1.11"))]);
    let mut bundles = vec![
        json!("@deepseek-ai/dsh-base"),
        json!("@deepseek-ai/dsh-web-app"),
        json!("@linxin666/dsh-web-ui-all"),
    ];
    if let Some((name, version)) = extra_dependency {
        dependencies.insert(name.to_string(), json!(version));
        bundles.push(json!(name));
    }
    json!({
        "name": "dsh-profile-web",
        "private": true,
        "dependencies": dependencies,
        "dsh": { "profile": { "bundles": bundles } }
    })
}

#[test]
fn removes_the_legacy_bundled_integration_and_its_profile_cache() {
    let home = TestHome::new();
    home.write_manifest(&bundled_manifest(None));
    fs::create_dir_all(home.path().join("profiles/web/node_modules/stale")).unwrap();
    fs::write(
        home.path().join("profiles/web/node_modules/stale/file.js"),
        "x",
    )
    .unwrap();
    fs::write(
        home.path().join("profiles/web/pnpm-lock.yaml"),
        "lockfileVersion: 9",
    )
    .unwrap();
    fs::write(
        home.path().join("profiles/web/.npmrc"),
        "node-linker=hoisted",
    )
    .unwrap();

    let migration = migrate_legacy_web_profile(home.path()).unwrap();
    let manifest = home.read_manifest();

    assert!(migration.changed);
    assert!(migration.cleared_bundled_cache);
    assert_eq!(manifest["dependencies"], json!({}));
    assert_eq!(
        manifest["dsh"]["profile"]["bundles"],
        json!(["@deepseek-ai/dsh-base", "@deepseek-ai/dsh-web-app"])
    );
    assert!(!home.path().join("profiles/web/node_modules").exists());
    assert!(!home.path().join("profiles/web/pnpm-lock.yaml").exists());
    assert!(!home.path().join("profiles/web/.npmrc").exists());
}

#[test]
fn preserves_user_plugins_and_their_cache() {
    let home = TestHome::new();
    home.write_manifest(&bundled_manifest(Some(("@example/user-plugin", "1.2.3"))));
    fs::create_dir_all(
        home.path()
            .join("profiles/web/node_modules/@example/user-plugin"),
    )
    .unwrap();

    let migration = migrate_legacy_web_profile(home.path()).unwrap();
    let manifest = home.read_manifest();

    assert!(migration.changed);
    assert!(!migration.cleared_bundled_cache);
    assert_eq!(
        manifest["dependencies"],
        json!({ "@example/user-plugin": "1.2.3" })
    );
    assert_eq!(
        manifest["dsh"]["profile"]["bundles"],
        json!([
            "@deepseek-ai/dsh-base",
            "@deepseek-ai/dsh-web-app",
            "@example/user-plugin"
        ])
    );
    assert!(home.path().join("profiles/web/node_modules").exists());
}

#[test]
fn leaves_an_official_profile_byte_for_byte_unchanged() {
    let home = TestHome::new();
    let original = "{\n  \"name\": \"dsh-profile-web\",\n  \"dependencies\": {},\n  \"dsh\": { \"profile\": { \"bundles\": [\"@deepseek-ai/dsh-base\", \"@deepseek-ai/dsh-web-app\"] } }\n}\n";
    fs::write(home.manifest_path(), original).unwrap();

    let migration = migrate_legacy_web_profile(home.path()).unwrap();

    assert!(!migration.changed);
    assert_eq!(fs::read_to_string(home.manifest_path()).unwrap(), original);
}

#[test]
fn invalid_json_returns_an_error_without_overwriting_the_manifest() {
    let home = TestHome::new();
    fs::write(home.manifest_path(), "{ definitely not json").unwrap();

    let error = migrate_legacy_web_profile(home.path()).unwrap_err();

    assert!(error.contains("parse"));
    assert_eq!(
        fs::read_to_string(home.manifest_path()).unwrap(),
        "{ definitely not json"
    );
}

#[test]
fn recovers_an_interrupted_manifest_backup_before_migrating() {
    let home = TestHome::new();
    let backup = home.manifest_path().with_extension("json.4242.backup");
    fs::write(
        &backup,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&bundled_manifest(None)).unwrap()
        ),
    )
    .unwrap();

    let migration = migrate_legacy_web_profile(home.path()).unwrap();

    assert!(migration.changed);
    assert!(home.manifest_path().exists());
    assert!(!backup.exists());
    assert_eq!(home.read_manifest()["dependencies"], json!({}));
}

#[test]
fn a_fresh_home_without_a_web_profile_needs_no_migration() {
    let home = TestHome::new();
    fs::remove_dir_all(home.path().join("profiles")).unwrap();

    let migration = migrate_legacy_web_profile(home.path()).unwrap();

    assert!(!migration.changed);
    assert!(!migration.cleared_bundled_cache);
}

#[test]
fn preserves_unknown_user_created_manifest_backups() {
    let home = TestHome::new();
    home.write_manifest(&json!({ "dependencies": {} }));
    let manual = home.manifest_path().with_extension("json.manual.backup");
    fs::write(&manual, "user backup").unwrap();

    migrate_legacy_web_profile(home.path()).unwrap();

    assert_eq!(fs::read_to_string(manual).unwrap(), "user backup");
}

#[test]
fn restores_a_valid_owned_backup_when_the_live_manifest_is_corrupt() {
    let home = TestHome::new();
    fs::write(home.manifest_path(), "{ corrupt live json").unwrap();
    let backup = home.manifest_path().with_extension("json.migration.backup");
    fs::write(
        &backup,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&bundled_manifest(None)).unwrap()
        ),
    )
    .unwrap();

    let migration = migrate_legacy_web_profile(home.path()).unwrap();

    assert!(migration.changed);
    assert!(!backup.exists());
    assert_eq!(home.read_manifest()["dependencies"], json!({}));
}

#[test]
fn cache_cleanup_failure_does_not_abort_a_committed_migration() {
    let home = TestHome::new();
    home.write_manifest(&bundled_manifest(None));
    let locked_path = home.path().join("profiles/web/node_modules/locked.js");
    fs::create_dir_all(locked_path.parent().unwrap()).unwrap();
    fs::write(&locked_path, "locked").unwrap();
    let _lock = fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&locked_path)
        .unwrap();

    let migration = migrate_legacy_web_profile(home.path()).unwrap();

    assert!(migration.changed);
    assert!(migration.cleared_bundled_cache);
    assert_eq!(home.read_manifest()["dependencies"], json!({}));
}

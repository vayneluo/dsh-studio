use super::{
    acquire_upgrade_lock, finish_failed_upgrade, reconcile_managed_web_profile,
    reconcile_managed_web_profile_with, recover_interrupted_upgrade, UpgradeResult,
};
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

struct Fixture {
    root: PathBuf,
    seed: PathBuf,
    home: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "dsh-studio-profile-upgrade-test-{}-{id}",
            std::process::id()
        ));
        let seed = root.join("seed");
        let home = root.join("home");
        let plugins = [
            (
                "dsh-at-file",
                "0.6.0",
                "github:omdsh-dev/dsh-at-file#e579d0deb2295d5fea37a89244f8d584999be850",
            ),
            ("dsh-better-sidebar", "0.12.1", "0.12.1"),
            ("dshmarket", "1.2.2", "1.2.2"),
            ("dsh-message-edit", "0.2.1", "0.2.1"),
        ];
        fs::create_dir_all(seed.join("node_modules")).unwrap();
        fs::create_dir_all(home.join("profiles/web")).unwrap();
        let dependencies = plugins
            .iter()
            .map(|(name, _, spec)| ((*name).to_string(), json!(spec)))
            .collect::<serde_json::Map<_, _>>();
        fs::write(
            seed.join("package.json"),
            serde_json::to_vec_pretty(&json!({
                "dependencies": dependencies,
                "dsh": { "profile": { "bundles": [
                    "@deepseek-ai/dsh-base",
                    "@deepseek-ai/dsh-web-app",
                    "dsh-at-file",
                    "dsh-better-sidebar",
                    "dshmarket",
                    "dsh-message-edit"
                ] } }
            }))
            .unwrap(),
        )
        .unwrap();
        fs::write(
            seed.join(".dsh-studio-managed.json"),
            b"{\"catalogVersion\":1}\n",
        )
        .unwrap();
        for (package, version, _) in plugins {
            let directory = seed.join("node_modules").join(package);
            fs::create_dir_all(&directory).unwrap();
            let package_json = if package == "dshmarket" {
                format!(
                    "{{\"name\":\"{package}\",\"version\":\"{version}\",\"dependencies\":{{\"managed-runtime\":\"1.0.0\"}}}}"
                )
            } else {
                format!("{{\"name\":\"{package}\",\"version\":\"{version}\"}}")
            };
            fs::write(directory.join("package.json"), package_json).unwrap();
            fs::write(directory.join("index.js"), format!("{package}-new")).unwrap();
        }
        let runtime = seed.join("node_modules/managed-runtime");
        fs::create_dir_all(&runtime).unwrap();
        fs::write(
            runtime.join("package.json"),
            "{\"name\":\"managed-runtime\",\"version\":\"1.0.0\"}",
        )
        .unwrap();
        fs::write(runtime.join("index.js"), "managed-runtime-new").unwrap();
        Self { root, seed, home }
    }

    fn profile(&self) -> PathBuf {
        self.home.join("profiles/web")
    }

    fn write_manifest(&self, dependencies: Value, bundles: Value) {
        fs::write(
            self.profile().join("package.json"),
            serde_json::to_vec_pretty(&json!({
                "name": "dsh-profile-web",
                "private": true,
                "dependencies": dependencies,
                "dsh": { "profile": { "bundles": bundles } }
            }))
            .unwrap(),
        )
        .unwrap();
    }

    fn read_manifest(&self) -> Value {
        serde_json::from_slice(&fs::read(self.profile().join("package.json")).unwrap()).unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn upgrades_a_bare_existing_profile_without_touching_user_data() {
    let fixture = Fixture::new();
    fixture.write_manifest(
        json!({}),
        json!(["@deepseek-ai/dsh-base", "@deepseek-ai/dsh-web-app"]),
    );
    fs::write(fixture.home.join("settings.yaml"), "model: preserved").unwrap();

    let result = reconcile_managed_web_profile(&fixture.seed, &fixture.home).unwrap();
    let manifest = fixture.read_manifest();

    assert_eq!(result, UpgradeResult::Upgraded);
    assert_eq!(manifest["dependencies"]["dshmarket"], "1.2.2");
    assert_eq!(
        manifest["dsh"]["profile"]["bundles"],
        json!([
            "@deepseek-ai/dsh-base",
            "@deepseek-ai/dsh-web-app",
            "dsh-at-file",
            "dsh-better-sidebar",
            "dshmarket",
            "dsh-message-edit"
        ])
    );
    assert_eq!(
        fs::read_to_string(fixture.profile().join("node_modules/dshmarket/index.js")).unwrap(),
        "dshmarket-new"
    );
    assert_eq!(
        fs::read_to_string(fixture.home.join("settings.yaml")).unwrap(),
        "model: preserved"
    );
}

#[test]
fn preserves_user_plugins_while_refreshing_managed_packages() {
    let fixture = Fixture::new();
    fixture.write_manifest(
        json!({ "user-plugin": "2.0.0", "dshmarket": "0.1.0" }),
        json!([
            "@deepseek-ai/dsh-base",
            "@deepseek-ai/dsh-web-app",
            "user-plugin",
            "dshmarket"
        ]),
    );
    fs::create_dir_all(fixture.profile().join("node_modules/user-plugin")).unwrap();
    fs::write(
        fixture.profile().join("node_modules/user-plugin/index.js"),
        "user-preserved",
    )
    .unwrap();
    fs::create_dir_all(fixture.profile().join("node_modules/dshmarket")).unwrap();
    fs::write(
        fixture.profile().join("node_modules/dshmarket/index.js"),
        "managed-old",
    )
    .unwrap();

    reconcile_managed_web_profile(&fixture.seed, &fixture.home).unwrap();
    let manifest = fixture.read_manifest();

    assert_eq!(manifest["dependencies"]["user-plugin"], "2.0.0");
    assert_eq!(manifest["dependencies"]["dshmarket"], "1.2.2");
    assert_eq!(
        manifest["dsh"]["profile"]["bundles"],
        json!([
            "@deepseek-ai/dsh-base",
            "@deepseek-ai/dsh-web-app",
            "dsh-at-file",
            "dsh-better-sidebar",
            "dshmarket",
            "dsh-message-edit",
            "user-plugin"
        ])
    );
    assert_eq!(
        fs::read_to_string(fixture.profile().join("node_modules/user-plugin/index.js")).unwrap(),
        "user-preserved"
    );
    assert_eq!(
        fs::read_to_string(fixture.profile().join("node_modules/dshmarket/index.js")).unwrap(),
        "dshmarket-new"
    );
}

#[test]
fn a_current_managed_profile_is_idempotent() {
    let fixture = Fixture::new();
    fixture.write_manifest(
        json!({}),
        json!(["@deepseek-ai/dsh-base", "@deepseek-ai/dsh-web-app"]),
    );

    assert_eq!(
        reconcile_managed_web_profile(&fixture.seed, &fixture.home).unwrap(),
        UpgradeResult::Upgraded
    );
    assert_eq!(
        reconcile_managed_web_profile(&fixture.seed, &fixture.home).unwrap(),
        UpgradeResult::AlreadyCurrent
    );
}

#[test]
fn refreshes_a_managed_package_with_stale_metadata() {
    let fixture = Fixture::new();
    fixture.write_manifest(
        json!({}),
        json!(["@deepseek-ai/dsh-base", "@deepseek-ai/dsh-web-app"]),
    );
    reconcile_managed_web_profile(&fixture.seed, &fixture.home).unwrap();
    fs::write(
        fixture
            .profile()
            .join("node_modules/dshmarket/package.json"),
        r#"{"name":"dshmarket","version":"0.0.0"}"#,
    )
    .unwrap();
    fs::write(
        fixture.profile().join("node_modules/dshmarket/index.js"),
        "stale",
    )
    .unwrap();

    assert_eq!(
        reconcile_managed_web_profile(&fixture.seed, &fixture.home).unwrap(),
        UpgradeResult::Upgraded
    );
    assert_eq!(
        fs::read_to_string(fixture.profile().join("node_modules/dshmarket/index.js")).unwrap(),
        "dshmarket-new"
    );
}

#[test]
fn preserves_a_user_dependency_that_collides_with_the_seed_closure() {
    let fixture = Fixture::new();
    fixture.write_manifest(
        json!({ "managed-runtime": "9.0.0" }),
        json!([
            "@deepseek-ai/dsh-base",
            "@deepseek-ai/dsh-web-app",
            "managed-runtime"
        ]),
    );
    fs::create_dir_all(fixture.profile().join("node_modules/managed-runtime")).unwrap();
    fs::write(
        fixture
            .profile()
            .join("node_modules/managed-runtime/package.json"),
        r#"{"name":"managed-runtime","version":"9.0.0"}"#,
    )
    .unwrap();
    fs::write(
        fixture
            .profile()
            .join("node_modules/managed-runtime/index.js"),
        "user-runtime",
    )
    .unwrap();

    reconcile_managed_web_profile(&fixture.seed, &fixture.home).unwrap();

    assert_eq!(
        fixture.read_manifest()["dependencies"]["managed-runtime"],
        "9.0.0"
    );
    assert_eq!(
        fs::read_to_string(
            fixture
                .profile()
                .join("node_modules/managed-runtime/index.js")
        )
        .unwrap(),
        "user-runtime"
    );
    assert_eq!(
        fs::read_to_string(
            fixture
                .profile()
                .join("node_modules/dshmarket/node_modules/managed-runtime/index.js")
        )
        .unwrap(),
        "managed-runtime-new"
    );
}

#[test]
fn recovers_an_interrupted_package_swap_before_retrying() {
    let fixture = Fixture::new();
    let work = fixture.profile().join(".dsh-studio-managed-upgrade");
    let backup = work.join("backup/node_modules/dshmarket");
    let active = fixture.profile().join("node_modules/dshmarket");
    fs::create_dir_all(&backup).unwrap();
    fs::write(backup.join("index.js"), "old-recoverable").unwrap();
    fs::create_dir_all(&active).unwrap();
    fs::write(active.join("index.js"), "partial-new").unwrap();

    recover_interrupted_upgrade(&fixture.profile()).unwrap();

    assert_eq!(
        fs::read_to_string(active.join("index.js")).unwrap(),
        "old-recoverable"
    );
    assert!(!work.exists());
}

#[test]
fn serializes_two_profile_migrations_with_an_os_lock() {
    use std::sync::mpsc;
    use std::time::Duration;

    let fixture = Fixture::new();
    let profile = fixture.profile();
    let first = acquire_upgrade_lock(&profile).unwrap();
    let (sender, receiver) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        let second = acquire_upgrade_lock(&profile).unwrap();
        sender.send(()).unwrap();
        drop(second);
    });

    assert!(receiver.recv_timeout(Duration::from_millis(150)).is_err());
    drop(first);
    receiver.recv_timeout(Duration::from_secs(2)).unwrap();
    worker.join().unwrap();
}

#[test]
fn rolls_back_packages_when_the_manifest_commit_fails() {
    let fixture = Fixture::new();
    fixture.write_manifest(
        json!({ "dshmarket": "0.1.0" }),
        json!([
            "@deepseek-ai/dsh-base",
            "@deepseek-ai/dsh-web-app",
            "dshmarket"
        ]),
    );
    let manifest_before = fs::read(fixture.profile().join("package.json")).unwrap();
    let active = fixture.profile().join("node_modules/dshmarket");
    fs::create_dir_all(&active).unwrap();
    fs::write(active.join("index.js"), "managed-old").unwrap();

    let error = reconcile_managed_web_profile_with(&fixture.seed, &fixture.home, |_, _| {
        Err("injected manifest failure".to_string())
    })
    .unwrap_err();

    assert!(error.contains("injected manifest failure"));
    assert_eq!(fs::read(active.join("index.js")).unwrap(), b"managed-old");
    assert_eq!(
        fs::read(fixture.profile().join("package.json")).unwrap(),
        manifest_before
    );
    assert!(!fixture
        .profile()
        .join(".dsh-studio-managed-upgrade")
        .exists());
}

#[test]
fn preserves_recovery_backups_when_a_rollback_is_incomplete() {
    let fixture = Fixture::new();
    let backup = fixture
        .profile()
        .join(".dsh-studio-managed-upgrade/backup/node_modules/dshmarket");
    fs::create_dir_all(&backup).unwrap();
    fs::write(backup.join("index.js"), "recoverable").unwrap();

    let error =
        finish_failed_upgrade(&fixture.profile(), "rollback failed".to_string()).unwrap_err();

    assert!(error.contains("preserved for recovery"));
    assert_eq!(
        fs::read_to_string(backup.join("index.js")).unwrap(),
        "recoverable"
    );
}

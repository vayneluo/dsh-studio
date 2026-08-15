use super::{copy_physical_tree, seed_new_web_profile, seed_with_copy, SeedResult};
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

const PLUGINS: [(&str, &str, &str); 5] = [
    (
        "dsh-at-file",
        "0.6.0",
        "github:omdsh-dev/dsh-at-file#e579d0deb2295d5fea37a89244f8d584999be850",
    ),
    ("@liustack/modlens", "3.16.6", "3.16.6"),
    ("dsh-better-sidebar", "0.12.1", "0.12.1"),
    ("dshmarket", "1.2.2", "1.2.2"),
    ("dsh-message-edit", "0.2.1", "0.2.1"),
];

struct SeedFixture {
    root: PathBuf,
}

impl SeedFixture {
    fn new() -> Self {
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "dsh-studio-profile-seed-test-{}-{id}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        Self { root }
    }

    fn seed(&self) -> PathBuf {
        self.root.join("seed/profiles/web")
    }

    fn home(&self) -> PathBuf {
        self.root.join("home")
    }

    fn destination(&self) -> PathBuf {
        self.home().join("profiles/web")
    }

    fn staging(&self) -> PathBuf {
        self.home().join("profiles/.web.dsh-studio-seed")
    }

    fn write_valid_seed(&self) {
        let seed = self.seed();
        fs::create_dir_all(&seed).unwrap();
        let dependencies = serde_json::Map::from_iter(
            PLUGINS
                .iter()
                .map(|(name, _, spec)| ((*name).to_string(), json!(spec))),
        );
        let manifest = json!({
            "name": "dsh-profile-web",
            "private": true,
            "dependencies": dependencies,
            "dsh": {
                "profile": {
                    "bundles": [
                        "@deepseek-ai/dsh-base",
                        "@deepseek-ai/dsh-web-app",
                        "dsh-at-file",
                        "@liustack/modlens",
                        "dsh-better-sidebar",
                        "dshmarket",
                        "dsh-message-edit"
                    ]
                }
            }
        });
        fs::write(
            seed.join("package.json"),
            format!("{}\n", serde_json::to_string_pretty(&manifest).unwrap()),
        )
        .unwrap();

        for (name, version, _) in PLUGINS {
            let package = seed.join("node_modules").join(name);
            fs::create_dir_all(&package).unwrap();
            fs::write(
                package.join("package.json"),
                format!("{{\"name\":\"{name}\",\"version\":\"{version}\"}}"),
            )
            .unwrap();
            fs::write(package.join("index.js"), format!("// {name}\n")).unwrap();
        }
    }

    fn update_manifest(&self, update: impl FnOnce(&mut Value)) {
        let path = self.seed().join("package.json");
        let mut manifest: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        update(&mut manifest);
        fs::write(
            path,
            format!("{}\n", serde_json::to_string_pretty(&manifest).unwrap()),
        )
        .unwrap();
    }
}

impl Drop for SeedFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn seeds_a_missing_web_profile() {
    let fixture = SeedFixture::new();
    fixture.write_valid_seed();

    let result = seed_new_web_profile(&fixture.seed(), &fixture.home()).unwrap();

    assert_eq!(result, SeedResult::Seeded);
    assert!(fixture
        .destination()
        .join("node_modules/dshmarket/package.json")
        .is_file());
    assert!(!fixture.staging().exists());
}

#[test]
fn preserves_any_existing_web_profile_byte_for_byte() {
    let fixture = SeedFixture::new();
    let destination = fixture.destination();
    fs::create_dir_all(&destination).unwrap();
    fs::write(destination.join("user.txt"), b"owned by user").unwrap();

    let result = seed_new_web_profile(&fixture.seed(), &fixture.home()).unwrap();

    assert_eq!(result, SeedResult::SkippedExisting);
    assert_eq!(
        fs::read(destination.join("user.txt")).unwrap(),
        b"owned by user"
    );
    assert!(!fixture.staging().exists());
}

#[test]
fn rejects_an_invalid_seed_without_creating_a_profile() {
    let fixture = SeedFixture::new();
    fixture.write_valid_seed();
    fixture.update_manifest(|manifest| {
        manifest["dependencies"]["unexpected-plugin"] = json!("1.0.0");
    });

    let error = seed_new_web_profile(&fixture.seed(), &fixture.home()).unwrap_err();

    assert!(error.contains("dependencies"));
    assert!(!fixture.destination().exists());
    assert!(!fixture.staging().exists());
}

#[test]
fn rejects_an_installed_plugin_version_mismatch() {
    let fixture = SeedFixture::new();
    fixture.write_valid_seed();
    fs::write(
        fixture.seed().join("node_modules/dshmarket/package.json"),
        r#"{"name":"dshmarket","version":"9.9.9"}"#,
    )
    .unwrap();

    let error = seed_new_web_profile(&fixture.seed(), &fixture.home()).unwrap_err();

    assert!(error.contains("dshmarket"));
    assert!(error.contains("version"));
    assert!(!fixture.destination().exists());
}

#[test]
fn rejects_reparse_points_in_the_seed() {
    let fixture = SeedFixture::new();
    fixture.write_valid_seed();
    let external = fixture.root.join("external");
    fs::create_dir_all(&external).unwrap();
    let linked = fixture.seed().join("linked-dir");
    let output = Command::new("powershell.exe")
        .env("DSH_TEST_JUNCTION_LINK", &linked)
        .env("DSH_TEST_JUNCTION_TARGET", &external)
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "New-Item -ItemType Junction -Path $env:DSH_TEST_JUNCTION_LINK -Target $env:DSH_TEST_JUNCTION_TARGET | Out-Null",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "failed to create test junction: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let error = seed_new_web_profile(&fixture.seed(), &fixture.home()).unwrap_err();

    assert!(error.contains("reparse"));
    assert!(!fixture.destination().exists());
}

#[test]
fn a_failed_copy_removes_only_owned_staging() {
    let fixture = SeedFixture::new();
    fixture.write_valid_seed();

    let error = seed_with_copy(&fixture.seed(), &fixture.home(), |_, staging| {
        fs::create_dir_all(staging).unwrap();
        fs::write(staging.join("partial"), b"partial").unwrap();
        Err("injected copy failure".to_string())
    })
    .unwrap_err();

    assert!(error.contains("injected copy failure"));
    assert!(!fixture.destination().exists());
    assert!(!fixture.staging().exists());
}

#[test]
fn a_profile_created_during_copy_wins_without_being_overwritten() {
    let fixture = SeedFixture::new();
    fixture.write_valid_seed();
    let destination = fixture.destination();

    let result = seed_with_copy(&fixture.seed(), &fixture.home(), |seed, staging| {
        copy_physical_tree(seed, staging)?;
        fs::create_dir_all(&destination).unwrap();
        fs::write(destination.join("user.txt"), b"created concurrently").unwrap();
        Ok(())
    })
    .unwrap();

    assert_eq!(result, SeedResult::SkippedExisting);
    assert_eq!(
        fs::read(destination.join("user.txt")).unwrap(),
        b"created concurrently"
    );
    assert!(!fixture.staging().exists());
}

#[test]
fn validates_the_copied_staging_tree_before_publish() {
    let fixture = SeedFixture::new();
    fixture.write_valid_seed();

    let error = seed_with_copy(&fixture.seed(), &fixture.home(), |_, staging| {
        fs::create_dir_all(staging).unwrap();
        fs::write(staging.join("package.json"), b"{}").unwrap();
        Ok(())
    })
    .unwrap_err();

    assert!(error.contains("dependencies"));
    assert!(!fixture.destination().exists());
    assert!(!fixture.staging().exists());
}

#[test]
fn replaces_a_stale_owned_staging_directory() {
    let fixture = SeedFixture::new();
    fixture.write_valid_seed();
    fs::create_dir_all(fixture.staging()).unwrap();
    fs::write(fixture.staging().join("stale.txt"), b"stale").unwrap();

    let result = seed_new_web_profile(&fixture.seed(), &fixture.home()).unwrap();

    assert_eq!(result, SeedResult::Seeded);
    assert!(!fixture.destination().join("stale.txt").exists());
    assert!(!fixture.staging().exists());
}

use serde_json::Value;
use std::path::{Path, PathBuf};

const LEGACY_PACKAGES: [&str; 2] = [
    "@linxin666/dsh-web-ui-all",
    "@linxin666/dsh-skins",
];

#[derive(Debug, PartialEq, Eq)]
pub struct Migration {
    pub changed: bool,
    pub cleared_bundled_cache: bool,
}

fn path_error(action: &str, path: &Path, error: impl std::fmt::Display) -> String {
    format!("failed to {action} {}: {error}", path.display())
}

fn replace_manifest(path: &Path, contents: &[u8]) -> Result<(), String> {
    let temporary = path.with_extension(format!("json.{}.tmp", std::process::id()));
    let backup = path.with_extension(format!("json.{}.backup", std::process::id()));
    std::fs::write(&temporary, contents)
        .map_err(|error| path_error("write", &temporary, error))?;

    std::fs::rename(path, &backup).map_err(|error| path_error("back up", path, error))?;
    if let Err(error) = std::fs::rename(&temporary, path) {
        let _ = std::fs::rename(&backup, path);
        let _ = std::fs::remove_file(&temporary);
        return Err(path_error("replace", path, error));
    }
    std::fs::remove_file(&backup).map_err(|error| path_error("remove", &backup, error))?;
    Ok(())
}

fn remove_legacy_entries(manifest: &mut Value) -> Result<(bool, bool), String> {
    let Some(dependencies) = manifest
        .get_mut("dependencies")
        .and_then(Value::as_object_mut)
    else {
        return Ok((false, false));
    };

    let has_legacy = LEGACY_PACKAGES
        .iter()
        .any(|package| dependencies.contains_key(*package));
    if !has_legacy {
        return Ok((false, false));
    }

    let has_user_dependencies = dependencies
        .keys()
        .any(|package| !LEGACY_PACKAGES.contains(&package.as_str()));
    for package in LEGACY_PACKAGES {
        dependencies.remove(package);
    }

    if let Some(bundles) = manifest
        .pointer_mut("/dsh/profile/bundles")
        .and_then(Value::as_array_mut)
    {
        bundles.retain(|bundle| {
            bundle
                .as_str()
                .is_none_or(|package| !LEGACY_PACKAGES.contains(&package))
        });
    }

    Ok((true, !has_user_dependencies))
}

fn remove_if_present(path: PathBuf) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    if path.is_dir() {
        return std::fs::remove_dir_all(&path).map_err(|error| path_error("remove", &path, error));
    }
    std::fs::remove_file(&path).map_err(|error| path_error("remove", &path, error))
}

pub fn migrate_legacy_web_profile(dsh_home: &Path) -> Result<Migration, String> {
    let web_profile = dsh_home.join("profiles/web");
    let manifest_path = web_profile.join("package.json");
    if !manifest_path.exists() {
        return Ok(Migration {
            changed: false,
            cleared_bundled_cache: false,
        });
    }

    let source = std::fs::read(&manifest_path)
        .map_err(|error| path_error("read", &manifest_path, error))?;
    let mut manifest: Value = serde_json::from_slice(&source)
        .map_err(|error| path_error("parse", &manifest_path, error))?;
    let (changed, clear_cache) = remove_legacy_entries(&mut manifest)?;
    if !changed {
        return Ok(Migration {
            changed: false,
            cleared_bundled_cache: false,
        });
    }

    let mut rendered = serde_json::to_vec_pretty(&manifest)
        .map_err(|error| path_error("serialize", &manifest_path, error))?;
    rendered.push(b'\n');
    replace_manifest(&manifest_path, &rendered)?;

    if clear_cache {
        for path in [
            web_profile.join("node_modules"),
            web_profile.join("pnpm-lock.yaml"),
            web_profile.join(".npmrc"),
        ] {
            remove_if_present(path)?;
        }
    }

    Ok(Migration {
        changed: true,
        cleared_bundled_cache: clear_cache,
    })
}

#[cfg(test)]
mod tests;

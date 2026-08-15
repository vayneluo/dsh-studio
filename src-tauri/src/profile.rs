use serde_json::Value;
use std::ffi::OsStr;
use std::fs::OpenOptions;
use std::io::Write;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use windows_sys::Win32::Storage::FileSystem::{ReplaceFileW, REPLACEFILE_WRITE_THROUGH};

const LEGACY_PACKAGES: [&str; 2] = ["@linxin666/dsh-web-ui-all", "@linxin666/dsh-skins"];

#[derive(Debug, PartialEq, Eq)]
pub struct Migration {
    pub changed: bool,
    pub cleared_bundled_cache: bool,
}

fn path_error(action: &str, path: &Path, error: impl std::fmt::Display) -> String {
    format!("failed to {action} {}: {error}", path.display())
}

fn wide_path(path: &OsStr) -> Vec<u16> {
    path.encode_wide().chain(std::iter::once(0)).collect()
}

fn migration_backup_path(path: &Path) -> PathBuf {
    path.with_extension("json.migration.backup")
}

fn owned_manifest_backups(path: &Path) -> Result<Vec<PathBuf>, String> {
    let Some(parent) = path.parent() else {
        return Ok(Vec::new());
    };
    if !parent.exists() {
        return Ok(Vec::new());
    }
    let Some(file_name) = path.file_name().and_then(OsStr::to_str) else {
        return Ok(Vec::new());
    };
    let prefix = format!("{file_name}.");
    let migration_name = format!("{file_name}.migration.backup");
    let entries = std::fs::read_dir(parent).map_err(|error| path_error("scan", parent, error))?;
    Ok(entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|candidate| {
            candidate
                .file_name()
                .and_then(OsStr::to_str)
                .is_some_and(|name| {
                    if name == migration_name {
                        return true;
                    }
                    name.strip_prefix(&prefix)
                        .and_then(|suffix| suffix.strip_suffix(".backup"))
                        .is_some_and(|pid| {
                            !pid.is_empty()
                                && pid.chars().all(|character| character.is_ascii_digit())
                        })
                })
        })
        .collect())
}

fn write_synced(path: &Path, contents: &[u8]) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)
        .map_err(|error| path_error("write", path, error))?;
    file.write_all(contents)
        .map_err(|error| path_error("write", path, error))?;
    file.sync_all()
        .map_err(|error| path_error("sync", path, error))
}

fn replace_file(path: &Path, replacement: &Path, backup: Option<&Path>) -> std::io::Result<()> {
    let path_wide = wide_path(path.as_os_str());
    let replacement_wide = wide_path(replacement.as_os_str());
    let backup_wide = backup.map(|backup| wide_path(backup.as_os_str()));
    let backup_pointer = backup_wide
        .as_ref()
        .map_or(std::ptr::null(), |backup| backup.as_ptr());
    let replaced = unsafe {
        ReplaceFileW(
            path_wide.as_ptr(),
            replacement_wide.as_ptr(),
            backup_pointer,
            REPLACEFILE_WRITE_THROUGH,
            std::ptr::null(),
            std::ptr::null(),
        )
    };
    if replaced == 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

fn valid_json(path: &Path) -> bool {
    std::fs::read(path)
        .ok()
        .and_then(|contents| serde_json::from_slice::<Value>(&contents).ok())
        .is_some()
}

fn cleanup_owned_backups(backups: &[PathBuf]) {
    for backup in backups {
        let _ = std::fs::remove_file(backup);
    }
}

fn recover_manifest_backup(path: &Path) -> Result<(), String> {
    let mut backups = owned_manifest_backups(path)?;
    if backups.is_empty() {
        return Ok(());
    }

    if path.exists() && valid_json(path) {
        cleanup_owned_backups(&backups);
        return Ok(());
    }

    backups.sort_by_key(|backup| {
        std::fs::metadata(backup)
            .and_then(|metadata| metadata.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH)
    });
    let Some(valid_backup) = backups.iter().rev().find(|backup| valid_json(backup)) else {
        return Ok(());
    };

    if path.exists() {
        let recovery = path.with_extension(format!("json.{}.recovery.tmp", std::process::id()));
        let contents =
            std::fs::read(valid_backup).map_err(|error| path_error("read", valid_backup, error))?;
        write_synced(&recovery, &contents)?;
        if let Err(error) = replace_file(path, &recovery, None) {
            let _ = std::fs::remove_file(&recovery);
            return Err(path_error("recover", path, error));
        }
    } else {
        std::fs::rename(valid_backup, path)
            .map_err(|error| path_error("recover", valid_backup, error))?;
    }
    cleanup_owned_backups(&backups);
    Ok(())
}

fn replace_manifest(path: &Path, contents: &[u8]) -> Result<(), String> {
    let temporary = path.with_extension(format!("json.{}.tmp", std::process::id()));
    let backup = migration_backup_path(path);
    if backup.exists() {
        std::fs::remove_file(&backup).map_err(|error| path_error("remove", &backup, error))?;
    }
    write_synced(&temporary, contents)?;

    if let Err(error) = replace_file(path, &temporary, Some(&backup)) {
        let _ = std::fs::remove_file(&temporary);
        return Err(path_error("replace", path, error));
    }
    let _ = std::fs::remove_file(&backup);
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
    recover_manifest_backup(&manifest_path)?;
    if !manifest_path.exists() {
        return Ok(Migration {
            changed: false,
            cleared_bundled_cache: false,
        });
    }

    let source =
        std::fs::read(&manifest_path).map_err(|error| path_error("read", &manifest_path, error))?;
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
            let _ = remove_if_present(path);
        }
    }

    Ok(Migration {
        changed: true,
        cleared_bundled_cache: clear_cache,
    })
}

#[cfg(test)]
mod tests;

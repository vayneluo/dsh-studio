use serde_json::Value;
use std::fs;
use std::io::ErrorKind;
use std::os::windows::fs::MetadataExt;
use std::path::Path;
use windows_sys::Win32::Storage::FileSystem::{
    FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_REPARSE_POINT,
};

const STAGING_NAME: &str = ".web.dsh-studio-seed";
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
const BUNDLES: [&str; 7] = [
    "@deepseek-ai/dsh-base",
    "@deepseek-ai/dsh-web-app",
    "dsh-at-file",
    "@liustack/modlens",
    "dsh-better-sidebar",
    "dshmarket",
    "dsh-message-edit",
];

#[derive(Debug, PartialEq, Eq)]
pub enum SeedResult {
    Seeded,
    SkippedExisting,
}

fn path_error(action: &str, path: &Path, error: impl std::fmt::Display) -> String {
    format!("failed to {action} {}: {error}", path.display())
}

pub(crate) fn path_entry_exists(path: &Path) -> Result<bool, String> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(false),
        Err(error) => Err(path_error("inspect", path, error)),
    }
}

fn is_reparse_point(metadata: &fs::Metadata) -> bool {
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

fn audit_physical_tree(root: &Path) -> Result<(), String> {
    let mut pending = vec![root.to_path_buf()];
    while let Some(path) = pending.pop() {
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| path_error("inspect seed path", &path, error))?;
        if is_reparse_point(&metadata) {
            return Err(format!(
                "default profile seed contains a reparse point: {}",
                path.display()
            ));
        }
        if metadata.is_dir() {
            let entries =
                fs::read_dir(&path).map_err(|error| path_error("scan seed path", &path, error))?;
            for entry in entries {
                pending.push(
                    entry
                        .map_err(|error| path_error("scan seed path", &path, error))?
                        .path(),
                );
            }
        } else if !metadata.is_file() {
            return Err(format!(
                "default profile seed contains an unsupported path: {}",
                path.display()
            ));
        }
    }
    Ok(())
}

fn read_json(path: &Path) -> Result<Value, String> {
    let contents = fs::read(path).map_err(|error| path_error("read", path, error))?;
    serde_json::from_slice(&contents).map_err(|error| path_error("parse", path, error))
}

pub(crate) fn validate_default_profile(profile: &Path) -> Result<(), String> {
    audit_physical_tree(profile)?;
    let manifest_path = profile.join("package.json");
    let manifest = read_json(&manifest_path)?;
    let dependencies = manifest
        .get("dependencies")
        .and_then(Value::as_object)
        .ok_or_else(|| "default profile dependencies are missing".to_string())?;
    if dependencies.len() != PLUGINS.len()
        || PLUGINS
            .iter()
            .any(|(name, _, spec)| dependencies.get(*name).and_then(Value::as_str) != Some(*spec))
    {
        return Err("default profile dependencies do not match the fixed plugin catalog".into());
    }

    let bundles = manifest
        .pointer("/dsh/profile/bundles")
        .and_then(Value::as_array)
        .ok_or_else(|| "default profile bundles are missing".to_string())?;
    if bundles.len() != BUNDLES.len()
        || bundles
            .iter()
            .zip(BUNDLES)
            .any(|(actual, expected)| actual.as_str() != Some(expected))
    {
        return Err("default profile bundles do not match the expected seven-bundle order".into());
    }

    for (name, version, _) in PLUGINS {
        let package_path = profile.join("node_modules").join(name).join("package.json");
        let package = read_json(&package_path)?;
        if package.get("name").and_then(Value::as_str) != Some(name) {
            return Err(format!("installed package name mismatch for {name}"));
        }
        if package.get("version").and_then(Value::as_str) != Some(version) {
            return Err(format!(
                "{name} version mismatch: expected {version}, got {}",
                package
                    .get("version")
                    .and_then(Value::as_str)
                    .unwrap_or("<missing>")
            ));
        }
    }
    Ok(())
}

pub(crate) fn remove_owned_path(path: &Path) -> Result<(), String> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(path_error("inspect", path, error)),
    };
    if is_reparse_point(&metadata) {
        let is_directory = metadata.file_attributes() & FILE_ATTRIBUTE_DIRECTORY != 0;
        return if is_directory {
            fs::remove_dir(path)
        } else {
            fs::remove_file(path)
        }
        .map_err(|error| path_error("remove owned staging path", path, error));
    }
    if metadata.is_dir() {
        fs::remove_dir_all(path)
            .map_err(|error| path_error("remove owned staging directory", path, error))
    } else {
        fs::remove_file(path).map_err(|error| path_error("remove owned staging file", path, error))
    }
}

pub(crate) fn copy_physical_tree(source: &Path, destination: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(source)
        .map_err(|error| path_error("inspect copy source", source, error))?;
    if is_reparse_point(&metadata) {
        return Err(format!(
            "default profile seed contains a reparse point: {}",
            source.display()
        ));
    }
    if metadata.is_dir() {
        fs::create_dir(destination)
            .map_err(|error| path_error("create staging directory", destination, error))?;
        let entries =
            fs::read_dir(source).map_err(|error| path_error("scan copy source", source, error))?;
        for entry in entries {
            let entry = entry.map_err(|error| path_error("scan copy source", source, error))?;
            copy_physical_tree(&entry.path(), &destination.join(entry.file_name()))?;
        }
        return Ok(());
    }
    if metadata.is_file() {
        fs::copy(source, destination)
            .map_err(|error| path_error("copy seed file", source, error))?;
        return Ok(());
    }
    Err(format!(
        "default profile seed contains an unsupported path: {}",
        source.display()
    ))
}

fn cleanup_after_failure(staging: &Path, error: String) -> String {
    match remove_owned_path(staging) {
        Ok(()) => error,
        Err(cleanup_error) => format!("{error}; {cleanup_error}"),
    }
}

fn seed_with_copy<F>(seed: &Path, dsh_home: &Path, copy: F) -> Result<SeedResult, String>
where
    F: FnOnce(&Path, &Path) -> Result<(), String>,
{
    let profiles = dsh_home.join("profiles");
    let destination = profiles.join("web");
    if path_entry_exists(&destination)? {
        return Ok(SeedResult::SkippedExisting);
    }

    validate_default_profile(seed)?;
    fs::create_dir_all(&profiles)
        .map_err(|error| path_error("create profiles directory", &profiles, error))?;
    let staging = profiles.join(STAGING_NAME);
    remove_owned_path(&staging)?;

    let operation = (|| {
        copy(seed, &staging)?;
        validate_default_profile(&staging)?;
        if path_entry_exists(&destination)? {
            return Ok(SeedResult::SkippedExisting);
        }
        match fs::rename(&staging, &destination) {
            Ok(()) => Ok(SeedResult::Seeded),
            Err(_) if path_entry_exists(&destination)? => Ok(SeedResult::SkippedExisting),
            Err(error) => Err(path_error(
                "publish default web profile",
                &destination,
                error,
            )),
        }
    })();

    match operation {
        Ok(SeedResult::Seeded) => Ok(SeedResult::Seeded),
        Ok(SeedResult::SkippedExisting) => {
            remove_owned_path(&staging)?;
            Ok(SeedResult::SkippedExisting)
        }
        Err(error) => Err(cleanup_after_failure(&staging, error)),
    }
}

pub fn seed_new_web_profile(seed: &Path, dsh_home: &Path) -> Result<SeedResult, String> {
    seed_with_copy(seed, dsh_home, copy_physical_tree)
}

#[cfg(test)]
mod tests;

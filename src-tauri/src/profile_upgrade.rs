use crate::{profile, profile_seed};
use serde_json::{Map, Value};
use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs;
use std::io::ErrorKind;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::time::Duration;
use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_LOCK_VIOLATION, ERROR_SHARING_VIOLATION, GENERIC_READ, GENERIC_WRITE,
    HANDLE, INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::Storage::FileSystem::{CreateFileW, FILE_ATTRIBUTE_NORMAL, OPEN_ALWAYS};

const MARKER_NAME: &str = ".dsh-studio-managed.json";
const WORK_NAME: &str = ".dsh-studio-managed-upgrade";
const LOCK_NAME: &str = ".dsh-studio-managed.lock";
const LOCK_RETRY_COUNT: usize = 600;
const LOCK_RETRY_DELAY: Duration = Duration::from_millis(100);

#[derive(Debug, PartialEq, Eq)]
pub enum UpgradeResult {
    Upgraded,
    AlreadyCurrent,
}

#[derive(Clone, Debug)]
struct PackagePlacement {
    source: PathBuf,
    destination: PathBuf,
}

struct ManagedPackagePlan {
    top_level: Vec<PathBuf>,
    placements: Vec<PackagePlacement>,
}

struct ProfileUpgradeLock(HANDLE);

impl Drop for ProfileUpgradeLock {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}

fn wide_path(path: &OsStr) -> Vec<u16> {
    path.encode_wide().chain(std::iter::once(0)).collect()
}

fn acquire_upgrade_lock(profile_dir: &Path) -> Result<ProfileUpgradeLock, String> {
    let lock_path = profile_dir.join(LOCK_NAME);
    let lock_wide = wide_path(lock_path.as_os_str());
    for _ in 0..LOCK_RETRY_COUNT {
        let handle = unsafe {
            CreateFileW(
                lock_wide.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                0,
                std::ptr::null(),
                OPEN_ALWAYS,
                FILE_ATTRIBUTE_NORMAL,
                std::ptr::null_mut(),
            )
        };
        if handle != INVALID_HANDLE_VALUE {
            return Ok(ProfileUpgradeLock(handle));
        }
        let error = std::io::Error::last_os_error();
        let error_code = error.raw_os_error().map(|code| code as u32);
        if error_code != Some(ERROR_SHARING_VIOLATION) && error_code != Some(ERROR_LOCK_VIOLATION) {
            return Err(path_error("lock managed web profile", &lock_path, error));
        }
        std::thread::sleep(LOCK_RETRY_DELAY);
    }
    Err(format!(
        "timed out waiting to lock managed web profile {}",
        lock_path.display()
    ))
}

fn path_error(action: &str, path: &Path, error: impl std::fmt::Display) -> String {
    format!("failed to {action} {}: {error}", path.display())
}

fn read_json(path: &Path) -> Result<Value, String> {
    let contents = fs::read(path).map_err(|error| path_error("read", path, error))?;
    serde_json::from_slice(&contents).map_err(|error| path_error("parse", path, error))
}

fn package_paths(node_modules: &Path) -> Result<Vec<PathBuf>, String> {
    let mut packages = Vec::new();
    let entries = fs::read_dir(node_modules)
        .map_err(|error| path_error("scan managed packages", node_modules, error))?;
    for entry in entries {
        let entry =
            entry.map_err(|error| path_error("scan managed packages", node_modules, error))?;
        let name = entry.file_name();
        let name_text = name.to_string_lossy();
        if name_text.starts_with('.') {
            continue;
        }
        let metadata = fs::symlink_metadata(entry.path())
            .map_err(|error| path_error("inspect managed package", &entry.path(), error))?;
        if !metadata.is_dir() {
            return Err(format!(
                "managed package entry is not a physical directory: {}",
                entry.path().display()
            ));
        }
        if name_text.starts_with('@') {
            let scope_entries = fs::read_dir(entry.path())
                .map_err(|error| path_error("scan managed package scope", &entry.path(), error))?;
            for package in scope_entries {
                let package = package.map_err(|error| {
                    path_error("scan managed package scope", &entry.path(), error)
                })?;
                let package_name = package.file_name();
                if package_name.to_string_lossy().starts_with('.') {
                    continue;
                }
                let package_metadata = fs::symlink_metadata(package.path()).map_err(|error| {
                    path_error("inspect managed package", &package.path(), error)
                })?;
                if !package_metadata.is_dir() {
                    return Err(format!(
                        "managed package entry is not a physical directory: {}",
                        package.path().display()
                    ));
                }
                packages.push(PathBuf::from(&name).join(package_name));
            }
        } else {
            packages.push(PathBuf::from(name));
        }
    }
    packages.sort();
    Ok(packages)
}

fn package_name(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn package_dependency_names(package_dir: &Path) -> Result<Vec<String>, String> {
    let manifest = read_json(&package_dir.join("package.json"))?;
    let mut names = HashSet::new();
    for field in ["dependencies", "optionalDependencies", "peerDependencies"] {
        if let Some(dependencies) = manifest.get(field).and_then(Value::as_object) {
            names.extend(dependencies.keys().cloned());
        }
    }
    let mut names: Vec<_> = names.into_iter().collect();
    names.sort();
    Ok(names)
}

fn package_path(name: &str) -> PathBuf {
    name.split('/').collect()
}

fn managed_package_plan(
    seed_manifest: &Value,
    active_manifest: &Value,
    seed_node_modules: &Path,
) -> Result<ManagedPackagePlan, String> {
    let seed_direct: HashSet<&str> = dependencies(seed_manifest)?
        .keys()
        .map(String::as_str)
        .collect();
    let user_direct: HashSet<&str> = active_manifest
        .get("dependencies")
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|dependencies| dependencies.keys())
        .map(String::as_str)
        .filter(|name| !seed_direct.contains(name))
        .collect();
    let all_packages = package_paths(seed_node_modules)?;
    let available_names: HashSet<String> =
        all_packages.iter().map(|path| package_name(path)).collect();
    let collisions: HashSet<String> = user_direct
        .into_iter()
        .filter(|name| available_names.contains(*name))
        .map(str::to_string)
        .collect();
    let top_level: Vec<_> = all_packages
        .into_iter()
        .filter(|path| !collisions.contains(&package_name(path)))
        .collect();
    let mut placements: Vec<_> = top_level
        .iter()
        .cloned()
        .map(|path| PackagePlacement {
            source: path.clone(),
            destination: path,
        })
        .collect();
    let mut seen_destinations: HashSet<PathBuf> = placements
        .iter()
        .map(|item| item.destination.clone())
        .collect();
    let mut index = 0;
    while index < placements.len() {
        let placement = placements[index].clone();
        index += 1;
        let source_dir = seed_node_modules.join(&placement.source);
        for dependency in package_dependency_names(&source_dir)? {
            if !collisions.contains(&dependency) {
                continue;
            }
            let dependency_path = package_path(&dependency);
            let nested_source = placement.source.join("node_modules").join(&dependency_path);
            let source =
                if profile_seed::path_entry_exists(&seed_node_modules.join(&nested_source))? {
                    nested_source
                } else {
                    dependency_path.clone()
                };
            let destination = placement
                .destination
                .join("node_modules")
                .join(dependency_path);
            if seen_destinations.insert(destination.clone()) {
                placements.push(PackagePlacement {
                    source,
                    destination,
                });
            }
        }
    }
    Ok(ManagedPackagePlan {
        top_level,
        placements,
    })
}

fn dependencies(manifest: &Value) -> Result<&Map<String, Value>, String> {
    manifest
        .get("dependencies")
        .and_then(Value::as_object)
        .ok_or_else(|| "web profile dependencies are missing".to_string())
}

fn bundles(manifest: &Value) -> Result<&Vec<Value>, String> {
    manifest
        .pointer("/dsh/profile/bundles")
        .and_then(Value::as_array)
        .ok_or_else(|| "web profile bundles are missing".to_string())
}

fn merged_manifest(seed: &Value, active: &Value) -> Result<Value, String> {
    let seed_dependencies = dependencies(seed)?;
    let seed_bundles = bundles(seed)?;
    let mut merged = active.clone();
    let root = merged
        .as_object_mut()
        .ok_or_else(|| "web profile manifest must be a JSON object".to_string())?;
    let active_dependencies = root
        .entry("dependencies")
        .or_insert_with(|| Value::Object(Map::new()))
        .as_object_mut()
        .ok_or_else(|| "web profile dependencies must be a JSON object".to_string())?;
    for (name, spec) in seed_dependencies {
        active_dependencies.insert(name.clone(), spec.clone());
    }

    let dsh = root
        .entry("dsh")
        .or_insert_with(|| Value::Object(Map::new()))
        .as_object_mut()
        .ok_or_else(|| "web profile dsh settings must be a JSON object".to_string())?;
    let profile = dsh
        .entry("profile")
        .or_insert_with(|| Value::Object(Map::new()))
        .as_object_mut()
        .ok_or_else(|| "web profile dsh.profile settings must be a JSON object".to_string())?;
    let active_bundles = profile
        .get("bundles")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let managed: HashSet<&str> = seed_bundles.iter().filter_map(Value::as_str).collect();
    let mut result = seed_bundles.clone();
    let mut seen: HashSet<String> = managed.iter().map(|value| (*value).to_string()).collect();
    for bundle in active_bundles {
        match bundle.as_str() {
            Some(name) if managed.contains(name) || !seen.insert(name.to_string()) => {}
            _ => result.push(bundle),
        }
    }
    profile.insert("bundles".to_string(), Value::Array(result));
    Ok(merged)
}

fn marker_matches(seed_marker: &Path, active_marker: &Path) -> Result<bool, String> {
    let seed = fs::read(seed_marker).map_err(|error| path_error("read", seed_marker, error))?;
    match fs::read(active_marker) {
        Ok(active) => Ok(active == seed),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(false),
        Err(error) => Err(path_error("read", active_marker, error)),
    }
}

fn is_current(
    seed_manifest: &Value,
    active_manifest: &Value,
    seed_marker: &Path,
    active_marker: &Path,
    placements: &[PackagePlacement],
    active_node_modules: &Path,
) -> Result<bool, String> {
    if !marker_matches(seed_marker, active_marker)? {
        return Ok(false);
    }
    if merged_manifest(seed_manifest, active_manifest)? != *active_manifest {
        return Ok(false);
    }
    for placement in placements {
        let active_path = active_node_modules
            .join(&placement.destination)
            .join("package.json");
        let seed_path = seed_marker
            .parent()
            .expect("managed marker always has a parent")
            .join("node_modules")
            .join(&placement.source)
            .join("package.json");
        let seed_package = read_json(&seed_path)?;
        let active_package = match fs::read(&active_path) {
            Ok(contents) => serde_json::from_slice::<Value>(&contents).ok(),
            Err(error) if error.kind() == ErrorKind::NotFound => None,
            Err(error) => return Err(path_error("read", &active_path, error)),
        };
        let same_identity = active_package.as_ref().is_some_and(|active| {
            active.get("name") == seed_package.get("name")
                && active.get("version") == seed_package.get("version")
        });
        if !same_identity {
            return Ok(false);
        }
    }
    Ok(true)
}

fn rollback_packages(
    active_node_modules: &Path,
    backup_node_modules: &Path,
    swapped: &[(PathBuf, bool)],
) -> Result<(), String> {
    let mut errors = Vec::new();
    for (package, had_backup) in swapped.iter().rev() {
        let active = active_node_modules.join(package);
        if let Err(error) = profile_seed::remove_owned_path(&active) {
            errors.push(error);
            continue;
        }
        if *had_backup {
            let backup = backup_node_modules.join(package);
            if let Some(parent) = active.parent() {
                if let Err(error) = fs::create_dir_all(parent) {
                    errors.push(path_error("create rollback directory", parent, error));
                    continue;
                }
            }
            if let Err(error) = fs::rename(&backup, &active) {
                errors.push(path_error("restore managed package", &active, error));
            }
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

fn swap_packages(
    packages: &[PathBuf],
    staged_node_modules: &Path,
    active_node_modules: &Path,
    backup_node_modules: &Path,
) -> Result<Vec<(PathBuf, bool)>, String> {
    let mut swapped = Vec::new();
    for package in packages {
        let result = (|| {
            let active = active_node_modules.join(package);
            let staged = staged_node_modules.join(package);
            let backup = backup_node_modules.join(package);
            if let Some(parent) = active.parent() {
                fs::create_dir_all(parent).map_err(|error| {
                    path_error("create managed package directory", parent, error)
                })?;
            }
            let had_backup = profile_seed::path_entry_exists(&active)?;
            if had_backup {
                if let Some(parent) = backup.parent() {
                    fs::create_dir_all(parent).map_err(|error| {
                        path_error("create upgrade backup directory", parent, error)
                    })?;
                }
                fs::rename(&active, &backup)
                    .map_err(|error| path_error("back up managed package", &active, error))?;
            }
            if let Err(error) = fs::rename(&staged, &active) {
                swapped.push((package.clone(), had_backup));
                return Err(path_error("publish managed package", &active, error));
            }
            Ok(had_backup)
        })();

        match result {
            Ok(had_backup) => swapped.push((package.clone(), had_backup)),
            Err(error) => {
                let rollback =
                    rollback_packages(active_node_modules, backup_node_modules, &swapped);
                return Err(match rollback {
                    Ok(()) => error,
                    Err(rollback_error) => {
                        format!("{error}; rollback failed: {rollback_error}")
                    }
                });
            }
        }
    }
    Ok(swapped)
}

fn recovery_backup_paths(profile_dir: &Path) -> Result<Vec<PathBuf>, String> {
    let backup_node_modules = profile_dir.join(WORK_NAME).join("backup/node_modules");
    if !profile_seed::path_entry_exists(&backup_node_modules)? {
        return Ok(Vec::new());
    }
    package_paths(&backup_node_modules)
}

fn recover_interrupted_upgrade(profile_dir: &Path) -> Result<(), String> {
    let work = profile_dir.join(WORK_NAME);
    if !profile_seed::path_entry_exists(&work)? {
        return Ok(());
    }
    let active_node_modules = profile_dir.join("node_modules");
    let backup_node_modules = work.join("backup/node_modules");
    for package in recovery_backup_paths(profile_dir)? {
        let active = active_node_modules.join(&package);
        let backup = backup_node_modules.join(&package);
        profile_seed::remove_owned_path(&active)?;
        if let Some(parent) = active.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| path_error("create recovery directory", parent, error))?;
        }
        fs::rename(&backup, &active)
            .map_err(|error| path_error("restore interrupted managed package", &active, error))?;
    }
    profile_seed::remove_owned_path(&work)
}

fn finish_failed_upgrade(profile_dir: &Path, error: String) -> Result<UpgradeResult, String> {
    let work = profile_dir.join(WORK_NAME);
    if recovery_backup_paths(profile_dir)?.is_empty() {
        let cleanup = profile_seed::remove_owned_path(&work);
        return Err(match cleanup {
            Ok(()) => error,
            Err(cleanup_error) => format!("{error}; {cleanup_error}"),
        });
    }
    Err(format!(
        "{error}; managed package backup was preserved for recovery on the next startup"
    ))
}

fn reconcile_managed_web_profile_with<F>(
    seed: &Path,
    dsh_home: &Path,
    replace_manifest: F,
) -> Result<UpgradeResult, String>
where
    F: Fn(&Path, &[u8]) -> Result<(), String>,
{
    profile_seed::validate_default_profile(seed)?;
    let profile_dir = dsh_home.join("profiles/web");
    let _upgrade_lock = acquire_upgrade_lock(&profile_dir)?;
    let active_manifest_path = profile_dir.join("package.json");
    let seed_manifest_path = seed.join("package.json");
    let seed_marker = seed.join(MARKER_NAME);
    let active_marker = profile_dir.join(MARKER_NAME);
    let seed_manifest = read_json(&seed_manifest_path)?;
    let mut active_manifest = read_json(&active_manifest_path)?;
    read_json(&seed_marker)?;
    let mut plan =
        managed_package_plan(&seed_manifest, &active_manifest, &seed.join("node_modules"))?;
    let active_node_modules = profile_dir.join("node_modules");
    let work = profile_dir.join(WORK_NAME);

    if profile_seed::path_entry_exists(&work)? {
        if is_current(
            &seed_manifest,
            &active_manifest,
            &seed_marker,
            &active_marker,
            &plan.placements,
            &active_node_modules,
        )? {
            profile_seed::remove_owned_path(&work)?;
            return Ok(UpgradeResult::AlreadyCurrent);
        }
        recover_interrupted_upgrade(&profile_dir)?;
        active_manifest = read_json(&active_manifest_path)?;
        plan = managed_package_plan(&seed_manifest, &active_manifest, &seed.join("node_modules"))?;
    }

    if is_current(
        &seed_manifest,
        &active_manifest,
        &seed_marker,
        &active_marker,
        &plan.placements,
        &active_node_modules,
    )? {
        return Ok(UpgradeResult::AlreadyCurrent);
    }

    let merged = merged_manifest(&seed_manifest, &active_manifest)?;
    let staged_node_modules = work.join("stage/node_modules");
    let backup_node_modules = work.join("backup/node_modules");
    fs::create_dir_all(&staged_node_modules).map_err(|error| {
        path_error(
            "create upgrade staging directory",
            &staged_node_modules,
            error,
        )
    })?;

    let operation = (|| {
        for placement in &plan.placements {
            let destination = staged_node_modules.join(&placement.destination);
            if profile_seed::path_entry_exists(&destination)? {
                continue;
            }
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent).map_err(|error| {
                    path_error("create upgrade staging directory", parent, error)
                })?;
            }
            profile_seed::copy_physical_tree(
                &seed.join("node_modules").join(&placement.source),
                &destination,
            )?;
        }

        fs::create_dir_all(&active_node_modules).map_err(|error| {
            path_error(
                "create web profile node_modules",
                &active_node_modules,
                error,
            )
        })?;
        let swapped = swap_packages(
            &plan.top_level,
            &staged_node_modules,
            &active_node_modules,
            &backup_node_modules,
        )?;

        let mut manifest_contents = serde_json::to_vec_pretty(&merged)
            .map_err(|error| format!("failed to serialize web profile manifest: {error}"))?;
        manifest_contents.push(b'\n');
        if let Err(error) = replace_manifest(&active_manifest_path, &manifest_contents) {
            let rollback = rollback_packages(&active_node_modules, &backup_node_modules, &swapped);
            return Err(match rollback {
                Ok(()) => error,
                Err(rollback_error) => format!("{error}; rollback failed: {rollback_error}"),
            });
        }
        let marker_contents =
            fs::read(&seed_marker).map_err(|error| path_error("read", &seed_marker, error))?;
        profile::replace_atomic_file(&active_marker, &marker_contents)?;
        Ok(())
    })();

    match operation {
        Ok(()) => {
            let _ = profile_seed::remove_owned_path(&work);
            Ok(UpgradeResult::Upgraded)
        }
        Err(error) => finish_failed_upgrade(&profile_dir, error),
    }
}

pub fn reconcile_managed_web_profile(
    seed: &Path,
    dsh_home: &Path,
) -> Result<UpgradeResult, String> {
    reconcile_managed_web_profile_with(seed, dsh_home, profile::replace_manifest)
}

#[cfg(test)]
mod tests;

mod port;
mod profile;
mod runtime;
mod sidecar;

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{Manager, Runtime};

const STARTUP_TIMEOUT: Duration = Duration::from_secs(90);
const DSH_BIN: &str = "host/node_modules/@deepseek-ai/dsh/lib/bin.js";

struct SidecarState(Mutex<Option<sidecar::SidecarHandle>>);

fn error_script(message: &str, log_path: Option<&Path>) -> String {
    let message = serde_json::to_string(message).unwrap_or_else(|_| "\"Unknown error\"".into());
    let log_path = serde_json::to_string(
        &log_path
            .map(|path| path.display().to_string())
            .unwrap_or_default(),
    )
    .unwrap_or_else(|_| "\"\"".into());
    format!("window.showStartupError?.({message}, {log_path})")
}

fn show_startup_error<R: Runtime>(
    app_handle: &tauri::AppHandle<R>,
    message: &str,
    log_path: Option<&Path>,
) {
    if let Some(window) = app_handle.get_webview_window("main") {
        let _ = window.eval(error_script(message, log_path));
    }
}

fn take_sidecar<R: Runtime>(app_handle: &tauri::AppHandle<R>) -> Option<sidecar::SidecarHandle> {
    let state = app_handle.try_state::<SidecarState>()?;
    let handle = state.0.lock().ok()?.take();
    handle
}

fn project_root() -> Result<PathBuf, String> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| "Cargo manifest directory has no parent".to_string())
}

fn start_sidecar<R: Runtime>(app: &mut tauri::App<R>) -> Result<(), String> {
    let resource_dir = app
        .path()
        .resource_dir()
        .map_err(|error| format!("failed to resolve resource directory: {error}"))?;
    let runtime_dir =
        runtime::resolve_runtime_dir(&resource_dir, &project_root()?, cfg!(debug_assertions))?;
    let runtime_dir = runtime::without_verbatim_prefix(&runtime_dir);
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("failed to resolve app data directory: {error}"))?;
    std::fs::create_dir_all(&data_dir)
        .map_err(|error| format!("failed to create {}: {error}", data_dir.display()))?;
    profile::migrate_legacy_web_profile(&data_dir)?;

    let log_path = data_dir.join("dsh-host.log");
    std::fs::write(&log_path, b"")
        .map_err(|error| format!("failed to reset {}: {error}", log_path.display()))?;
    let port = port::find_free_port().map_err(|error| format!("failed to find port: {error}"))?;
    let handle = sidecar::spawn(
        &runtime_dir.join("node/node.exe"),
        &runtime_dir.join(DSH_BIN),
        port,
        &data_dir,
        &log_path,
    )
    .map_err(|error| format!("failed to start DSH host: {error}"))?;

    let state = app.state::<SidecarState>();
    *state
        .0
        .lock()
        .map_err(|_| "sidecar state lock is poisoned".to_string())? = Some(handle);

    let app_handle = app.handle().clone();
    std::thread::spawn(move || {
        if sidecar::wait_ready(port, STARTUP_TIMEOUT) {
            let Ok(url) = tauri::Url::parse(&format!("http://127.0.0.1:{port}/")) else {
                show_startup_error(
                    &app_handle,
                    "DSH host returned an invalid URL",
                    Some(&log_path),
                );
                return;
            };
            if let Some(window) = app_handle.get_webview_window("main") {
                let _ = window.navigate(url);
            }
            return;
        }

        if let Some(handle) = take_sidecar(&app_handle) {
            handle.kill();
        }
        show_startup_error(
            &app_handle,
            "DSH host did not become ready within 90 seconds.",
            Some(&log_path),
        );
    });
    Ok(())
}

pub fn run() {
    tauri::Builder::default()
        .manage(SidecarState(Mutex::new(None)))
        .setup(|app| {
            if let Err(error) = start_sidecar(app) {
                let app_handle = app.handle().clone();
                let log_path = app
                    .path()
                    .app_data_dir()
                    .ok()
                    .map(|path| path.join("dsh-host.log"));
                std::thread::spawn(move || {
                    std::thread::sleep(Duration::from_millis(500));
                    show_startup_error(&app_handle, &error, log_path.as_deref());
                });
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::Exit = event {
                if let Some(handle) = take_sidecar(app_handle) {
                    handle.kill();
                }
            }
        });
}

#[cfg(test)]
mod app_tests {
    use super::error_script;

    #[test]
    fn startup_error_script_serializes_untrusted_text() {
        let script = error_script("bad 'quote'\n</script>", None);
        assert!(script.contains("bad 'quote'\\n</script>"));
        assert!(!script.contains("showStartupError?.(bad"));
    }
}

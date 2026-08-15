mod interface_copy;
mod job;
mod port;
mod profile;
mod profile_seed;
mod runtime;
mod sidecar;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};
use std::sync::Mutex;
use std::thread::JoinHandle;
use std::time::Duration;
use tauri::{Manager, Runtime};

const STARTUP_TIMEOUT: Duration = Duration::from_secs(90);

struct SidecarState {
    handle: Mutex<Option<sidecar::SidecarHandle>>,
    exiting: AtomicBool,
    web_port: AtomicU16,
}

struct StartupUiState(AtomicBool);

#[tauri::command]
fn startup_ui_ready(state: tauri::State<'_, StartupUiState>) {
    state.0.store(true, Ordering::Release);
}

fn spawn_background<F>(task: F) -> JoinHandle<()>
where
    F: FnOnce() + Send + 'static,
{
    std::thread::spawn(task)
}

fn require_navigation<T, E: std::fmt::Display>(
    navigation: Option<Result<T, E>>,
) -> Result<T, String> {
    match navigation {
        None => Err("DSH Web window is unavailable".to_string()),
        Some(Err(error)) => Err(format!("failed to navigate to DSH Web: {error}")),
        Some(Ok(value)) => Ok(value),
    }
}

fn error_script(message: &str, log_path: Option<&Path>) -> String {
    let message = serde_json::to_string(message).unwrap_or_else(|_| "\"Unknown error\"".into());
    let log_path = serde_json::to_string(
        &log_path
            .map(|path| path.display().to_string())
            .unwrap_or_default(),
    )
    .unwrap_or_else(|_| "\"\"".into());
    format!(
        "window.__dshStartupError = {{ message: {message}, logPath: {log_path} }}; \
         if (window.showStartupError) {{ \
           window.showStartupError(window.__dshStartupError.message, window.__dshStartupError.logPath); \
           delete window.__dshStartupError; \
         }}"
    )
}

fn show_startup_error<R: Runtime>(
    app_handle: &tauri::AppHandle<R>,
    message: &str,
    log_path: Option<&Path>,
) {
    let script = error_script(message, log_path);
    let mut last_error = "startup page did not acknowledge readiness".to_string();
    for _ in 0..50 {
        let ready = app_handle
            .try_state::<StartupUiState>()
            .is_some_and(|state| state.0.load(Ordering::Acquire));
        if ready {
            if let Some(window) = app_handle.get_webview_window("main") {
                match window.eval(&script) {
                    Ok(()) => return,
                    Err(error) => last_error = format!("failed to evaluate startup error: {error}"),
                }
            } else {
                last_error = "startup window was unavailable".to_string();
            }
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    let diagnostic = format!("{message}\nUnable to render startup error: {last_error}\n");
    if let Some(log_path) = log_path {
        use std::io::Write;
        if let Ok(mut log) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)
        {
            let _ = log.write_all(diagnostic.as_bytes());
        }
    }
    eprintln!("{diagnostic}");
}

fn take_sidecar<R: Runtime>(app_handle: &tauri::AppHandle<R>) -> Option<sidecar::SidecarHandle> {
    let state = app_handle.try_state::<SidecarState>()?;
    let handle = state.handle.lock().ok()?.take();
    state.web_port.store(0, Ordering::Release);
    handle
}

fn project_root() -> Result<PathBuf, String> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| "Cargo manifest directory has no parent".to_string())
}

fn start_sidecar<R: Runtime>(app: &tauri::AppHandle<R>) -> Result<(), String> {
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
    profile_seed::seed_new_web_profile(&runtime_dir.join("profile-seed/profiles/web"), &data_dir)?;

    let log_path = data_dir.join("dsh-host.log");
    std::fs::write(&log_path, b"")
        .map_err(|error| format!("failed to reset {}: {error}", log_path.display()))?;
    let port = port::find_free_port().map_err(|error| format!("failed to find port: {error}"))?;
    let handle = sidecar::spawn(
        &runtime_dir.join("node/node.exe"),
        &runtime_dir.join(runtime::DSH_BIN),
        port,
        &data_dir,
        &log_path,
    )
    .map_err(|error| format!("failed to start DSH host: {error}"))?;

    let state = app.state::<SidecarState>();
    let mut slot = state
        .handle
        .lock()
        .map_err(|_| "sidecar state lock is poisoned".to_string())?;
    if state.exiting.load(Ordering::Acquire) {
        return Err("application exited before the DSH host started".to_string());
    }
    *slot = Some(handle);
    drop(slot);

    if !sidecar::wait_ready(port, STARTUP_TIMEOUT) {
        return Err("DSH host did not become ready within 90 seconds.".to_string());
    }

    state.web_port.store(port, Ordering::Release);
    let url = tauri::Url::parse(&format!("http://127.0.0.1:{port}/"))
        .map_err(|_| "DSH host returned an invalid URL".to_string())?;
    let navigation = app
        .get_webview_window("main")
        .map(|window| window.navigate(url));
    if let Err(error) = require_navigation(navigation) {
        state.web_port.store(0, Ordering::Release);
        return Err(error);
    }
    Ok(())
}

pub fn run() {
    tauri::Builder::default()
        .manage(SidecarState {
            handle: Mutex::new(None),
            exiting: AtomicBool::new(false),
            web_port: AtomicU16::new(0),
        })
        .manage(StartupUiState(AtomicBool::new(false)))
        .invoke_handler(tauri::generate_handler![startup_ui_ready])
        .on_page_load(|webview, payload| {
            let port = webview
                .state::<SidecarState>()
                .web_port
                .load(Ordering::Acquire);
            interface_copy::handle_page_load(webview, payload, (port != 0).then_some(port));
        })
        .setup(|app| {
            let app_handle = app.handle().clone();
            spawn_background(move || {
                if let Err(error) = start_sidecar(&app_handle) {
                    if let Some(handle) = take_sidecar(&app_handle) {
                        handle.kill();
                    }
                    let log_path = app_handle
                        .path()
                        .app_data_dir()
                        .ok()
                        .map(|path| path.join("dsh-host.log"));
                    show_startup_error(&app_handle, &error, log_path.as_deref());
                }
            });
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::Exit = event {
                if let Some(state) = app_handle.try_state::<SidecarState>() {
                    state.exiting.store(true, Ordering::Release);
                }
                if let Some(handle) = take_sidecar(app_handle) {
                    handle.kill();
                }
            }
        });
}

#[cfg(test)]
mod app_tests {
    use super::{error_script, require_navigation, spawn_background};
    use std::sync::mpsc;
    use std::time::Duration;

    #[test]
    fn main_window_uses_the_ds_studio_title() {
        let config: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        assert_eq!(config["app"]["windows"][0]["title"], "DS Studio");
    }

    #[test]
    fn startup_error_script_serializes_untrusted_text() {
        let script = error_script("bad 'quote'\n</script>", None);
        assert!(script.contains("bad 'quote'\\n</script>"));
        assert!(!script.contains("showStartupError?.(bad"));
        assert!(script.contains("window.__dshStartupError"));
    }

    #[test]
    fn startup_work_runs_on_a_background_thread() {
        let caller = std::thread::current().id();
        let (sender, receiver) = mpsc::channel();

        let worker = spawn_background(move || {
            sender.send(std::thread::current().id()).unwrap();
        });

        let worker_id = receiver.recv_timeout(Duration::from_secs(1)).unwrap();
        worker.join().unwrap();
        assert_ne!(worker_id, caller);
    }

    #[test]
    fn navigation_requires_a_window_and_a_successful_navigation() {
        assert_eq!(
            require_navigation(None::<Result<(), &str>>).unwrap_err(),
            "DSH Web window is unavailable"
        );
        assert_eq!(
            require_navigation(Some(Err::<(), _>("navigation blocked"))).unwrap_err(),
            "failed to navigate to DSH Web: navigation blocked"
        );
        assert!(require_navigation(Some(Ok::<_, &str>(()))).is_ok());
    }
}

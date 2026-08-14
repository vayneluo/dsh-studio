mod port;
mod sidecar;

use std::sync::Mutex;
use tauri::Manager;

struct SidecarState(Mutex<Option<sidecar::SidecarHandle>>);

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let resource_dir = app.path().resource_dir()?;
            let node = resource_dir.join("runtime/node/node.exe");
            let dsh_bin = resource_dir
                .join("runtime/host/node_modules/@deepseek-ai/dsh/lib/bin.js");
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;

            let port = port::find_free_port()?;
            let handle = sidecar::spawn(
                node.to_str().ok_or("node path not utf8")?,
                dsh_bin.to_str().ok_or("dsh path not utf8")?,
                port,
                data_dir.to_str().ok_or("data dir not utf8")?,
            )?;

            if !sidecar::wait_ready(port, std::time::Duration::from_secs(30)) {
                return Err("harness 未在 30s 内就绪".into());
            }

            let url = tauri::Url::parse(&format!("http://127.0.0.1:{}/", port))?;
            if let Some(window) = app.get_webview_window("main") {
                window.navigate(url)?;
            }

            app.manage(SidecarState(Mutex::new(Some(handle))));
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::Exit = event {
                if let Some(state) = app_handle.try_state::<SidecarState>() {
                    if let Some(handle) = state.0.lock().unwrap().take() {
                        handle.kill();
                    }
                }
            }
        });
}

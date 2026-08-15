mod port;
mod profile;
mod sidecar;

use std::path::Path;
use std::sync::Mutex;
use tauri::Manager;

struct SidecarState(Mutex<Option<sidecar::SidecarHandle>>);

/// 递归复制目录（bundle-host.mjs 打包的 runtime/home 不含符号链接/junction，
/// 普通 fs::copy 即可完整复制）。
fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let resource_dir = app.path().resource_dir()?;
            let node = resource_dir.join("runtime/node/node.exe");
            let dsh_bin = resource_dir
                .join("runtime/host/node_modules/@deepseek-ai/dsh/lib/bin.js");
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;

            // 首次运行：把打包进 resources 的 runtime/home（含 web profile + 插件）
            // 播种到 app_data_dir 作为 DSH_HOME。仅当 profiles 不存在时复制，
            // 避免覆盖用户后续写入的 key / 设置。
            let bundled_home = resource_dir.join("runtime/home");
            let profiles = data_dir.join("profiles");
            if !profiles.exists() && bundled_home.exists() {
                copy_dir_recursive(&bundled_home, &data_dir)?;
            }

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

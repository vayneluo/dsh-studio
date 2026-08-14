mod port;

pub fn run() {
    tauri::Builder::default()
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app, _event| {});
}

pub mod config;
pub mod git;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .run(tauri::generate_context!())
        .expect("tauri çalıştırılamadı");
}

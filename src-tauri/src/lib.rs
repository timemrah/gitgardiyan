pub mod commands;
pub mod config;
pub mod git;
pub mod log;
pub mod notifier;
pub mod rules;
pub mod scheduler;
pub mod state;
pub mod tray;

use state::AppState;
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let dir = app.path().app_config_dir().expect("config klasörü yok");
            let config_path = dir.join("config.json");
            app.manage(AppState {
                config: Mutex::new(config::Config::load(&config_path)),
                config_path,
                log_dir: dir,
                notifications: Mutex::new(HashMap::new()),
            });
            scheduler::start(app.handle().clone());
            tray::create(app)?;
            // ilk kurulumda otomatik başlatmayı aç; kullanıcı arayüzden kapatabilir
            let _ = tauri_plugin_autostart::ManagerExt::autolaunch(app.handle()).enable();
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_projects,
            commands::add_project,
            commands::remove_project,
            commands::update_project,
            commands::get_notification,
            commands::notify_action,
            commands::get_autostart,
            commands::set_autostart,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    let _ = window.hide();
                    api.prevent_close();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("tauri çalıştırılamadı");
}

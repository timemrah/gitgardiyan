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
            let first_run = !config_path.exists();
            if first_run {
                if let Err(e) = tauri_plugin_autostart::ManagerExt::autolaunch(app.handle()).enable() {
                    log::line(&dir, &format!("autostart etkinleştirilemedi: {e}"));
                }
            }
            app.manage(AppState {
                config: Mutex::new(config::Config::load(&config_path)),
                config_path,
                log_dir: dir,
                notifications: Mutex::new(HashMap::new()),
            });
            let mut git_version_cmd = std::process::Command::new("git");
            git_version_cmd.arg("--version");
            #[cfg(target_os = "windows")]
            {
                use std::os::windows::process::CommandExt;
                git_version_cmd.creation_flags(0x08000000);
            }
            if git_version_cmd.output().is_err() {
                log::line(&app.state::<state::AppState>().log_dir, "git bulunamadı — uygulama çalışamaz");
                use tauri_plugin_dialog::DialogExt;
                app.dialog()
                    .message("Sistemde 'git' komutu bulunamadı. GitGardiyan çalışmak için git'e ihtiyaç duyar. Lütfen git'i kurun ve uygulamayı yeniden başlatın.")
                    .title("GitGardiyan — git bulunamadı")
                    .show(|_| {});
            }
            scheduler::start(app.handle().clone());
            tray::create(app)?;
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

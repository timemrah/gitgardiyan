use crate::config::Project;
use crate::rules::Decision;
use crate::state::{AppState, NotifyPayload};
use std::hash::{Hash, Hasher};
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

fn path_hash(p: &std::path::Path) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    p.hash(&mut h);
    h.finish()
}

const W: f64 = 420.0;
const H: f64 = 220.0;

/// Karara uyan bildirim penceresini açar. Aynı proje için pencere zaten
/// açıksa hiçbir şey yapmaz (spec: proje başına tek bildirim).
pub fn show(app: &AppHandle, p: &Project, d: &Decision) {
    let (ptype, rule, message) = match d {
        Decision::AutoBackup { changed } => (
            "countdown",
            1u8,
            format!(
                "{} projesinde çok fazla dosya değişikliği var ({} dosya). Commit ve push işlemi yapılacak.",
                p.name, changed
            ),
        ),
        Decision::PullQuestion { commits } => (
            "question",
            2u8,
            format!(
                "{} projesinin GitHub deposunda {} yeni commit var. Çekmek ister misiniz?",
                p.name, commits
            ),
        ),
        Decision::DailyBackup => (
            "countdown",
            3u8,
            format!(
                "{} projesinde bekleyen commit/push var. Günlük yedek alınacak.",
                p.name
            ),
        ),
        Decision::SilentPush | Decision::Nothing => return,
    };

    let label = format!("notify-{:x}", path_hash(&p.path));
    if app.get_webview_window(&label).is_some() {
        return;
    }

    let state = app.state::<AppState>();
    state.notifications.lock().unwrap().insert(
        label.clone(),
        NotifyPayload {
            ptype: ptype.into(),
            rule,
            path: p.path.to_string_lossy().into_owned(),
            message,
            seconds: 300,
        },
    );

    // açık bildirim sayısına göre üst üste binmeyecek şekilde diz
    let open_count = app
        .webview_windows()
        .keys()
        .filter(|k| k.starts_with("notify-"))
        .count() as f64;

    let win = WebviewWindowBuilder::new(app, &label, WebviewUrl::App("notify.html".into()))
        .title("GitGardiyan")
        .inner_size(W, H)
        .resizable(false)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .build();

    if let Ok(win) = win {
        if let Ok(Some(mon)) = win.current_monitor() {
            let sf = mon.scale_factor();
            let ms = mon.size();
            let x = ms.width as f64 - (W + 16.0) * sf;
            let y = ms.height as f64 - (H + 60.0) * sf - open_count * (H + 12.0) * sf;
            let _ = win.set_position(tauri::PhysicalPosition::new(x as i32, y.max(0.0) as i32));
        }
    }
}

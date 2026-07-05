use crate::config::Project;
use crate::rules::{self, Decision, Snapshot};
use crate::state::AppState;
use crate::{git, log, notifier};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;
use tauri::{AppHandle, Manager};

fn snapshot(p: &Project) -> Snapshot {
    let changed_files = git::changed_file_count(&p.path).unwrap_or(0);
    let (remote_ahead, unpushed) = git::behind_ahead(&p.path).unwrap_or((0, 0));
    Snapshot { changed_files, remote_ahead, unpushed }
}

pub fn start(app: AppHandle) {
    std::thread::spawn(move || {
        let mut last_check: HashMap<PathBuf, Instant> = HashMap::new();
        let mut last_backup: HashMap<PathBuf, String> = HashMap::new();
        loop {
            tick(&app, &mut last_check, &mut last_backup);
            std::thread::sleep(std::time::Duration::from_secs(60));
        }
    });
}

fn tick(
    app: &AppHandle,
    last_check: &mut HashMap<PathBuf, Instant>,
    last_backup: &mut HashMap<PathBuf, String>,
) {
    let state = app.state::<AppState>();
    let projects = state.config.lock().unwrap().projects.clone();
    let log_dir = state.log_dir.clone();
    let now = chrono::Local::now();
    let today = now.format("%Y-%m-%d").to_string();
    let now_hm = now.format("%H:%M").to_string();

    for p in projects {
        if !git::is_git_repo(&p.path) {
            continue; // yönetim penceresi "bulunamadı" gösterir
        }

        // Kural 3: günlük yedek. ">=" karşılaştırması: uygulama 23:00'ten
        // sonra açılırsa aynı gün içinde yine tetiklenir.
        if p.rule_backup
            && now_hm.as_str() >= p.backup_time.as_str()
            && last_backup.get(&p.path) != Some(&today)
        {
            last_backup.insert(p.path.clone(), today.clone());
            let snap = snapshot(&p);
            if let Decision::DailyBackup = rules::decide(&p, &snap, &today, true) {
                log::line(&log_dir, &format!("{}: günlük yedek bildirimi", p.name));
                notifier::show(app, &p, &Decision::DailyBackup);
                continue; // aynı projede aynı anda tek bildirim
            }
        }

        // Saatlik kontrol (proje başına interval_minutes)
        let due = last_check
            .get(&p.path)
            .map_or(true, |t| t.elapsed().as_secs() >= p.interval_minutes as u64 * 60);
        if !due {
            continue;
        }
        last_check.insert(p.path.clone(), Instant::now());

        if p.rule_remote {
            if let Err(e) = git::fetch(&p.path) {
                log::line(&log_dir, &format!("{}: fetch başarısız: {}", p.name, e.trim()));
            }
        }
        let snap = snapshot(&p);
        match rules::decide(&p, &snap, &today, false) {
            Decision::Nothing => {}
            Decision::SilentPush => match git::push(&p.path) {
                Ok(()) => log::line(&log_dir, &format!("{}: bekleyen commit'ler push edildi", p.name)),
                Err(e) => log::line(&log_dir, &format!("{}: push tekrar denendi, başarısız: {}", p.name, e.trim())),
            },
            d => {
                log::line(&log_dir, &format!("{}: bildirim — {:?}", p.name, d));
                notifier::show(app, &p, &d);
            }
        }
    }
}

use crate::config::Project;
use crate::state::{AppState, NotifyPayload};
use crate::{git, log};
use serde::Serialize;
use std::path::PathBuf;
use tauri::State;

#[derive(Serialize)]
pub struct ProjectView {
    pub project: Project,
    pub found: bool,
    pub changed: u32,
    pub remote_ahead: u32,
    pub unpushed: u32,
}

#[tauri::command]
pub async fn list_projects(state: State<'_, AppState>) -> Result<Vec<ProjectView>, String> {
    let projects = state.config.lock().unwrap().projects.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut out = Vec::new();
        for p in projects {
            let found = git::is_git_repo(&p.path);
            let (changed, (remote_ahead, unpushed)) = if found {
                (
                    git::changed_file_count(&p.path).unwrap_or(0),
                    git::behind_ahead(&p.path).unwrap_or((0, 0)),
                )
            } else {
                (0, (0, 0))
            };
            out.push(ProjectView { project: p, found, changed, remote_ahead, unpushed });
        }
        out
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_project(state: State<'_, AppState>, path: String) -> Result<Project, String> {
    let pb = PathBuf::from(&path);
    if !git::is_git_repo(&pb) {
        return Err("Bu klasör bir git deposu değil. Önce 'git init' yapılmalı.".into());
    }
    let project = Project::new(pb);
    {
        let mut cfg = state.config.lock().unwrap();
        if cfg.projects.iter().any(|p| p.path == project.path) {
            return Err("Bu proje zaten listede.".into());
        }
        cfg.projects.push(project.clone());
        cfg.save(&state.config_path).map_err(|e| e.to_string())?;
    }
    log::line(&state.log_dir, &format!("proje eklendi: {path}"));
    Ok(project)
}

#[tauri::command]
pub async fn remove_project(state: State<'_, AppState>, path: String) -> Result<(), String> {
    let pb = PathBuf::from(&path);
    let mut cfg = state.config.lock().unwrap();
    cfg.projects.retain(|p| p.path != pb);
    cfg.save(&state.config_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_project(state: State<'_, AppState>, project: Project) -> Result<(), String> {
    let mut cfg = state.config.lock().unwrap();
    match cfg.projects.iter_mut().find(|p| p.path == project.path) {
        Some(p) => *p = project,
        None => return Err("Proje bulunamadı.".into()),
    }
    cfg.save(&state.config_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_notification(
    state: State<'_, AppState>,
    label: String,
) -> Result<Option<NotifyPayload>, String> {
    Ok(state.notifications.lock().unwrap().get(&label).cloned())
}

fn today() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

fn now_minute() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M").to_string()
}

#[tauri::command]
pub async fn notify_action(
    state: State<'_, AppState>,
    path: String,
    rule: u8,
    action: String,
) -> Result<String, String> {
    let repo = PathBuf::from(&path);
    let name = repo
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or(path.clone());
    let log_dir = state.log_dir.clone();

    match action.as_str() {
        "mute" => {
            let mut cfg = state.config.lock().unwrap();
            if let Some(p) = cfg.projects.iter_mut().find(|p| p.path == repo) {
                p.muted_date = Some(today());
            }
            cfg.save(&state.config_path).map_err(|e| e.to_string())?;
            log::line(&log_dir, &format!("{name}: bugün için susturuldu"));
            Ok("Bugün bir daha sorulmayacak.".into())
        }
        "proceed" => {
            let changed = git::changed_file_count(&repo)?;
            if changed > 0 {
                let msg = format!("Otomatik yedek: {} ({} dosya)", now_minute(), changed);
                git::commit_all(&repo, &msg)?;
                log::line(&log_dir, &format!("{name}: commit — {msg} (kural {rule})"));
            }
            match git::push(&repo) {
                Ok(()) => {
                    log::line(&log_dir, &format!("{name}: push tamamlandı"));
                    Ok(format!("{name}: commit ve push tamamlandı."))
                }
                Err(e) => {
                    log::line(&log_dir, &format!("{name}: push başarısız: {e}"));
                    Err(format!(
                        "{name}: Push başarısız. Commit yerelde duruyor, sonraki saatte tekrar denenecek. Hata: {e}"
                    ))
                }
            }
        }
        "pull" => {
            let changed = git::changed_file_count(&repo)?;
            if changed > 0 {
                let msg = format!("Otomatik yedek: {} ({} dosya)", now_minute(), changed);
                git::commit_all(&repo, &msg)?;
                log::line(&log_dir, &format!("{name}: pull öncesi commit — {msg}"));
            }
            match git::pull_rebase(&repo)? {
                git::PullOutcome::Ok => {
                    log::line(&log_dir, &format!("{name}: pull --rebase tamamlandı"));
                    Ok(format!("{name}: GitHub'daki değişiklikler çekildi."))
                }
                git::PullOutcome::Conflict => {
                    log::line(&log_dir, &format!("{name}: rebase çakışması, geri alındı"));
                    Err(format!(
                        "{name}: Çakışma oluştu, rebase geri alındı. Elle çözülmeli."
                    ))
                }
            }
        }
        other => Err(format!("bilinmeyen işlem: {other}")),
    }
}

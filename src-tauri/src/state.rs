use crate::config::Config;
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Serialize, Clone, Debug)]
pub struct NotifyPayload {
    /// "countdown" | "question"
    pub ptype: String,
    /// 1 = değişiklik, 2 = pull sorusu, 3 = günlük yedek
    pub rule: u8,
    pub path: String,
    pub message: String,
    pub seconds: u32,
}

pub struct AppState {
    pub config: Mutex<Config>,
    pub config_path: PathBuf,
    pub log_dir: PathBuf,
    /// pencere etiketi -> payload; bildirim penceresi açılışta çeker
    pub notifications: Mutex<HashMap<String, NotifyPayload>>,
}

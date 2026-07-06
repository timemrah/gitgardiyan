use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Project {
    pub path: PathBuf,
    pub name: String,
    #[serde(default = "d_threshold")]
    pub threshold: u32,
    // Eski tek "interval_minutes" alanı kural 1 sıklığına devrolur.
    #[serde(default = "d_interval", alias = "interval_minutes")]
    pub interval_changes_minutes: u32,
    #[serde(default = "d_interval")]
    pub interval_remote_minutes: u32,
    #[serde(default = "d_backup_time")]
    pub backup_time: String,
    #[serde(default = "d_true")]
    pub rule_changes: bool,
    #[serde(default = "d_true")]
    pub rule_remote: bool,
    #[serde(default = "d_true")]
    pub rule_backup: bool,
    #[serde(default)]
    pub muted_date: Option<String>,
}

impl Project {
    pub fn new(path: PathBuf) -> Project {
        let name = path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string_lossy().into_owned());
        Project {
            path,
            name,
            threshold: d_threshold(),
            interval_changes_minutes: d_interval(),
            interval_remote_minutes: d_interval(),
            backup_time: d_backup_time(),
            rule_changes: true,
            rule_remote: true,
            rule_backup: true,
            muted_date: None,
        }
    }
}

fn d_threshold() -> u32 { 10 }
fn d_interval() -> u32 { 60 }
fn d_backup_time() -> String { "23:00".into() }
fn d_true() -> bool { true }

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Config {
    pub projects: Vec<Project>,
}

impl Config {
    /// Dosya yoksa veya bozuksa boş config döner — uygulama asla açılamaz duruma düşmez.
    pub fn load(path: &Path) -> Config {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(path, serde_json::to_string_pretty(self).unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kayit_ve_okuma_birbirini_tutar() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("alt/config.json");
        let mut cfg = Config::default();
        cfg.projects.push(Project::new(PathBuf::from("/tmp/ornek-proje")));
        cfg.save(&file).unwrap();
        assert_eq!(Config::load(&file), cfg);
    }

    #[test]
    fn eksik_alanlar_varsayilan_alir() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("config.json");
        std::fs::write(&file, r#"{"projects":[{"path":"/tmp/p","name":"p"}]}"#).unwrap();
        let cfg = Config::load(&file);
        let p = &cfg.projects[0];
        assert_eq!(p.threshold, 10);
        assert_eq!(p.interval_changes_minutes, 60);
        assert_eq!(p.interval_remote_minutes, 60);
        assert_eq!(p.backup_time, "23:00");
        assert!(p.rule_changes && p.rule_remote && p.rule_backup);
        assert_eq!(p.muted_date, None);
    }

    #[test]
    fn eski_interval_minutes_kural1_sikligina_devrolur() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("config.json");
        std::fs::write(
            &file,
            r#"{"projects":[{"path":"/tmp/p","name":"p","interval_minutes":15}]}"#,
        )
        .unwrap();
        let p = &Config::load(&file).projects[0];
        assert_eq!(p.interval_changes_minutes, 15);
        assert_eq!(p.interval_remote_minutes, 60);
    }

    #[test]
    fn dosya_yoksa_bos_config() {
        assert_eq!(Config::load(Path::new("/olmayan/yol.json")), Config::default());
    }

    #[test]
    fn bozuk_json_bos_config() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("config.json");
        std::fs::write(&file, "{bozuk").unwrap();
        assert_eq!(Config::load(&file), Config::default());
    }
}

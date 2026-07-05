use crate::config::Project;

pub struct Snapshot {
    pub changed_files: u32,
    pub remote_ahead: u32,
    pub unpushed: u32,
}

#[derive(Debug, PartialEq)]
pub enum Decision {
    /// Kural 1: geri sayımlı otomatik commit+push
    AutoBackup { changed: u32 },
    /// Kural 2: soru — çekilsin mi?
    PullQuestion { commits: u32 },
    /// Kural 3: günlük yedek, geri sayımlı
    DailyBackup,
    /// Push'u başarısız olmuş commit'leri sessizce tekrar dene
    SilentPush,
    Nothing,
}

/// Saf karar fonksiyonu. Öncelik: günlük yedek > kural 1 > kural 2 > sessiz push.
/// `today`: "YYYY-MM-DD". `is_backup_run`: bu tetik günlük yedek tetiği mi.
pub fn decide(p: &Project, s: &Snapshot, today: &str, is_backup_run: bool) -> Decision {
    if is_backup_run {
        if p.rule_backup && (s.changed_files > 0 || s.unpushed > 0) {
            return Decision::DailyBackup;
        }
        return Decision::Nothing;
    }
    let muted = p.muted_date.as_deref() == Some(today);
    if p.rule_changes && !muted && s.changed_files > p.threshold {
        return Decision::AutoBackup { changed: s.changed_files };
    }
    if p.rule_remote && s.remote_ahead > 0 {
        return Decision::PullQuestion { commits: s.remote_ahead };
    }
    if s.unpushed > 0 {
        return Decision::SilentPush;
    }
    Decision::Nothing
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn proje() -> Project {
        Project::new(PathBuf::from("/tmp/p"))
    }

    fn snap(changed: u32, remote: u32, unpushed: u32) -> Snapshot {
        Snapshot { changed_files: changed, remote_ahead: remote, unpushed }
    }

    #[test]
    fn esik_siniri_10dan_fazla() {
        let p = proje();
        assert_eq!(decide(&p, &snap(10, 0, 0), "2026-07-06", false), Decision::Nothing);
        assert_eq!(
            decide(&p, &snap(11, 0, 0), "2026-07-06", false),
            Decision::AutoBackup { changed: 11 }
        );
    }

    #[test]
    fn susturma_kural1i_keser_kural2yi_kesmez() {
        let mut p = proje();
        p.muted_date = Some("2026-07-06".into());
        assert_eq!(decide(&p, &snap(50, 0, 0), "2026-07-06", false), Decision::Nothing);
        assert_eq!(
            decide(&p, &snap(50, 3, 0), "2026-07-06", false),
            Decision::PullQuestion { commits: 3 }
        );
        // ertesi gün susturma düşer
        assert_eq!(
            decide(&p, &snap(50, 0, 0), "2026-07-07", false),
            Decision::AutoBackup { changed: 50 }
        );
    }

    #[test]
    fn remote_ilerideyse_soru() {
        let p = proje();
        assert_eq!(
            decide(&p, &snap(0, 2, 0), "2026-07-06", false),
            Decision::PullQuestion { commits: 2 }
        );
    }

    #[test]
    fn kural1_kural2den_oncelikli() {
        let p = proje();
        assert_eq!(
            decide(&p, &snap(11, 2, 0), "2026-07-06", false),
            Decision::AutoBackup { changed: 11 }
        );
    }

    #[test]
    fn push_bekleyen_sessiz_push() {
        let p = proje();
        assert_eq!(decide(&p, &snap(0, 0, 2), "2026-07-06", false), Decision::SilentPush);
    }

    #[test]
    fn gunluk_yedek_degisiklik_veya_unpushed() {
        let p = proje();
        assert_eq!(decide(&p, &snap(1, 0, 0), "2026-07-06", true), Decision::DailyBackup);
        assert_eq!(decide(&p, &snap(0, 0, 1), "2026-07-06", true), Decision::DailyBackup);
        assert_eq!(decide(&p, &snap(0, 5, 0), "2026-07-06", true), Decision::Nothing);
    }

    #[test]
    fn kapali_kurallar_calismaz() {
        let mut p = proje();
        p.rule_changes = false;
        p.rule_remote = false;
        p.rule_backup = false;
        assert_eq!(decide(&p, &snap(99, 0, 0), "2026-07-06", false), Decision::Nothing);
        assert_eq!(
            decide(&p, &snap(0, 5, 0), "2026-07-06", false),
            Decision::Nothing
        );
        assert_eq!(decide(&p, &snap(99, 0, 0), "2026-07-06", true), Decision::Nothing);
    }
}

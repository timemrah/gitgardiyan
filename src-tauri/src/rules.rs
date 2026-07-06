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

/// Hangi kuralların denetim vadesi geldi (kural bazlı sıklıklar).
pub struct Due {
    pub changes: bool,
    pub remote: bool,
}

/// Saf karar fonksiyonu. Öncelik: günlük yedek > kural 1 > kural 2 > sessiz push.
/// `today`: "YYYY-MM-DD". `is_backup_run`: bu tetik günlük yedek tetiği mi.
/// Kural 1 yalnız `due.changes`, kural 2 yalnız `due.remote` iken değerlendirilir;
/// SilentPush kural 1'in sıklığına bağlıdır.
/// SilentPush yalnızca en az bir otomatik commit kuralı (kural 1 veya kural 3)
/// aktifken çalışır — aksi halde kullanıcının kasıtlı yaptığı commit'ler sessizce
/// push edilmiş olur.
pub fn decide(p: &Project, s: &Snapshot, today: &str, is_backup_run: bool, due: &Due) -> Decision {
    if is_backup_run {
        if p.rule_backup && (s.changed_files > 0 || s.unpushed > 0) {
            return Decision::DailyBackup;
        }
        return Decision::Nothing;
    }
    let muted = p.muted_date.as_deref() == Some(today);
    if due.changes && p.rule_changes && !muted && s.changed_files > p.threshold {
        return Decision::AutoBackup { changed: s.changed_files };
    }
    if due.remote && p.rule_remote && s.remote_ahead > 0 {
        return Decision::PullQuestion { commits: s.remote_ahead };
    }
    if due.changes && (p.rule_changes || p.rule_backup) && s.unpushed > 0 {
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

    /// Her iki kuralın da vadesi gelmiş.
    fn hepsi() -> Due {
        Due { changes: true, remote: true }
    }

    #[test]
    fn esik_siniri_10dan_fazla() {
        let p = proje();
        assert_eq!(decide(&p, &snap(10, 0, 0), "2026-07-06", false, &hepsi()), Decision::Nothing);
        assert_eq!(
            decide(&p, &snap(11, 0, 0), "2026-07-06", false, &hepsi()),
            Decision::AutoBackup { changed: 11 }
        );
    }

    #[test]
    fn susturma_kural1i_keser_kural2yi_kesmez() {
        let mut p = proje();
        p.muted_date = Some("2026-07-06".into());
        assert_eq!(decide(&p, &snap(50, 0, 0), "2026-07-06", false, &hepsi()), Decision::Nothing);
        assert_eq!(
            decide(&p, &snap(50, 3, 0), "2026-07-06", false, &hepsi()),
            Decision::PullQuestion { commits: 3 }
        );
        // ertesi gün susturma düşer
        assert_eq!(
            decide(&p, &snap(50, 0, 0), "2026-07-07", false, &hepsi()),
            Decision::AutoBackup { changed: 50 }
        );
    }

    #[test]
    fn remote_ilerideyse_soru() {
        let p = proje();
        assert_eq!(
            decide(&p, &snap(0, 2, 0), "2026-07-06", false, &hepsi()),
            Decision::PullQuestion { commits: 2 }
        );
    }

    #[test]
    fn kural1_kural2den_oncelikli() {
        let p = proje();
        assert_eq!(
            decide(&p, &snap(11, 2, 0), "2026-07-06", false, &hepsi()),
            Decision::AutoBackup { changed: 11 }
        );
    }

    #[test]
    fn push_bekleyen_sessiz_push() {
        let p = proje();
        assert_eq!(decide(&p, &snap(0, 0, 2), "2026-07-06", false, &hepsi()), Decision::SilentPush);
    }

    #[test]
    fn gunluk_yedek_degisiklik_veya_unpushed() {
        let p = proje();
        assert_eq!(decide(&p, &snap(1, 0, 0), "2026-07-06", true, &hepsi()), Decision::DailyBackup);
        assert_eq!(decide(&p, &snap(0, 0, 1), "2026-07-06", true, &hepsi()), Decision::DailyBackup);
        assert_eq!(decide(&p, &snap(0, 5, 0), "2026-07-06", true, &hepsi()), Decision::Nothing);
    }

    #[test]
    fn kapali_kurallar_calismaz() {
        let mut p = proje();
        p.rule_changes = false;
        p.rule_remote = false;
        p.rule_backup = false;
        assert_eq!(decide(&p, &snap(99, 0, 0), "2026-07-06", false, &hepsi()), Decision::Nothing);
        assert_eq!(
            decide(&p, &snap(0, 5, 0), "2026-07-06", false, &hepsi()),
            Decision::Nothing
        );
        assert_eq!(decide(&p, &snap(99, 0, 0), "2026-07-06", true, &hepsi()), Decision::Nothing);
    }

    #[test]
    fn vadesi_gelmeyen_kural_degerlendirilmez() {
        let p = proje();
        let sadece_remote = Due { changes: false, remote: true };
        let sadece_changes = Due { changes: true, remote: false };
        // kural 1 vadesi gelmedi: eşik aşılsa da sessiz kalır, kural 2 çalışır
        assert_eq!(
            decide(&p, &snap(50, 2, 0), "2026-07-06", false, &sadece_remote),
            Decision::PullQuestion { commits: 2 }
        );
        assert_eq!(decide(&p, &snap(50, 0, 0), "2026-07-06", false, &sadece_remote), Decision::Nothing);
        // kural 2 vadesi gelmedi: remote ileride olsa da sorulmaz
        assert_eq!(decide(&p, &snap(0, 5, 0), "2026-07-06", false, &sadece_changes), Decision::Nothing);
        // sessiz push kural 1 sıklığına bağlı
        assert_eq!(decide(&p, &snap(0, 0, 2), "2026-07-06", false, &sadece_remote), Decision::Nothing);
        assert_eq!(decide(&p, &snap(0, 0, 2), "2026-07-06", false, &sadece_changes), Decision::SilentPush);
    }

    #[test]
    fn tum_kurallar_kapaliyken_silent_push_da_calismaz() {
        let mut p = proje();
        p.rule_changes = false;
        p.rule_remote = false;
        p.rule_backup = false;
        assert_eq!(decide(&p, &snap(0, 0, 3), "2026-07-06", false, &hepsi()), Decision::Nothing);
    }
}

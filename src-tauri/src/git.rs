use std::path::Path;
use std::process::Command;

fn run(repo: &Path, args: &[&str]) -> Result<String, String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .map_err(|e| format!("git çalıştırılamadı: {e}"))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        Err(format!(
            "{}{}",
            String::from_utf8_lossy(&out.stderr),
            String::from_utf8_lossy(&out.stdout)
        ))
    }
}

pub fn is_git_repo(repo: &Path) -> bool {
    run(repo, &["rev-parse", "--is-inside-work-tree"])
        .map(|s| s.trim() == "true")
        .unwrap_or(false)
}

/// Değişen dosya sayısı, untracked dahil (git status --porcelain satır sayısı).
pub fn changed_file_count(repo: &Path) -> Result<u32, String> {
    Ok(run(repo, &["status", "--porcelain"])?
        .lines()
        .filter(|l| !l.trim().is_empty())
        .count() as u32)
}

pub fn fetch(repo: &Path) -> Result<(), String> {
    run(repo, &["fetch", "--quiet"]).map(|_| ())
}

/// (remote_ileride, push_bekleyen). Upstream yoksa Err döner — çağıran kural atlar.
pub fn behind_ahead(repo: &Path) -> Result<(u32, u32), String> {
    let out = run(repo, &["rev-list", "--left-right", "--count", "@{u}...HEAD"])?;
    let mut it = out.split_whitespace();
    let behind = it.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let ahead = it.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    Ok((behind, ahead))
}

pub fn commit_all(repo: &Path, msg: &str) -> Result<(), String> {
    run(repo, &["add", "-A"])?;
    run(repo, &["commit", "-m", msg]).map(|_| ())
}

pub fn push(repo: &Path) -> Result<(), String> {
    run(repo, &["push", "--quiet"]).map(|_| ())
}

#[derive(Debug, PartialEq)]
pub enum PullOutcome {
    Ok,
    Conflict,
}

/// Çakışmada rebase'i geri alır (--abort) ve Conflict döner. Veri kaybı olmaz.
pub fn pull_rebase(repo: &Path) -> Result<PullOutcome, String> {
    match run(repo, &["pull", "--rebase", "--quiet"]) {
        Ok(_) => Ok(PullOutcome::Ok),
        Err(e) => {
            let gitdir_raw = run(repo, &["rev-parse", "--git-dir"])?;
            let gitdir = repo.join(gitdir_raw.trim());
            if gitdir.join("rebase-merge").exists() || gitdir.join("rebase-apply").exists() {
                run(repo, &["rebase", "--abort"])
                    .map_err(|e| format!("rebase geri alınamadı, depo rebase ortasında olabilir: {e}"))?;
                Ok(PullOutcome::Conflict)
            } else {
                Err(e)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// Yerel bare depo (remote) + iki klon kurar. Ağ gerekmez.
    fn setup() -> (tempfile::TempDir, PathBuf, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let remote = dir.path().join("remote.git");
        let a = dir.path().join("a");
        let b = dir.path().join("b");
        sh(dir.path(), &["init", "--bare", "-b", "main", remote.to_str().unwrap()]);
        sh(dir.path(), &["clone", remote.to_str().unwrap(), a.to_str().unwrap()]);
        sh(dir.path(), &["clone", remote.to_str().unwrap(), b.to_str().unwrap()]);
        for r in [&a, &b] {
            ident(r);
            std::process::Command::new("git")
                .args(["-C", r.to_str().unwrap(), "checkout", "-b", "main"])
                .output()
                .unwrap();
        }
        std::fs::write(a.join("ilk.txt"), "ilk").unwrap();
        commit_all(&a, "ilk").unwrap();
        sh(&a, &["push", "-u", "origin", "main"]);
        sh(&b, &["pull", "origin", "main"]);
        sh(&b, &["branch", "--set-upstream-to=origin/main", "main"]);
        (dir, a, b)
    }

    fn sh(cwd: &Path, args: &[&str]) {
        let out = std::process::Command::new("git")
            .arg("-C")
            .arg(cwd)
            .args(args)
            .output()
            .unwrap();
        assert!(out.status.success(), "git {:?}: {}", args, String::from_utf8_lossy(&out.stderr));
    }

    fn ident(repo: &Path) {
        sh(repo, &["config", "user.email", "test@test"]);
        sh(repo, &["config", "user.name", "Test"]);
        sh(repo, &["config", "commit.gpgsign", "false"]);
    }

    #[test]
    fn repo_tanima() {
        let (dir, a, _b) = setup();
        assert!(is_git_repo(&a));
        let plain = dir.path().join("duz");
        std::fs::create_dir(&plain).unwrap();
        assert!(!is_git_repo(&plain));
    }

    #[test]
    fn degisen_dosya_sayisi_untracked_dahil() {
        let (_dir, a, _b) = setup();
        assert_eq!(changed_file_count(&a).unwrap(), 0);
        std::fs::write(a.join("yeni1.txt"), "x").unwrap();
        std::fs::write(a.join("ilk.txt"), "degisti").unwrap();
        assert_eq!(changed_file_count(&a).unwrap(), 2);
    }

    #[test]
    fn behind_ahead_dogru_sayar() {
        let (_dir, a, b) = setup();
        std::fs::write(b.join("uzak.txt"), "u").unwrap();
        commit_all(&b, "uzak commit").unwrap();
        push(&b).unwrap();
        std::fs::write(a.join("yerel.txt"), "y").unwrap();
        commit_all(&a, "yerel commit").unwrap();
        fetch(&a).unwrap();
        assert_eq!(behind_ahead(&a).unwrap(), (1, 1));
    }

    #[test]
    fn upstream_yoksa_err() {
        let dir = tempfile::tempdir().unwrap();
        let solo = dir.path().join("solo");
        std::fs::create_dir(&solo).unwrap();
        sh(&solo, &["init", "-b", "main"]);
        ident(&solo);
        assert!(behind_ahead(&solo).is_err());
    }

    #[test]
    fn commit_push_sonrasi_temiz() {
        let (_dir, a, _b) = setup();
        std::fs::write(a.join("yeni.txt"), "x").unwrap();
        commit_all(&a, "test commit").unwrap();
        push(&a).unwrap();
        fetch(&a).unwrap();
        assert_eq!(changed_file_count(&a).unwrap(), 0);
        assert_eq!(behind_ahead(&a).unwrap(), (0, 0));
    }

    #[test]
    fn pull_rebase_temiz_cekme() {
        let (_dir, a, b) = setup();
        std::fs::write(b.join("uzak.txt"), "u").unwrap();
        commit_all(&b, "uzak").unwrap();
        push(&b).unwrap();
        fetch(&a).unwrap();
        assert_eq!(pull_rebase(&a).unwrap(), PullOutcome::Ok);
        assert!(a.join("uzak.txt").exists());
    }

    #[test]
    fn pull_rebase_cakismada_abort() {
        let (_dir, a, b) = setup();
        std::fs::write(b.join("ilk.txt"), "b degisikligi").unwrap();
        commit_all(&b, "b").unwrap();
        push(&b).unwrap();
        std::fs::write(a.join("ilk.txt"), "a degisikligi").unwrap();
        commit_all(&a, "a").unwrap();
        assert_eq!(pull_rebase(&a).unwrap(), PullOutcome::Conflict);
        // rebase geri alındı: çalışma ağacı a'nın kendi commit'inde, rebase devam etmiyor
        assert_eq!(std::fs::read_to_string(a.join("ilk.txt")).unwrap(), "a degisikligi");
        assert_eq!(changed_file_count(&a).unwrap(), 0);
    }
}

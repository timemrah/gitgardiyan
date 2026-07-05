# GitGardiyan Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Git projelerinde unutulan commit/push/pull işlemlerini geri sayımlı bildirimlerle otomatikleştiren, sistem tepsisinde çalışan çok platformlu masaüstü uygulaması.

**Architecture:** Tauri v2 uygulaması. Rust çekirdek: config (JSON), git modülü (sistem `git` komutu alt süreç), saf kural motoru, dakikalık zamanlayıcı döngüsü. Arayüz: bundler'sız düz HTML/JS (`withGlobalTauri`), iki pencere türü (yönetim + bildirim). Bildirim pencereleri geri sayımı JS'te yürütür, karar verilince `notify_action` komutuyla Rust'a döner.

**Tech Stack:** Rust (Tauri 2, serde, chrono), vanilla HTML/JS/CSS, GitHub Actions (tauri-action), tauri eklentileri: dialog, autostart, single-instance.

## Global Constraints

- Spec: `docs/superpowers/specs/2026-07-06-gitgardiyan-design.md` — çelişkide spec kazanır.
- Arayüz dili Türkçe. Bildirim metinleri spec'teki gibi (aşağıdaki görevlerde birebir verildi).
- Varsayılanlar: dosya eşiği **10** (kural `değişen > eşik`, yani 10'dan **fazla**), kontrol sıklığı **60 dk**, günlük yedek saati **"23:00"**, geri sayım **300 sn**.
- Otomatik commit mesajı formatı: `Otomatik yedek: YYYY-MM-DD HH:MM (N dosya)`.
- Aynı projede aynı anda **tek** bildirim penceresi (pencere etiketi proje bazlı: `notify-<hash>`).
- Git işlemleri daima sistemdeki `git` CLI ile; kütüphane yok.
- Rust dosyaları tek sorumluluk: config.rs, git.rs, rules.rs, log.rs, state.rs, commands.rs, notifier.rs, scheduler.rs, tray.rs.
- Commit'ler Conventional Commits; Türkçe özet serbest.
- `cargo test` her görev sonunda yeşil olmalı.

## File Structure

```
gitgardiyan/
├── package.json                  # sadece @tauri-apps/cli
├── ui/                           # frontend (bundler yok)
│   ├── index.html  main.js  style.css      # yönetim penceresi
│   └── notify.html notify.js               # bildirim penceresi
├── src-tauri/
│   ├── Cargo.toml  build.rs  tauri.conf.json
│   ├── capabilities/default.json
│   ├── icons/                    # şablondan gelen ikonlar
│   └── src/
│       ├── main.rs  lib.rs
│       ├── config.rs             # Config/Project + load/save
│       ├── git.rs                # git alt süreç sarmalayıcı
│       ├── rules.rs              # saf karar motoru (Decision)
│       ├── log.rs                # gitgardiyan.log
│       ├── state.rs              # AppState, NotifyPayload
│       ├── commands.rs           # tauri komutları
│       ├── notifier.rs           # bildirim penceresi açar
│       ├── scheduler.rs          # dakikalık döngü
│       └── tray.rs               # tepsi ikonu + menü
├── .github/workflows/release.yml
└── docs/...
```

---

### Task 1: Ortam kurulumu ve proje iskeleti

**Files:**
- Create: `package.json`, `src-tauri/` (şablondan), `ui/index.html`, `ui/notify.html`, `.gitignore`
- Modify: `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `src-tauri/src/main.rs`, `src-tauri/src/lib.rs`, `src-tauri/build.rs`

**Interfaces:**
- Produces: derlenen boş Tauri uygulaması; `gitgardiyan_lib::run()` giriş noktası; `ui/` frontend klasörü; sonraki tüm görevlerin çalışma zemini.

- [ ] **Step 1: Sistem önkoşulları (kullanıcıdan interaktif sudo iste)**

Kullanıcıya şunu çalıştırmasını söyle (`!` öneki ile):

```bash
sudo apt-get update && sudo apt-get install -y libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

- [ ] **Step 2: Rust kur (yoksa)**

```bash
command -v cargo || (curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y && . "$HOME/.cargo/env")
cargo --version
```
Expected: `cargo 1.8x.x` benzeri sürüm satırı.

- [ ] **Step 3: Tauri şablonunu geçici klasöre üret, ikon ve iskeleti taşı**

```bash
cd /tmp && npx create-tauri-app@latest gg-scaffold --template vanilla --manager npm --identifier com.emrah.gitgardiyan --yes
cp -rn /tmp/gg-scaffold/src-tauri /home/emrah/CLAUDE_SOFTWARE/gitgardiyan/
cp -n /tmp/gg-scaffold/package.json /home/emrah/CLAUDE_SOFTWARE/gitgardiyan/ 2>/dev/null || true
```
Not: bayrak adları CLI sürümüne göre değişebilir; interaktif sorulursa: template=vanilla, manager=npm. Amaç yalnızca `src-tauri/icons/` ve temel iskelet — diğer her dosyayı sonraki adımlar ezer.

- [ ] **Step 4: package.json'u sadeleştir**

```json
{
  "name": "gitgardiyan",
  "version": "0.1.0",
  "private": true,
  "scripts": { "tauri": "tauri" },
  "devDependencies": { "@tauri-apps/cli": "^2" }
}
```
Sonra: `npm install`

- [ ] **Step 5: src-tauri/Cargo.toml yaz**

```toml
[package]
name = "gitgardiyan"
version = "0.1.0"
edition = "2021"

[lib]
name = "gitgardiyan_lib"
crate-type = ["staticlib", "cdylib", "rlib"]

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = ["tray-icon"] }
tauri-plugin-dialog = "2"
tauri-plugin-autostart = "2"
tauri-plugin-single-instance = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
chrono = "0.4"

[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 6: build.rs, main.rs, lib.rs yaz**

`src-tauri/build.rs`:
```rust
fn main() {
    tauri_build::build()
}
```

`src-tauri/src/main.rs`:
```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    gitgardiyan_lib::run()
}
```

`src-tauri/src/lib.rs` (ilk hali; sonraki görevler genişletir):
```rust
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .run(tauri::generate_context!())
        .expect("tauri çalıştırılamadı");
}
```

- [ ] **Step 7: tauri.conf.json yaz**

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "GitGardiyan",
  "version": "0.1.0",
  "identifier": "com.emrah.gitgardiyan",
  "build": { "frontendDist": "../ui" },
  "app": {
    "withGlobalTauri": true,
    "windows": [
      { "label": "main", "title": "GitGardiyan", "width": 780, "height": 580, "visible": false }
    ],
    "security": { "csp": null }
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": ["icons/32x32.png", "icons/128x128.png", "icons/128x128@2x.png", "icons/icon.icns", "icons/icon.ico"]
  }
}
```

- [ ] **Step 8: ui/ iskeletini oluştur**

`ui/index.html`:
```html
<!doctype html>
<html lang="tr">
<head><meta charset="utf-8"><title>GitGardiyan</title></head>
<body><h1>GitGardiyan</h1></body>
</html>
```

`ui/notify.html`:
```html
<!doctype html>
<html lang="tr">
<head><meta charset="utf-8"><title>Bildirim</title></head>
<body></body>
</html>
```

- [ ] **Step 9: capabilities/default.json yaz**

`src-tauri/capabilities/default.json`:
```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "windows": ["main", "notify-*"],
  "permissions": [
    "core:default",
    "core:window:allow-close",
    "core:window:allow-hide",
    "core:window:allow-show",
    "core:window:allow-set-focus",
    "dialog:default",
    "autostart:default"
  ]
}
```

- [ ] **Step 10: .gitignore ve derleme doğrulaması**

`.gitignore` (repo köküne):
```
node_modules/
src-tauri/target/
dist/
```

Run: `cd src-tauri && cargo check`
Expected: hatasız tamamlanır (ilk seferde bağımlılık derlemesi uzun sürer, ~5-10 dk).

- [ ] **Step 11: Commit**

```bash
git add -A && git commit -m "chore: Tauri v2 proje iskeleti"
```

---

### Task 2: Config modülü

**Files:**
- Create: `src-tauri/src/config.rs`
- Modify: `src-tauri/src/lib.rs` (mod bildirimi)

**Interfaces:**
- Produces: `Config { projects: Vec<Project> }`, `Config::load(&Path) -> Config`, `Config::save(&self, &Path) -> io::Result<()>`; `Project { path: PathBuf, name: String, threshold: u32, interval_minutes: u32, backup_time: String, rule_changes: bool, rule_remote: bool, rule_backup: bool, muted_date: Option<String> }`.

- [ ] **Step 1: config.rs'i testleriyle yaz**

`src-tauri/src/config.rs`:
```rust
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Project {
    pub path: PathBuf,
    pub name: String,
    #[serde(default = "d_threshold")]
    pub threshold: u32,
    #[serde(default = "d_interval")]
    pub interval_minutes: u32,
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
            interval_minutes: d_interval(),
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
        assert_eq!(p.interval_minutes, 60);
        assert_eq!(p.backup_time, "23:00");
        assert!(p.rule_changes && p.rule_remote && p.rule_backup);
        assert_eq!(p.muted_date, None);
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
```

- [ ] **Step 2: lib.rs'e modülü ekle**

`src-tauri/src/lib.rs` başına:
```rust
pub mod config;
```

- [ ] **Step 3: Testleri çalıştır**

Run: `cd src-tauri && cargo test config`
Expected: 4 test PASS.

- [ ] **Step 4: Commit**

```bash
git add -A && git commit -m "feat: config modülü (yükle/kaydet, varsayılanlar)"
```

---

### Task 3: Git modülü

**Files:**
- Create: `src-tauri/src/git.rs`
- Modify: `src-tauri/src/lib.rs` (mod bildirimi)

**Interfaces:**
- Produces: `is_git_repo(&Path) -> bool`, `changed_file_count(&Path) -> Result<u32, String>`, `fetch(&Path) -> Result<(), String>`, `behind_ahead(&Path) -> Result<(u32, u32), String>` (dönen: `(remote_ileride, push_bekleyen)`; upstream yoksa `Err`), `commit_all(&Path, &str) -> Result<(), String>`, `push(&Path) -> Result<(), String>`, `pull_rebase(&Path) -> Result<PullOutcome, String>` (`PullOutcome::{Ok, Conflict}`; Conflict dönüşünde rebase zaten abort edilmiştir).

- [ ] **Step 1: git.rs'i testleriyle yaz**

`src-tauri/src/git.rs`:
```rust
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
                let _ = run(repo, &["rebase", "--abort"]);
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
```

- [ ] **Step 2: lib.rs'e modülü ekle**

```rust
pub mod git;
```

- [ ] **Step 3: Testleri çalıştır**

Run: `cd src-tauri && cargo test git`
Expected: 7 test PASS.

- [ ] **Step 4: Commit**

```bash
git add -A && git commit -m "feat: git modülü (status, fetch, commit, push, pull --rebase)"
```

---

### Task 4: Kural motoru

**Files:**
- Create: `src-tauri/src/rules.rs`
- Modify: `src-tauri/src/lib.rs` (mod bildirimi)

**Interfaces:**
- Consumes: `config::Project`.
- Produces: `Snapshot { changed_files: u32, remote_ahead: u32, unpushed: u32 }`; `Decision::{AutoBackup { changed: u32 }, PullQuestion { commits: u32 }, DailyBackup, SilentPush, Nothing}`; `decide(&Project, &Snapshot, today: &str, is_backup_run: bool) -> Decision`.

- [ ] **Step 1: rules.rs'i testleriyle yaz**

`src-tauri/src/rules.rs`:
```rust
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
```

- [ ] **Step 2: lib.rs'e modülü ekle**

```rust
pub mod rules;
```

- [ ] **Step 3: Testleri çalıştır**

Run: `cd src-tauri && cargo test rules`
Expected: 7 test PASS.

- [ ] **Step 4: Commit**

```bash
git add -A && git commit -m "feat: kural motoru (eşik, susturma, öncelik, günlük yedek)"
```

---

### Task 5: Log, AppState ve temel komutlar

**Files:**
- Create: `src-tauri/src/log.rs`, `src-tauri/src/state.rs`, `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`

**Interfaces:**
- Consumes: `config::{Config, Project}`, `git`.
- Produces: `log::line(&Path, &str)`; `AppState { config: Mutex<Config>, config_path: PathBuf, log_dir: PathBuf, notifications: Mutex<HashMap<String, NotifyPayload>> }`; `NotifyPayload { ptype: String, rule: u8, path: String, message: String, seconds: u32 }`; komutlar: `list_projects`, `add_project`, `remove_project`, `update_project`, `get_notification`.

- [ ] **Step 1: log.rs yaz**

`src-tauri/src/log.rs`:
```rust
use std::io::Write;
use std::path::Path;

/// `<dir>/gitgardiyan.log` dosyasına zaman damgalı satır ekler. Hata yutar —
/// loglama asla uygulamayı düşürmez.
pub fn line(dir: &Path, msg: &str) {
    let ts = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    let _ = std::fs::create_dir_all(dir);
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("gitgardiyan.log"))
        .and_then(|mut f| f.write_all(format!("[{ts}] {msg}\n").as_bytes()));
}

#[cfg(test)]
mod tests {
    #[test]
    fn satir_eklenir() {
        let dir = tempfile::tempdir().unwrap();
        super::line(dir.path(), "deneme");
        super::line(dir.path(), "ikinci");
        let s = std::fs::read_to_string(dir.path().join("gitgardiyan.log")).unwrap();
        assert!(s.contains("deneme") && s.contains("ikinci"));
        assert_eq!(s.lines().count(), 2);
    }
}
```

- [ ] **Step 2: state.rs yaz**

`src-tauri/src/state.rs`:
```rust
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
```

- [ ] **Step 3: commands.rs yaz**

`src-tauri/src/commands.rs`:
```rust
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
    Ok(out)
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
```

- [ ] **Step 4: lib.rs'i genişlet**

`src-tauri/src/lib.rs` tamamı:
```rust
pub mod commands;
pub mod config;
pub mod git;
pub mod log;
pub mod rules;
pub mod state;

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
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_projects,
            commands::add_project,
            commands::remove_project,
            commands::update_project,
            commands::get_notification,
        ])
        .run(tauri::generate_context!())
        .expect("tauri çalıştırılamadı");
}
```

- [ ] **Step 5: Derle ve test et**

Run: `cd src-tauri && cargo test`
Expected: önceki tüm testler + `log` testi PASS; `cargo check` hatasız.

- [ ] **Step 6: Commit**

```bash
git add -A && git commit -m "feat: uygulama durumu, log ve proje yönetim komutları"
```

---

### Task 6: Bildirim penceresi (notifier + notify.html/js + notify_action)

**Files:**
- Create: `src-tauri/src/notifier.rs`, `ui/notify.js`
- Modify: `ui/notify.html`, `src-tauri/src/commands.rs` (notify_action ekle), `src-tauri/src/lib.rs` (mod + handler)

**Interfaces:**
- Consumes: `rules::{Decision, Snapshot}`, `state::{AppState, NotifyPayload}`, `git`, `log`.
- Produces: `notifier::show(&AppHandle, &Project, &Decision)` — pencere açar, proje başına tek pencere garantisi; komut `notify_action(path, rule, action) -> Result<String, String>`, action ∈ `"proceed" | "pull" | "mute"`, dönen `String` kullanıcıya gösterilecek sonuç metni.

- [ ] **Step 1: notifier.rs yaz**

`src-tauri/src/notifier.rs`:
```rust
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
```

- [ ] **Step 2: commands.rs'e notify_action ekle**

`src-tauri/src/commands.rs` sonuna:
```rust
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
```
Not: `"mute"` kolunda `MutexGuard` await sınırı geçmiyor (fonksiyonda await yok) — derleyici uyarı verirse guard'ı `{ }` bloğuna al.

- [ ] **Step 3: lib.rs'e ekle**

`pub mod notifier;` satırı ve handler listesine `commands::notify_action`.

- [ ] **Step 4: notify.html ve notify.js yaz**

`ui/notify.html`:
```html
<!doctype html>
<html lang="tr">
<head>
<meta charset="utf-8">
<title>GitGardiyan</title>
<style>
  * { margin: 0; padding: 0; box-sizing: border-box; }
  body {
    font-family: system-ui, sans-serif;
    background: #1e2430; color: #e8ecf1;
    height: 100vh; display: flex; flex-direction: column;
    padding: 16px; border: 1px solid #3a4356; border-radius: 8px;
    user-select: none; cursor: default;
  }
  #title { font-size: 12px; color: #8b96a8; margin-bottom: 8px; }
  #msg { font-size: 14px; line-height: 1.45; flex: 1; }
  #timer { font-size: 26px; font-weight: 700; text-align: center; margin: 6px 0; color: #ffb454; }
  #result { font-size: 13px; flex: 1; color: #9ecb7d; }
  #result.err { color: #e06c75; }
  #buttons { display: flex; gap: 8px; justify-content: flex-end; }
  button {
    padding: 7px 12px; border: 1px solid #3a4356; border-radius: 6px;
    background: #2a3244; color: #e8ecf1; cursor: pointer; font-size: 12px;
  }
  button:hover { background: #364059; }
  button.primary { background: #2d7d46; border-color: #2d7d46; }
  button.primary:hover { background: #35934f; }
</style>
</head>
<body>
  <div id="title">GitGardiyan</div>
  <div id="msg"></div>
  <div id="timer"></div>
  <div id="result" hidden></div>
  <div id="buttons"></div>
  <script src="notify.js"></script>
</body>
</html>
```

`ui/notify.js`:
```js
const { invoke } = window.__TAURI__.core;
const appWindow = window.__TAURI__.webviewWindow.getCurrentWebviewWindow();

let payload = null;
let timerId = null;
let remaining = 0;
let acted = false;

function addBtn(text, cls, fn) {
  const b = document.createElement('button');
  b.textContent = text;
  if (cls) b.className = cls;
  b.addEventListener('click', fn);
  document.getElementById('buttons').appendChild(b);
}

function tick() {
  if (remaining <= 0) { act('proceed'); return; }
  const m = String(Math.floor(remaining / 60)).padStart(2, '0');
  const s = String(remaining % 60).padStart(2, '0');
  document.getElementById('timer').textContent = `${m}:${s}`;
  remaining--;
}

async function act(action) {
  if (acted) return;
  acted = true;
  clearInterval(timerId);
  document.getElementById('timer').textContent = '';
  document.getElementById('buttons').innerHTML = '';
  const result = document.getElementById('result');
  result.hidden = false;
  result.textContent = 'Çalışıyor…';
  let closeDelay = 6000;
  try {
    result.textContent = await invoke('notify_action', {
      path: payload.path,
      rule: payload.rule,
      action,
    });
    if (action === 'mute') closeDelay = 2500;
  } catch (e) {
    result.textContent = String(e);
    result.className = 'err';
    closeDelay = 15000;
  }
  setTimeout(() => appWindow.close(), closeDelay);
}

async function init() {
  payload = await invoke('get_notification', { label: appWindow.label });
  if (!payload) { appWindow.close(); return; }
  document.getElementById('msg').textContent = payload.message;

  if (payload.ptype === 'countdown') {
    remaining = payload.seconds;
    tick();
    timerId = setInterval(tick, 1000);
    addBtn('Şimdi yap', 'primary', () => act('proceed'));
    if (payload.rule === 1) addBtn('Bugün bir daha sorma', '', () => act('mute'));
    addBtn('İptal', '', () => appWindow.close());
  } else {
    addBtn('Çek', 'primary', () => act('pull'));
    addBtn('Boşver', '', () => appWindow.close());
  }
}

init();
```

- [ ] **Step 5: Derle**

Run: `cd src-tauri && cargo check && cargo test`
Expected: hatasız; tüm testler PASS.

- [ ] **Step 6: Commit**

```bash
git add -A && git commit -m "feat: geri sayımlı bildirim penceresi ve notify_action akışı"
```

---

### Task 7: Zamanlayıcı

**Files:**
- Create: `src-tauri/src/scheduler.rs`
- Modify: `src-tauri/src/lib.rs` (mod + setup içinde start)

**Interfaces:**
- Consumes: `AppState`, `git`, `rules::{decide, Snapshot, Decision}`, `notifier::show`, `log`.
- Produces: `scheduler::start(AppHandle)` — dakikada bir tüm projeleri değerlendirir.

- [ ] **Step 1: scheduler.rs yaz**

`src-tauri/src/scheduler.rs`:
```rust
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
    tauri::async_runtime::spawn(async move {
        let mut last_check: HashMap<PathBuf, Instant> = HashMap::new();
        let mut last_backup: HashMap<PathBuf, String> = HashMap::new();
        loop {
            tick(&app, &mut last_check, &mut last_backup);
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
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
```
Not: `tokio` doğrudan bağımlılık değil; `tauri::async_runtime` tokio kullanır ama `tokio::time` yolu için Cargo.toml'a `tokio = { version = "1", features = ["time"] }` eklenmeli.

- [ ] **Step 2: Cargo.toml'a tokio ekle**

`[dependencies]` altına:
```toml
tokio = { version = "1", features = ["time"] }
```

- [ ] **Step 3: lib.rs'e bağla**

`pub mod scheduler;` ekle; `setup` içinde `app.manage(...)` sonrasına:
```rust
scheduler::start(app.handle().clone());
```

- [ ] **Step 4: Derle ve test et**

Run: `cd src-tauri && cargo test`
Expected: tüm testler PASS, uyarısız derleme hedef.

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat: dakikalık zamanlayıcı — saatlik kontrol ve günlük yedek tetiği"
```

---

### Task 8: Tepsi ikonu, otomatik başlatma, kapatınca gizlenme

**Files:**
- Create: `src-tauri/src/tray.rs`
- Modify: `src-tauri/src/lib.rs`, `src-tauri/src/commands.rs` (autostart komutları)

**Interfaces:**
- Consumes: Tauri tray/menu API, `tauri_plugin_autostart::ManagerExt`.
- Produces: `tray::create(&App) -> tauri::Result<()>`; komutlar `get_autostart() -> bool`, `set_autostart(enabled: bool)`. Ana pencere kapatılınca gizlenir, uygulama tepsiden "Çıkış" ile kapanır.

- [ ] **Step 1: tray.rs yaz**

`src-tauri/src/tray.rs`:
```rust
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{App, Manager};

pub fn create(app: &App) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "Yönetim Penceresi", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Çıkış", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;

    TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip("GitGardiyan")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok(())
}
```

- [ ] **Step 2: autostart komutlarını ekle**

`src-tauri/src/commands.rs` sonuna:
```rust
use tauri_plugin_autostart::ManagerExt;

#[tauri::command]
pub async fn get_autostart(app: tauri::AppHandle) -> Result<bool, String> {
    app.autolaunch().is_enabled().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_autostart(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    let al = app.autolaunch();
    if enabled { al.enable() } else { al.disable() }.map_err(|e| e.to_string())
}
```

- [ ] **Step 3: lib.rs'i tamamla**

`pub mod tray;` ekle. Builder zincirine (single_instance'tan sonra):
```rust
.plugin(tauri_plugin_autostart::init(
    tauri_plugin_autostart::MacosLauncher::LaunchAgent,
    None,
))
```
`setup` içine (scheduler::start'tan sonra):
```rust
tray::create(app)?;
// ilk kurulumda otomatik başlatmayı aç; kullanıcı arayüzden kapatabilir
let _ = tauri_plugin_autostart::ManagerExt::autolaunch(app.handle()).enable();
```
Builder zincirine `.invoke_handler(...)`'dan sonra:
```rust
.on_window_event(|window, event| {
    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
        if window.label() == "main" {
            let _ = window.hide();
            api.prevent_close();
        }
    }
})
```
Handler listesine `commands::get_autostart, commands::set_autostart` ekle.

- [ ] **Step 4: Derle**

Run: `cd src-tauri && cargo check && cargo test`
Expected: hatasız, testler PASS.

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat: tepsi ikonu, otomatik başlatma, kapatınca gizlenme"
```

---

### Task 9: Yönetim penceresi arayüzü

**Files:**
- Modify: `ui/index.html`
- Create: `ui/main.js`, `ui/style.css`

**Interfaces:**
- Consumes: komutlar `list_projects` (dönen `ProjectView[]`: `{ project: { path, name, threshold, interval_minutes, backup_time, rule_changes, rule_remote, rule_backup, muted_date }, found, changed, remote_ahead, unpushed }`), `add_project(path)`, `remove_project(path)`, `update_project(project)`, `get_autostart()`, `set_autostart(enabled)`; dialog: `window.__TAURI__.dialog.open({ directory: true })`.

- [ ] **Step 1: index.html yaz**

`ui/index.html`:
```html
<!doctype html>
<html lang="tr">
<head>
<meta charset="utf-8">
<title>GitGardiyan</title>
<link rel="stylesheet" href="style.css">
</head>
<body>
  <header>
    <h1>GitGardiyan</h1>
    <label class="auto"><input type="checkbox" id="autostart"> Açılışta başlat</label>
    <button id="refresh">Yenile</button>
    <button id="add" class="primary">Proje Ekle</button>
  </header>
  <div id="error" hidden></div>
  <main id="list"></main>
  <template id="row">
    <section class="project">
      <div class="head">
        <strong class="name"></strong>
        <span class="status"></span>
        <button class="remove">Kaldır</button>
      </div>
      <div class="path"></div>
      <div class="settings">
        <label>Dosya eşiği <input type="number" class="threshold" min="1"></label>
        <label>Kontrol (dk) <input type="number" class="interval" min="5"></label>
        <label>Yedek saati <input type="time" class="backup"></label>
        <label><input type="checkbox" class="r1"> Değişiklik kuralı</label>
        <label><input type="checkbox" class="r2"> Uzak kontrol</label>
        <label><input type="checkbox" class="r3"> Günlük yedek</label>
        <button class="save">Kaydet</button>
      </div>
    </section>
  </template>
  <script src="main.js"></script>
</body>
</html>
```

- [ ] **Step 2: style.css yaz**

`ui/style.css`:
```css
* { margin: 0; padding: 0; box-sizing: border-box; }
body { font-family: system-ui, sans-serif; background: #1e2430; color: #e8ecf1; padding: 16px; }
header { display: flex; align-items: center; gap: 12px; margin-bottom: 14px; }
h1 { font-size: 18px; flex: 1; }
.auto { font-size: 13px; color: #8b96a8; }
button { padding: 7px 12px; border: 1px solid #3a4356; border-radius: 6px; background: #2a3244; color: #e8ecf1; cursor: pointer; font-size: 12px; }
button:hover { background: #364059; }
button.primary { background: #2d7d46; border-color: #2d7d46; }
#error { background: #5c2b31; border: 1px solid #e06c75; border-radius: 6px; padding: 8px 12px; font-size: 13px; margin-bottom: 12px; }
.project { background: #262e3e; border: 1px solid #3a4356; border-radius: 8px; padding: 12px; margin-bottom: 10px; }
.head { display: flex; align-items: center; gap: 10px; }
.head .name { flex: 0 0 auto; }
.head .status { flex: 1; font-size: 12px; color: #8b96a8; }
.head .status.warn { color: #ffb454; }
.path { font-size: 11px; color: #6b7688; margin: 4px 0 10px; }
.settings { display: flex; flex-wrap: wrap; gap: 10px; align-items: center; font-size: 12px; color: #b6bfcd; }
.settings input[type="number"] { width: 60px; }
.settings input { background: #1e2430; color: #e8ecf1; border: 1px solid #3a4356; border-radius: 4px; padding: 4px 6px; }
.empty { color: #6b7688; text-align: center; padding: 40px 0; font-size: 14px; }
```

- [ ] **Step 3: main.js yaz**

`ui/main.js`:
```js
const { invoke } = window.__TAURI__.core;
const { open } = window.__TAURI__.dialog;

const errBox = document.getElementById('error');

function showError(e) {
  errBox.hidden = false;
  errBox.textContent = String(e);
  setTimeout(() => { errBox.hidden = true; }, 6000);
}

function statusText(v) {
  if (!v.found) return { text: 'Klasör bulunamadı ya da git deposu değil', warn: true };
  const parts = [];
  if (v.changed > 0) parts.push(`${v.changed} dosya değişti`);
  if (v.unpushed > 0) parts.push(`${v.unpushed} commit push bekliyor`);
  if (v.remote_ahead > 0) parts.push(`GitHub ${v.remote_ahead} commit ileride`);
  if (parts.length === 0) return { text: 'Temiz ✓', warn: false };
  return { text: parts.join(' · '), warn: true };
}

function render(views) {
  const list = document.getElementById('list');
  list.innerHTML = '';
  if (views.length === 0) {
    list.innerHTML = '<div class="empty">Henüz proje yok. "Proje Ekle" ile başla.</div>';
    return;
  }
  const tpl = document.getElementById('row');
  for (const v of views) {
    const p = v.project;
    const el = tpl.content.cloneNode(true);
    el.querySelector('.name').textContent = p.name;
    const st = statusText(v);
    const stEl = el.querySelector('.status');
    stEl.textContent = st.text;
    if (st.warn) stEl.classList.add('warn');
    el.querySelector('.path').textContent = p.path;
    el.querySelector('.threshold').value = p.threshold;
    el.querySelector('.interval').value = p.interval_minutes;
    el.querySelector('.backup').value = p.backup_time;
    el.querySelector('.r1').checked = p.rule_changes;
    el.querySelector('.r2').checked = p.rule_remote;
    el.querySelector('.r3').checked = p.rule_backup;

    const root = el.querySelector('.project');
    el.querySelector('.remove').addEventListener('click', async () => {
      try { await invoke('remove_project', { path: p.path }); refresh(); }
      catch (e) { showError(e); }
    });
    el.querySelector('.save').addEventListener('click', async () => {
      const updated = {
        ...p,
        threshold: parseInt(root.querySelector('.threshold').value, 10) || 10,
        interval_minutes: parseInt(root.querySelector('.interval').value, 10) || 60,
        backup_time: root.querySelector('.backup').value || '23:00',
        rule_changes: root.querySelector('.r1').checked,
        rule_remote: root.querySelector('.r2').checked,
        rule_backup: root.querySelector('.r3').checked,
      };
      try { await invoke('update_project', { project: updated }); refresh(); }
      catch (e) { showError(e); }
    });
    list.appendChild(el);
  }
}

async function refresh() {
  try { render(await invoke('list_projects')); }
  catch (e) { showError(e); }
}

document.getElementById('add').addEventListener('click', async () => {
  try {
    const dir = await open({ directory: true, title: 'Proje klasörü seç' });
    if (!dir) return;
    await invoke('add_project', { path: dir });
    refresh();
  } catch (e) { showError(e); }
});

document.getElementById('refresh').addEventListener('click', refresh);

const autoEl = document.getElementById('autostart');
autoEl.addEventListener('change', async () => {
  try { await invoke('set_autostart', { enabled: autoEl.checked }); }
  catch (e) { showError(e); }
});

(async () => {
  try { autoEl.checked = await invoke('get_autostart'); } catch (_) {}
  refresh();
})();
```

- [ ] **Step 4: Geliştirme modunda elle doğrula**

Run: `npx tauri dev`
Kontrol listesi: tepsi ikonu görünür; "Yönetim Penceresi" menüsü pencereyi açar; "Proje Ekle" klasör seçtirir, git olmayan klasör Türkçe hata verir; git deposu eklenir ve durum satırı görünür; ayarlar kaydedilir (config.json'a bakılır: `~/.config/com.emrah.gitgardiyan/config.json`); pencere X ile kapatılınca uygulama tepsiden yaşamaya devam eder.

- [ ] **Step 5: Commit**

```bash
git add -A && git commit -m "feat: yönetim penceresi — proje listesi, durum ve ayarlar"
```

---

### Task 10: GitHub Actions dağıtımı ve README

**Files:**
- Create: `.github/workflows/release.yml`, `README.md`

**Interfaces:**
- Produces: `v*` etiketi pushlanınca üç platformda paket üretip GitHub Release'e ekleyen iş akışı.

- [ ] **Step 1: release.yml yaz**

`.github/workflows/release.yml`:
```yaml
name: release

on:
  push:
    tags: ['v*']

jobs:
  build:
    permissions:
      contents: write
    strategy:
      fail-fast: false
      matrix:
        include:
          - platform: ubuntu-22.04
          - platform: windows-latest
          - platform: macos-latest
    runs-on: ${{ matrix.platform }}
    steps:
      - uses: actions/checkout@v4

      - uses: actions/setup-node@v4
        with:
          node-version: 20

      - uses: dtolnay/rust-toolchain@stable

      - name: Linux bağımlılıkları
        if: matrix.platform == 'ubuntu-22.04'
        run: |
          sudo apt-get update
          sudo apt-get install -y libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev

      - run: npm install

      - uses: tauri-apps/tauri-action@v0
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        with:
          tagName: ${{ github.ref_name }}
          releaseName: 'GitGardiyan ${{ github.ref_name }}'
          releaseDraft: false
          prerelease: false
```

- [ ] **Step 2: README.md yaz**

```markdown
# GitGardiyan

Git projelerinde commit, push ve pull işlemlerini unutmayı engelleyen sistem tepsisi uygulaması. Linux, macOS ve Windows.

## Ne yapar?

- **Saatlik değişiklik kontrolü:** Eşikten (varsayılan 10) fazla dosya değiştiyse 5 dakika geri sayan bildirim gösterir; iptal edilmezse otomatik commit + push yapar.
- **Saatlik uzak kontrol:** GitHub yereldekinden ilerideyse "çekmek ister misiniz?" diye sorar (`pull --rebase`; çakışmada geri alır, elle çözüme bırakır).
- **Günlük yedek (23:00):** Bekleyen commit/push varsa geri sayımlı bildirimle günlük yedek alır.

Tasarım: `docs/superpowers/specs/2026-07-06-gitgardiyan-design.md`

## Kurulum (geliştirici)

Gereksinimler: Rust (rustup), Node.js, sistemde `git`. Linux'ta ayrıca:
`libwebkit2gtk-4.1-dev build-essential libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev`

```bash
npm install
npx tauri dev      # geliştirme
npx tauri build    # paket üretimi
```

## Sürüm çıkarma

`v*` etiketi pushlayınca GitHub Actions üç platform için paketleri üretip Release'e ekler:

```bash
git tag v0.1.0 && git push origin v0.1.0
```

Not: macOS paketi imzasızdır; ilk açılışta sağ tık → Aç gerekir.

## Ayarlar

Config: işletim sisteminin standart config klasöründe `com.emrah.gitgardiyan/config.json`. Log: aynı klasörde `gitgardiyan.log`.
```

- [ ] **Step 3: Commit**

```bash
git add -A && git commit -m "ci: üç platform için release iş akışı ve README"
```

---

### Task 11: Uçtan uca duman testi (elle, Linux)

**Files:** yok (doğrulama görevi)

**Interfaces:**
- Consumes: tüm uygulama.

- [ ] **Step 1: Deneme deposu hazırla**

```bash
mkdir -p /tmp/gg-test && cd /tmp/gg-test
git init --bare -b main remote.git
git clone remote.git proje && cd proje
git config user.email t@t && git config user.name Test
echo ilk > ilk.txt && git add -A && git commit -m ilk && git push -u origin main
```

- [ ] **Step 2: Kural 1'i tetikle**

Uygulamayı `npx tauri dev` ile başlat, `/tmp/gg-test/proje`'yi ekle. Kontrol sıklığını 5 dk'ya indirilebilir yapmak yerine hızlı test için: projede 11 dosya oluştur:

```bash
cd /tmp/gg-test/proje && for i in $(seq 1 11); do echo $i > dosya$i.txt; done
```

Uygulamayı kapatıp yeniden başlat (ilk tick açılışta hemen çalışır). Beklenen: sağ altta geri sayımlı bildirim, metin "proje projesinde çok fazla dosya değişikliği var (11 dosya)…". "Şimdi yap" → pencere "commit ve push tamamlandı" gösterir. Doğrula:

```bash
git -C /tmp/gg-test/proje log --oneline -1   # "Otomatik yedek: ..." commit'i
git -C /tmp/gg-test/proje status --porcelain # boş
```

- [ ] **Step 3: Kural 2'yi tetikle**

```bash
cd /tmp/gg-test && git clone remote.git proje2 && cd proje2
git config user.email t@t && git config user.name Test
echo uzak > uzak.txt && git add -A && git commit -m uzak && git push
```

Uygulamayı yeniden başlat. Beklenen: "proje projesinin GitHub deposunda 1 yeni commit var. Çekmek ister misiniz?" — "Çek" → `uzak.txt` yerelde belirir.

- [ ] **Step 4: Susturma ve iptal davranışı**

11 dosya daha oluştur, uygulamayı yeniden başlat → bildirimde "Bugün bir daha sorma" → config.json'da `muted_date` bugünün tarihi olmalı; uygulama yeniden başlatılınca bildirim ÇIKMAMALI.

- [ ] **Step 5: Günlük yedek**

Yönetim penceresinden yedek saatini şu andan 2 dk sonrasına ayarla (susturma kural 3'ü etkilemez). Beklenen: saati gelince "bekleyen commit/push var. Günlük yedek alınacak." bildirimi; süre dolunca (veya "Şimdi yap") commit + push.

- [ ] **Step 6: Sonuçları raporla ve temizle**

Tüm adımlar geçtiyse `/tmp/gg-test` silinir, sonuç kullanıcıya raporlanır. Geçmeyen adım varsa superpowers:systematic-debugging ile ele alınır.

---

## Self-Review Notları

- Spec kapsaması: Kural 1 (Task 4+6+7), Kural 2 (Task 3+4+6), Kural 3 (Task 4+7), susturma (Task 4+6), tek bildirim (Task 6 notifier + Task 7 continue), push hatası yeniden deneme (SilentPush — Task 4+7), config (Task 2), log (Task 5), tepsi+autostart (Task 8), yönetim penceresi (Task 9), CI (Task 10). Kapsam dışılar plana alınmadı.
- Tip tutarlılığı: `Decision`/`Snapshot`/`Project`/`NotifyPayload` alan adları görevler arasında birebir aynı; JS tarafı `project.interval_minutes` gibi snake_case kullanır (serde varsayılanı).
- Bilinen esneklikler: create-tauri-app bayrakları sürüme göre değişebilir (Task 1 Step 3 notu); Tauri API imzalarında küçük sapmalar derleyici yönlendirmesiyle düzeltilir.

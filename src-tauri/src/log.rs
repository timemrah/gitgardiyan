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

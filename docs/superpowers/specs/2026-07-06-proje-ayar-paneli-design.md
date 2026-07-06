# Proje Ayar Paneli — Tasarım

Tarih: 2026-07-06 · Durum: Onaylandı

## Amaç

Proje listesindeki ayar inputları kalabalık yapıyor. Ayarlar varsayılan gizli olacak,
"Ayarlar" butonuyla açılacak. Her kural kendi satırında: toggle + parametreler + kısa
açıklama. Ayrıca "Kontrol (dk)" tek ortak denetim sıklığı olmaktan çıkıyor — her kural
kendi sıklığını taşıyor.

## UI (ui/index.html, main.js, style.css)

Proje satırı: ad + durum + [Ayarlar] + [Kaldır], altında path. Panel varsayılan gizli,
"Ayarlar" butonu açar/kapar (buton açıkken aktif görünümde).

Panel — 3 kural satırı, her satırda toggle switch (saf CSS), ad, parametre inputları,
altında kısa açıklama:

| Kural | Toggle alanı | Parametreler | Açıklama |
|---|---|---|---|
| Dosya eşiği | `rule_changes` | eşik (sayı, min 1), sıklık dk (min 5) | Bu sıklıkla bakılır; değişen dosya sayısı eşiği aşarsa otomatik commit + push. |
| Uzak kontrol | `rule_remote` | sıklık dk (min 5) | Bu sıklıkla GitHub'a bakılır; yeni commit varsa çekmek isteyip istemediğin sorulur. |
| Günlük yedek | `rule_backup` | saat (time) | Her gün bu saatte bekleyen değişiklikler yedeklenir. |

Toggle kapalıyken o satırın inputları disabled + soluk. [Kaydet] panelde kalır.
Koyu tema korunur; panel ince ayraç çizgisiyle gövdeden ayrılır. Modern, sade.

## Config (src-tauri/src/config.rs)

- `interval_minutes` kalkar; yerine:
  - `interval_changes_minutes: u32` — `#[serde(default = "d_interval", alias = "interval_minutes")]`
    (eski config değeri kural 1 sıklığına devrolur)
  - `interval_remote_minutes: u32` — `#[serde(default = "d_interval")]`

## Karar mantığı (src-tauri/src/rules.rs)

`decide` hangi kuralların vadesinin geldiğini bilmeli:

```rust
pub struct Due { pub changes: bool, pub remote: bool }
pub fn decide(p: &Project, s: &Snapshot, today: &str, is_backup_run: bool, due: &Due) -> Decision
```

- Kural 1 yalnız `due.changes` iken değerlendirilir.
- Kural 2 yalnız `due.remote` iken değerlendirilir.
- `SilentPush` kural 1 sıklığına bağlıdır (`due.changes` gerektirir).
- Öncelik sırası değişmez: günlük yedek > kural 1 > kural 2 > sessiz push.
- Testler yeni imzaya göre güncellenir + vade bayrakları için yeni testler.

## Zamanlayıcı (src-tauri/src/scheduler.rs)

- `last_check: HashMap<PathBuf, Instant>` yerine kural bazlı iki harita
  (veya `HashMap<PathBuf, (Instant kural1, Instant kural2)>`).
- Tick 60 sn kalır. Her projede: kural 1 vadesi `interval_changes_minutes`,
  kural 2 vadesi `interval_remote_minutes` ile ayrı hesaplanır.
- `git fetch` yalnız kural 2 vadesi gelmiş ve `rule_remote` açıkken çalışır.
- Hiçbir kuralın vadesi gelmediyse snapshot alınmaz (gereksiz git çağrısı yok).

## Kapsam dışı

- Bildirim penceresi (notify.html/js) değişmez.
- Kural öncelik mantığı değişmez.

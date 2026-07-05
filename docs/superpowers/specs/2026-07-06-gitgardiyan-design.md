# GitGardiyan — Tasarım Dokümanı

**Tarih:** 2026-07-06
**Durum:** Onay bekliyor

## Amaç

Geliştiricinin git projelerinde commit, push ve pull işlemlerini unutmasını engelleyen, sistem tepsisinde sürekli çalışan bir masaüstü uygulaması. Belirlenen kurallara göre projeleri periyodik olarak kontrol eder, gerektiğinde geri sayımlı bildirimler gösterir ve kullanıcı iptal etmezse commit/push işlemlerini otomatik yapar.

## Platformlar

Linux, macOS ve Windows. Tek kod tabanı, üç platforma da kurulum paketi üretilir.

## Teknoloji

- **Çatı:** Tauri v2 (Rust çekirdek + HTML/JS arayüz). Tercih sebebi: uygulama 7/24 arka planda çalışacağı için düşük kaynak tüketimi (~10 MB paket, ~40 MB RAM).
- **Zamanlama:** İşletim sistemi cron'u kullanılmaz. Uygulama içi zamanlayıcı ile tüm platformlarda aynı davranış sağlanır (Windows'ta cron yoktur; ayrıca geri sayımlı iptal penceresi sistem bildirimleriyle güvenilir yapılamaz).
- **Git işlemleri:** Sistemde kurulu `git` komutu çağrılır, kütüphane kullanılmaz. Böylece kullanıcının mevcut kimlik doğrulaması (SSH anahtarı, credential helper) olduğu gibi çalışır.
- **Dağıtım:** GitHub reposu + GitHub Actions. Her sürümde otomatik derlenen paketler: Linux (.deb, .AppImage), Windows (.msi), macOS (.dmg).

## Kural Motoru

Her izlenen proje için üç kural çalışır. Tüm eşik ve saatler proje başına ayarlanabilir.

### Kural 1 — Saatlik değişiklik kontrolü

- Her saat (varsayılan 60 dk) `git status` çalıştırılır.
- Değişen dosya sayısı (untracked dahil) eşiği (varsayılan 10) aşarsa geri sayım penceresi gösterilir:
  > "X projesinde çok fazla dosya değişikliği var. Commit ve push işlemi yapılacak."
- Pencere 5 dakikadan geriye sayar. Butonlar:
  - **İptal:** İşlem yapılmaz; 1 saat sonraki kontrolde eşik hâlâ aşılıyorsa tekrar sorulur.
  - **Bugün bir daha sorma:** O proje için bu kural gün sonuna kadar susturulur (gece yarısı sıfırlanır).
  - **Şimdi yap:** Geri sayım beklenmeden hemen çalışır.
- Süre dolarsa (iptal edilmezse): `git add -A` → `git commit -m "Otomatik yedek: <tarih saat> (<n> dosya)"` → `git push`.

### Kural 2 — Saatlik uzak (remote) kontrolü

- Her saat `git fetch` çalıştırılır.
- Uzak dal yerelden ilerideyse **soru** bildirimi gösterilir (geri sayım yok, otomatik işlem yok):
  > "X projesinin GitHub deposunda N yeni commit var. Çekmek ister misiniz?"
- Butonlar: **Çek** / **Boşver**.
- "Çek" seçilirse: yerelde commit edilmemiş değişiklik varsa önce otomatik commit yapılır, sonra `git pull --rebase` çalıştırılır.
- Rebase çakışması çıkarsa `git rebase --abort` ile geri alınır ve "elle çözülmeli" bildirimi gösterilir. Veri kaybı olmaz.

### Kural 3 — Günlük yedek (varsayılan 23:00)

- Her gün belirlenen saatte kontrol: commit edilmemiş değişiklik **veya** push edilmemiş commit varsa geri sayım penceresi gösterilir:
  > "X projesinde bekleyen commit/push var. Günlük yedek alınacak."
- Kural 1 ile aynı pencere davranışı (5 dk geri sayım, İptal / Şimdi yap; "bugün bir daha sorma" bu kuralda gereksiz çünkü günde bir kez çalışır).
- Süre dolarsa otomatik commit + push. Sadece push edilmemiş commit varsa yalnız push yapılır.

### Kural çakışması

- Aynı projede aynı anda tek bildirim gösterilir.
- Günlük yedek saati ile saatlik kontrol çakışırsa yalnız günlük yedek kuralı çalışır.

## Bileşenler

### Zamanlayıcı (Rust)

Uygulama içi döngü. Her projeye saatlik kontrol ve günlük yedek tetiği üretir. Bilgisayar uykudan uyanınca bir sonraki periyodu bekler; kaçan tetikler telafi edilmez (basitlik tercihi — örn. 23:00'te bilgisayar kapalıysa yedek bir sonraki 23:00'te alınır).

### Git modülü (Rust)

`git` komutunu alt süreç olarak çalıştırır: `status`, `fetch`, `add -A`, `commit`, `push`, `pull --rebase`, `rebase --abort`. Çıktıları ayrıştırıp kural motoruna durum bilgisi verir (değişen dosya sayısı, ahead/behind commit sayıları).

### Bildirim penceresi (HTML/JS)

Ekranın sağ altında çıkan, her zaman üstte, küçük pencere. İki tür:

1. **Geri sayımlı:** mesaj + kalan süre + İptal + Bugün bir daha sorma + Şimdi yap.
2. **Soru:** mesaj + Çek + Boşver.

### Yönetim penceresi (HTML/JS)

Tepsi ikonuna tıklayınca açılır:

- Proje listesi ve anlık durumları (temiz / bekleyen değişiklik / GitHub ileride / bulunamadı).
- Proje ekleme (klasör seçici) ve çıkarma. Git deposu olmayan klasör kabul edilmez.
- Proje başına ayarlar: dosya eşiği, kontrol sıklığı, günlük yedek saati, her kuralı ayrı aç/kapa.

### Ayar dosyası

JSON formatında, işletim sisteminin standart config klasöründe:

- Linux: `~/.config/gitgardiyan/config.json`
- macOS: `~/Library/Application Support/gitgardiyan/config.json`
- Windows: `%APPDATA%\gitgardiyan\config.json`

İçerik: proje listesi, proje başına ayarlar, "bugün susturuldu" kayıtları.

### Otomatik başlatma

Uygulama işletim sistemi açılışında otomatik başlar (Tauri autostart eklentisi). Yönetim penceresinden kapatılabilir.

## Hata Durumları

| Durum | Davranış |
|---|---|
| Push başarısız (internet yok, kimlik hatası) | Bildirim gösterilir; commit yerelde kalır, sonraki saatte tekrar denenir. |
| Proje klasörü silinmiş/taşınmış | Listede "bulunamadı" işaretlenir, kontroller atlanır. |
| Rebase çakışması | `--abort` + "elle çözülmeli" bildirimi. |
| Git olmayan klasör eklenmek istenirse | Ekleme reddedilir, uyarı gösterilir. |
| `git` komutu sistemde yok | Uygulama açılışta uyarır. |

## Loglama

`gitgardiyan.log` (config klasöründe): her otomatik commit, push, pull ve hata kaydı. Basit metin formatı.

## Test Stratejisi

- **Git modülü:** Geçici (temp) git depolarıyla Rust birim/entegrasyon testleri — status ayrıştırma, ahead/behind tespiti, commit/push/pull akışları.
- **Kural motoru:** Birim testleri — eşik kontrolü, susturma mantığı, gece yarısı sıfırlama, kural çakışması önceliği.
- **Arayüz:** Elle test (üç platformda duman testi).

## Kapsam Dışı (bilinçli olarak yapılmayacaklar)

- Claude/AI ile commit mesajı üretme (ileride eklenebilir).
- Kaçan tetiklerin telafisi (uyku/kapalı durum sonrası).
- Branch yönetimi — uygulama yalnızca aktif dalda çalışır.
- Merge çakışması çözme arayüzü — çakışma her zaman kullanıcıya bırakılır.

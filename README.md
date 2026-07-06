# GitGardiyan

Git projelerinde commit, push ve pull işlemlerini unutmayı engelleyen sistem tepsisi uygulaması. Linux, macOS ve Windows.

## Ne yapar?

- **Saatlik değişiklik kontrolü:** Eşikten (varsayılan 10) fazla dosya değiştiyse 5 dakika geri sayan bildirim gösterir; iptal edilmezse otomatik commit + push yapar.
- **Saatlik uzak kontrol:** GitHub yereldekinden ilerideyse "çekmek ister misiniz?" diye sorar (`pull --rebase`; çakışmada geri alır, elle çözüme bırakır).
- **Günlük yedek (23:00):** Bekleyen commit/push varsa geri sayımlı bildirimle günlük yedek alır.

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

Config: işletim sisteminin standart config klasöründe `com.gitgardiyan.app/config.json`. Log: aynı klasörde `gitgardiyan.log`.

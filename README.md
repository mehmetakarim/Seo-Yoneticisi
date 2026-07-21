# SEO Yöneticisi (Faz 1)

kurumsalit.com'un IdeaSoft XML ürün feed'ini okuyup her ürünün meta SEO alanlarını
(Sayfa Başlığı / Meta Açıklama / Site İçi Arama) validasyondan geçiren **Tauri v2 + Vue 3**
masaüstü uygulaması. Kullanıcı üretilen/düzenlenen metni kopyalayıp IdeaSoft paneline
yapıştırır, sonra "Tamamlandı" işaretler.

Bu Faz 1: feed okuma + SQLite senkron + meta validasyonu + liste/detay UI + Ayarlar.
Gemini üretimi (meta = Faz 2, details = Faz 3) backend'de iskelet olarak hazır; UI butonları
görünür ama placeholder.

## Stack

- **Tauri v2** kabuk, **Rust** backend (ağ + DB + validasyon burada)
- **Vue 3** (Composition API) + **Pinia** frontend
- **SQLite** (rusqlite, bundled) yerel iş durumu
- **quick-xml** feed parse, **reqwest** feed fetch, **unicode-segmentation** grapheme sayımı

## Çalıştırma

```bash
npm install
npm run tauri dev      # geliştirme (pencere açar)
npm run tauri build    # üretim derlemesi
```

DB konumu: `%APPDATA%/com.kurumsalit.seo-yoneticisi/seo-yoneticisi.db`
Varsayılan feed: `https://www.kurumsalit.com/output/2567783262` (Ayarlar'dan değiştirilebilir).

## Testler

```bash
cd src-tauri
cargo test                         # 14 birim testi (parse, senkron, validasyon)

# Gerçek feed'e karşı uçtan uca (opsiyonel):
# XML'i bir dosyaya kaydedip:
SEO_FEED_FILE=/yol/feed.xml cargo test real_feed -- --ignored --nocapture
```

## Mimari

- `src-tauri/src/`
  - `db.rs` — SQLite şema/migration + settings key-value
  - `feed.rs` — quick-xml parse (CDATA + trim) + reqwest fetch
  - `sync.rs` — sku bazlı upsert (seo_status'a dokunmadan) + düşen temizliği + sync_log
  - `validation.rs` — meta rozet kuralları, grapheme sayımı, html_strip/word_count/density (Faz 2/3 hazır)
  - `gemini.rs` — Faz 2/3 iskeleti (imzalar hazır, "aktif değil" döner)
  - `commands.rs` — Tauri komutları (sync, list, get, draft, mark done, settings, export/import)
- `src/`
  - `store.ts` — Pinia (liste, filtre/arama istemcide, seçim, senkron, tema, toast)
  - `validation.ts` — canlı meta kuralları (`[...str].length` grapheme)
  - `components/` — Sidebar, ProductList, ProductDetail, MetaSeoCard, DetailsSeoCard, SyncSummaryBar, SettingsPage, Icon

## Kullanıcı düzenlemeleri

Detay panelindeki elle düzenlemeler `seo_status` tablosuna **taslak** (`draft_*`) olarak
kaydedilir (600 ms debounce). Senkron feed'i güncellese bile taslaklar ve done/hedef kelime
korunur. Faz 2'de Gemini çıktısı da bu taslak alanlarına yazılacak.

## Faz sınırı

Faz 1'de **yapılmaz**: gerçek Gemini çağrıları, details HTML optimizasyonu, img-src koruma.
Butonlar placeholder; kısayol `G` ve details üretimi "Faz 2/3'te aktif" bilgisi verir.

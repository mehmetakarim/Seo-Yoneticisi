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

## Faz 2 (tamamlandı) — Gemini meta üretimi

"Gemini ile Üret" / `G` kısayolu gerçek Gemini API'ye bağlı:
- `gemini.rs` — v1beta structured output; **model fallback zinciri** (2.0-flash → 2.5-flash → 1.5-flash),
  429/kota'da sıradaki modele geçer; tek retry + kelime sınırında kırpma ile uzunluk kuralı garanti.
- Sonuç `seo_status.draft_*` + `target_keyword`'e yazılır, validasyon anında güncellenir.
- Kart 2'nin "Açıklamayı Tamamlandı" toggle'ı aktif (details ÜRETİMİ Faz 3).

Gerçek API testi:
```bash
cd src-tauri
GEMINI_API_KEY=... cargo test gen_meta_real -- --ignored --nocapture
```

## Faz 3 (tamamlandı) — details üretimi

"Açıklamayı Üret" / `⇧G` kısayolu:
- `gemini.rs::generate_details` — **yapı korunur**: orijinal HTML iskeletinden h2/p iç metinleri
  çıkarılıp Gemini'ye yeniden yazdırılır, aynı konuma splice edilir. section/class/`<img>` dokunulmaz
  (görsel güvenliği by-design; ayrıca img src invariant kontrolü + `<strong>/<em>` dışı etiket temizliği).
- İki boyutlu liste durumu aktif: **Açıklama Bekliyor** filtresi (meta done + details pending),
  Kart 2 durum rozeti (kelime ≥ 50, yoğunluk 1.5–3.5).
- Sonuç `seo_status.draft_details`'e yazılır (feed'in `details` alanı korunur).

Gerçek API testi:
```bash
GEMINI_API_KEY=... cargo test gen_details_real -- --ignored --nocapture
```

Üç fazın tamamı bitti. Yol haritası ve olası sonraki işler: `brain.md`.

# 🧠 brain.md — SEO Yöneticisi Proje Beyni

> Bu dosya projenin kalıcı hafızasıdır. Oturum (session) değişse bile buraya bakarak
> nerede kaldığımızı anlar ve devam ederiz. **Her anlamlı ilerlemede güncelle.**

**Son güncelleme:** 2026-07-21
**Aktif faz:** Faz 2 ✅ tamamlandı (Gemini meta üretimi) → Faz 3 (details) bekliyor
**Repo:** https://github.com/mehmetakarim/Seo-Yoneticisi (main, ✅ push edildi)

## Faz 2 kararları (kullanıcı onayı)
- **Model: kademeli fallback zinciri** — biri günlük limite/429'a takılınca sıradaki modele geç.
  Öneri sıra: `gemini-2.0-flash` → `gemini-2.5-flash` → `gemini-1.5-flash`.
- **API anahtarı:** test için kullanıcı bir anahtar verdi → SADECE yerel testte kullan,
  ASLA koda/git'e gömme; app zaten anahtarı SQLite settings'te (APPDATA) saklıyor.
- **Kart 2:** "Açıklamayı Tamamlandı" toggle'ı Faz 2'de aktifleşecek (details ÜRETİMİ yine Faz 3).

---

## 🎯 Proje özeti

kurumsalit.com'un IdeaSoft XML ürün feed'ini okuyup her ürünün SEO metinlerini
(meta + uzun açıklama) Gemini API ile üretip validasyondan geçiren **Tauri v2 + Vue 3**
masaüstü uygulaması. Kullanıcı üretilen metni kopyalayıp IdeaSoft paneline yapıştırır,
sonra "Tamamlandı" işaretler.

- **Spec:** `SEO-Asistani.md` (kök klasör)
- **Tasarım referansı:** `design/` içindeki zip (Claude Design handoff) — birebir UI kaynağı
- **Feed:** `https://www.kurumsalit.com/output/2567783262` (265 ürün, read-only, stok>0 & aktif)
- **Feed anahtarı:** `sku` (id kararsız, ANAHTAR DEĞİL)

## 🧱 Stack / kararlar

- Tauri v2 kabuk · Vue 3 (Composition API) + Pinia · Rust backend (ağ + DB + validasyon Rust'ta)
- SQLite (rusqlite, bundled) · quick-xml (parse) · reqwest (fetch) · unicode-segmentation (grapheme)
- **Ağ ve API işlemleri Rust tarafında** (CORS'tan kaçınmak + API anahtarını frontend'den uzak tutmak)
- DB konumu: `%APPDATA%/com.kurumsalit.seo-yoneticisi/seo-yoneticisi.db`
- **Kullanıcı kararı:** elle düzenlemeler `seo_status.draft_*` alanlarına 600ms debounce ile kaydedilir;
  senkron taslakları + done + hedef kelimeyi korur. Faz 2'de Gemini çıktısı da bu draft alanlarına yazılacak.

---

## ✅ Faz 1 — TAMAMLANDI (feed + senkron + meta validasyon + UI)

### Yapıldı
- **Backend** (`src-tauri/src/`):
  - `db.rs` — şema (products, seo_status[+draft_* kolonları], sync_log, settings) + migration + settings kv
  - `feed.rs` — quick-xml parse (CDATA + trim; `quantityStatus "[ var ]" → "var"`) + reqwest fetch
  - `sync.rs` — sku bazlı upsert (seo_status'a dokunmadan) + düşen ürün cascade DELETE + sync_log
  - `validation.rs` — grapheme bazlı meta rozet kuralları + html_strip/word_count/keyword_density (Faz 2/3 hazır)
  - `gemini.rs` — Faz 2/3 iskeleti (imzalar var, "aktif değil" döner)
  - `commands.rs` — 14 Tauri komutu
- **Frontend** (`src/`):
  - `store.ts` (Pinia) — liste, filtre/arama **istemcide**, seçim, senkron, tema, toast
  - `validation.ts` — canlı meta kuralları (`[...str].length` grapheme)
  - `components/` — Sidebar, ProductList, ProductDetail, MetaSeoCard, DetailsSeoCard, SyncSummaryBar, SettingsPage, Icon
  - Tasarıma birebir renk token'ları (`styles.css`), açık/koyu tema, kısayollar (↑↓ / G / D / ⌘F)

### Doğrulama
- **14/14 cargo birim testi geçti** (parse, senkron 4 senaryo, Türkçe grapheme, validasyon)
- **Gerçek feed testi** (`real_feed`, #[ignore]): 265 ürün parse+senkron; 2. senkronda eklenen=0/güncellenen=265/silinen=0
- `npm run build` (vue-tsc) temiz · `cargo build` temiz · `npm run tauri dev` runtime hatasız açıldı

### Faz 1'de bilerek YAPILMADI
- Gerçek Gemini çağrıları (butonlar placeholder; G kısayolu ve details üretimi "Faz 2/3'te aktif" toast'ı verir)
- Details HTML optimizasyonu + img-src koruma
- Kart 2 "Açıklamayı Tamamlandı" butonu devre dışı (details_status Faz 2/3'e ait)
  - ⚠️ **Açık soru:** kullanıcıya soruldu; istenirse Faz 1'de de toggle edilebilir (küçük iş).

---

## ✅ Faz 2 — Meta üretimi (Gemini) — TAMAMLANDI

### Yapıldı
- **`gemini.rs`** gerçek implementasyon:
  - Google Generative Language API v1beta, `system_instruction` + `responseSchema` (structured JSON)
  - **Model fallback zinciri** `MODEL_CHAIN = [gemini-2.0-flash, gemini-2.5-flash, gemini-1.5-flash]`;
    429/503 (kota) → sıradaki modele geç. Anahtar/ağ/biçim hatası → hemen dön (fallback anlamsız).
  - **Tek retry** (spec) + iki denemenin daha az ihlal edenini seç (`violation_count`) +
    son güvenlik `clamp_lengths` (fazla uzun title/descriptions'ı kelime sınırında grapheme-bazlı kırpar).
  - `test_key` → models endpoint ile gerçek anahtar doğrulaması (üretim tüketmez).
- **`commands.rs`**: `generate_meta(sku)` (kilit await'e taşınmaz), `mark_details_done(sku)`,
  `test_gemini_key` async'e çevrildi. `read_detail` helper'ı get_product + generate_meta ortak kullanır.
- **Frontend**: api.ts (generateMeta/markDetailsDone), store.ts (`generating` state + generateMeta/toggleDetailsDone),
  MetaSeoCard "Gemini ile Üret" spinner + disabled, App.vue `G` kısayolu → generateMeta,
  DetailsSeoCard "Açıklamayı Tamamlandı" toggle AKTİF. ProductDetail watch: `detail` nesnesini izler
  (üretim sonrası alanlar tazelenir, yazarken ezilmez).

### Doğrulama
- 16/16 cargo testi geçti (clamp + violation birim testleri dahil)
- **Gerçek API testi** (`gen_meta_real`, #[ignore], GEMINI_API_KEY env): Lenovo AIO ürünü için
  title 49 / descriptions 143, hedef kelime içeriyor → **rozet: Uygun** ✅
- Frontend build temiz, tauri dev runtime hatasız açıldı

### Dikkat / notlar
- API anahtarı yalnızca yerel testte kullanıldı, **koda/git'e gömülmedi**. App anahtarı SQLite settings'te tutar.
- Test komutu: `GEMINI_API_KEY=... cargo test gen_meta_real -- --ignored --nocapture`
- ⚠️ Model isimleri zamanla değişebilir — 429 dışı "model bulunamadı" hatası gelirse MODEL_CHAIN güncelle.

## 🔮 Faz 3 — Details (uzun açıklama) üretimi — SONRAKİ

- Ayrı Gemini çağrısı; `details` HTML gönderilir, **yapı korunur** (section/col-md/img/h2/p sırası + class'lar aynı),
  sadece h2/p metinleri yenilenir
- Üretimden önce tüm `<img src>` URL'leri Rust'ta listeye çıkarılır; dönen HTML'de src bozulmuşsa
  orijinalleriyle geri yazılır (kredi harcamadan görsel güvenliği)
- Kart 2'yi aktif et: "Açıklamayı Üret" + "Açıklamayı Tamamlandı" + details_status validasyonu
  (word_count ≥ 50, density %2–3 — yardımcılar `validation.rs`'te hazır)

---

## 📌 Kabul kriterleri (Faz 1 — hepsi karşılandı)
1. Manuel Güncelle gerçek feed'i çeker, ~265 ürünü yazar, sayaçlar doğru ✅
2. İkinci senkron: eklenen=0, güncellenen=hepsi; done/target_keyword korunur ✅
3. Düşen ürün sonraki senkronda silinir, "silinen" raporlanır ✅ (test)
4. Liste rozetleri doğru, filtreler çalışır, Tamamlandı gizli ✅
5. İki kart görünür, canlı göstergeler, Kopyala panoya alır, Meta'yı Tamamlandı ürünü düşürür ✅
6. Ayarlarda feed URL + Gemini anahtarı saklanır, DB export/import ✅
7. Türkçe karakter sayımı grapheme bazlı ✅

## 🗺️ Süreç / operasyon
- **Testler:** `cd src-tauri && cargo test` (14 test) · gerçek feed: `SEO_FEED_FILE=... cargo test real_feed -- --ignored`
- **Çalıştır:** `npm run tauri dev`
- **Repo push (bekliyor):** `git init` yapılıp https://github.com/mehmetakarim/seo-yoneticisi 'a push edilecek
  - `.gitignore` scaffold ile geldi (node_modules, target/, dist/ hariç tutulmalı — kontrol et)
- **Bu dosyayı güncelle:** her faz/önemli karar sonrası "Son güncelleme" + ilgili bölüm

## 🧩 Açık sorular / kullanıcı kararları bekleyen
- [x] Kart 2 "Açıklamayı Tamamlandı" aktif olsun mu? → EVET, Faz 2'de aktifleştirildi.
- [x] Gemini model tercihi? → Kademeli fallback zinciri (2.0-flash → 2.5-flash → 1.5-flash).
- [ ] Faz 3: details üretiminde hangi model? (muhtemelen aynı zincir, ama uzun içerik → maliyet dikkat)
- [ ] "Açıklama Bekliyor" filtresi: meta done + details pending mantığı Faz 3'te devreye alınacak (şu an count 0)

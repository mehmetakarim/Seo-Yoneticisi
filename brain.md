# 🧠 brain.md — SEO Yöneticisi Proje Beyni

> Bu dosya projenin kalıcı hafızasıdır. Oturum (session) değişse bile buraya bakarak
> nerede kaldığımızı anlar ve devam ederiz. **Her anlamlı ilerlemede güncelle.**

**Son güncelleme:** 2026-07-22
**Aktif faz:** Faz 6 ✅ tamamlandı (Trends + Ahrefs domain) → **SEO araştırma entegrasyonu (Faz 4-6) BİTTİ** 🎉
**Repo:** https://github.com/mehmetakarim/Seo-Yoneticisi (main, ✅ push edildi)

## 🌍 Vizyon (kullanıcı kararı — 2026-07-22)
Uygulama **şahsileştirilmiyor**, global kullanım için geliştiriliyor: aynı IdeaSoft XML yapısını
kuran farklı müşteriler de kendi feed URL'si + Gemini/CapSolver/GSC anahtarlarını Ayarlar'dan girip
kullanabilir. Amaç exe/dmg release. → **Yeni özellikler kullanıcıya özel değer gömmeden, ayarlanabilir
olmalı.** Anahtarlar her zaman SQLite `settings`'te; koda/git'e ASLA gömülmez.

## 🔬 SEO araştırma entegrasyonu (Faz 4-6 planı)
3 harici MCP (seo-research-mcp, gsc-mcp, google-news-trends) **paketlenmiyor**; altlarındaki HTTP
çağrıları **native Rust'a yeniden yazılıyor** (Python/Chromium yok → tek binary). Amaç: Gemini'nin
tahminle üretmesi yerine gerçek verilerle beslenmesi. Akış: ayrı **"SEO Araştır"** paneli → kullanıcı
hedef kelimeyi onaylar → üretim onaylı veriyle çalışır. Plan dosyası:
`~/.claude/plans/harika-npm-install-yaparak-atomic-river.md`.

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

## ✅ Faz 3 — Details (uzun açıklama) üretimi — TAMAMLANDI

### Yapıldı — yaklaşım: **structure-preserving splice** (send-full-HTML yerine daha sağlam)
- **`gemini.rs::generate_details`**: orijinal HTML iskeletinden h2/p iç metinleri sırayla çıkarılır
  (`extract_segments`, byte-indeksli, ASCII-lowercase kopyayla hizalı), Gemini bunları JSON dizi olarak
  yeniden yazar, aynı konumlara `splice` edilir. section/col-md/class/`<img>` HİÇ dokunulmaz →
  görsel güvenliği by-design. Model fallback zinciri + uzunluk uyuşmazsa tek retry + best-effort
  (eksik parça → orijinali korunur). `sanitize_inline`: yalnızca <strong>/<b>/<em>/<br> izinli,
  <img>/<script>/başlık etiketi enjeksiyonu atılır. Ek güvenlik: img src listesi üretim öncesi/sonrası
  karşılaştırılır, farklıysa orijinal HTML döner.
- **db.rs**: `seo_status.draft_details` kolonu idempotent migration (`add_column_if_missing`).
- **validation.rs**: `details_badge` (done→Tamamlandı, boş→Eksik, kelime≥50 & yoğunluk 1.5–3.5→Uygun,
  aksi Hatalı) + `overall_status` (iki boyutlu: meta+details → Eksik/Hatalı/Bekliyor/Uygun/Tamamlandı,
  prototipteki `overall()` mantığı).
- **commands.rs**: `generate_details(sku)` (draft_details'e yazar), `read_detail` + `list_products`
  artık meta_badge + details_badge + overall döner. Filtreler overall'a göre → **"Açıklama Bekliyor" aktif**.
- **Frontend**: DetailsSeoCard üret butonu aktif + spinner + "Uzun içerik üretiliyor…" overlay,
  önizleme/metrikler draft_details ?? feed'den, durum rozeti details_badge. ProductList dual gösterge
  artık meta_badge + details_badge. store counts/rows overall'a göre. `⇧G` = açıklama üret kısayolu.
  ProductDetail draft_details ?? details geçirir.

### Doğrulama
- 20/20 cargo birim testi (extract_segments/splice/sanitize/img birim testleri dahil)
- **Gerçek API testi** (`gen_details_real`): iskelet + 2 img korundu, h2/p yeniden yazıldı,
  hedef kelime <strong> ile vurgulandı, 131 kelime ✅
- Frontend build temiz, tauri dev runtime hatasız (migration mevcut DB'de sorunsuz)

## ✅ Faz 4 — Kontrollü SEO araştırma + keyword grounding (seo-mcp native) — TAMAMLANDI

### Yapıldı
- **`seo_data/` modülü** (yeni):
  - `mod.rs` — `SeoInsights { seed, target_candidates, seed_difficulty, gsc_queries, trends, notes }`
    + `prompt_block()` (üretim prompt'una enjekte edilen "GERÇEK ARAMA VERİLERİ" bloğu) + `has_data()`
    + `unwrap_ok()` (Ahrefs `["Ok", {...}]` sarmalı).
  - `ahrefs.rs` — **CapSolver Turnstile** (`AntiTurnstileTaskProxyLess`, siteKey `0x4AAAAAAAAzi9ITzSN9xKMi`,
    createTask→getTaskResult poll 1s/60s) + Ahrefs free-tools `stGetFreeKeywordIdeas` (keyword_ideas) &
    `stGetFreeSerpOverviewForKeywordDifficultyChecker` (keyword_difficulty). difficultyLabel/volumeLabel
    → sayı map'leri seo-research-mcp'den portlandı. `test_key` = CapSolver getBalance.
- **db.rs**: `seo_status.research_json` kolonu (idempotent migration).
- **commands.rs**: `research_seo(sku, seed?)` — tohum seçimi (seed→onaylı kelime→kategori→ad ilk 4 söz),
  ideas+difficulty **eşzamanlı** (`tokio::join!`), hacme göre sıralı, `research_json`'a yazar, panele döner.
  `test_capsolver_key`. `generate_meta`/`generate_details` artık `research_json`+`target_keyword` okuyup
  `ProductContext`'e geçirir. Settings'e `capsolver_api_key` + `seo_country` (varsayılan `tr`).
- **gemini.rs**: `ProductContext`'e `target_keyword` + `insights` alanları. Meta prompt'unda onaylı kelime
  varsa "türetme, bunu kullan" + `insights.prompt_block()`. Details prompt'una da insights bloğu. Geriye
  dönük uyumlu (None → eski davranış).
- **Cargo.toml**: `tokio = { features = ["time","macros"] }` (CapSolver polling + join).
- **Frontend**: `SeoResearchPanel.vue` (sağdan kayan animasyonlu drawer, `cubic-bezier(.32,.72,0,1)`,
  reduced-motion, skeleton/popIn, mevcut `--badge-*`/`--c-*` token'ları — yeni renk YOK). ProductDetail'e
  "SEO Araştır" butonu + panel + `onPickKeyword`. store: `researching`/`research` + `runResearch`.
  api/types: `researchSeo`, `testCapsolverKey`, genişletilmiş `saveSettings`, `SeoInsights` tipleri.
  SettingsPage: CapSolver anahtarı (göster/gizle) + araştırma ülkesi + "Anahtarı test et".

### Doğrulama
- **29 cargo birim testi geçti** (9'u Faz 4 için yeni: difficulty/volume label map, parse_ideas×2,
  parse_difficulty, tool_url, prompt_block×2, unwrap_ok; +5 ignored gerçek-API testi).
  `cargo build` + `npm run build` (vue-tsc+vite) temiz.
- **Gerçek Gemini testi** (`gen_meta_with_insights_real`, #[ignore]): onaylı hedef kelime + insights
  enjekte edildiğinde model kelimeyi **aynen kullandı** (türetmedi), title/desc doğal işledi ✅.
- **CapSolver/Ahrefs canlı yolu DOĞRULANDI** (2026-07-22, kullanıcının CapSolver anahtarıyla):
  `keyword_ideas` "all in one bilgisayar" (tr) için 23 gerçek aday döndü (hacim+zorluk dolu),
  `keyword_difficulty` yanıt verdi. CapSolver Turnstile çözümü çalışıyor.
  ⚠️ **Ahrefs API şekli değişmişti — 2 düzeltme gerekti** (seo-mcp fork'u eski):
  1) `stGetFreeKeywordIdeas` `keyword` alanı artık **düz string** (`["Some", kw]` sarmalı → InvalidInput).
  2) `volumeLabel` artık **isimli kova enum'u** (`"MoreThanOneThousand"` = 1000, `MoreThanTenThousand`=10000,
     `MoreThanOneHundred`=100, `MoreThanTen`=10, `Zero`=0) — eski aralık/sayı biçimi değil.
  Ayrıca Ahrefs POST'una tarayıcı benzeri başlıklar (User-Agent/Origin/Referer) eklendi.

### Notlar / dikkat
- Ahrefs free-tools + CapSolver gayriresmi → biçim değişirse parse kırılabilir; graceful degrade (notes).
  Her araştırma = 2 CapSolver çözümü (kredi + ~saniyeler). Orijinaldeki cwd `signature_cache.json` sorunu
  yok (Rust'ta bellekte, henüz backlink Faz 6'da).
- Faz 4 kapsamı: yalnızca Ahrefs (keyword ideas + difficulty). GSC gerçek sorgular Faz 5, Trends+backlink Faz 6.

## ✅ Faz 5 — GSC gerçek arama sorguları (service-account) — TAMAMLANDI

### Yapıldı
- **`seo_data/gsc.rs`** (yeni): service-account JWT (RS256, `jsonwebtoken`) → Google token endpoint →
  access token → `searchAnalytics.query` (`page` filtresi = `products.url`, son 90 gün, 25 satır).
  `validate_json` (client_email + private_key + PEM yüklenebilirlik), `client_email_of` (UI'da göster,
  private key sızmaz), `test` (token + `sites.list` → mülk erişimini doğrula). `pct()` siteUrl path encode.
  **Mimari not:** plan `yup-oauth2` diyordu; daha hafif olduğu için `jsonwebtoken` + reqwest seçildi
  (tüm HTTP tek yığında; scope `webmasters.readonly`).
- **commands.rs**: `set_gsc_service_account(path)` (dosyayı Rust okur+doğrular+saklar, email döner),
  `clear_gsc_service_account`, `test_gsc_credentials`. Settings'e `gsc_site_url` + türetilen
  `gsc_client_email` (raw JSON asla frontend'e gitmez). `research_seo` artık GSC'yi de çeker
  (CapSolver ve/veya GSC — biri yeterli; ikisi de yoksa hata). Ürün `url`'i GSC page filtresi.
- **Frontend**: SettingsPage'e **Google Search Console kartı** — mülk adresi + "JSON yükle" (dialog),
  yüklüyse client_email + "Bağlantıyı test et"/"Kaldır". **Animasyonlu "nasıl alırım?" rehber modalı**
  (5 adım, her adımda `opener` ile Google linki; fade+scale `cubic-bezier`, reduced-motion). Panelde
  **"Google'daki gerçek sorgular"** bölümü (en üstte, accent kenarlı; gösterim/sıra + "Hedef yap").
  api/types: `setGscServiceAccount`/`clearGscServiceAccount`/`testGscCredentials`, genişletilmiş saveSettings.
- **Cargo.toml**: `jsonwebtoken = "9"` (ring/rsa çeker; ilk derleme ~9dk).

### Doğrulama
- **34 cargo birim testi geçti** (5 yeni GSC: pct encode, client_email, validate_json, parse_rows×2).
  `cargo build` + `npm run build` temiz.
- **GSC canlı yolu DOĞRULANDI** (2026-07-22, kullanıcının taze SA'sı `kurumsalitgscsa@kitindexapi`):
  JWT+token+API çalıştı; erişilen mülk **`https://www.kurumsalit.com/`** (URL-prefix, sc-domain değil).
  Lenovo Neo 50a ürün sayfası için gerçek sorgu döndü: `"12sca078tr"` (28 gösterim, 1 tıklama, sıra 8.2).
  → Kurulumda GSC mülkü olarak `https://www.kurumsalit.com/` girilmeli. gsc-mcp'deki sızmış SA hâlâ iptal edilmeli.

## ✅ Faz 6 — Google Trends + Ahrefs domain özeti — TAMAMLANDI

### Yapıldı
- **`seo_data/trends.rs`** (yeni, keyless) — **ŞU AN DEVRE DIŞI** (kod korunuyor, `#[allow(dead_code)]`):
  - v1 denemesi: trending RSS (`/trending/rss`) → geo-geneli **günlük** trendler; ama bunlar hedef
    kelimeyle **alakasız** (kullanıcı geri bildirimi). ⚠️ Eski `dailytrends` JSON zaten 404.
  - v2 denemesi: `explore`→`widgetdata/relatedsearches` ile hedef kelimeye **ilgili** sorgular
    (doğru yaklaşım). Ama Google anti-bot **HTTP 429** veriyor (cookie warmup + `cookies` feature'a
    rağmen; tarayıcı consent çerezi gerekiyor). → **research_seo'da çağrılmıyor.**
  - **Karar:** keyword-relevant ihtiyaç zaten **Ahrefs fikirleri + GSC sorgularıyla** karşılanıyor;
    güvenilmez Trends "fayda değil boşluk" olurdu. İleride güvenilir yol bulunursa yeniden açılır.
    (`Cargo.toml`'a eklenen `reqwest` `cookies` feature'ı + client `cookie_store(true)` korunuyor.)
- **`ahrefs.rs::backlinks_overview`**: `stGetFreeBacklinksOverview` → `data:{domainRating,backlinks,refdomains}`
  → `DomainOverview`. (Traffic denendi ama `country:"None"` artık `InvalidInput`; backlink özeti tek
  çözümde yeterli otorite verisi verdiği için traffic atlandı.)
- **mod.rs**: `DomainOverview` tipi + `SeoInsights.domain`. `has_data()` domain'i de sayar.
- **commands.rs `research_seo`**: `host_of(url)` ile alan çıkarılır; CapSolver bloğunda keyword ideas +
  difficulty + **backlinks_overview eşzamanlı** (`tokio::join!` 3'lü). Trends her zaman denenir (keyless,
  geo = ülke.upper()). Notlar'a hata düşer.
- **Frontend**: panelde **"Güncel trendler"** chip'leri (tıkla→hedef yap) + **alan (domain) özet şeridi**
  (DR / backlink / ref-domain, bilgi amaçlı). types: `DomainOverview` + `SeoInsights.domain`.

### Doğrulama
- **39 cargo birim testi geçti** (5 yeni: parse_overview, parse_rss×3, parse_traffic). build'ler temiz.
- **Canlı DOĞRULANDI** (kullanıcının CapSolver anahtarı): Trends RSS TR için 10 gerçek trend döndü
  (galatasaray 20B, hava durumu bursa…); backlinks_overview kurumsalit.com → **DR 30, 2332 backlink,
  672 ref-domain**.

### Notlar
- Trends geo-geneli (ürüne özel değil) → mevsimsel bağlam; prompt'a `trends` olarak katılır. Domain özeti
  prompt'a KATILMAZ (dashboard bilgisi). Her araştırma artık ≤3 CapSolver çözümü (eşzamanlı → ~aynı süre).

## 🎯 Sonraki olası işler (opsiyonel — SEO araştırma entegrasyonu bitti)
- Toplu üretim (seçili ürünler için sırayla meta/details) + ilerleme çubuğu
- Gemini kota/kullanım göstergesi, model seçimi Ayarlar'da
- IdeaSoft'a doğrudan yazma (şu an kullanıcı elle kopyalıyor) — API varsa
- details üretiminde de yoğunluk hedef dışıysa tek retry (şu an sadece uzunluk uyuşmazlığında retry)

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

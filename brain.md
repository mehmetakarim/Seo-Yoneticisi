# 🧠 brain.md — SEO Yöneticisi Proje Beyni

> Bu dosya projenin kalıcı hafızasıdır. Oturum (session) değişse bile buraya bakarak
> nerede kaldığımızı anlar ve devam ederiz. **Her anlamlı ilerlemede güncelle.**

**Son güncelleme:** 2026-07-28
**Aktif faz:** Fırsat analizi ✅ + sürüm notları ✅ · Mimari toparlama Faz 1/2a/3 ✅ ·
**2b (modül bölme) ⏸️ ERTELENDİ** — kozmetik, kullanıcı kararı (bkz. madde 0b)
**Repo:** https://github.com/mehmetakarim/Seo-Yoneticisi (main) · **PUBLIC** (2026-07-26'dan beri)
**Yayınlanan sürümler:** v0.1.0 → v0.5.2 · v0.5.3 = Gemini 404 düzeltmesi ·
v0.5.4 = zincir + model rozeti · v0.5.5 = rozet kart başlığına ·
**v0.5.6 = Fırsatlar sayfası + gerçek sürüm notları**

**Yapı (2026-07-28'den beri workspace):**
`src-tauri/Cargo.toml` hem paket hem workspace kökü → `src-tauri/core/` (saf mantık, Tauri'ye
bağımlı DEĞİL, 81 test) + `src-tauri/src/` (ince Tauri katmanı: `commands.rs`, `lib.rs`).
İş döngüsü: `cargo test -p seo-core` ≈ 60 sn soğuk / 17 sn sıcak — Tauri hiç derlenmiyor.

## ⏭️ KALDIĞIMIZ YER (yeni oturum buradan devam etsin)

0. ⚠️ **GEMINI MODELLERİ EMEKLİYE AYRILIYOR — bu tekrar edecek (2026-07-28 saha hatası).**
   `gemini-1.5-flash` emekli olunca üretim TAMAMEN durdu. İki sebep vardı, ikisi de düzeltildi
   (`033996d`) ama dersi burada duruyor:
   - Zincir bayatlamıştı; ama asıl kusur, **404'ün geri düşmeyi tetiklememesiydi**
     (`is_quota = 429 || 503` → kota olmayan her şey zinciri anında kırıyordu). Bir modelin
     yokluğu tam da sıradakini denemek için sebeptir. Artık `classify_error()` bunu yönetiyor.
   - **Zincire asla `-preview` model konmaz** — habersiz kaybolurlar, bu hatanın sebebi buydu.
     Zincirin son halkası `gemini-flash-latest` (takma ad) → liste bayatlasa bile canlı kalır.
   - **Model eklemeden önce canlı doğrula.** Uygulamanın istek biçimi `system_instruction` +
     `responseSchema` ve her model ikisini desteklemiyor. Doğrulama komutu (anahtar DB'den
     okunur, ekrana basılmaz):
     ```
     DB="$HOME/Library/Application Support/com.kurumsalit.seo-yoneticisi/seo-yoneticisi.db"
     KEY=$(sqlite3 "$DB" "select value from settings where key='gemini_api_key';")
     curl -s "https://generativelanguage.googleapis.com/v1beta/models?key=${KEY}&pageSize=200"
     ```
     ⚠️ zsh tuzağı: URL'de `${M}:generateContent` yaz — `$M:generateContent` yazarsan zsh `:g`'yi
     geçmiş değiştiricisi sanar, URL bozulur ve **her model sahte 404 verir** (bir tur kaybettirdi).
   - Kota gerçeği: ücretsiz katmanda her modelin AYRI havuzu var; zincirdeki nesil çeşitliliği
     bu yüzden bilinçli. 2.0/2.5-flash kotası dolduğunda 3.x hâlâ çalışıyordu.

0a. ✅ **MODEL ZİNCİRİ LİMİTLERE GÖRE SIRALANDI + KULLANILAN MODEL GÖRÜNÜYOR (v0.5.4).**
   Konsoldan doğrulanan gerçek limitler (ücretsiz katman, 2026-07-28) sıralamayı değiştirdi —
   havuzlar **25× fark ediyor**:
   | model | dk | **gün** |
   |---|---|---|
   | 3.6 / 3.5 / 2.5 Flash | 5 | **20** |
   | 3.5 / 3.1 Flash Lite | 15 | **500** |
   | 2.5 Flash Lite | 10 | 20 |
   | Gemma 4 31B | 30 | **14.400** |

   Sıralama ilkesi: **kalite azalan, havuz büyüyen.** Kıt ama iyi olanlar önce harcanır,
   arkada gittikçe genişleyen emniyet ağı durur. `gemma-4-31b-it` bilinçli olarak EN SONDA:
   farklı model ailesi, üslup Gemini'lerden sapabilir; oraya ancak diğerlerinin tamamı
   tükendiğinde düşülür. (Halüsinasyon kalkanı kod düzeyinde olduğu için teknik tabloda
   yine de korunuyoruz.)

   **`Produced<T>`**: üretim fonksiyonları sonucu ÜRETEN MODELLE döndürüyor. Model üç sütunda
   KALICI saklanıyor (`meta_model`/`details_model`/`tech_model`) — geçici gösterim "şu an
   hangi modeldeyim"i cevaplar ama asıl değer şu: içerik son çare modeliyle üretildiyse
   kullanıcı bunu günler sonra görüp limitler yenilendiğinde yeniden üretebilir.
   `ModelTag.vue` eylem satırında satır içi rozet (altına koymak hizayı bozuyordu);
   Gemma'da amber uyarı rengine dönüyor.

0c. ✅ **FIRSAT ANALİZİ + SÜRÜM NOTLARI (v0.5.6).**

   **Fırsatlar sayfası** — "önce hangi ürüne bakmalıyım?". `gsc.rs::page_stats()` ile TEK API
   çağrısında tüm site (`page` bir filtre değil BOYUT; ürün başına çağrı 262 istek olurdu).
   `core/src/opportunity.rs` saf mantık: soyut puan yerine **kaçırılan tıklama** =
   `gösterim × (beklenen_ctr(konum) − gerçek_ctr)`, negatife düşmez.

   ⚠️ **Sınıflandırmada sıra kritik: ÖNCE KONUM.** İlk sürümde "tıklama yok" kontrolü öndeydi
   ve 2. sayfadaki bir ürün (konum 12.7, 0 tıklama) "Tıklama yok" etiketlendi — ama orada
   tıklama zaten beklenmez; etiket meta sorunu varmış gibi yanıltıp operatörü yanlış işe
   yönlendiriyordu. Artık tıklama/CTR yorumu YALNIZCA ilk sayfadakiler için yapılıyor.
   Regresyon testi: `zero_clicks_on_page_two_is_a_position_problem`.

   **Gerçek veriyle doğrulandı (2026-07-28, kurumsalit.com):** GSC'de 8087 sayfa ·
   262 üründen 260'ı eşleşti (%99.2) · 60 fırsat (42 düşük CTR, 12 tıklama yok, 6 ikinci sayfa)
   · 2 ürün Google'da hiç görünmüyor. Eşikler ne boş liste ne boğucu sonuç veriyor.
   URL eşleşmesi `norm_url()` ile sondaki `/` ve harf farkına dayanıklı.

   **Sürüm notları** — `CHANGELOG.md` tek doğruluk kaynağı; CI (`release.yml`) etikete karşılık
   gelen `## vX.Y.Z` bölümünü awk ile çıkarıp `releaseBody`'ye koyuyor. `tauri-action` bunu hem
   Release gövdesine hem `latest.json`'ın `notes` alanına yazıyor → güncelleme ekranında görünüyor.
   Bölüm bulunamazsa anlamlı metne düşüyor (release kırılmasın).
   ⚠️ `UpdateModal` notları satır satır render ediyor, **`v-html` YOK** — metin uzak sunucudan
   geliyor, HTML basmak sürüm notu yazabilen herkese kod çalıştırma imkânı verirdi.
   CHANGELOG maddeleri **tek satır** olmalı (sarma satırlar önceki maddeye ekleniyor ama
   okunabilirlik için tek satır tercih edilir).

0b. ⏸️ **FAZ 2B YARIM KALDI — `gemini.rs` (1923 satır) modül bölme.** Faz 1/2a/3 bitti (bkz. aşağı).
   **Önce şunu bil: bu iş TAMAMEN KOZMETİK.** Rust'ta derleme birimi dosya değil crate'tir;
   dosyayı bölmek derleme süresini DÜŞÜRMEZ — bu ölçülerek doğrulandı. Değeri yalnızca
   okunabilirlik. (Yine de değersiz değil: 2026-07-28 hatasında aynı hata sınıflandırması
   4 yerde kopyalanmıştı ve dördü de yanlıştı.)

   Hazır olan analiz — tekrar çıkarmaya gerek yok:
   - **mod.rs (paylaşılan):** `use`'lar, `MODEL_CHAIN`, `classify_error`, `API_BASE`,
     `ProductContext`, `short`, `esc`, `test_key`. Alt modüller bunlara `use super::*` ile erişir
     (Rust'ta çocuk modül, atasının private öğelerini görebilir).
   - **meta.rs:** `GeneratedMeta`, `system_prompt`, `build_prompt`, `response_schema`,
     `call_model`, `violation_count`, `clamp_lengths`, `clamp_to`, `correction_for`, `generate_meta`
   - **details.rs:** `ascii_lower_bytes` → `optimize_details` arası her şey (`esc` hariç, o paylaşımlı)
   - **tech.rs:** `TECH_GROUPS`, `LIST_GROUP`, `TechRow`, `TechGroup`, `TechSpecsResult`,
     `tech_system_prompt`, `call_specs_model`, `verify_traceable`, `structure_tech_specs`,
     `assemble_tech_html`
   - **Dış yüzey korunmalı:** `commands.rs` şu 12 öğeyi `gemini::X` olarak kullanıyor →
     mod.rs'ten `pub use` ile yeniden dışa açılmalı: `ProductContext`, `TechGroup`, `TechRow`,
     `TechSpecsResult`, `assemble_tech_html`, `generate_details`, `generate_details_scratch`,
     `generate_meta`, `has_rewritable_content`, `optimize_details`, `structure_tech_specs`, `test_key`.
   - **Testler (566 satır) konularıyla birlikte taşınmalı** — private fonksiyonları test ediyorlar,
     tek yerde kalamazlar.

   ⚠️ **Tuzak:** prompt fonksiyonları çok satırlı string literal döndürüyor; naif süslü-parantez
   sayan bir betik bunlarda kırılıyor (denendi, `scratch_system_prompt`'ta patladı). Ya durum
   takip eden bir tarayıcı yaz, ya da elle taşı. Her hâlükârda parçaların birleşimi orijinali
   birebir vermeli — yazmadan önce bunu doğrula.

   Doğrulama ucuz: `cargo test -p seo-core` ≈ 60 sn (Tauri derlenmiyor), 67 test geçmeli.

1. ✅ **Updater zinciri UÇTAN UCA ÇALIŞIYOR (2026-07-26).** İki ayrı sorun vardı, ikisi de kapandı:
   - **(a) `latest.json` üretilmiyordu** → `bundle.createUpdaterArtifacts: true` eksikti (v0.5.1'de eklendi).
   - **(b) Üretildi ama indirilemiyordu** → *depo private'dı.* GitHub, özel depoların release dosyalarını
     kimlik doğrulaması olmadan sunmaz; uygulama kimliksiz istek attığı için 404 → *"Could not fetch a
     valid release JSON from the remote"*. **Çözüm: depo public yapıldı.**
     ⚠️ **Ders:** GitHub Releases'i dağıtım kanalı olarak kullanan her şey (updater + son kullanıcı
     indirmesi) deponun public olmasını gerektirir. Private kalması istenirse ayrı bir public
     "releases" deposu ya da kendi sunucusunda barındırma gerekir.
   - Kimliksiz doğrulama (2026-07-26): `latest.json` → HTTP 200, 7 platform girdisi, hepsinde geçerli
     imza · `.app.tar.gz` → 200 · `.dmg` → 200 · `.exe` → 200.
   - Public'e almadan önce tam kimlik bilgisi taraması yapıldı: çalışma ağacında ve **tüm git
     geçmişinde** gerçek anahtar YOK (Gemini/CapSolver/GSC/minisign hepsi depo dışında). Tek bulgu bir
     input placeholder'ındaki token parçasıydı → temizlendi (`061cec9`). `.gitignore`'a
     `.env`/`*.key`/`*.pem`/`*-service-account*.json`/`*.db` koruması eklendi.
   - ✅ **SAHA TESTİ GEÇTİ (v0.5.1 → v0.5.2, 2026-07-26).** Kullanıcının kurulu v0.5.1'inde güncelleme
     modalı kendiliğinden çıktı, indirme + yeniden başlatma sorunsuz. **Faz 10 KAPANDI** —
     otomatik güncelleme artık varsayım değil, kanıtlanmış. (v0.5.0 ve öncesi elle kurulmalı.)
   - Her release'de doğrulanacak son halka: imzaların key ID'si `tauri.conf.json`'daki pubkey ile
     eşleşmeli (`6cbd59ca8b792915`). Eşleşmezse güncelleme *iner* ama doğrulamada reddedilir —
     bu sessiz hatayı yalnızca son kullanıcı görür, o yüzden yayından sonra kontrol et.

### 🎨 Tooltip deseni (v0.5.2)
Butonların ALTINA açıklama satırı koymak eylem satırının hizasını bozuyor (flex + `align-items:center`).
Bunun yerine global `[data-tip="metin"]` tooltip'i var (`styles.css`). Kurallar:
- `data-tip`'i **disabled olabilecek butonun kendisine değil, sarmalayıcısına** ver — disabled butonlar
  fare olayı üretmez, tooltip tam gerektiği anda açılmaz.
- Baloncuk yukarı + **sola hizalı** açılır (ortalanmış olsa dar kartta sol kenardan taşar).
  Yukarı açıldığı için `.card{overflow:hidden}` kırpmaz.
- Ok, 10px'lik üçgen kutusunun üst yarısı renkli olduğundan `bottom: calc(100% - 2px)` ile konumlanır;
  `100% + 3px` yapılırsa renkli yarı baloncuğun ARKASINDA kalır ve ok görünmez.
- Diğer kartlarda hâlâ native `title=` var; istenirse `data-tip`'e çevrilebilir.
- **`.card { overflow: hidden }` tooltip'i her yönde kırpar.** Kart BAŞLIĞINDAKİ öğeler için
  `[data-tip].tip-below` varyantı var: aşağı + sağa hizalı açar. Yine de baloncuk kartın
  İÇİNDE kalıyor → metin iki satırı geçerse alt kenardan taşıp kırpılıyor (v0.5.5'te
  Gemma tooltip'i tam bunu yaptı). Tooltip metinleri kısa tutulmalı.
- Görsel test yöntemi: gerçek CSS'i `styles.css`'ten çekip küçük bir harness sayfası üret,
  `[data-tip]::after{opacity:1!important}` ile tooltip'i zorla göster, en kötü durumu
  (en kısa kart gövdesi) dene. Bu yöntem üç ayrı kırpma hatasını yakaladı.
2. **Sıradaki kuyruk (kullanıcı onaylı):** mimari toparlama (navigasyon kabuğu + gruplandırılmış menü;
   `gemini.rs` ~2000, `commands.rs` ~1700 satır → modül bölme; store ayırma; derleme ~8-9 dk çok yavaş)
   → GSC fırsat analizi (2. sayfadaki ürünler, gösterim var tıklama yok) + meta/açıklama sürüm geçmişi
   → onboarding sihirbazı → (opsiyonel) toplu üretim, kod imzalama.

## 🔑 Updater imza anahtarı (KRİTİK)
- Konum: **`~/.tauri/seo-yoneticisi-updater.key`** (+ `.pub`). Depoda DEĞİL, olmamalı.
- GitHub secret'ları ayarlı: `TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` (boş).
- ⚠️ **Kaybolursa** dağıtılmış tüm kurulumlar bir daha otomatik güncellenemez (yeni anahtar = yeni pubkey =
  eski kurulumlar imzayı doğrulayamaz). Makine dışına da yedeklenmeli.

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

## ✅ Faz 7 — Görsel skoru + görsele bağlı üretim + sıfırdan semantik açıklama — TAMAMLANDI

### Bağlam
Çoğu kullanıcı tek görsel bırakıyor. Standart: **min 3 galeri görseli** + **1:1 kare, ≥1000px**. Feed
galeri görsellerini **1000×1000** servis ediyor (1080 değil) → kontrol "1:1 + ≥1000". Feed'de **4 galeri
slotu** (imgUrl 264, picture2 187, picture3 169, picture4 111 dolu). Bu feed'de boş `<details>`=0 →
sıfırdan üretim global özellik.

### Yapıldı
- **feed.rs**: `picture2/3/4` (`rename="picture[2-4]Path"`) + trimmed. **db.rs**: products `picture2/3/4`,
  seo_status `image_check_json`+`image_check_fp` (idempotent migration). **sync.rs**: upsert'e eklendi.
- **validation.rs**: `image_badge(count, dims)` (count<3→Eksik kapısı; ≥3 & tüm ok→Uygun; ≥3 & fail→Hatalı).
- **`images.rs`** (yeni): `imagesize` ile decode'suz boyut; `evaluate` (1:1 ±%2, `MIN_DIM=1000`);
  `check_dimensions`. **commands.rs::check_images(sku)** async + `?revision` parmak iziyle cache.
- **commands.rs**: `read_detail` galeri + `image_count` + `image_badge` + cache'li `image_check` döner.
  `generate_details` **kapı** (<3 → hata) + **3 yollu dallanma** (aşağıda 7b).
- **gemini.rs::generate_details_scratch**: Gemini'den `[{h2,p}]` (görsel sayısı kadar), grounding korunur;
  `assemble_scratch` **SEMANTİK** HTML montajlar — dış `<section class="yeni-aciklama {center|left|right}">`
  (sınıf döngüsü), iç `<div>` container/row/col-md (iç içe section YOK), anlamlı `alt`, görsel sola/sağa
  alternatif. Verilen CSS yalnızca dış section'ı hedeflediği için görünüm birebir korunur.
- **Frontend**: `ImageScoreCard.vue` (X/4 + thumbnail'lerde 1:1/çözünürlük rozeti, spinner/popIn, mevcut
  token'lar), ProductDetail'e eklendi. DetailsSeoCard "Açıklamayı Üret" **<3'te disabled** + tooltip.
  store `imageChecking/imageCheck` + `checkImages` (seçimde async, cache seed). Üretim kapısı **3 katman**:
  UI disabled + store guard + backend hata. api/types: `checkImages`, `ImageCheck`, ProductDetail görsel alanları.

### Doğrulama
- **42 birim testi geçti** (yeni: image_badge, evaluate×2, assemble_scratch_is_semantic). build'ler temiz, 0 uyarı.
- **Canlı DOĞRULANDI** (2026-07-24): `check_one_real` gerçek galeri → **1000×1000 kare=true min=true ok=true**;
  `gen_scratch_real` (gerçek Gemini) → 3 semantik section, iç div, görseller alternatif, anlamlı alt,
  hedef kelime `<strong>`/`<em>` ile doğal, kelime≥50. HTML örneği kusursuz.

## ✅ Faz 7b — MEVCUT açıklamanın optimizasyonu (yapı + metin + alt) — TAMAMLANDI

**Kullanıcı geri bildirimi:** "Açıklama Üret"in asıl işi zaten mevcut içeriği optimize etmek; semantik
HTML yalnızca sıfırdan üretimde değil, **mevcut içeriğin optimizasyonunda da** kullanılmalı (yapı + veri +
anlamlı `alt`). Sınıf döngüsü: **center → left → right → left → right…**

### Feed yapısı analizi (264 ürün)
Düzenli (N section = N img = N h2 = N p): **257 (%97)** — dağılım 4'lü:94, 3'lü:90, 2'li:70, 1'li:3.
Düzensiz 7: düz metin (3), `pre-order` banner + fazladan h2 (2), çoklu img (2).
Kullanılan sınıflar: `left` 543, `center` 261, `pre-order` 2 (`right` hiç kullanılmamış).

### Yapıldı — `gemini.rs`
- `split_top_sections` (derinlik sayan üst-düzey `<section>` ayırıcı; bloklar arası boşluk dışı içerik
  varsa None) + `extract_blocks` (blok başına özel sınıf / img'ler / 1 h2 / p'ler; **düzensizse None**).
- `optimize_details(...) -> Result<Option<String>>`: mevcut metinleri Gemini'ye **"anlamı koruyarak SEO
  için optimize et"** talimatıyla gönderir, `assemble_optimized` ile **semantik** HTML kurar
  (dış `<section>` + iç `<div>`, iç içe section YOK), **anlamlı `alt`** (ürün adı — başlık) ekler,
  `pre-order` gibi özel sınıfları KORUR, görselsiz bloğu `col-md-12` yapar.
  **Görsel invariant**: çıktı src listesi orijinalle aynı değilse `None` → eski yola düşülür.
- `class_for(i)`: 0→center, sonra left/right alternatif (kullanıcı isteği).
- `has_rewritable_content` (h2/p var mı) — dallanma için.

### `commands.rs` — 3 yollu dallanma
1. İçerik yok / yeniden yazılabilir metin yok → **`generate_details_scratch`** (galeri görselleri).
2. Düzenli yapı → **`optimize_details`** (metin + yapı + alt optimizasyonu). ← %97 ürün
3. Düzensiz yapı → **eski `generate_details`** (yapıyı aynen koruyarak yalnızca metni yeniden yaz).

### Doğrulama
- **47 birim testi geçti** (yeni 5: class_cycle, extract_blocks×2, optimize_assembly, özel-sınıf/görselsiz).
- **Canlı (gerçek Gemini)**: `optimize_real` → iç içe section'lar `<div>`'e döndü, `alt="image"` →
  `alt="Lenovo ThinkCentre… — Kesintisiz Güç"`, metin zengin/optimize, görseller birebir korundu,
  center→left döngüsü + görsel sola/sağa alternatif ✅

### Notlar
- Cargo: `imagesize = "0.13"`. Kapı SAYI bazlı (anında/güvenilir); 1:1/çözünürlük yalnızca uyarı.
- ⚠️ export/import (yedek) picture2/3/4 + image_check kolonlarını içermiyor (sync feed'den geri getirir;
  pre-existing gap: draft_details/research_json de yok). Kritik değil.
- Artık mevcut ürünlerin hatalı iç içe `<section>` yapısı da üretimde **düzeltiliyor** (düzenli olanlarda);
  düzensiz 7 üründe eski davranış korunur (güvenlik).

## ✅ Faz 7c — Hedef kelime yoğunluğu düzeltmesi (saha testi geri bildirimi) — TAMAMLANDI

**Sorun (kullanıcı):** Üretilen açıklamada yoğunluk %2-3 hedefine rağmen bazen %9/%4 çıkıp "Hatalı"
görünüyordu.

**Kök neden:** Yoğunluk formülü **ağırlıklıydı** (`geçiş × öbek_kelime_sayısı / toplam`). Çok kelimeli
hedef kelime ("all in one bilgisayar" = 4 kelime) her geçişte 4× sayılıyordu → 4 doğal geçiş = %10.
Standart SEO yoğunluğu **öbek başınadır**.

**Düzeltme:**
- `validation.rs::keyword_density` + `validation.ts::density`: **öbek-bazlı** (`geçiş / toplam`), öbek
  kelime sayısıyla çarpılmaz. Artık aynı içerik (Gemini'nin ürettiği 4-6 doğal geçiş) doğru ölçülüyor:
  optimize %2.5, sıfırdan %2.69 → **Uygun**. (`density_counts_phrase` testi 18.18'e güncellendi.)
- **Yoğunluk güvenlik ağı**: `gemini.rs`'e `density_out_of_range`/`density_correction`/`density_dist` +
  her üç üretim yolunda (yeniden yaz / sıfırdan / optimize) aralık dışıysa **tek retry** (modele "azalt/
  artır" talimatı), iki denemeden hedefe (%2.5) yakın olanı seçilir; görsel invariant korunur.

**Doğrulama:** 48 birim testi geçti (yeni `density_range_and_correction`), build'ler temiz. Canlı Gemini
çıktıları ölçüldü → %2.5 / %2.69 (Uygun). ⚠️ Formül değişikliği tüm ürünlerin yoğunluk rozetini yeniden
hesaplar (çok kelimeli hedef kelimeliler artık doğru/daha düşük görünür).

## ✅ Faz 8 — Teknik Özellik Tablosu (elle-yapıştır, halüsinasyon-sıfır) — TAMAMLANDI

### Bağlam
IdeaSoft'ta teknik tablo **ayrı alanda** ve **feed'e girmiyor** → uygulama okuyamaz, sıfırdan üretmeli.
Ama yanlış spec = iade + mevzuat riski. Kullanıcı: "halüsinasyon riskini göze alamam."

### Icecat araştırması (ölçüldü, 2026-07-25)
- Open Icecat sponsor markaları katalogun **%50'si** (Lenovo/HP/Dell/Logitech/TP-Link/Asus/SanDisk=131/264).
  Creality 32, Aruba 26, Ergotron 10, Anycubic 6, Bambu Lab 5, HPE 5, Digitus 5, Snapmaker 4 kapsam dışı.
- Sponsorlu markalarda bile **gerçek isabet %35** (20 ürünlük canlı örneklem, demo API).
  Sebep: Dell `BTO107_PC14250_UB` (build-to-order) ve HP `AV3Z0AW` (ülkeye özel) SKU'lar global katalogda yok.
  Standart perakende (monitör/mouse/çanta) ✅, konfigüre notebook ❌.
- Net otomatik doldurulabilirlik **~%20-25** → **elle yapıştır ANA YOL**, Icecat kapsam dışı bırakıldı.

### Yapıldı
- **db.rs**: `seo_status`'a `tech_source_text`, `tech_specs_json`, `tech_status` (idempotent migration).
- **gemini.rs**: `TechRow/TechGroup/TechSpecsResult`, `TECH_GROUPS` kanonik sırası,
  `structure_tech_specs` (ham metin → gruplu JSON, responseSchema + MODEL_CHAIN, temp 0.2),
  **`verify_traceable`** ← halüsinasyon kalkanı, `assemble_tech_html` (deterministik, modelsiz).
- **`verify_traceable` kuralı:** üretilen değerdeki **her sayı kaynak metinde birebir geçmeli**;
  geçmiyorsa satır **atılır** + kullanıcıya raporlanır. Ondalık ayırıcı toleransı (4.90 ↔ 4,90).
  Sayısız değerler (IPS, FreeDOS) doğrulanamaz → prompt kısıtına güvenilir.
- **HTML çıktısı semantik**: grup `<caption>`, satır `<th scope="row">`, genişlik `<colgroup>`.
  **`<thead>`+boş `<th>` ve Bootstrap `col-4/col-8` KULLANILMAZ** (kullanıcının mevcut tablosundaki hata).
  "Kutu İçeriği" tablo değil `<h3>`+`<ul>`.
- **commands.rs**: `save_tech_source`, `structure_tech_specs`, `save_tech_specs`, `tech_table_html`,
  `mark_tech_done`; `read_detail` → tech alanları + `tech_badge`.
- **⚠️ Yedekleme düzeltildi**: export/import artık `draft_details`, `research_json`, `image_check_*`,
  `tech_*` ve `products.picture2/3/4` kolonlarını içeriyor. (Teknik tablo feed'den geri gelemez!)
- **Frontend**: `TechTableCard.vue` — yapıştır → "Yapılandır" → gruplu **düzenlenebilir** önizleme
  (contenteditable hücreler, satır sil) → "HTML kopyala" → "Tamamlandı". Atılan satırlar `--warn-bg`
  şeridinde raporlanır. Kart kabuğu + butonlar diğer 3 kartla birebir aynı.

### Doğrulama
- **53 birim testi geçti** (yeni 5: verify_traceable×3, assemble_tech_html×2). build'ler temiz, 0 uyarı.
- **CANLI HALÜSİNASYON TESTİ** (`tech_specs_real`, gerçek Gemini): modelin iyi bildiği Lenovo AIO'ya
  **kasıtlı eksik** metin verildi (parlaklık/ağırlık/renk gamı yok) → çıktıda **hiçbir uydurma sayı yok** ✅
  Gruplar kanonik taksonomiye doğru atandı, kutu içeriği liste olarak geldi.

### Tema CSS'i — **Ayarlar > Teknik Tablo CSS** kartında saklanıyor
Siteye bir kez eklenmeli; tema dosyasından silinirse uygulamadan yeniden alınabilir (göster + kopyala).
`caption-side: top` **Bootstrap'i ezmeli** (varsayılan `bottom`!). Legacy `thead-light` ile aynı
görünmesi için ikisi birden stillenir. Mobil: yatay scroll YOK, sütun %35→%42, `overflow-wrap: anywhere`, ≥14px.

## ✅ Faz 8b — Teknik tablo sürüm geçmişi

**Kullanıcı sorusu:** "Tablo veritabanına kaydediliyor mu? Yeniden üretilirse önceki sürüme erişilebilsin."

**Cevap 1 — kayıt ZATEN vardı:** `tech_specs_json`/`tech_source_text` yazılıyor (structure + elle kayıt),
`read_detail` okuyor, yedeklemede var. Ürün tekrar seçilince tablo geliyor → tekrar üretim/kredi kaybı yok.

**Cevap 2 — sürüm geçmişi eklendi:**
- `seo_status.tech_history_json` (migration) — `Vec<TechVersion{at, groups, source}>`, **en yeni başta,
  son 5 sürüm** (`TECH_HISTORY_MAX`).
- **Yalnızca yeniden üretim** anlık görüntü alır (elle hücre düzenlemeleri geçmişi kirletmez).
  `structure_tech_specs` yazmadan önce mevcut tabloyu geçmişe iter.
- **`restore_tech_version(sku, index)` — TAKAS mantığı:** seçilen sürüm güncel olur, mevcut tablo
  geçmişin başına konur → geri yükleme de kayıpsız, istenirse geri dönülebilir. Kaynak metin de
  sürümle birlikte geri gelir (tutarlılık).
- `read_detail` → `tech_history: Vec<TechVersionMeta{at, rows, groups}>` (hafif özet; tam sürümler
  payload'ı şişirmesin). Yedeklemeye `tech_history_json` da eklendi.
- **Frontend:** meta satırında "önceki sürümler (N)" → tarih · N satır · **Geri yükle**.

**Doğrulama:** 56 birim testi (yeni 3: push_history cap/sıra, parse_history bozuk-JSON toleransı,
roundtrip). build'ler temiz.

### Saha testi düzeltmeleri (kullanıcı geri bildirimi)
- Kart alt başlığı: "IdeaSoft teknik özellik alanı için" → **"Web site teknik özellik alanı için"**
  (uygulama global; platform adı sabitlenmemeli)
- "Yapılandır" altındaki "metinden tablo · uydurma yok" alt metni kaldırıldı (gereksiz)

### Kapsam dışı (bilinçli)
Icecat entegrasyonu (aynı `tech_specs_json` modelini sonradan doldurabilir), PSREF/üretici kazıma,
Schema.org `additionalProperty` çıktısı (veri hazır, tek adımlık ek iş).

## ✅ Faz 9 — IdeaSoft Gönderim Modülü (opsiyonel) — TAMAMLANDI

### Bağlam
Üretilen içerik elle kopyalanıp panele yapıştırılıyordu. IdeaSoft API'siyle tek tık mümkün — ama uygulama
**global**: token'ı olmayan kullanıcı için **kopyala-yapıştır ANA YOL kalmalı** (kullanıcı kararı).
Bu yüzden **modül**: Ayarlar'da domain+token dolunca kartlarda "IdeaSoft'a Gönder" belirir, boşsa hiçbir şey değişmez.

### Canlı doğrulanmış API bulguları (2026-07-25, gerçek mağaza)
- **MCP'ye GEREK YOK.** `mcp.myideasoft.com` "keşif + genel çağırıcı" sarmalayıcı (LLM ajanları için);
  altındaki gerçek yüzey `https://{domain}/admin-api/...` → **doğrudan reqwest** (Node/npx/mcp-remote yok).
- Kimlik: `Authorization: Bearer {token}` ✅ · sku→id: **`GET /admin-api/products?s={sku}`** ✅
  (⚠️ `?sku=` ve `?name=` **yok sayılıyor**; `q` dizi bekliyor) · `GET|PUT /admin-api/products/{id}` ✅
- **Alan eşlemesi (ürün #119894 Anycubic üzerinde doğrulandı):**
  `pageTitle` · `metaDescription` · `metaKeywords` · `searchKeywords` · **`targetKeyword`** (IdeaSoft'un
  kendi hedef kelime alanı) · açıklama → **`detail.details`** · **teknik tablo → `detail.extraDetails`** ✅
  Bonus okunabilir: `seoTotalRuleCount` (IdeaSoft'un kendi SEO skoru).
- Hız sınırı ~40 istek/dk. Mağaza admin domaini `3ekurumsal.myideasoft.com` = kurumsalit.com.

### Yapıldı
- **`ideasoft.rs`** (yeni): `resolve_id` (**sku birebir eşleşme** — `ABC-123` ≠ `ABC-123-XL`),
  `fetch_product`, `build_payload`, `push_product`, `test_connection`, `base_url` normalize,
  401/404/429 → anlaşılır Türkçe mesaj.
- **commands.rs**: `test_ideasoft`, `ideasoft_preview` (fark), `ideasoft_push`; `ideasoft_local`
  (yerel içerik derleme, teknik tablo `gemini::assemble_tech_html` ile), `ideasoft_id_for` (id cache).
  Settings'e `ideasoft_domain`/`ideasoft_token`/**`ideasoft_active`**; `read_detail` → `ideasoft_pushed_at`.
- **db.rs**: `seo_status.ideasoft_product_id`, `ideasoft_pushed_at` (migration + yedeklemeye eklendi).
- **Frontend**: `IdeasoftPushModal.vue` (alan alan fark: "IdeaSoft'ta şu an" ↔ "Gönderilecek",
  değişmeyenler soluk, canlı-mağaza uyarısı), 3 kartta `is-push` butonu (`ideasoft_active` ise),
  Ayarlar'da "IdeaSoft Bağlantısı" kartı (opsiyonel etiketi + test), ürün başlığında yeşil
  "IdeaSoft'a gönderildi · …".

### Güvenlik kuralları (payload)
- Yalnızca seçilen `parts` (`meta`|`details`|`tech`) gönderilir; **boş alan gönderilmez** (uzaktakini silmesin).
- `details`+`tech` birlikte → **tek `detail` nesnesi** (biri diğerini ezmez).
- Gönderim öncesi **fark önizlemesi zorunlu**; toplu gönderim YOK (operatör kontrolü).

### Doğrulama
- **64 birim testi geçti** (8 yeni: payload×4, sku eşleşme, base_url, hata mesajları, nested detail parse).
  frontend 65 modül temiz, 0 uyarı.
- Canlı test **yalnızca OKUMA** (`ideasoft_read_real`, env-gated) — otomatik test canlı mağazaya yazmaz.
  İlk gerçek `PUT` kullanıcı tarafından UI'dan onaylanarak yapılır.

## ✅ Faz 9b — Saha testi düzeltmeleri + 4 boyutlu durum + ilerleme çubuğu

Kullanıcının IdeaSoft modülünü sahada denemesiyle çıkan 6 madde:

1. **Hedef kelime senkronu** — `ideasoft_pull_keyword` komutu (IdeaSoft'tan çek) + `parts=["keyword"]`
   ile yalnızca `targetKeyword` gönderimi. Hedef kelime satırında **Getir / Gönder** butonları
   (modül aktifken). IdeaSoft'un SEO kural skoru bu alana bağlı.
2. **`seoTotalRuleCount` gösterimi** — ⚠️ **yalnızca LİSTE ucunda dolu** (`/products/{id}` → null).
   `resolve()` artık `Resolved{id, seo_rule_count}` döndürüyor → ekstra istek yok, `ideasoft_seo_rule`
   kolonunda cache'lenip ürün başlığında "IdeaSoft SEO: 13" olarak gösteriliyor.
3. **BUG: metaKeywords yazılmıyordu** — kök neden: `ProductDetail`'de **`draft_keywords` alanı hiç yoktu**;
   feed'in (boş) `keywords` alanı gönderiliyordu. Artık `draft_keywords ?? keywords ?? draft_search_keywords`.
4. **BUG: teknik tablo gönderiminde HTTP 400** — `{"detail":{"details":"This value should not be null."}}`.
   IdeaSoft `detail` nesnesinde `details`'in null olmasına izin vermiyor. `fill_detail_from_remote`:
   gönderim öncesi uzaktaki ürün okunup **eksik alt alan mevcut değeriyle doldurulur** (dokunulmayan taraf korunur).
5. **Kart 3 buton taşması** — "HTML kopyala" DetailsSeoCard'daki gibi **üst bilgi şeridine** taşındı;
   alt sıra üç kartta da aynı düzende.
6. **İlerleme çubuğu + 4 boyutlu durum** (kullanıcı kararı):
   - `overall_status` artık `OverallInput{meta, details, tech_done, has_tech, image_count}` alıyor.
   - **Tamamlandı = meta_done && details_done && tech_done**; `image_count<3` → Eksik (üretim zaten engelli);
     hiç teknik tablo yoksa → Eksik. Aksi halde bazıları işaretliyse Bekliyor, hepsi hazırsa Uygun.
   - `list_products` teknik tablo + galeri sayısını da okuyor; `ProductRow`'a `tech_done`, `image_count`.
   - Üst şeritte "Son güncelleme" altında **tek ilerleme çubuğu**: "164/264 tamamlandı".
   - ⚠️ Beklenen etki: teknik tablosu olmayan ürünler artık "Tamamlandı" görünmez (dürüst ölçüm).

## ✅ Faz 10 — Otomatik güncelleme (Tauri updater)

Kullanıcı: "olmazsa olmazlarımızdan biri" — açılışta kontrol, bildirim, tek tıkla kendi kendine güncelleme.

### Önemli: imza anahtarı ≠ kod imzalama
Tauri updater kendi **minisign** anahtar çiftini kullanır; **ücretsiz** ve Apple/Microsoft kod imzalama
sertifikalarından bağımsızdır. Yani kod imzalama ertelenmişken bile otomatik güncelleme çalışır
(Gatekeeper/SmartScreen uyarısı devam eder).

### Yapıldı
- Anahtar çifti üretildi; **GitHub secret'ları ayarlandı**: `TAURI_SIGNING_PRIVATE_KEY`,
  `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` (boş).
  ⚠️ **Gizli anahtar yedeklenmeli** — kaybolursa mevcut kurulumlar bir daha otomatik güncellenemez.
  (Anahtar depoya KONULMADI; kullanıcıya kalıcı yere kopyalaması söylendi.)
- `Cargo.toml`: `tauri-plugin-updater` + `tauri-plugin-process`, **yalnızca masaüstü hedefinde**
  (`[target.'cfg(not(any(target_os="android",target_os="ios")))'.dependencies]`, dosya SONUNDA —
  `[dependencies]` ortasına konursa kalan bağımlılıklar bozulur).
- `lib.rs`: eklentiler **tek `setup()` içinde** kaydedildi (ikinci bir `.setup()` birincisini ezer).
- `tauri.conf.json`: `plugins.updater` → endpoint `.../releases/latest/download/latest.json` + public key.
- `capabilities/default.json`: `updater:default`, `process:allow-restart`.
- CI: imzalama env değişkenleri + **`includeUpdaterJson: true`** → her release'e `latest.json` eklenir.
- **Frontend**: `UpdateModal.vue` (sürüm notu, indirme yüzdesi/boyutu, "Şimdi güncelle"/"Sonra",
  diğer modallarla aynı animasyon dili). store: `checkUpdate(silent)`, `runUpdate`, `dismissUpdate`,
  `appVersion`. Açılışta **sessiz** kontrol (ağ yoksa kullanıcıyı rahatsız etmez) +
  Ayarlar'da "Uygulama Sürümü" kartı → "Şimdi denetle".

### ⚠️ v0.5.0'da atlanan zorunlu ayar → v0.5.1'de düzeltildi
v0.5.0 derlendi ama Release'e **`latest.json` eklenmedi**; log: *"Signature not found for the updater
JSON. Skipping upload"*. Sebep: **Tauri v2'de `bundle.createUpdaterArtifacts: true` ZORUNLU** — bu bayrak
olmadan `.app.tar.gz` / `.nsis.zip` updater paketleri ve manifest üretilmez. (İki platform da `.sig`
üretiyordu, bu yanıltıcıydı.) v0.5.1'de eklendi.

**Devreye girme:** otomatik güncelleme **v0.5.1'den itibaren** çalışır (ilk `latest.json` orada).
v0.5.1 kurulduktan sonraki sürümde modal kendiliğinden çıkar; öncesi elle kurulum.

## 🎯 Sonraki olası işler (opsiyonel)
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

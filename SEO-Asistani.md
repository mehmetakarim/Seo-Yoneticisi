# SEO Yöneticisi — Masaüstü Uygulama (Faz 1)

## Tasarımı içe aktar

> **Seçenek A — Claude Design MCP ile (tercih edilen):**
> Use the claude_design MCP (https://api.anthropic.com/v1/design/mcp, auth via /design-login) to import this project:
> https://claude.ai/design/p/0b63b06a-274d-4614-930b-f7c7a3e72809?file=SEO+Y%C3%B6neticisi.dc.html
> Implement: SEO Yöneticisi.dc.html
>
> **Seçenek B — Zip ile:**
> Import the attached e-ticaret-seo-y-netim-arac.zip — read the README inside.
> Implement: SEO Yöneticisi.dc.html

Tasarım, uygulamanın birebir arayüz referansıdır. Renkler, tipografi, açık/koyu tema, sol sidebar + iki panelli ana alan (ürün listesi + iki kartlı detay paneli), üst sync özet şeridi, klavye kısayolları (G = üret, D = tamamla, ⌘F = ara) ve Ayarlar ekranı (Kaynaklar + Yedekleme) tasarımdaki gibi uygulanmalı. MCP ile tasarıma ulaşamazsan projenin kök klasöründe design klasörünün içine zip dosyasını ekledim.

---

## Ne yapıyoruz

kurumsalit.com'un IdeaSoft XML ürün feed'ini okuyup, her ürünün SEO metinlerini (meta ve uzun açıklama) tek tek Gemini API ile üretip validasyondan geçiren bir **Tauri + Vue masaüstü uygulaması**. Kullanıcı üretilen metni kopyalayıp IdeaSoft paneline yapıştırır, sonra "Tamamlandı" işaretler. Bu Faz 1: feed okuma + SQLite senkron + meta validasyonu + liste/detay UI. Gemini üretimi ve details optimizasyonu sonraki fazlar (iskelet şimdiden hazırlanır).

## Stack

- **Tauri v2** (masaüstü kabuk)
- **Vue 3** (Composition API) + Pinia (durum) frontend
- **Rust** backend: XML fetch + parse, SQLite okuma/yazma, ileride Gemini çağrısı. **Ağ ve API işlemleri Rust tarafında** olmalı (CORS'tan kaçınmak ve API anahtarını frontend'den uzak tutmak için).
- **SQLite** (rusqlite veya sqlx) — yerel iş durumu.
- XML parse: **quick-xml** (serde ile).

## Feed

- URL (ayarlardan değiştirilebilir, varsayılan): `https://www.kurumsalit.com/output/2567783262`
- Feed **read-only**. Uygulama feed'i sadece okur, asla yazmaz. Feed sunucuda otomatik üretilir.
- Feed'e sadece **stok > 0 ve aktif** ürün girer (`status` her zaman `1`).

### Feed XML yapısı

Kök `<products>`, her ürün `<product>`. Alanlar (çoğu CDATA):

| Alan | Açıklama | Kullanım |
|---|---|---|
| `id` | Export sıra no | **Anahtar DEĞİL** (kararsız, aynı ürünün iki id'si olabiliyor) |
| `sku` | Stok kodu | **Birincil anahtar** — tüm eşleştirme bununla |
| `name` | Ürün adı | Liste + hedef kelime kaynağı |
| `status` | 1 = aktif | Feed'de hep 1 |
| `productBrand` | Marka | Liste + detay başlığı |
| `searchKeywords` | Site içi arama kriteri | Meta keywords DEĞİL — site içi SEO. Şu an tüm ürünlerde boş. |
| `mainCategory` | 1. seviye kategori | Detay başlığı |
| `category` | 2. seviye kategori | Detay başlığı |
| `quantity` | Stok adedi | Detay başlığı |
| `quantityStatus` | CDATA `[ var ]` — **baştasonda boşluk, trim şart** | Bilgi |
| `imgUrl` | Ürün görseli | Liste küçük görsel |
| `details` | **Uzun HTML açıklama** (section/col-md/img/h2/p; ürün başına 2–4 KB) | Faz 2 details optimizasyonu için sakla, listede gösterme |
| `url` | Ürün sayfası | Detay dış link |
| `title` | Meta sayfa başlığı | Meta validasyonu |
| `keywords` | Meta anahtar kelimeler | Meta validasyonu |
| `descriptions` | Meta açıklama | Meta validasyonu |

**Önemli:** feed'deki `title`/`descriptions`/`keywords` şu an çoğunlukla ürün adının birebir kopyası — yani dolu ama SEO kurallarına aykırı. Bu normal; uygulamanın işi bunları ayıklamak. `details` = ürün detay sayfasındaki uzun görsel açıklama HTML'i (feed'de tam haliyle var).

## SQLite şeması

```sql
CREATE TABLE products (
  sku TEXT PRIMARY KEY,
  id TEXT,
  name TEXT NOT NULL,
  brand TEXT,
  main_category TEXT,
  category TEXT,
  quantity INTEGER,
  url TEXT,
  img_url TEXT,
  title TEXT,
  descriptions TEXT,
  keywords TEXT,
  search_keywords TEXT,
  details TEXT,              -- ham HTML, Faz 2 için
  last_synced_at TEXT
);

CREATE TABLE seo_status (
  sku TEXT PRIMARY KEY REFERENCES products(sku) ON DELETE CASCADE,
  meta_status TEXT DEFAULT 'pending',     -- pending | done  (Kart 1)
  details_status TEXT DEFAULT 'pending',  -- pending | done  (Kart 2, Faz 2)
  target_keyword TEXT,
  updated_at TEXT
);

CREATE TABLE sync_log (
  run_at TEXT,
  active INTEGER,
  added INTEGER,
  updated INTEGER,
  deleted INTEGER,
  duplicate_skipped INTEGER
);
```

## Senkron mantığı ("Manuel Güncelle" butonu)

1. Feed'i Rust'tan fetch et, quick-xml ile parse et.
2. Gelen ürünleri `sku` bazlı işle:
   - Feed içinde **aynı sku ikinci kez** gelirse ilkini al, sonrakini say → `duplicate_skipped++`.
   - `sku` DB'de **yoksa** → INSERT, `added++`, `seo_status` satırı da oluştur (meta_status/details_status = pending).
   - `sku` DB'de **varsa** → feed alanlarını UPDATE (title/descriptions/keywords/searchKeywords/details/quantity/... güncellenir), `updated++`. **`seo_status` satırına DOKUNMA** (kullanıcının done/target_keyword durumu korunur).
3. **Düşen ürün temizliği:** `db_skus − feed_skus` = feed'de olmayan sku'lar → **tam DELETE** (products + seo_status cascade), `deleted++`. (Feed'de `title`/`descriptions` zaten dolu geldiği için geri gelen ürün validasyondan geçerse listede görünmez; soft-delete gereksiz.)
4. `sync_log`'a bir satır yaz. Üst şeritte göster: **Aktif: N · Eklenen · Güncellenen · Silinen (düşen) · Mükerrer atlanan**. (Stok-0 sayacı gereksiz — feed'e zaten stok>0 giriyor; UI'da 0 gösterilebilir ya da hiç gösterilmez.)

## Meta validasyonu (Faz 1'in çekirdeği)

Her ürün için feed'deki `title` / `descriptions` / `keywords` üzerinden hesaplanır. Hedef kelime kaynağı **`seo_status.target_keyword`**; Faz 1'de bu alan henüz boşsa validasyonun "hedef kelime içeriyor" kuralı "belirsiz/bekliyor" olarak gösterilsin (hedef kelime Gemini üretiminde dolacak — bkz. Faz 2).

**Sayfa Başlığı (title) kuralları:**
- Boş değil.
- 20 ≤ karakter ≤ 60. (Türkçe karakterler için `[...str].length` / grapheme sayımı kullan, ham byte değil.)
- Hedef kelime içeriyor (target_keyword doluysa, case-insensitive).

**Meta Açıklama (descriptions) kuralları:**
- Boş değil.
- 50 ≤ karakter ≤ 155.
- Hedef kelime içeriyor.

**Site İçi Arama (searchKeywords) kuralı (Faz 1'de sadece gösterim):**
- Boş değil. (Şu an tümü boş → hepsi kırmızı çıkacak, normal.)

**Ürün durum rozeti:**
- Meta'nın üç title + üç descriptions kuralı da geçiyorsa → **Uygun**.
- Bir/birkaç kural fail → **Hatalı**.
- title veya descriptions tamamen boşsa → **Eksik**.
- `seo_status.meta_status = 'done'` → **Tamamlandı** (listede varsayılan gizli).

Liste durumu iki boyutlu olacak (Meta + Details); Faz 1'de Details boyutu "pending" sabit, ama UI ikili göstergeyi (Meta ✓ · Açıklama ○) destekleyecek şekilde kurulsun.

## UI (tasarıma birebir)

- **Sol panel:** arama (⌘F), filtre sekmeleri (Eksik / Hatalı / Uygun / Tamamlandı / Tümü + ileride "Açıklama Bekliyor"), ürün satırları (küçük görsel + ad + SKU + durum rozeti). Tamamlandı olanlar varsayılan gizli. Klavye ile gezinme (↑↓), G = üret, D = tamamla.
- **Detay paneli — iki bağımsız kart:**
  - Üstte: ürün adı + marka · kategori · stok + dış link. Ortak **Hedef Kelime** alanı.
  - **Kart 1 — Meta SEO:** Sayfa Başlığı, Meta Açıklama, Site İçi Arama Kelimeleri (her biri canlı karakter/kural göstergesi + Kopyala butonu). Altında "Gemini ile Üret" + "Meta'yı Tamamlandı işaretle". Kendi durum rozeti.
  - **Kart 2 — Açıklama SEO (Details):** uzun HTML içerik alanı + kelime sayısı (min 50) + keyword yoğunluğu (%2–3) + "boş değil" göstergeleri. "HTML yapısı korunur, yalnızca metin içeriği yenilenir" bilgi etiketi. Altında "Açıklamayı Üret" (+ "uzun içerik · daha fazla kredi" alt metni) + "Açıklamayı Tamamlandı işaretle". Kendi durum rozeti. **Faz 1'de bu kart görünür ama üretim butonu devre dışı/placeholder** (Faz 2'de bağlanacak).
- **Boş durum:** ürün seçilmemişse "Soldan bir ürün seçin".
- **Ayarlar ekranı:** Kaynaklar kartı (XML Feed URL + "Test et"; Gemini API Anahtarı + göster/gizle + "Bağlantıyı test et" — anahtar SQLite'ta saklanır, Rust tarafında kullanılır) ve Yedekleme kartı (Veritabanını dışa aktar .db/.json; Yedekten içe aktar; "içe aktarma mevcut DB'nin üzerine yazar" uyarısı).

## Faz 1 teslim kapsamı (net sınır)

**Yap:**
- Rust: feed fetch + quick-xml parse + SQLite şema + senkron mantığı (upsert + düşen temizliği + sync_log) + meta validasyon fonksiyonları (HTML-strip yardımcı fonksiyonu Faz 2 için hazır dursun ama details validasyonu henüz bağlanmasın).
- Tauri komutları: `sync_feed()`, `list_products(filter, search)`, `get_product(sku)`, `set_target_keyword(sku, kw)`, `mark_meta_done(sku)`, `get_settings()`, `save_settings()`, `export_db()`, `import_db()`.
- Vue: liste + iki kartlı detay paneli + Ayarlar, tasarıma birebir. Klavye kısayolları. Açık/koyu tema.

**Yapma (sonraki fazlar):**
- Gemini API çağrısı (meta üretimi = Faz 2, details üretimi = Faz 3). Butonlar dursun ama placeholder.
- Details HTML optimizasyonu ve img-src koruma doğrulaması (Faz 3).

## Faz 2/3 için not (backend'i şimdiden buna hazırla)

- **Meta üretimi (Faz 2):** tek Gemini çağrısı, JSON structured output. Ürün adından **hedef kelime türet** → title (20–60, hedef kelime içerir) + descriptions (50–155, hedef kelime içerir) + keywords + searchKeywords (kullanıcının ürünü sitede nasıl arayacağı mantığıyla). Dönen JSON parse edilip 5 alan doldurulur, validasyon otomatik yeniden çalışır. Üretim sonrası kural fail ederse tek retry ("kısalt/uzat").
- **Details üretimi (Faz 3):** ayrı Gemini çağrısı. `details` HTML'i gönderilir; **yapı korunur** (section/col-md/img/h2/p sırası ve class'lar aynı), sadece h2/p metinleri yenilenir. Üretimden önce tüm `<img src>` URL'leri Rust'ta bir listeye çıkarılır; dönen HTML'de src'ler bozulmuşsa orijinalleriyle geri yazılır (kredi harcamadan görsel güvenliği). Kelime sayısı ve keyword yoğunluğu HTML-strip edilmiş düz metinden ölçülür.

## Kabul kriterleri (Faz 1)

1. "Manuel Güncelle" gerçek feed'i çeker, ~250–500 ürünü SQLite'a yazar, sync özet sayaçları doğru gösterir.
2. Aynı feed'i ikinci kez çekince "eklenen" 0, "güncellenen" tüm ürünler olur; done/target_keyword bilgisi kaybolmaz.
3. Feed'den bir ürün düşerse sonraki senkronda silinir ve "silinen: 1" raporlanır.
4. Liste, durum rozetlerini doğru hesaplar; filtre sekmeleri çalışır; Tamamlandı olanlar gizlenir.
5. Detay panelinde iki kart görünür; Meta kartında canlı karakter/kural göstergeleri doğru; Kopyala butonları panoya alır; "Meta'yı Tamamlandı" ürünü listeden düşürür.
6. Ayarlarda feed URL ve Gemini anahtarı kaydedilir/saklanır; DB export/import çalışır.
7. Türkçe karakter sayımı doğru (grapheme bazlı).

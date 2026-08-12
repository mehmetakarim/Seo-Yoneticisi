use crate::fingerprint::FeedFacts;
use rusqlite::Connection;

pub fn open(path: &std::path::Path) -> Result<Connection, String> {
    let conn = Connection::open(path).map_err(|e| format!("Veritabanı açılamadı: {e}"))?;
    init(&conn)?;
    Ok(conn)
}

pub fn init(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        r#"
        PRAGMA foreign_keys = ON;

        CREATE TABLE IF NOT EXISTS products (
          sku TEXT PRIMARY KEY,
          id TEXT,
          name TEXT NOT NULL,
          brand TEXT,
          main_category TEXT,
          category TEXT,
          quantity INTEGER,
          url TEXT,
          img_url TEXT,
          picture2 TEXT,
          picture3 TEXT,
          picture4 TEXT,
          title TEXT,
          descriptions TEXT,
          keywords TEXT,
          search_keywords TEXT,
          details TEXT,
          last_synced_at TEXT
        );

        CREATE TABLE IF NOT EXISTS seo_status (
          sku TEXT PRIMARY KEY REFERENCES products(sku) ON DELETE CASCADE,
          meta_status TEXT DEFAULT 'pending',
          details_status TEXT DEFAULT 'pending',
          target_keyword TEXT,
          draft_title TEXT,
          draft_descriptions TEXT,
          draft_keywords TEXT,
          draft_search_keywords TEXT,
          updated_at TEXT
        );

        CREATE TABLE IF NOT EXISTS sync_log (
          run_at TEXT,
          active INTEGER,
          added INTEGER,
          updated INTEGER,
          deleted INTEGER,
          duplicate_skipped INTEGER
        );

        CREATE TABLE IF NOT EXISTS settings (
          key TEXT PRIMARY KEY,
          value TEXT
        );

        -- ===== Ölçüm omurgası (Faz Ö) =====
        -- ⚠️ Anlık görüntüler DEĞİŞTİRİLEMEZ. Fırsat raporu tek bir `settings` anahtarına
        -- yazılıyordu ve her analiz bir öncekini siliyordu; "işe yaradı mı?" sorusunun
        -- cevapsız kalmasının sebebi buydu. Buraya yalnızca eklenir, güncellenmez.
        CREATE TABLE IF NOT EXISTS metric_snapshots (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          captured_at TEXT NOT NULL,
          window_start TEXT NOT NULL,
          window_end TEXT NOT NULL,
          source TEXT NOT NULL DEFAULT 'gsc',
          rows INTEGER NOT NULL DEFAULT 0,
          clicks REAL NOT NULL DEFAULT 0,
          impressions REAL NOT NULL DEFAULT 0,
          UNIQUE (window_start, window_end)
        );

        -- Satır eşiği ÖLÇÜLEREK seçildi (bkz. `metrics::kept`): tıklama > 0 veya
        -- gösterim >= 10. Gerçek veride satırların %34'ü tutuluyor ama tıklamaların
        -- %100'ü kapsanıyor — 12 anlık görüntü 8,7 MB yerine 2,9 MB.
        CREATE TABLE IF NOT EXISTS metric_page_rows (
          snapshot_id INTEGER NOT NULL REFERENCES metric_snapshots(id) ON DELETE CASCADE,
          url TEXT NOT NULL,
          sku TEXT,
          clicks REAL NOT NULL DEFAULT 0,
          impressions REAL NOT NULL DEFAULT 0,
          position REAL NOT NULL DEFAULT 0,
          PRIMARY KEY (snapshot_id, url)
        );
        CREATE INDEX IF NOT EXISTS idx_page_rows_sku ON metric_page_rows(sku);

        -- "Ne yaptık, ne zaman". ⚠️ `reaches_store` merkezi kural: yalnızca Google'ın
        -- göreceği değişiklikler (gönderim, canonical) puanlanıyor; yerel "tamamlandı"
        -- işaretleri zaman çizelgesinde bağlam olarak duruyor.
        CREATE TABLE IF NOT EXISTS work_events (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          at TEXT NOT NULL,
          sku TEXT,
          url TEXT,
          kind TEXT NOT NULL,
          reaches_store INTEGER NOT NULL DEFAULT 0,
          payload_json TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_work_events_sku ON work_events(sku, at);

        -- Bugün kuyruğundan çıkarılan maddeler (Faz K).
        -- ⚠️ Kuyruğun KENDİSİ saklanmıyor (her açılışta hesaplanıyor); kalıcı olan tek şey
        -- kullanıcının kararı. `until` NULL ise kalıcı gizleme, doluysa o tarihe kadar erteleme.
        -- `ref` ürün için sku, satışta olmayan sayfa için slug — EOL satırlarında sku YOK.
        --
        -- Üç çıkarma biçimi var, üçü de bu tabloda:
        --   until NULL + done_at_analysis NULL → kalıcı gizleme
        --   until = 'YYYY-AA-GG'               → o tarihe kadar erteleme
        --   done_at_analysis = analiz damgası  → "yapıldı", SONRAKİ ANALİZE KADAR
        -- ⚠️ "Yapıldı" bilerek kalıcı değil: iş bugün yapıldı, ama analiz yenilendiğinde
        -- sorun sürüyorsa madde geri gelmeli. Kalıcı gizleseydik gerçekten çözülmemiş bir
        -- iş sessizce kaybolurdu.
        CREATE TABLE IF NOT EXISTS queue_dismissals (
          kind TEXT NOT NULL,
          ref TEXT NOT NULL,
          until TEXT,
          at TEXT NOT NULL,
          PRIMARY KEY (kind, ref)
        );

        -- ===== EOL karar deposu (Faz D) =====
        -- 🔴 Bu tablonun varlık sebebi: halef önerileri bugüne kadar HİÇBİR YERDE
        -- saklanmıyordu (yalnızca `store.successors`, yani bellekte). Uygulama kapanınca
        -- verilen kararlar kayboluyordu ve panele taşınacak bir çıktı üretilemiyordu.
        --
        -- ⚠️ Karar ≠ öneri. Model bir hedef önerebilir ama satır ancak kullanıcı ONAYLAYINCA
        -- buraya yazılır. Sebebi ölçülmüş: deterministik eşleştirici tek başına güvenilir
        -- değil (bkz. `opportunity::successor_candidates`) ve yanlış yönlendirme,
        -- yönlendirmemekten kötüdür.
        --
        -- `action`: 'redirect_301' | 'canonical' | 'keep' (sayfa BİLİNÇLİ tutuluyor)
        -- `source`: 'ai' | 'manual' — hedefi model mi önerdi, kullanıcı mı seçti (CSV'de görünür)
        -- `exported_at`: CSV'ye çıkmış mı — "bunu panele girdim mi?" sorusunu cevaplıyor
        CREATE TABLE IF NOT EXISTS eol_decisions (
          slug TEXT PRIMARY KEY,
          url TEXT NOT NULL,
          action TEXT NOT NULL,
          target_slug TEXT,
          target_sku TEXT,
          source TEXT NOT NULL DEFAULT 'manual',
          decided_at TEXT NOT NULL,
          exported_at TEXT,
          note TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_eol_decisions_action ON eol_decisions(action);

        -- ===== Odak seansı (Faz S) =====
        -- Seansın asıl ürünü GERÇEK SÜRE ÖLÇÜMÜ: kuyruktaki dakikalar bugüne kadar elle
        -- yazılmış tahminlerdi ("tahmin, ölçüm değil" diye işaretliydi). Ölçülen tek şey
        -- duvar saati süresi — düşünme ve düzenleme dahil, çünkü asıl bilinmeyen o.
        -- ⚠️ `work_events` bunu ölçemiyordu: ölçüldü (2026-08-08), yalnızca uç noktaları
        -- yakalıyor (meta_done → ideasoft_push farkı 0,6 dk — bu işin değil, gönderimin süresi).
        CREATE TABLE IF NOT EXISTS focus_sessions (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          started_at TEXT NOT NULL,
          ended_at TEXT,
          planned_minutes INTEGER NOT NULL,
          break_minutes INTEGER NOT NULL,
          -- 'queue_empty' | 'time_up' | 'stopped'
          ended_reason TEXT
        );

        -- ⚠️ Süre YALNIZCA outcome='done' satırlarından hesaplanıyor: atlanan iş "ne kadar
        -- sürdüğü" bilgisi taşımıyor, listeye girerse süreyi olduğundan kısa gösterir.
        -- 'abandoned' = seans yarıda kesildi (uygulama kapandı); ölçüme hiç girmez.
        CREATE TABLE IF NOT EXISTS focus_session_items (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          session_id INTEGER NOT NULL REFERENCES focus_sessions(id) ON DELETE CASCADE,
          kind TEXT NOT NULL,
          ref TEXT NOT NULL,
          bucket TEXT NOT NULL,
          started_at TEXT NOT NULL,
          ended_at TEXT,
          -- 'done' | 'skipped' | 'dismissed' | 'abandoned'
          outcome TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_focus_items_bucket
          ON focus_session_items(bucket, outcome);

        -- ===== CRM ince dilim (Faz C) =====
        -- 🔴 Bu tablolarla birlikte veritabanına İLK KEZ kişisel veri giriyor. İki sonucu var
        -- ve ikisi de bilinçli:
        --   1. Kişiler asistana AÇILMIYOR (`assistantSources.ts`) — asistan bağlamı Gemini'ye
        --      gidiyor, müşteri adı/telefonu Google'a gönderilmez.
        --   2. Yedeğe DAHİL (K2 dersi: kısmi yedek "geri yükledim ama eksik" sürprizi üretir),
        --      ama dışa aktarma ekranı kişisel veri içerdiğini söylüyor.
        --
        -- `next_step_at` fazın kalbi: yol haritasının deyimiyle "CRM'in %80'i". Tarih
        -- verildiği gün kuyruğa iş düşer, dönüş yapılınca temizlenir.
        CREATE TABLE IF NOT EXISTS contacts (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          name TEXT NOT NULL,
          company TEXT NOT NULL DEFAULT '',
          email TEXT NOT NULL DEFAULT '',
          phone TEXT NOT NULL DEFAULT '',
          -- 'mail' | 'telefon' | 'instagram' | 'fuar' | 'referans' | 'diğer' (tek değerli)
          channel TEXT NOT NULL DEFAULT '',
          note TEXT NOT NULL DEFAULT '',
          last_contact_at TEXT,
          next_step_at TEXT,
          next_step_note TEXT NOT NULL DEFAULT '',
          created_at TEXT NOT NULL,
          updated_at TEXT NOT NULL,
          -- Arşiv silme DEĞİL: geçmiş temaslar kayıt, kişi listeden çıksa da kalmalı.
          archived INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_contacts_next_step ON contacts(next_step_at)
          WHERE next_step_at IS NOT NULL;
        CREATE INDEX IF NOT EXISTS idx_contacts_email ON contacts(email);

        -- Temas geçmişi = CRM'in olay günlüğü.
        -- ⚠️ `work_events`'e AYNA SATIR YAZILMIYOR. Aynı olguyu iki tabloya yazmak bu projede
        -- üç kez ölçtüğümüz sapma tuzağı; ayrıca Faz Ö'nün günlüğü sku-anahtarlı ve
        -- `reaches_store` eksenli — bir telefon görüşmesi oraya ait değil.
        CREATE TABLE IF NOT EXISTS contact_events (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          contact_id INTEGER NOT NULL REFERENCES contacts(id) ON DELETE CASCADE,
          at TEXT NOT NULL,
          -- 'call' | 'email' | 'meeting' | 'note' | 'followup_done'
          kind TEXT NOT NULL,
          note TEXT NOT NULL DEFAULT ''
        );
        CREATE INDEX IF NOT EXISTS idx_contact_events_contact
          ON contact_events(contact_id, at);

        -- İlgi etiketleri çoklu → ayrı tablo. Virgüllü metin olsaydı filtre LIKE ile
        -- çalışırdı ve "IG" etiketi "IGNORE"u da yakalardı.
        CREATE TABLE IF NOT EXISTS contact_tags (
          contact_id INTEGER NOT NULL REFERENCES contacts(id) ON DELETE CASCADE,
          tag TEXT NOT NULL,
          PRIMARY KEY (contact_id, tag)
        );

        -- "Bu ürünle ilgilendi" — SEO tarafıyla CRM'i birbirine bağlayan tek yer.
        CREATE TABLE IF NOT EXISTS contact_products (
          contact_id INTEGER NOT NULL REFERENCES contacts(id) ON DELETE CASCADE,
          sku TEXT NOT NULL,
          at TEXT NOT NULL,
          PRIMARY KEY (contact_id, sku)
        );
        CREATE INDEX IF NOT EXISTS idx_contact_products_sku ON contact_products(sku);

        -- ===== Teklif (Faz T) =====
        -- Teklif bugüne kadar Excel'de veya mail gövdesinde hazırlanıyordu: hangi fiyatın
        -- verildiği, teklifin ne olduğu ve niye kaybedildiği hiçbir yerde kayıtlı değildi.
        --
        -- 🔑 Teklif bir **KAYIT**, canlı bir hesap tablosu değil. Bu yüzden satırlar o günkü
        -- fiyat ve maliyeti DONMUŞ hâlde taşıyor: katalog yarın değişse de teklifin ne
        -- olduğu değişmiyor. Aynı sebeple `fx_rate`/`fx_date` teklifin üstünde duruyor.
        --
        -- `status`: 'draft' | 'sent' | 'won' | 'lost' | 'expired' (geçişler quote::can_transition)
        CREATE TABLE IF NOT EXISTS quotes (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          no TEXT NOT NULL UNIQUE,
          contact_id INTEGER REFERENCES contacts(id) ON DELETE SET NULL,
          status TEXT NOT NULL DEFAULT 'draft',
          -- 'USD' | 'TRY'
          currency TEXT NOT NULL DEFAULT 'USD',
          -- USD/TRY kuru; yalnızca USD teklifte USD OLMAYAN ürün varsa gerekiyor.
          fx_rate REAL,
          fx_date TEXT,
          valid_until TEXT,
          note TEXT NOT NULL DEFAULT '',
          created_at TEXT NOT NULL,
          updated_at TEXT NOT NULL,
          sent_at TEXT,
          closed_at TEXT,
          -- Kazanma/kaybetme nedeni — raporlanabilmesi fazın bitiş şartı.
          close_reason TEXT NOT NULL DEFAULT ''
        );
        CREATE INDEX IF NOT EXISTS idx_quotes_contact ON quotes(contact_id);
        CREATE INDEX IF NOT EXISTS idx_quotes_status ON quotes(status, updated_at);

        -- 🔴 `cost` MÜŞTERİYE GİDEN ÇIKTIYA ASLA GİRMEZ. Burada duruyor çünkü marj onsuz
        -- hesaplanamaz ve "bu işi ne marjla aldık?" sorusunun cevabı O GÜNKÜ maliyet olmalı.
        -- Sızıntı bir dikkat meselesi değil: çıktı üreten kod ayrı bir yapı alıyor ve o
        -- yapıda maliyet ALANI YOK (bkz. quote_html).
        --
        -- `sku` NULL = elle satır (montaj, nakliye). `unit_price` ve `cost` teklifin para
        -- biriminde — çevrim satır eklenirken bir kez yapıldı (quote::catalog_line).
        CREATE TABLE IF NOT EXISTS quote_items (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          quote_id INTEGER NOT NULL REFERENCES quotes(id) ON DELETE CASCADE,
          sku TEXT,
          name TEXT NOT NULL,
          qty REAL NOT NULL DEFAULT 1,
          unit_price REAL NOT NULL DEFAULT 0,
          tax_rate REAL NOT NULL DEFAULT 0,
          cost REAL,
          sort INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_quote_items_quote ON quote_items(quote_id, sort);

        -- Gönderilmiş teklif düzenlenirse önce anlık görüntüsü saklanıyor → v1/v2 fiyat
        -- geçmişi. ⚠️ Müşteriye giden belge değişti mi, sonradan bunu kanıtlayabilmek gerek.
        CREATE TABLE IF NOT EXISTS quote_versions (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          quote_id INTEGER NOT NULL REFERENCES quotes(id) ON DELETE CASCADE,
          version INTEGER NOT NULL,
          snapshot_json TEXT NOT NULL,
          at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_quote_versions_quote ON quote_versions(quote_id, version);

        -- ===== Gemini kullanım kaydı (Faz G) =====
        -- Ücretsiz katmanda flash modellerin günlük limiti 20; kullanıcı bunu ancak üretim
        -- alt modele düştüğünde fark ediyordu. Kapasitenin görünmesi için her istek buraya.
        --
        -- ⚠️ Burada sayılan tek şey **bu uygulamanın gönderdikleri**. Aynı API anahtarı başka
        -- bir yerde kullanılırsa sayı eksik kalır — bu yüzden ekranda "kalan hak" YAZILMIYOR,
        -- yalnızca yapılan istek gösteriliyor (kullanıcı kararı, 2026-08-12: eksik bir sayıyı
        -- "20 hakkın var, 14 kullandın" diye sunmak yanlış güven verir).
        --
        -- `run_id`: kullanıcının başlattığı TEK üretim. Zincir üç modele düşerse üç satır
        -- oluşur, üçü de aynı `run_id`. "Üretim başına kaç istek harcanıyor" ancak böyle
        -- hesaplanır — bir üretimin bir istek olduğunu varsaymak sayıyı düşük gösterir.
        -- `http_code`: 0 = istek hiç gitmedi (ağ hatası). 429 ayrı sayılıyor; "limit gerçekten
        -- darboğaz mı?" sorusunun cevabı toplam istek değil, kotaya çarpma sayısıdır.
        CREATE TABLE IF NOT EXISTS gemini_calls (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          at TEXT NOT NULL,
          model TEXT NOT NULL,
          -- 'meta' | 'details' | 'tech' | 'successor' | 'chat' | 'probe'
          kind TEXT NOT NULL,
          run_id TEXT NOT NULL,
          ok INTEGER NOT NULL,
          http_code INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_gemini_calls_at ON gemini_calls(at);

        -- ===== Sorgu × sayfa satırları (Faz İ) =====
        -- 🔴 Bu tablonun varlık sebebi iki kat: (1) sorgu düzeyi veri bugüne kadar HİÇ
        -- saklanmıyordu — her analizde GSC'den çekilip rapora girmeyen kısmı çöpe gidiyordu;
        -- (2) "hangi sorguya hangi sayfa sıralanıyor" bilgisi içerik açığı analizinin tek
        -- dayanağı ve rapor JSON'una konamaz (0b6: ölçek sınırı zaten o blob).
        --
        -- ⚠️ Eşik UYDURULMADI, ölçüldü (2026-08-12, 90 gün, filtresiz): 30.190 satırın
        -- `metrics::kept` ile 5.226'sı tutuluyor (%17,3) ve bu **tıklamaların %100'ünü**,
        -- gösterimlerin %79,9'unu kapsıyor. Sayfa satırlarıyla AYNI kural — iki ayrı eşik
        -- olsaydı hangi veriye hangi kuralın uygulandığı zamanla karışırdı.
        --
        -- Bölüm kırılımı (aynı ölçüm), yani kapatılan kör nokta: ürün 23.914 satır /
        -- 169.944 gösterim · blog 2.467 / 54.519 · kategori 2.211 / 29.682 · marka 1.187 /
        -- 10.338 · anasayfa+diğer 411 / 6.579. Ürün dışı toplam **5.700 sorgu, 101.118
        -- gösterim** — uygulamanın bugüne kadar hiç görmediği kısım.
        CREATE TABLE IF NOT EXISTS query_rows (
          snapshot_id INTEGER NOT NULL REFERENCES metric_snapshots(id) ON DELETE CASCADE,
          page TEXT NOT NULL,
          query TEXT NOT NULL,
          clicks REAL NOT NULL DEFAULT 0,
          impressions REAL NOT NULL DEFAULT 0,
          position REAL NOT NULL DEFAULT 0,
          PRIMARY KEY (snapshot_id, page, query)
        );
        CREATE INDEX IF NOT EXISTS idx_query_rows_query ON query_rows(query);

        -- ===== Mağazanın ürün-dışı sayfa envanteri (Faz İ) =====
        -- 🔴 İki işi birden yapıyor ve ikisi de ölçümle ortaya çıktı:
        --
        -- 1. **Sayfa tipi sınıflandırması.** Bir URL'nin kategori mi blog mu olduğu, yol
        --    desenine BAKILARAK tahmin edilebilir ama envantere sorulunca ÖLÇÜLÜR. İlk tasarım
        --    kullanıcıya segment etiketletiyordu; IdeaSoft uçlarının okunabildiği görülünce
        --    (blogs · categories · brands, hepsi 200) tahmine gerek kalmadı.
        --    ⚠️ Segment tabanlı yol yine de duruyor (`page_kind`): IdeaSoft modülü OPSİYONEL,
        --    kullanmayan kullanıcı bu özellikten mahrum kalmamalı.
        --
        -- 2. **Meta optimizasyonu.** Ölçüldü (2026-08-12): görünen 47 kategorinin 26'sında üst
        --    açıklama metni yok, 75 marka sayfasının 24'ünde özel başlık hiç yazılmamış,
        --    158 blog yazısının 147'sinde başlık yazının aynısı. Alan ailesi ürünlerle aynı
        --    (`pageTitle` · `metaDescription` · `metaKeywords` · `targetKeyword`), yani ürün
        --    meta hattı bu kayıtlara da uygulanabilir.
        --
        -- `kind`: 'category' | 'brand' | 'blog' — IdeaSoft'taki uç adına karşılık gelir.
        -- `remote_id` + `kind` birlikte tekil: üç uçta kimlikler çakışabilir.
        -- ⚠️ `showcase_content` yalnızca kategori ve markada var, blogda yok → NULL olabilir.
        CREATE TABLE IF NOT EXISTS store_pages (
          kind TEXT NOT NULL,
          remote_id INTEGER NOT NULL,
          slug TEXT NOT NULL,
          name TEXT NOT NULL,
          page_title TEXT NOT NULL DEFAULT '',
          meta_description TEXT NOT NULL DEFAULT '',
          meta_keywords TEXT NOT NULL DEFAULT '',
          target_keyword TEXT NOT NULL DEFAULT '',
          showcase_content TEXT,
          status INTEGER NOT NULL DEFAULT 1,
          fetched_at TEXT NOT NULL,
          PRIMARY KEY (kind, remote_id)
        );
        CREATE INDEX IF NOT EXISTS idx_store_pages_slug ON store_pages(slug);
        "#,
    )
    .map_err(|e| format!("Şema oluşturulamadı: {e}"))?;

    migrate(conn)?;
    Ok(())
}

/// Eski DB'lere sonradan eklenen kolonları idempotent şekilde ekler.
fn migrate(conn: &Connection) -> Result<(), String> {
    // Faz K sonrası: "Yapıldı" işareti hangi analize karşı verildi. ⚠️ CREATE TABLE ile
    // eklenemez — tablo v0.11.0 sonrası kurulumlarda zaten var, `IF NOT EXISTS` sütun eklemez.
    // 🔴 Onarım (2026-08-12): `contact_products.sku` sütununa SKU yerine **slug** yazılmış
    // satırlar. Sebep: yerel ürün araması SKU'yu `canonical` alanında saklıyordu, ekran ise
    // `slug`ı SKU sanıyordu. Hata sessizdi — satır ekleniyor ama `products` ile eşleşmiyor,
    // ürün detayındaki "bu ürünle ilgilenenler" hiç dolmuyordu.
    //
    // Slug, ürünün adresinin son parçası; eşleşen ürün varsa satır düzeltiliyor. Eşleşme
    // yoksa DOKUNULMUYOR: veriyi silmektense bozuk bırakmak yeğdir, kullanıcı görüp karar
    // verebilir. Idempotent — zaten doğru olan satırlara değmiyor.
    let _ = conn.execute(
        "UPDATE contact_products SET sku = (
             SELECT p.sku FROM products p
             WHERE lower(p.url) LIKE '%/' || lower(contact_products.sku)
                OR lower(p.url) LIKE '%/' || lower(contact_products.sku) || '/'
             LIMIT 1)
         WHERE sku NOT IN (SELECT sku FROM products)
           AND EXISTS (
             SELECT 1 FROM products p
             WHERE lower(p.url) LIKE '%/' || lower(contact_products.sku)
                OR lower(p.url) LIKE '%/' || lower(contact_products.sku) || '/')",
        [],
    );

    // Faz T: katalog fiyatları. ⚠️ `feed_fp` parmak izine GİRMİYOR — fiyat üretimi
    // beslemiyor ve dolar günlük oynuyor (fingerprint.rs'in `quantity` gerekçesinin aynısı).
    add_column_if_missing(conn, "products", "buying_price", "REAL")?;
    add_column_if_missing(conn, "products", "price1", "REAL")?;
    add_column_if_missing(conn, "products", "tax_rate", "REAL")?;
    // 🔴 Katalog tek para biriminde DEĞİL (ölçüldü: USD 273 · EUR 8 · TL 1).
    add_column_if_missing(conn, "products", "currency_abbr", "TEXT")?;
    // Mağazanın kendi TL fiyatı (KDV dahil) — TL teklifte kur sormaya gerek bırakmıyor.
    add_column_if_missing(conn, "products", "price_tl", "REAL")?;
    add_column_if_missing(conn, "queue_dismissals", "done_at_analysis", "TEXT")?;
    add_column_if_missing(conn, "seo_status", "draft_details", "TEXT")?;
    // Faz 4: SEO araştırma çıktısı (SeoInsights JSON) ürün başına saklanır.
    add_column_if_missing(conn, "seo_status", "research_json", "TEXT")?;
    // Faz 7: galeri görsel slotları + boyut kontrolü cache'i.
    add_column_if_missing(conn, "products", "picture2", "TEXT")?;
    add_column_if_missing(conn, "products", "picture3", "TEXT")?;
    add_column_if_missing(conn, "products", "picture4", "TEXT")?;
    add_column_if_missing(conn, "seo_status", "image_check_json", "TEXT")?;
    add_column_if_missing(conn, "seo_status", "image_check_fp", "TEXT")?;
    // Faz 8: teknik özellik tablosu. Bu veri feed'de YOK → tek kaynağı uygulama, yedeklemede korunmalı.
    add_column_if_missing(conn, "seo_status", "tech_source_text", "TEXT")?;
    add_column_if_missing(conn, "seo_status", "tech_specs_json", "TEXT")?;
    add_column_if_missing(conn, "seo_status", "tech_status", "TEXT DEFAULT 'pending'")?;
    // Faz 8b: önceki teknik tablo sürümleri (yeniden üretim öncesi anlık görüntü).
    add_column_if_missing(conn, "seo_status", "tech_history_json", "TEXT")?;
    // Faz 9: IdeaSoft gönderim modülü (id cache + son gönderim zamanı).
    add_column_if_missing(conn, "seo_status", "ideasoft_product_id", "INTEGER")?;
    add_column_if_missing(conn, "seo_status", "ideasoft_pushed_at", "TEXT")?;
    add_column_if_missing(conn, "seo_status", "ideasoft_seo_rule", "INTEGER")?;
    // Hangi Gemini modeli üretti. Zincir kotaya takıldıkça alt modellere düştüğü için
    // kullanıcının bunu görmesi gerekiyor: içerik son çare modeliyle üretildiyse limitler
    // yenilendiğinde yeniden üretmek isteyebilir. Üç üretim türü ayrı izlenir.
    add_column_if_missing(conn, "seo_status", "meta_model", "TEXT")?;
    add_column_if_missing(conn, "seo_status", "details_model", "TEXT")?;
    add_column_if_missing(conn, "seo_status", "tech_model", "TEXT")?;
    // IdeaSoft katalog önbelleği. XML feed bilinçli olarak sınırlı (bu mağazada 10.909
    // üründen 262'si); feed dışı sayfalar Google'dan ciddi trafik alıyor ve uygulama
    // onları hiç göremiyordu. Slug ile eşleştirme yapılabilsin diye ayrı tabloda tutulur.
    conn.execute(
        "CREATE TABLE IF NOT EXISTS ideasoft_catalog (
            slug TEXT PRIMARY KEY,
            id INTEGER NOT NULL,
            name TEXT NOT NULL,
            status INTEGER NOT NULL,
            stock REAL NOT NULL,
            canonical TEXT,
            synced_at TEXT NOT NULL
        )",
        [],
    )
    .map_err(|e| format!("ideasoft_catalog tablosu oluşturulamadı: {e}"))?;
    // AI Asistanı sohbet geçmişi. Kullanıcı geri bildirimi (2026-07-30): uygulama kapanınca
    // sohbet kayboluyordu; meta/açıklama/teknik tabloda olduğu gibi geçmişe erişilebilmeli.
    // ⚠️ Mesajlar JSON sütunda: projenin mevcut geçmiş idiomu bu (`*_history_json`) ve
    // sohbet her zaman bir bütün olarak okunup yazılıyor — satır bazlı sorgu ihtiyacı yok.
    conn.execute(
        "CREATE TABLE IF NOT EXISTS chat_sessions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            tool_page TEXT,
            messages_json TEXT NOT NULL,
            model TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )",
        [],
    )
    .map_err(|e| format!("chat_sessions tablosu oluşturulamadı: {e}"))?;
    // Feed değişikliği tespiti (bkz. core/src/fingerprint.rs).
    // `feed_fp`      → ürünün ŞU ANKİ parmak izi, her senkronda güncellenir (sahibi: senkron).
    // `feed_changed` → son değişiklikte hangi alanların oynadığı, kullanıcıya gösterilir.
    // `reviewed_fp`  → kullanıcı "tamamlandı" işaretlediği andaki iz (sahibi: kullanıcı eylemi).
    // İkisi ayrışırsa ürün "feed verisi değişti, gözden geçir" olarak işaretlenir.
    add_column_if_missing(conn, "products", "feed_fp", "TEXT")?;
    add_column_if_missing(conn, "products", "feed_changed", "TEXT")?;
    add_column_if_missing(conn, "seo_status", "reviewed_fp", "TEXT")?;
    // "Ne değişti?" sorusunun cevabı: onay anındaki alan değerleri (bkz. fingerprint::FeedFacts).
    add_column_if_missing(conn, "seo_status", "reviewed_facts_json", "TEXT")?;
    // Sürüm geçmişi: yeniden üretmeden önceki hâl saklanır (bkz. core/src/history.rs).
    // Teknik tabloda zaten vardı; meta ve açıklamada içerik geri dönüşsüz kayboluyordu.
    add_column_if_missing(conn, "seo_status", "meta_history_json", "TEXT")?;
    add_column_if_missing(conn, "seo_status", "details_history_json", "TEXT")?;
    Ok(())
}

fn add_column_if_missing(
    conn: &Connection,
    table: &str,
    column: &str,
    col_type: &str,
) -> Result<(), String> {
    let exists: bool = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .and_then(|mut stmt| {
            let names: Vec<String> = stmt
                .query_map([], |row| row.get::<_, String>(1))?
                .filter_map(Result::ok)
                .collect();
            Ok(names.iter().any(|n| n == column))
        })
        .map_err(|e| format!("{table} kolonları okunamadı: {e}"))?;
    if !exists {
        conn.execute(
            &format!("ALTER TABLE {table} ADD COLUMN {column} {col_type}"),
            [],
        )
        .map_err(|e| format!("{table}.{column} eklenemedi: {e}"))?;
    }
    Ok(())
}

pub fn get_setting(conn: &Connection, key: &str) -> Result<Option<String>, String> {
    conn.query_row(
        "SELECT value FROM settings WHERE key = ?1",
        [key],
        |row| row.get::<_, String>(0),
    )
    .map(Some)
    .or_else(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => Ok(None),
        other => Err(format!("Ayar okunamadı: {other}")),
    })
}

pub fn set_setting(conn: &Connection, key: &str, value: &str) -> Result<(), String> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [key, value],
    )
    .map_err(|e| format!("Ayar kaydedilemedi: {e}"))?;
    Ok(())
}

/// Kayıtlı feed adresi; **ayarlanmamışsa boş string**.
///
/// 🔴 Burada eskiden `DEFAULT_FEED_URL` sabiti vardı ve tek bir mağazanın (kurumsalit.com)
/// feed adresine düşüyordu. Yani uygulamayı kuran **herhangi bir işletme** varsayılan olarak
/// başka birinin kataloğunu senkronluyordu — vizyonun açık ihlali (*"kullanıcıya özel değer
/// gömmeden, ayarlanabilir olmalı"*) ve ilk çalıştırmada somut bir veri kazası.
///
/// Boş dönmesi bilinçli: çağıranlar "ayarlanmamış" durumunu görüp kullanıcıya ne yapması
/// gerektiğini söyleyebilsin (bkz. `sync_feed`).
pub fn feed_url(conn: &Connection) -> Result<String, String> {
    Ok(get_setting(conn, "feed_url")?.unwrap_or_default())
}

/// Kurulum sihirbazı gösterilmeli mi?
///
/// **Üç koşul birden** aranıyor — çünkü asıl risk yanlış pozitif: mevcut bir kullanıcı
/// yükseltme yaptığında sihirbazın açılması, çalışan bir kurulumun üstüne kurulum teklif
/// etmek olurdu.
///
/// Durum baştan hesaplanıyor; şema göçü ve tek seferlik geriye dönük yazma (backfill) YOK.
pub fn needs_setup(conn: &Connection) -> Result<bool, String> {
    if get_setting(conn, "setup_done")?.is_some() {
        return Ok(false);
    }
    if get_setting(conn, "feed_url")?.is_some_and(|v| !v.trim().is_empty()) {
        return Ok(false);
    }
    let products: i64 = conn
        .query_row("SELECT count(*) FROM products", [], |r| r.get(0))
        .map_err(|e| format!("Ürün sayısı okunamadı: {e}"))?;
    Ok(products == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init(&conn).unwrap();
        conn
    }

    #[test]
    fn taze_kurulumda_sihirbaz_gerekir() {
        let conn = fresh();
        assert!(needs_setup(&conn).unwrap());
        // Feed adresi ayarlanmamışsa BOŞ döner — eskiden tek bir mağazanın adresine düşüyordu.
        assert_eq!(feed_url(&conn).unwrap(), "");
    }

    /// ⚠️ Asıl korunan risk yanlış pozitif: çalışan bir kuruluma sihirbaz teklif etmemek.
    /// Üç çıkış yolunun üçü de ayrı ayrı sınanıyor.
    #[test]
    fn mevcut_kurulumda_sihirbaz_acilmaz() {
        // 1) Sihirbaz daha önce tamamlanmış
        let a = fresh();
        set_setting(&a, "setup_done", "2026-07-31T10:00").unwrap();
        assert!(!needs_setup(&a).unwrap());

        // 2) Feed adresi girilmiş (sihirbaz atlanıp Ayarlar'dan yapılandırılmış olabilir)
        let b = fresh();
        set_setting(&b, "feed_url", "https://magaza.com/feed.xml").unwrap();
        assert!(!needs_setup(&b).unwrap());

        // 3) Katalog senkronlanmış — kullanıcı zaten çalışıyor
        let c = fresh();
        c.execute("INSERT INTO products (sku, name) VALUES ('A1', 'Ürün')", []).unwrap();
        assert!(!needs_setup(&c).unwrap());
    }

    #[test]
    fn bos_feed_ayari_sihirbazi_engellemez() {
        // Boş string "ayarlanmış" sayılmamalı; aksi halde yarım kalan bir kayıt
        // sihirbazı sonsuza dek kapatırdı.
        let conn = fresh();
        set_setting(&conn, "feed_url", "   ").unwrap();
        assert!(needs_setup(&conn).unwrap());
    }

    /// Slug yazılmış kişi-ürün bağları onarılıyor mu?
    ///
    /// ⚠️ Bu testin varlık sebebi: hatanın kendisi **sessizdi** — satır yazılıyor, hiçbir
    /// uyarı çıkmıyor, yalnızca ürün detayındaki liste hiç dolmuyordu. Onarım da sessiz;
    /// sınanmazsa çalışıp çalışmadığını kimse fark etmez.
    #[test]
    fn slug_yazilmis_urun_baglari_onariliyor() {
        let conn = fresh();
        conn.execute(
            "INSERT INTO products (sku, name, url) VALUES
               ('ABC.123', 'Yazıcı', 'https://magaza.com/urun/hizli-yazici'),
               ('DEF.456', 'Tarayıcı', 'https://magaza.com/urun/tarayici')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO contacts (name, created_at, updated_at)
             VALUES ('Ali', '2026-08-12', '2026-08-12')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO contact_products (contact_id, sku, at) VALUES
               (1, 'hizli-yazici', '2026-08-12'),
               (1, 'DEF.456', '2026-08-12'),
               (1, 'artik-satmadigimiz-urun', '2026-08-12')",
            [],
        )
        .unwrap();

        init(&conn).unwrap(); // açılışta göç yeniden koşuyor

        let mut st = conn.prepare("SELECT sku FROM contact_products ORDER BY sku").unwrap();
        let skus: Vec<String> =
            st.query_map([], |r| r.get(0)).unwrap().map(|x| x.unwrap()).collect();
        assert_eq!(
            skus,
            vec![
                "ABC.123".to_string(),               // slug → SKU'ya çevrildi
                "DEF.456".to_string(),               // zaten doğruydu, dokunulmadı
                "artik-satmadigimiz-urun".to_string(), // eşleşme yok: SİLİNMEDİ, duruyor
            ]
        );
    }
}

/// Ürünün üretimi besleyen alanlarını `products` satırından okur.
///
/// Tek yerde duruyor çünkü iki farklı yol aynı alan kümesini okuyor: senkron (parmak izi
/// hesabı) ve onay damgası (karşılaştırma kaydı). İkisi ayrışırsa "değişti" bayrağı ile
/// gösterilen fark birbirini tutmaz.
pub fn read_feed_facts(conn: &Connection, sku: &str) -> Option<FeedFacts> {
    conn.query_row(
        "SELECT name, brand, main_category, category, details,
                img_url, picture2, picture3, picture4
         FROM products WHERE sku = ?1",
        [sku],
        |r| {
            let g = |i: usize| -> rusqlite::Result<String> {
                Ok(r.get::<_, Option<String>>(i)?.unwrap_or_default())
            };
            Ok(FeedFacts {
                name: g(0)?,
                brand: g(1)?,
                main_category: g(2)?,
                category: g(3)?,
                details: g(4)?,
                images: vec![g(5)?, g(6)?, g(7)?, g(8)?],
            })
        },
    )
    .ok()
}

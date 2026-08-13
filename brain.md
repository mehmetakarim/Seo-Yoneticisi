# 🧠 brain.md — SEO Yöneticisi Proje Beyni

> Bu dosya projenin kalıcı hafızasıdır. Oturum (session) değişse bile buraya bakarak
> nerede kaldığımızı anlar ve devam ederiz. **Her anlamlı ilerlemede güncelle.**

**Son güncelleme:** 2026-08-13 (v0.19.0 — Faz İ)
**Aktif faz:** **v0.17.0 yayında — yol haritasının SEKİZ fazı da bitti.** Sıra:
**B** (ortak tablo, v0.9.0) · **Ö** (ölçüm omurgası, v0.10.0) · **A** (asistan bağlam seçimi,
v0.11.0) · **K** (Bugün + iş kuyruğu, v0.12.0) · **S** (odak seansı, v0.13.0) ·
**D** (EOL karar deposu + 301 CSV + sağlık skoru, v0.14.0) · **C** (CRM ince dilim, v0.15.0).
v0.14.1 = kuyruk uçuş süzgeci (0b1) ·
v0.15.0 = Faz C: CRM ince dilim + kuyruk saha düzeltmeleri (0b2, 0b3) ·
v0.16.0 = Faz T1: teklif çekirdeği (katalog fiyatları + teklif ekranı) ·
v0.17.0 = Faz T2: teklif belgesi, takip önerisi, kazanma/kaybetme raporu ·
v0.17.1 = PDF açma, sütun hizası, satır geri alma ·
v0.17.2 = ödünç alınmış alan: slug/SKU karışıklığı (0b5) ·
v0.18.0 = Faz G: model zinciri ayarlara + Gemini kullanım sayacı (0b7) ·
v0.19.0 = Faz İ: içerik açığı + mağaza sayfası iyileştirme (0b8, 0b9, 0c0) ·
v0.19.1 = içerik gönderimlerinin sonucu ölçülebilir oldu (0c1) ·
v0.19.2 = sessiz şema uyuşmazlığı: sorgu satırları hiç yazılmıyordu (0c2) ·
**v0.19.3 = paragraflar sayfada birleşiyordu (0c3)**
**Yön ve fazlar `yol-haritasi.md`'de** (2026-08-07). Burası **ne olduğunun** kaydı,
orası **nereye gittiğimizin**. ⚠️ Faz tanımları burada ÇOĞALTILMAZ; ölçüm sonuçları da yol
haritasına yazılmaz — aynı bilgi iki yerde durursa zamanla ayrışır.
**Sıradaki faz: P (çoklu mağaza)** — ikinci mağaza gelince. Serbest kalemler: G.
**Repo:** https://github.com/mehmetakarim/Seo-Yoneticisi (main) · **PUBLIC** (2026-07-26'dan beri)
**Yayınlanan sürümler:** v0.1.0 → v0.5.2 · v0.5.3 = Gemini 404 düzeltmesi ·
v0.5.4 = zincir + model rozeti · v0.5.5 = rozet kart başlığına ·
v0.5.6 = Fırsatlar sayfası · v0.5.7 = meta/açıklama sürüm geçmişi ·
v0.5.8 = EOL sayfalar + filtreler · v0.5.9 = boş ekran düzeltmesi ·
v0.6.0 = sorgu düzeyi analizler · v0.6.1 = EOL halef önerisi ·
v0.6.2 = Ahrefs zorluk düzeltmesi · v0.6.3 = düşüşte olanlar (trend) ·
v0.6.4 = canonical yazma akışı · v0.6.5 = canonical senkron ön koşulu kaldırıldı ·
v0.7.0 = araç ekranları + gruplu navigasyon + Yapay Zekâ Asistanı ·
v0.7.1 = kenar çubuğu etiketi ·
v0.7.2 = sohbet geçmişi kalıcı + canonical hedefi satıştaki ürünler ·
v0.8.0 = ilk kurulum sihirbazı + varsayılan feed adresi kaldırıldı ·
⚠️ **v0.8.1 HİÇ YAYINLANMADI** — notları v0.8.2'ye taşındı (gerekçe aşağıda, "Yayın") ·
v0.8.2 = feed değişikliği tespiti + Schema.org JSON-LD ·
v0.8.3 = "Neler değişti?" karşılaştırması + IdeaSoft'tan teknik tablo getirme ·
v0.9.0 = Faz B: araç ekranlarında ortak tablo + İşlem sütunu ·
v0.10.0 = Faz Ö: ölçüm omurgası (GSC geçmişi + olay günlüğü + sonuç rozetleri) ·
v0.11.0 = Faz A: asistan bağlam seçimi ("+" menüsü) + halef akışı düzeltmeleri ·
v0.12.0 = Faz K: Bugün ekranı + iş kuyruğu (tasarım turu, "Yapıldı", koyu pencere) ·
v0.13.0 = Faz S: odak seansı — kuyruğu ölçerek tüketmek (süreler artık ölçülüyor) ·
v0.14.0 = Faz D: EOL karar deposu + 301 CSV + ürün sağlık skoru ·
**v0.14.1 = kuyruk uçuş süzgeci (yapılan iş geri gelmiyor)

**Yapı (2026-07-28'den beri workspace):**
`src-tauri/Cargo.toml` hem paket hem workspace kökü → `src-tauri/core/` (saf mantık, Tauri'ye
bağımlı DEĞİL, **243 test** + 32 canlı `--ignored`) + `src-tauri/src/` (ince Tauri katmanı,
**46 test**; `commands/` 11 dosyaya bölünmüş durumda).
İş döngüsü: `cargo test -p seo-core` ≈ 60 sn soğuk / 17 sn sıcak — Tauri hiç derlenmiyor.

## ⏭️ KALDIĞIMIZ YER (yeni oturum buradan devam etsin)

### 🧭 Durum (2026-08-13)

**v0.19.0 hazır — Faz İ (içerik açığı) bitti.** Öncesi: v0.18.0 yayında ve doğrulandı.
Sekiz fazın hepsi bitti —
**B** (v0.9.0) · **Ö** (v0.10.0) · **A** (v0.11.0) · **K** (v0.12.0) · **S** (v0.13.0) ·
**D** (v0.14.0) · **C** (v0.15.0) · **T** (v0.16.0 + v0.17.0) — ve **Faz G** kalemlerinin
kod gerektirenleri de kapandı (v0.18.0). **Faz İ yol haritasında YOKTU** — GSC dışa
aktarımlarından çıkan ölçümle doğdu (0b8): uygulama ürün dışı 5.700 sorgu / 101.118
gösterimi hiç görmüyordu.
Ara yayınlar: v0.14.1 kuyruk uçuş süzgeci · v0.17.1 PDF/hiza/geri alma · v0.17.2 slug-SKU.

**Sıradaki iş — içerik pilotu (karar bekliyor).** Faz İ'nin makinesi hazır ve v0.19.1 ile
sonucu **ölçülebilir** hâle geldi (0c1: v0.19.0'da ölçülemiyordu). Kullanıcı zaten bir
kategori sayfası göndermiş (**Access Point, 2026-08-13**) — pilot fiilen başladı; hangi
sayfalarla devam edileceği kullanıcı kararı. Önerilen devam: TO'nun kaldıraç olduğu üç
sayfa daha (Güvenlik Duvarı · Stand ve Dock · Video Konferans), poz 40'takilere girmeden.
📅 İlk sonucun okunabileceği gün **~2026-10-04** (hesap aşağıda, 3. maddede). Ölçülen adaylar: TO'nun kaldıraç olduğu **dört kategori**
(Güvenlik Duvarı 4.831 gösterim/poz 9,1 · Access Point 1.274/15,7 · Stand ve Dock 1.084/13,8 ·
Video Konferans 849/10,8) ya da 26 kategorinin tamamı (poz 40'takiler dahil — orada metin
TO'yu değil sıralamayı hedefler, iki mekanizma ölçüme karışır).
⚠️ Marka fırsatı ÇÜRÜDÜ: eksik alanı olan 25 marka sayfasının 34.445 gösteriminin 32.845'i
TeamViewer, o da navigasyonel. Kalan 24 sayfa 1.600 gösterim taşıyor.

**Bunun dışında üç şey; ikisi bize bağlı değil, biri takvimde:**

1. **Faz P (çoklu mağaza)** — ⏸️ **başlama koşulu henüz oluşmadı.** İkinci mağaza gerçekten
   gelmeden ya da genel dağıtımdan önce başlanmıyor; tek `db_path` varsayımı kodun birçok
   yerinde ve erken refactor bedava değil. Karar yol haritasında, burada çoğaltılmıyor.

2. **Faz G** — ✅ **bitti.** `MODEL_CHAIN` → ayarlar + Gemini kullanım sayacı (0b7, v0.18.0) ·
   ölçek modellemesi (0b6, sonucu: eşikler ayarlanabilir olmayacak).
   ⏸️ Geriye yalnızca **kod imzalama** kaldı: kod işi değil, ~$99/yıl **maliyet kararı** —
   kullanıcıda.
   ❌ i18n elendi: gerekçesi çürüdü (kullanıcı düzeltmesi — "global dağıtım" = IdeaSoft
   kullanan herhangi bir işletme, farklı dilde bir pazar değil).

3. 📅 **EKİM BAŞI — sonuç eşiği kalibrasyonu.**
   **Kullanıcı kararı (2026-08-13): Eylül sonundan EKİM BAŞINA alındı.** Gerekçe ölçüldü:
   ilk içerik gönderiminin sonucu **~4 Ekim'den önce okunamıyor** ve kalibrasyonda o
   sonucun da masada olması isteniyor.

   🔴 **Bu tarih hesaplandı, tahmin edilmedi** — ve önceki tahminim ("~10 Eylül") yanlıştı.
   Access Point gönderimi 2026-08-13. `MIN_WAIT_DAYS = 21` → takip penceresi en erken
   2026-09-03'te BAŞLAYABİLİR. Pencereler 28 günlük döşeniyor (`next_window`) ve mevcut son
   pencere 2026-08-04'te bitiyor, yani sıradaki döşemeler: 08-05→09-02 (takip sayılmaz,
   21 günü doldurmuyor) · **09-03→10-01 (ilk geçerli takip)**. `GSC_LAG_DAYS = 3` yüzünden
   o pencere ancak **2026-10-04'te** yakalanabiliyor.
   *Ders: "21 gün sonra ölçülür" cümlesi yanlış — pencere DÖŞEMESİ ve GSC gecikmesi
   araya giriyor. Tarih söylemeden önce hesaplanmalı.*

   ⚠️ Ürün gönderimleri için veri daha erken hazır (72 gönderim 26 Temmuz–7 Ağustos, pencere
   Eylül ortasında oluşuyor — 0au). Yani Ekim başını bekleten şey ürün tarafı değil, içerik
   tarafı. İstenirse ürün eşikleri Eylül ortasında ayrı kalibre edilebilir; kullanıcı ikisini
   birlikte yapmayı seçti.

   ⚠️ Faz İ ile birlikte ölçülecek şey arttı: kategori/marka/blog düzeltmeleri de
   `work_events`'e `store_page_push` (`reaches_store = 1`) olarak yazılıyor ve v0.19.1'den
   itibaren gerçekten okunuyor (0c1). Kalibrasyonda bu olaylar da dağılıma girecek.

   Yapılacak: iyileşti/geriledi eşikleri bugün **gerekçeli varsayımla** duruyor; gerçek delta
   dağılımına bakılıp yeniden belirlenecek. Bugünkü kural, iki koşul birden (≥%20 VE
   ≥3 tıklama) — gerekçesi 0au'da, kalibrasyonda o gerekçe de sınanacak.
   O güne kadar sonuç kontrolü kovasının boş kalması **kusur değil**; ekran tarihi söylüyor.

Fazların **ne olduğu** aşağıdaki numaralı girdilerde (B → 0at, Ö → 0au, A → 0av, halef
düzeltmeleri → 0aw, K → 0ax, pencere teması → 0ay, S → 0az, D → 0b0, C → 0b2, T → 0b4,
ölçek modellemesi → 0b6, model zinciri + sayaç → 0b7, Faz İ ön ölçümleri → 0b8,
12ay/90gün çöküşü → 0b9, Faz İ inşası → 0c0);
**nereye gittiğimiz** `yol-haritasi.md`'de.

### 📋 Eski kuyruk — kullanıcı önceliğiyle (2026-07-31), tamamı bitti

Sıra **kullanıcı kararı**, tahmini efor değil. **Kuyruk boş — üç kalem de bitti.**

**K1. İlk kurulum sihirbazı** ✅ **TAMAMLANDI (v0.8.0)** — ayrıntı ve çıkan kusur 0ao/0ap'de.

**K2. Feed değişikliği tespiti** ✅ **TAMAMLANDI** — kod v0.8.1'de yazıldı ama o sürüm hiç
   yayınlanmadı, kullanıcıya **v0.8.2** ile ulaştı. Karşılaştırma ("Neler değişti?") v0.8.3'te
   eklendi. Ayrıntı 0aq'da.

**K3. Schema.org JSON-LD çıktısı** ✅ **TAMAMLANDI (v0.8.2)** — ayrıntı 0ar'de.
   ⚠️ Kuyruk tanımında `offers` da vardı; **ölçüm bunu çürüttü** — feed'de fiyat alanı yok,
   mağaza fiyatı zaten canlı basıyor. Gerekçe 0ar'de.

### ✅ Kuyruk sonrası — saha testinden gelen işler (v0.8.3, 2026-08-07)

Üçü de kullanıcının uygulamayı gerçek veriyle kullanırken bulduğu şeyler; ikisi kusur
değil **eksik**ti, biri düpedüz tasarım hatasıydı.

1. **"İçerik hâlâ doğru" diyecektim ama neye bakarak?"** → onay anındaki değerler saklanıp
   karşılaştırma eklendi (0aq'nun sonu). *Karar vermesi istenen kişiye kararın dayanağı da
   verilmeli.*
2. **Uyarı şeridinin boşluğu** → üstte 0, altta 16px ölçüldü. *"Boşluk da bir özelliktir ve
   uygulama genelinde bu tip hatalar yer almamalı"* (kullanıcı).
3. **Teknik tablo IdeaSoft'ta duruyordu, uygulama göremiyordu** → 0as. Ölçüm: satıştaki
   ürünlerin **%94'ünde** veri varmış.

### 🔍 Eski özellik listesinin denetimi (2026-07-31, kodda doğrulandı)

Daha eski bir öneri listesindeki 9 maddenin durumu — **bu soru tekrar sorulmasın diye**:

| Madde | Durum |
|---|---|
| overall_status'a görsel + teknik | ✅ `validation.rs` — üç boyut da dahil, testi var |
| GSC fırsat analizi | ✅ **kapsamı fazlasıyla aştı** — 6 araç ekranı (bkz. 0ai) |
| Meta/açıklama sürüm geçmişi | ✅ v0.5.7 |
| Otomatik güncelleme | ✅ Faz 10, saha testinden geçti |
| Feed değişikliği tespiti | ✅ v0.8.1 (K2) |
| Schema.org JSON-LD | ✅ v0.8.2 (K3) |
| İlk kurulum sihirbazı | → **K1** |
| Kod imzalama + notarization | ⏸️ kod işi değil, **maliyet kararı** (Apple ~$99/yıl + Windows sertifikası) |
| Toplu üretim (batch) | 🔴 **YAPILMAYACAK — yeniden karar gerekiyor, aşağıya bak** |

⚠️ **Toplu üretim (batch) neden kuyrukta değil:** listenin yazıldığı zamandan beri iki şey
değişti ve ikisi de bu maddeyi çürütüyor.
1. **Kullanıcı kısıtı:** *"halüsinasyon riskini göze alamam"* — üretim tek tek, operatör
   kontrolünde. Toplu işlem tam da bu kontrolü kaldırıyor. (Aynı gerekçeyle canonical yazma
   akışı da bilinçli olarak toplu DEĞİL, bkz. 0af.)
2. **Ölçülen kota gerçeği varsayımı çürütüyor:** liste *"47 ürün ≈ 47 Gemini çağrısı"* diyor;
   ama flash modellerin günlük limiti **20** (bkz. 0a). 47'lik bir kuyruk yarısına gelmeden
   kotayı bitirir, kalanlar Gemma'ya düşer → çıktı model bakımından tekdüze olmaz.
   Yapılacaksa ancak sınırlı bir biçimde: küçük kuyruk + satır başına onay + kota göstergesi.
   **Bu bir karar, iş değil — kullanıcıya sorulmadan başlanmaz.**

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

0z. ⚠️ **ÖNBELLEĞE YAZILAN YAPIYA ALAN EKLERKEN DİKKAT (v0.5.9 saha hatası).**
   `opportunity_json` gibi DB'ye serialize edilen bir yapıya yeni alan eklemek, ESKİ
   ÖNBELLEĞİ olan kurulumlarda ekranı komple düşürebilir.

   **Yaşanan:** v0.5.8'de rapora `eol` eklendi. `get_opportunity_cache` önbelleği ham
   `serde_json::Value` döndürüyordu — yani Rust yapısından GEÇMİYORDU, dolayısıyla
   `serde(default)` korumaları hiç devreye girmiyordu. Ön yüz `report.eol.length` deyince
   `undefined.length` attı ve Vue **tüm sayfayı** düşürdü; koşulsuz render edilen buton bile
   görünmedi. Kullanıcı "ekran bomboş" diye bildirdi.

   **Kural:** önbellekten okunan her yapı **Deserialize + `#[serde(default)]`** taşımalı ve
   komut ham JSON değil **yapının kendisini** döndürmeli. Ön yüzde `?.` ikinci savunma hattıdır,
   birincisi değil.

   ⚠️ **Neden testler yakalayamadı:** `vite dev`'de Tauri yok → önbellek yok → `report` null
   kalır ve boş durum sorunsuz açılır. Hata YALNIZCA eski önbelleği olan kurulumda çıkar.
   Harness tabanlı görsel doğrulama da bu senaryoyu görmez. Alan eklerken önbellek uyumu
   ayrıca düşünülmeli; regresyon testi `old_opportunity_cache_still_parses`.

0aa. 🔴 **EN BÜYÜK SEO FIRSATI: SATIŞTA OLMAYAN AMA TRAFİK ALAN SAYFALAR (v0.5.8).**
   Kullanıcının notundan çıktı ("feed'de satıştaki ürünler var, EOL nesillerin linkleri de
   indekste") ve ölçünce planlananın ~10 KATI çıktı.

   **Ölçüm (2026-07-29, kurumsalit.com, son 90 gün, GSC):**
   | | sayfa | tıklama | gösterim |
   |---|---|---|---|
   | satıştaki (feed'de) | 237 | 1.728 | 24.358 |
   | **feed'de olmayan (EOL)** | **4.527** | **3.840** | **157.768** |

   → Ürün trafiğinin **%69'u**, gösterimlerin **%87'si** satın alınamayan sayfalara gidiyor.
   Kıyas: optimize ettiğimiz 60 fırsatın toplam kaybı yalnızca 374 tıklama.

   - Sayfalar **HTTP 200 dönüyor** (curl ile doğrulandı) — ölü değiller, Google'da sıralanıyorlar;
     ziyaretçi geliyor, ürünü satın alamıyor. Çözüm: güncel nesle 301.
   - **Uygulama yönlendirme YAPAMAZ** — IdeaSoft panelinden tanımlanır. Ekranda açıkça yazılı.
   - Eşik ölçümle seçildi: 4.523 EOL sayfanın yalnızca **967'si ≥1 tıklama** alıyor, gerisi gürültü.
   - ⚠️ **Yol öneki zorunlu** (`common_path_prefix`, ürünlerden türetilir): olmadan blog ve
     kategori sayfaları "satışta olmayan ürün" sanılır.
   - ⚠️ **Tuzak (yaşandı):** "satışta" kümesi ÜRÜNLERDEN kurulmalı. Bir ara `by_url`den
     (GSC sayfaları) kurmuştum — öyle kalsaydı her sayfa satışta sayılır, EOL listesi hep boş
     çıkardı ve kimse fark etmezdi.

0ad. ✅ **EOL HALEF ÖNERİSİ — ölçüm tasarımı değiştirdi (v0.6.1).**

   Önce deterministik eşleştirme denendi (bedava, kotasız) ve **ölçülerek elendi**:
   - En çok trafik alan 12 EOL sayfanın yalnızca **3'ünde** güçlü eşleşme
   - Eşik 0.45'te tıklamaların ancak **%23'ü** kapsanıyor
   - Hatalar TEHLİKELİ: `asus-zenbook-17-fold` için en iyi aday **"Microsoft Windows 11 Pro"**
     çıktı (yalnızca "windows" sözcüğü örtüştüğü için). Bu yöntemle gidilseydi kullanıcıya
     güvenle yanlış 301 önerilecekti.

   **Sonuç mimari:** kod ADAY üretir (262 ürün → 5), model KARAR verir.
   - `opportunity::successor_candidates()` — IDF ağırlıklı sözcük örtüşmesi. Skor bir öneri
     DEĞİL, sıralama ölçütü; dokümanda ve testte böyle yazılı.
   - `gemini::suggest_successor()` — ⚠️ **"halef yok" diyebilmesi ZORUNLU.** Her EOL sayfanın
     karşılığı yok; olmayan halefi varmış gibi göstermek gerçek SEO hasarı. Şema
     `successor_sku`'yu opsiyonel bırakıyor, prompt "emin değilsen boş bırak" diyor.
     Ayrıca **uydurma engeli**: dönen SKU adaylar arasında değilse "halef yok" sayılır.
   - Canlı test (`successor_real`, gemini-3.6-flash) tam bu davranışı sabitliyor:
     halefi olan vakada doğru ürün, olmayan vakada `None`.

   **Kota tasarımı:** istek üzerine, satır bazında, önbellekli. 1.073 sayfa için toplu çağrı
   günlük kotayı (flash 20/gün) anında tüketirdi. Trafik tepedeki sayfalarda yoğun.
   Arayüzde "halef yok" NÖTR renkte — başarısızlık değil, geçerli cevap.

0ar. ✅ **SCHEMA.ORG JSON-LD (v0.8.2, K3).** Ürün detayında "Schema.org (JSON-LD)" kartı:
   kodu göster + kopyala. Mantık `core/src/jsonld.rs`, komut `get_jsonld`. **Model çağrısı yok** —
   elde olan veriden derleniyor, dolayısıyla halüsinasyon yüzeyi de yok.

   🔬 **Ölçüm kapsamı belirledi — IdeaSoft ZATEN yapılandırılmış veri basıyor.** Canlı ürün
   sayfasının ham HTML'i incelendi:

   | | durum |
   |---|---|
   | `application/ld+json` | **0 tane** |
   | microdata `schema.org/Product` | **var** (gizli div) — name·description·sku·brand·category·image·url |
   | microdata `Offer` | priceCurrency·price·**availability**, canlı |

   Yani temel alanları tekrar etmenin değeri yok. **Bizim eklediğimiz tek gerçek şey
   `additionalProperty`** — teknik özellik tablosu. IdeaSoft'un mikroverisinde ürün özelliği YOK;
   tablo mağazada yalnızca HTML olarak duruyor, arama motoru tarafında yapılandırılmış veri
   olarak okunmuyor. Kazanç orada.

   ⚠️ **Kuyruk tanımındaki `offers` ölçümle çürüdü:** feed'de fiyat alanı yok (`FeedProduct`),
   stok anlık değişiyor. Kopyalanan koda yazılmış bir fiyat sayfadaki canlı mikroveriyle çelişir
   ve Google bunu **hata** raporlar. **Yanlış fiyat, fiyat olmamasından kötüdür.**
   `aggregateRating` de yok: puan verisi elimizde yok, uydurmak politika ihlali.

   🔬 **Gerçek veri (254 ürün):** 254/254 JSON-LD üretiyor, **14'ünde özellik var (519 satır)**,
   en büyük çıktı 8,5 KB. Ölçüm bir gürültü de gösterdi: teknik tablonun ilk satırları
   ad/marka/kategoriyi tekrar ediyordu (üst düzeyde zaten yazılı) → **tam eşitlik** ölçütüyle
   ayıklanıyor, 550 satır 519'a indi. Benzer değerler korunuyor.

   🔴 **`</script>` kaçışı — ve aynı tuzağa harness'te düşüldü.** Özellik değerinde `</script>`
   geçerse tarayıcı script'i erken kapatır. `render_script` bunu kaçırıyor ve testi var; ama
   `scripts/harness.py` örneği gömerken kaçırmayı unutunca **harness sessizce boş açıldı** —
   konsolda hata bile yok. Bir tur kaybettirdi. Ders: bu kaçış, JSON'un HTML'e gömüldüğü
   HER yerde gerekli, yalnızca üretim kodunda değil.

   ⚖️ **Yayın yolu — kullanıcı kararı (2026-08-01): şimdilik panoya kopyalama YETERLİ.**

   📌 **ERTELENEN SEÇENEK (kullanıcı notu, 2026-08-01):** teknik tablo verisiyle birlikte
   JSON-LD'nin **IdeaSoft'a gönderilmesi** ileride değerlendirilecek. Kullanıcının koyduğu
   ön koşullar, sırasıyla:
   1. **Saha testleri** — *"Saha testleri sonrasında ancak karar verebilirim."*
   2. **Site teması kontrol edilecek** — tema `extraDetails` içeriğini nasıl basıyor,
      script etiketi sayfaya olduğu gibi düşüyor mu.
   3. ⚠️ **Ölçülmemiş soru: IdeaSoft `<script>` etiketini kırpıyor mu?** Cevap BİLİNMİYOR;
      canlı bir yazma denemesi gerektiriyor, onay olmadığı için denenmedi. Karar bu ölçüm
      yapılmadan verilmemeli.

   Teknik zemin hazır: gönderim modülü `extraDetails` alanına zaten yazıyor (teknik tablo
   oradan gidiyor), yani iş "yeni yetenek" değil, mevcut gönderime bir parça eklemek.

0ax. ✅ **BUGÜN EKRANI + İŞ KUYRUĞU (v0.12.0, Faz K).** Uygulama dokuz ekran sunuyordu ve
   "nereden başlayayım?" sorusunu kullanıcı kendisi çözüyordu. Genel Bakış bunu **araç
   düzeyinde** cevaplıyor; **satır düzeyinde** cevaplayan bir yer yoktu. Veri zaten vardı —
   eksik olan tek şey **SEÇİMDİ**: 2.226 aday satırdan bugünün 10 işi.

   Saf seçim mantığı `core/src/queue.rs`'te (8 test), kovaların tanımı `commands/today.rs`'te.
   Kuyruk **saklanmıyor, her açılışta hesaplanıyor** (`metrics.rs` ile aynı gerekçe); kalıcı
   olan tek şey kullanıcının kararı → `queue_dismissals`.

   🔬 **Üç ölçüm tasarımı doğrudan belirledi:**

   1. 🔴 **Feed bayrağı ham hâliyle %87 gürültü.** 70 bayraktan **61'i yalnızca "görseller"**;
      görsel değişikliği üretilmiş meta/açıklama METNİNİ geçersiz kılmıyor. Acil kovası
      daraltıldı: *metin alanı değişmiş VE mağazaya gönderilmiş* → **9 ürün**.
   2. 🔴 **Ham skor sırası amiral analizi kuyruğa hiç sokmuyordu.** İlk 10: kaçak 4 · bakım 3 ·
      acil 3 · **kaldıraç 0**. Sebep ölçek farkı (en büyük EOL sayfası 515 tıklama, en büyük
      fırsat 37 kaçırılan tıklama). **Kova başına 3** sınırıyla ilk 10'da dört kova temsil
      ediliyor.
   3. 🔴 **12 ürün birden çok kovada** (4'ü üç kovada birden): 106 ham satır → **90 benzersiz**.
      Bir ürün = bir madde; en güçlü sebep başlık, diğerleri "ayrıca" satırında.

   **Skor şeffaf:** tıklama × kova ağırlığı, ağırlıklar ekranda gerekçesiyle. Ağırlık keyfi
   değil — *"bu kaybın ne kadarı geri kazanılabilir ve uygulama bunu yapabilir mi?"*. Kaçak
   trafiğin 0,6 olmasının sebebi asıl çözümün 301 olması ve **uygulamanın onu yapamaması**.

   ⚠️ Süreler **tahmin, ölçüm değil** — üretim süreleri loglanmıyor; kodda ve ekranda böyle
   işaretli. Loglanmaya başlarsa gerçek medyanla değişmeli.

   🔴 **SAHA HATASI: "gün hiç bitmiyordu."** İlk sürümde "Yapıldı" maddeyi anında düşürüyor,
   yerine 11. aday geliyordu; sayaç hep 10'da kalıyordu — *"bu mantık ile günlük iş hiç
   bitmez"*. Tasarım turu da aynı cevabı verdi: **işaretlenen madde YERİNDE kalsın.** Üstü
   çizili, soluk, "geri al" ile. Arka uçta iki liste ayrıştı: `hidden()` çıkarır (kalıcı
   gizleme + erteleme), `completed()` yerinde bırakıp işaretler. Ders: *bitmesi beklenen bir
   liste, tamamlandıkça kendini yeniden doldurmamalı.*

   🔑 **"Yapıldı" bir ölçüm boşluğunu kapattı.** Faz Ö şunu itiraf ediyordu: *"içeriği elle
   kopyalayıp yapıştıran kullanıcı için olay oluşmuyor → ölçülemiyor"*. Kuyruktaki işlerin bir
   kısmını uygulama zaten yapamıyor (301). Artık `manual_done` + `reaches_store=1` yazılıyor.
   ⚠️ Beyan ile doğrulanmış gönderim ayrı etiketli ("Elle yapıldı olarak işaretlendi" ↔
   "IdeaSoft'a gönderildi"). ⚠️ İşaret **kalıcı değil** (`done_at_analysis`): analiz
   yenilenince düşer — iş yaramadıysa madde geri gelmeli.

   **Derin link için yeni odak mekanizması:** `store.focus` + `useRowFocus` + SeoTable
   kaydırması. Üç ekran listeyi kırpıyor (EOL 25, Düşüşte 30, Yükselmeye yakın 40); gerektiğinde
   kırpma yükseltiliyor. SeoTable'ın Faz B'den beri kullanılmayan `selected` alanı nihayet
   kullanılıyor.

   🔴 **Kaydırmada ölçümle bulunan tuzak:** satırı DOM'da bulmak YETMİYOR. Ekran değişimiyle kap
   yeniden oluştuğu için `scrollIntoView` sessizce hiçbir şey yapmıyordu — satır seçili
   görünüyor ama ekranın **2.228 piksel altında** duruyordu, yani "çalışıyor" sanılabilirdi.
   Artık kaydırmanın SONUCU doğrulanıyor, olmadıysa sonraki karede tekrar deneniyor.

   ⚠️ **Sonuç kontrolü kovası bugün BOŞ** ve Eylül ortasına kadar boş kalacak (72 gönderim
   0–12 gün önce, ölçüm için 28 gün gerekiyor). Sessizce boş bırakmak yerine tarihi söylüyor.

0b4. ✅ **TEKLİF VE OFFLINE SATIŞ (Faz T).** Faz C müşteriyi uygulamaya soktu; satışın
   kendisi hâlâ dışarıdaydı. Teklif Excel'de hazırlanıyor, hangi fiyatın verildiği ve niye
   kaybedildiği hiçbir yerde durmuyordu.

   🔴 **FAZIN ÖN KOŞULU ÖLÇÜMLE ÇIKTI: KATALOGDA HİÇ FİYAT YOKTU.** `products` tablosunda,
   `ideasoft_catalog`ta ve feed'in 24 alanında tek bir fiyat/kur/KDV alanı yoktu. Kullanıcı
   planlama sırasında feed'e beş alan ekledi: `buyingPrice` · `price1` · `tax` ·
   `currencyAbbr` · `priceTaxWithCur`. Yani faz, veri kaynağı hazırlanarak başladı.

   🔴 **İKİNCİ ÖLÇÜM İKİNCİ VARSAYIMI ÇÜRÜTTÜ: KATALOG TEK PARA BİRİMİNDE DEĞİL.**
   `priceTaxWithCur`ın örtük kuru 1,00 ile 54,93 arasında değişiyordu; `currencyAbbr` sebebini
   söyledi: **USD 273 · EUR 8 · TL 1**. Elle girilen tek bir kur o 9 üründe yanlış fiyat
   üretirdi. Beyan güvenilir çıktı (USD 47,5906–47,5917 · EUR 54,9344 sabit · TL 1,0000).

   **Çözüm: çevrim satır eklenirken BİR KEZ yapılıp donuyor** (`quote::catalog_line`):
   | Teklif | Ürün | Kaynak |
   |---|---|---|
   | TL | hepsi | `priceTaxWithCur ÷ (1+KDV)` — mağazanın kendi TL fiyatı, **kur gerekmiyor** |
   | USD | USD | `price1` birebir |
   | USD | EUR/TL | TL fiyatı ÷ kullanıcının USD/TRY kuru (tek kur, N kur değil) |

   🔑 Yan faydası büyük: **marj aritmetiği para biriminden bağımsız kaldı.** Fiyat da maliyet
   de aynı birimde donduğu için TL teklifte de USD teklifte de aynı yüzde çıkıyor; kur mantığı
   hesaba hiç bulaşmıyor. Ayrıca teklif bir **kayıt**: altı ay sonra bakıldığında o günkü
   maliyet ve kur donmuş hâlde duruyor.

   🔴 **MALİYET MÜŞTERİYE GİDEN BELGEYE GİREMEZ — DERLEME MESELESİ.** `QuoteOut` ve `OutLine`
   yapılarında maliyet/marj **alanı yok**; `Quote → QuoteOut` dönüşümü bilinçli **kayıplı**.
   Belge üreten kod göremediği şeyi basamaz. Üstüne bir test davranışı da sabitliyor.
   ⚠️ İlk hâlinde o test **yanlış sebeple** patladı: ham HTML'de "600" arıyordu ve
   `font-weight:600`a takıldı. Testin sorusu düzeltildi — belgede **ne yazdığı** (etiketler
   ayıklanmış görünür metin), biçim değerlerinde hangi rakamların geçtiği değil.

   **Ölçümler (282 ürün):** KDV %20'de 276 ürün, %10'da 6 → teklif KDV'yi **orana göre
   kırıyor**, tek satır yetmiyor. Marj medyanı %46,7 ama **7 ürün negatif** (en düşük −%106,7)
   ve 31 ürün %10 altı → uyarı süs değil. Senkron sonrası **feed bayrağı 80 → 80**: fiyat
   parmak izine girmedi (`fingerprint.rs`in `quantity` gerekçesinin aynısı — girseydi dolar
   her oynadığında 282 ürün bayraklanır, acil kovası çöpe dönerdi).

   ♻️ **Kuyruğa yeni kova EKLENMEDİ.** Teklif gönderilince kişinin **sonraki adımı öneriliyor**
   (Faz C); madde Müşteri kovasında çıkıyor. ⚠️ Kişinin mevcut randevusu varsa **ezilmiyor**,
   hatırlatılıyor. İkinci bir hatırlatma sistemi kurulsaydı hangisinin doğru olduğu
   belirsizleşirdi.

   ⚠️ Durum geçişleri `contact_events`e yazılıyor, `work_events`e **değil**: o günlük
   sku-anahtarlı ve `reaches_store` eksenli, bir teklif oraya ait değil (Faz C kararının
   devamı).

   🔴 **YEDEKTE İNCELİK:** `settings.rs`'in mevcut `f()` yardımcısı eksik sayıyı **0.0**
   yapıyor. Teklif satırının `NULL` maliyeti 0'a düşseydi elle satır geri yüklemede **%100
   marjlı** görünür ve teklifin kârı olduğundan yüksek okunurdu. Nullable için `fo()` eklendi,
   testle sabitlendi. **Ders: "eksikse sıfır" varsayılanı, eksikliğin kendisi bilgi taşıyan
   alanlarda sessiz bir yalan üretiyor.**

   🔴 **BELGE ÜÇ KEZ YAZILDI — HER SEFERİNDE SAHA DÜZELTTİ (2026-08-11).**
   | Deneme | Neden olmadı |
   |---|---|
   | `window.open` + yazdır | Tauri'de açılır pencere yok, `null` döndü |
   | Uygulama içinden `window.print()` | macOS'ta **WKWebView bu çağrıyı uygulamıyor** — düğme sessizce hiçbir şey yapardı |
   | ✅ Geçici dosya + varsayılan tarayıcı | Yazdırma penceresi her platformda "PDF olarak kaydet" veriyor |

   ⚠️ İkinci denemeden kalan `@media print` kuralları bir süre kodda kaldı; gösterecekleri
   kopya kaldırılmıştı, yani uygulamada Cmd+P **bomboş sayfa** basacaktı. Kalıntı temizlendi.
   **Ders: bir mekanizmayı değiştirirken ondan geriye kalan CSS de mekanizmanın parçası.**

   🔴 **MAİL İSTEMCİLERİ MODERN CSS'İ YOK SAYIYOR.** Kullanıcı belgeyi Outlook'a yapıştırdı ve
   tablo dağıldı. Ölçülen üç kısıt: `max-width` (div'de) yok sayılıyor · `margin:auto`
   çalışmıyor (toplamlar sağ kenara uçtu) · `display:flex` yok sayılıyor. Belge baştan sona
   tablolarla kuruldu (`width="560"` **özniteliği**, sağa yaslama boş hücreyle,
   `cellpadding=0`). Ara adımda toplamlar sağa değil **ortaya** düştü: ayırıcı hücre `&nbsp;`
   kadar daralıyordu, `width="100%"` ile açgözlü yapıldı.

   ⚠️ **Sütun hizası kayması** (teklif düzenleyici): sayı girişlerinin artırma okları sağ
   kenardan ~18px yiyordu ve girişli hücrede iç boşluk varken düz metin hücresinde yoktu.
   Oklar gizlendi, boşluklar eşitlendi; beş sütunun sapması ölçüldü → **hepsi 0px**.

   🔴 **AYNI TDZ HATASI ÜÇ KEZ YAPILDI** (`ContactCard`, sonra `QuoteEditor` iki kez):
   `immediate` izleyici, ref'ler altta. **Üçünü de yalnızca konsol yakaladı** — ekran her
   seferinde doğru çiziliyordu.

   → Üçüncüsünden sonra not yetmedi, **koruma** eklendi: `scripts/harness.py::tdz_uyar`
   her üretimde `<script setup>` dosyalarını tarayıp `immediate` izleyicinin kendisinden
   sonra tanımlanan bir ref'e yazdığı yerleri uyarıyor.
   ⚠️ Korumanın **ilk iki sürümü hatayı yakalayamadı** ve bunu ancak hatayı bilerek geri
   koyup deneyince gördüm: (1) `ref<Jenerik>()` biçimini kaçırıyordu, (2) `immediate: true`
   ifadesini kendi doküman yorumunda buluyordu. **Ders: sınanmamış bir koruma, koruma
   değil — yalnızca güven yanılsaması.**

   ⚠️ **Saat gece yarısını geçince bir test patladı** (2026-08-12): `to_rfc3339()` saat dilimi
   eki taşıyor, SQLite'ın `date()`i onu UTC'ye çevirip günü geri alıyor → "90 gündür" yerine
   "91 gündür". Uygulama `now_str()` ile **eksiz yerel** zaman yazdığı için üründe sorun yok;
   hatalı olan testti. Harness'taki `toISOString()` sapmasının (158 dk) aynı ailesi.

0b3. 🔴 **İKİ SAHA HATASI VE BİR DOĞRULAMA AÇIĞI (2026-08-10).** Kullanıcı `npm run tauri dev`
   ile test etti: *"Müşteriler ekranının tasarımını göremiyorum"* ve *"Bugün sayfasında hâlâ
   önceki günün yapıldı olarak işaretlenmiş işleri mevcut."*

   **1) Müşteriler ekranı bomboş açılıyordu — TDZ.** `ContactCard.vue`'da
   `watch(..., { immediate: true })` kurulum sırasında hemen çalışıp `temas` ref'ine yazıyordu
   ama `const temas` **20 satır aşağıdaydı**: *"Cannot access 'temas' before initialization"*.
   Vue kurulum hatasını yutuyor ve `<component :is>` hiçbir şey çizmiyor — üst şeritte başlık
   görünüyor, gövde bomboş. Düzeltme: tanım `watch`tan öncesine alındı.

   🔴 **ASIL MESELE BU DEĞİL — DOĞRULAMA YÖNTEMİM AÇIK VERDİ.** Bu hata harness'ta da VARDI:
   ölçüldü, üretim paketleri aynı hatayı küçültülmüş hâlde (`Cannot access 'c'…`) atıyordu.
   Ama ekran **yine de doğru çiziliyordu** ve ben ekran görüntüsüne bakıp "çalışıyor" dedim.
   **Piksele baktım, konsola bakmadım.**

   → Görsel doğrulama artık iki adımlı: ekran görüntüsü **ve** `read_console_messages`.
   ⚠️ Konsol tamponu sayfa yenilemede temizlenmiyor; eski hatalarla yenileri karışmasın diye
   `console.log("ISARET")` işareti bırakılıp **işaretten sonrasına** bakılıyor. Bu oturumda
   tam olarak böyle ayrıştırıldı (5 hatanın hepsi işaretten önceydi, güncel paket temiz).

   **2) "Bugün" listesi güne değil ANALİZE bağlıydı.** "Yapıldı" işareti yalnızca
   `done_at_analysis` ile eşleşiyordu; analiz yenilenmediği sürece dünkü bitmiş işler ekranda
   kalıyordu. Ölçüm (kullanıcının veritabanı, 10 Ağustos): eski kural **11 madde** döndürüyor,
   gün koşulu eklenince **0**. Ekranın adı *Bugün* — içeriği de güne bağlı olmalı.

   Düzeltme: `completed()` artık `done_at_analysis = ?1 AND date(at) = bugün`. Yarın işaret
   düşüyor ama madde **geri gelmiyor** — v0.14.1'in uçuş süzgeci devralıyor (ölçüldü: bu
   maddelerin karşılığı 89 uçuştaki iş). İki mekanizma sırayla: `completed` bugün,
   `in_flight` sonrasında.

   **3) "Yapıldı" ACİL koşulunu kaldırmıyordu — madde her gün geri geliyordu.** Kullanıcı
   ikinci turda sordu: *"birbirimizi anlayamıyor olabilir miyiz?"* — haklıydı, eksik bendeydi.

   Uçuş süzgeci (v0.14.1) bilinçli olarak **acil kovasını kapsamıyor**: acilin kanıtı GSC
   değil canlı feed, yarın yine değişirse madde çıkmalı. Ama "yapıldı" acil koşulunu da
   kaldırmayınca madde ertesi gün geri geliyordu. Ölçüm (kullanıcının veritabanı): işaretli
   9 üründen **3'ü** hâlâ acil koşulunu sağlıyordu — 2'sinde feed bayrağı duruyordu, 1'i
   "hiç gönderilmemiş" görünüyordu **oysa `manual_done` beyanı vardı**.

   🔑 **Ayrım net: kanıtı kim tutuyor?**
   | Kova | Kanıt | "Yapıldı" ne yapıyor |
   |---|---|---|
   | Kaldıraç · Kaçak · Bakım | GSC (90 gün gecikmeli) | madde **susturuluyor** (28 gün) |
   | Acil | uygulamanın kendi verisi | **veri düzeltiliyor** (feed bayrağı kapanıyor) |
   | Müşteri | uygulamanın kendi verisi | **veri düzeltiliyor** (sonraki adım temizleniyor) |

   Yani susturma, kanıtı bekleyemediğimiz yerlerde bir çare; kanıtı biz tutuyorsak doğru
   davranış veriyi düzeltmek. Faz C'de kişi tarafında zaten böyle yapılmıştı, acil kovası
   geride kalmıştı.

   İkinci delik: *"içerik hazır ama mağazaya hiç gönderilmemiş"* koşulu yalnızca
   `ideasoft_pushed_at`e bakıyordu. İçeriği elle mağazaya yapıştıran kullanıcının o alanı
   hiç dolmuyor — Faz Ö'nün itiraf ettiği boşluğun aynısı. Artık `reaches_store = 1` olayı
   da gönderim sayılıyor.

   **Ders: ekranın adı bir sözleşmedir.** "Bugün" diyen bir liste analiz damgasına değil güne
   bağlanmalı; kullanıcı ekranın adına inanıyor, koddaki anahtara değil.

0b2. ✅ **CRM İNCE DİLİM (Faz C).** Uygulama **ziyaretçiyi** görüyordu, **müşteriyi**
   görmüyordu: bir ürün 955 gösterimden 36 tıklama getiriyor, o tıklamalardan biri mail
   atıyor — ve bu noktadan sonra uygulama kördü.

   ⚠️ **Bu fazda "fırsatı ölçen" bir sayı YOK ve bu dürüstçe söylendi.** Önceki fazlar mevcut
   veriden besleniyordu (2.115 sayfa, 279 ürün); CRM'in verisini kullanıcı sıfırdan üretiyor.
   Ölçüm ancak veri birikince mümkün — fazın eşik kararı doğrudan buna dayanıyor.

   **C1 — çekirdek (`8d6d937`).** `contacts` + `contact_events` + `contact_tags` +
   `contact_products`; kişi listesi/kartı; `next_step_at` (yol haritasının deyimiyle "CRM'in
   %80'i"); kuyruğun **6. kovası**.
   - **`TOTAL` 10 → 13** (kullanıcı kararı: *"günlük iş sayısını artırabiliriz"*). Ölçüm
     gerekçeyi verdi: 10 slotun tamamı doluydu (Bakım 3 · Kaçak 3 · Acil 3 · Kaldıraç 1),
     yani CRM ancak SEO işlerini düşürerek girebilirdi.
   - Skor `CONTACT_BASE (40) + gecikme`, tavan +30. Taban `URGENT_BASE` ile aynı: ikisi de
     "çözülmemiş bir şey duruyor" sınıfı. Fark, bekleyen insanın **bozulabilir** olması.
   - ⚠️ `evidence_lags() = false` — v0.14.1'in uçuş süzgeci CRM'e dokunmuyor; kanıtı GSC değil.
   - 🔴 **08-08 hatası CRM'de yeniden doğuyordu:** "yapıldı" sonraki adımı temizliyor, kişi
     adaylıktan düşüyor, yerine 11. madde geliyordu. `contacts_by_ids` o maddeleri günün
     listesinde tutuyor. Aynı tuzağın üçüncü görünüşü — mekanizma değişti, hata aynı kaldı.

   **C2 — bağlar ve toplu giriş.** Etiketler (kanal sütun / ilgi tablo), "bu ürünle ilgilendi"
   bağı (iki yön de **tek tablodan sorguyla**, kopya sayaç yok), CSV içe aktarma, sessizlik
   eşiği, yedek kapsamı.
   - 🔬 **CSV gerçek dosyayla ölçüldü** (Türkçe Excel taklidi: BOM + `;` + CRLF + tırnaklı
     alan + tekrar eden e-posta + adsız satır): ayraç `;` sezildi, BOM temizlendi, tahmin
     `Yetkili→ad · Ünvan→firma · E-Posta→e-posta · Cep→telefon` çıktı, *"Anadolu Yapı, A.Ş."*
     bozulmadan geçti → **3 eklendi · 1 güncellendi · 1 atlandı**. Tekrar eden kişi
     ikizlenmedi, güncellendi.
   - ⚠️ **Önizleme atlanamaz adım:** 300 satırlık yanlış eşleşmiş aktarım geri alınamaz.
   - **Sessizlik eşiği KAPALI doğuyor** ve veriden öğreniyor: ≥5 kişide ≥2 temas biriktiğinde
     medyan aralık öneriliyor (*"25 gün diyelim mi?"*), **sessizce yazılmıyor**. Faz D'de stok
     eşiği "veri yokken eşik uydurma" diye elenmişti; buradaki fark eşiğin kullanıcının kendi
     verisinden çıkması. ⚠️ Sonraki adımı olan kişi eşikten etkilenmiyor — çift hatırlatma.
   - ⚠️ Sessiz kişi **kendi cümlesini** kuruyor (*"41 gündür temas yok"*). Ayrılmasaydı
     sonraki adım tarihi olmadığı için gecikme 0 çıkar ve *"bugün dönülecek"* derdi.

   🔴 **GİZLİLİK — bu fazın yeni sorumluluğu.** Veritabanına ilk kez kişisel veri girdi.
   Kişiler asistana **açılmıyor**: asistan bağlamı Gemini'ye gidiyor. Projede JS test koşucusu
   olmadığı için garanti bir test değil, **dev kipinde açılışta patlayan kilit**
   (`assistantSources.ts` · `YASAKLI_KAYNAKLAR`). Yedek dört tabloyu da taşıyor (K2 dersi) ama
   Ayarlar artık dosyanın kişisel veri içerdiğini **söylüyor**.

   ♻️ **ÜÇ KOPYA BULUNDU VE PAYLAŞIMA ÇIKARILDI** — hepsi altıncı kova eklenirken görüldü:
   | Kopya | Nereye | Nasıl yakalandı |
   |---|---|---|
   | Liste kroması (arama + sekmeler, ~70 satır) | `styles.css` | ikinci liste ekranı yazılırken |
   | Kova etiketi/tonu (`TodayPage` + `FocusBar`) | `buckets.ts` | **derleyici** (`Record<Bucket, …>`) |
   | Kova adları (`SettingsPage`) | `buckets.ts` | ölçümle — derleyici SUSMUŞTU |

   🔴 **Üçüncüsünün dersi:** ilk ikisi `Record<Bucket, string>` olduğu için 6. kova eklenince
   **anında patladı**; üçüncüsü `Record<string, string>` olduğu için sessiz kaldı ve Ayarlar'da
   kova adı ham `contact` görünecekti. **Gevşek tip, kopyayı görünmez yapıyor.** Aynı dosyada
   satır tipi de elle yeniden yazılmıştı (`CalibrationRow` yerine anonim nesne) — o da
   bağlandı.

   🔴 **`.hint` cümleleri bozuyormuş (ölçüldü, 2026-08-10).** Global sınıf `display: flex`
   olduğu için içindeki her `<b>` ayrı bir sütun oluyordu: *"… bu eşikten | etkilenmez |
   — zaten söz verdiğiniz gün…"*. Ekrandaki 6 ipucunun **4'ü** bu hâldeydi, yani istisna değil
   kuraldı. Blok akış + satır içi ikona çevrildi; dört mevcut satır da düzeldi.
   **Ders: bir yardımcı sınıf, en sık kullanıldığı içerikle sınanmalı.**

0b1. 🔴 **YAPILAN İŞ KUYRUĞA GERİ GELİYORDU — "kanıtın gelme süresi" hesaba katılmamıştı
   (2026-08-09, saha geri bildirimi).** Kullanıcı: *"daha önce yaptığım ve yapıldı olarak
   işaretlediğim işler bugün hâlâ listedeydi."*

   Faz K'nın kuralı şuydu: "yapıldı" işareti **analiz damgasına** bağlı, analiz yenilenince
   düşer — *"iş işe yaradıysa madde yeni raporda çıkmaz, yaramadıysa geri gelmeli."* Mantık
   doğru, **varsayımı yanlış**: raporun kaynağı GSC ve GSC **90 günlük** pencereye bakıyor.

   🔬 **Ölçüm (kullanıcının gerçek veritabanı):** `NB.LEN.21SR006RTX` 7 Ağustos'ta mağazaya
   gönderildi, 8'inde "yapıldı" işaretlendi, 9'unda analiz yenilenince **bakım kovasında geri
   geldi**. Gelmemesi mümkün de değildi — iki günlük düzeltme 90 günlük ortalamayı kıpırdatamaz.
   Kullanıcının cümlesi kararı verdi: *"28 gün beklersem kalan işi tamamlamaya ömrüm yetmez."*

   **Çözüm — ölçüm uçuştayken madde susuyor** (`queue::drop_in_flight`). Bir referans için
   mağazaya ulaşan iş yapıldıysa ve üstünden `REVIEW_AFTER_DAYS` (28) geçmediyse, **kanıtı
   geciken kovalarda** (kaldıraç · kaçak · bakım) yeniden iş çıkarmıyor. Kaybolmuyor: 28. günde
   **sonuç kontrolü** kovası onu getiriyor — Faz Ö'nün omurgası zaten bunun için vardı.

   ⚠️ **İki kova bilinçli muaf.** Acil'in kanıtı GSC değil canlı feed: dün gönderilen üründe
   bugün metin değiştiyse bu bugün bilinen bir gerçek. Sonuç kontrolü ise maddeyi geri
   getirecek olan kova; susturmak onu sonsuza dek görünmez yapardı.

   🔴 **İnce yer — iki zaman ölçeği karışırsa İKİ ayrı hata geri gelir.** `completed` **bu
   analiz** boyunca konuşuyor (madde yerinde kalır, üstü çizili), `in_flight` **sonraki
   analizlerde**. Bu analizde işaretlenenler süzgece girseydi adaylıktan düşer, yerlerine 11.
   madde gelir ve 08-08'de düzeltilen *"gün hiç bitmiyor"* hatası dönerdi. Test ikisini birden
   sabitliyor: `ucus_suzgeci_bu_analizi_degil_oncekileri_susturuyor`.

   **Ölçülen etki (gerçek veri, 2.188 aday):** kaldıraç 49→**35**, bakım 55→**31**, kaçak
   2.118→**2.117**. 88 iş uçuşta, 10'u bu analizde işaretli (listede kalıyor) → **81 iş
   susturuldu.** Ekran bunu söylüyor: *"81 iş ölçüm bekliyor"* — süzgeç sessiz çalışsaydı
   kullanıcı yaptığı işin neden listeden düştüğünü bilemezdi (`review_ready_at` ile aynı gerekçe).

   ♻️ İkinci düzeltme aynı turda: **kararı verilmiş EOL sayfaları kaçak kovasından çıkıyor**
   (Faz D'nin `eol_decisions`'ı). 301'i panelde tanımladıysanız uygulama bunu doğrulayamıyor,
   tek kanıt sizin kararınız. **`keep` de dahil** — bilinçli tutulan sayfa iş değil.

   ♻️ Sonuç kontrolü kovasının sorgusu `store_events`e çıkarıldı: 28 günü **dolanlar** ile
   **dolmayanlar** aynı sorgudan okuyor. İki kopya olsaydı eşikler ayrışır, madde ikisinin
   arasına düşüp tamamen kaybolabilirdi.

   **Ders: bir kuralın mantığı doğru olabilir ama girdisinin ne zaman geleceğini bilmiyorsa
   yanlış davranır.** "İşe yaradıysa listede çıkmaz" cümlesi, kanıtın 28 gün sonra geleceğini
   hesaba katmadığı için kullanıcıyı iş yapmakla cezalandırıyordu.

0b0. ✅ **EOL KARAR DEPOSU + 301 CSV + SAĞLIK SKORU (Faz D).** Uygulama **2.115 satışta olmayan
   sayfa** buluyor, 673'ü ≥3 tıklama alıyor — projedeki en büyük tek SEO fırsatı. Ama bulgu işe
   dönüşemiyordu: halef önerileri **yalnızca `store.successors`'ta**, yani bellekteydi.
   Uygulamayı kapatınca verdiğiniz kararlar kayboluyordu. Eksik olan özellik değil **kalıcılıktı**.

   🔴 **FAZI ŞEKİLLENDİREN ÖLÇÜM GEÇMİŞTEN GELDİ** (`opportunity.rs`, 2026-07-29): deterministik
   eşleştirici **hedefi otomatik dolduramaz**. `asus-zenbook-17-fold` için en iyi aday
   *"Microsoft Windows 11 Pro"* çıkıyor (yalnızca "windows" örtüştüğü için, 0,20 benzerlik).
   Kodun kendi cümlesi: **"yanlış yönlendirme, yönlendirmemekten kötüdür."**
   → CSV **karar verilmiş** satırlardan doğmalı. Bu, karar deposunu fazın merkezi yaptı.
   `csv_real` testi bunu canlı üretiyor: 673 kararsız satırın **hepsinde `hedef_yol` boş**,
   Windows 11 eşleşmesi yalnızca bilgi sütununda (`aday` 0,20) duruyor.

   **Karar deposu** `eol_decisions(slug PK, url, action, target_slug, target_sku, source,
   decided_at, exported_at, note)`. `action`: `redirect_301` · `canonical` · `keep`.
   ⚠️ **`keep` bilinçli bir karar türü** — ekran zaten *"bazı sayfaları bilinçli tutuyor
   olabilirsiniz"* diyor; kayıt altına alınca aynı sayfa her analizde yeniden karşınıza çıkmıyor.
   `source` (`ai`/`manual`) hedefi modelin mi sizin mi seçtiğinizi CSV'ye taşıyor.

   ⚠️ **Excel'de doldurulan hedefler uygulamaya GERİ DÖNMÜYOR.** Bilinçli sınır: karar deposu
   yalnızca uygulama içinde verilen kararları bilir. CSV başlığındaki `#` satırları bunu yazıyor.

   **Sağlık skoru `overall_status`'ın YERİNE DEĞİL, YANINA** (`core/src/health.rs`,
   meta 25 · açıklama 25 · teknik 20 · görsel 15 · **mağazaya ulaştı 15**). İki gerekçe:
   (1) `overall` bir kova döndürüyor, "Eksik" bir üründe 45 ile 80 arasındaki fark görünmüyor;
   (2) 🔑 **`overall` mağazaya ulaşıp ulaşmadığına HİÇ bakmıyor** — oysa Faz Ö'nün merkezi kuralı
   bu. Ölçüm (279 ürün): 54 ürün üçü de tam, **1'i mağazaya hiç gönderilmemiş**, 19'u gönderilmiş
   ama içerik eksik, 12 üründe görsel sorunu. O 1 ürün `overall`a göre "Tamamlandı", Google
   içinse hiç yapılmamış — skorun varlık sebebi tam olarak o satır (`ucu_tam_ama_
   gonderilmemis_urun_yuz_almiyor` testi onu sabitliyor: 85, tek eksik "mağazaya gönderim").
   ⚠️ `overall` **değişmedi**: filtreler, sayaçlar ve Bugün kuyruğu ona bağlı, değiştirmek beş
   ekranı birden etkilerdi. Görsel bileşeni **kısmi puan** alabiliyor (4 görselin 1'i sorunlu →
   15'in 11'i), çünkü sorunlu görseli olan ürün hiç görseli olmayanla aynı sayılmamalı.

   ❌ **ÜÇ KALEM KOD YAZILMADAN ÖLÇÜMLE ELENDİ** (yol haritasında kapsamdaydılar):
   - **Stok eşiği uyarısı** — tükenen ürün **yok** (en düşük 1 adet); ≤3 stoklu 72 ürünün GSC
     trafiği de yok. Kod yazılır, ekranda boş durur.
   - **Varyant/aile gruplama** — SKU şeması aileyi kodlamıyor: `NB.LEN` 28 ürün ama bu
     "Lenovo dizüstü", aile değil. Güvenilir gruplama için feed'de olmayan bir alan gerekiyor.
   - **İçerik borç listesi** — Ürünler filtreleri ve Bugün kuyruğunun Katalog kaynağı aynı şeyi
     zaten sayıyor; dördüncü kopya olurdu (bu oturumda üç kez ölçtüğümüz "kopyalanan şey sapar").

   ⏸️ **Canonical toplu yazma ertelendi** — `canonical_set` olayı veritabanında **0**, yani
   akış hiç kullanılmamış; kullanıcı önceliği 301. `action` alanı hazır, yazma katmanı sonra.

   ♻️ **Faz B'de ayrılan `redirect` eylemi ilk kez kullanıldı** (ikon `cornerUpRight`, ipucu
   "301 listesine ekle") ve hedef seçimi **mevcut** canonical modalini yeniden kullandı —
   yalnızca başlığı kipe göre değişiyor. Yeni akış icat edilmedi, sonucu artık kaydediliyor.

0az. ✅ **ODAK SEANSI (v0.13.0, Faz S).** Faz K "bugün ne yapayım"ı cevapladı ama işi yapma
   **ritmi** yoktu. Asıl açık daha somuttu: kuyruktaki süreler uydurmaydı (1–3 dk, kodda ve
   ekranda "tahmin, ölçüm değil" diye işaretli). Yol haritası bunu Faz S'in bitiş şartı yapmış.

   🔬 **Planlarken ölçtüm: GERÇEK SÜRE VERİSİ HİÇ YOKMUŞ.** `work_events` zaman damgalarında
   aynı ürün için `meta_done → ideasoft_push` farkı **0,6 dakika** — bu işin değil
   **gönderimin** süresi. Olay günlüğü yalnızca uç noktaları yakalıyor; kullanıcının düşünme
   ve düzenleme süresi hiçbir yerde kayıtlı değildi. Seans **duvar saati** süresini ölçüyor.

   **Kalibrasyon (fazın çıktısı):**
   - **MEDYAN, ortalama değil.** Test sabitliyor: `[3,4,4,5,40]` → ortalama 11 (yanlış),
     medyan 4 (doğru). Duvar saati ölçtüğümüz için "çayı tazeledim" kesintileri kural.
   - **Kova başına en az 5 örnek**; azsa tahmin yerinde kalıyor.
   - **Yalnızca bitirilen işler.** Atlanan iş "ne kadar sürdüğü" bilgisi taşımıyor; listeye
     girerse süreyi olduğundan kısa gösterir. Yarım kalan seans (`abandoned`) da girmiyor.
   - Ekran ikisini ayırt ediyor: **"≈2 dk"** (tahmin) ↔ **"4 dk"** (ölçüldü).

   **Çubuk KABUKTA (App.vue) yaşıyor** çünkü iş başka ekranlarda yapılıyor: meta Ürünler'de,
   canonical Satışta olmayanlar'da. Ekrana bağlı olsaydı kullanıcı işi yapmaya gittiği anda
   sayacı ve "Bitti"yi kaybederdi. Altı ekranda da durduğu ölçüldü.

   ⚠️ **Sayaç ön yüzde ama ÖLÇÜM arka uçtaki zaman damgalarından.** Uygulama uyusa ya da
   kapansa bile ölçüm bozulmuyor. Açılışta yarım seans kapanıyor, açık madde `abandoned`.

   🚫 **Oyunlaştırma yasağı koda yazıldı:** XP · lig · seri cezası · konfeti · "Harikasın! 🔥"
   yok. Özet sakin bir bilanço; Ayarlar'da kullanıcıya da "puan/rozet/seri tutmuyor" deniyor.

   🔴 **SAHA HATASI: kilitlenecek iş yokken de seans açılıyordu.** Kullanıcı günün 10 işinin
   10'unu bitirmişti; "Odak seansı başlat" deyince karşısına *"Seans bitti · 0 iş · 0 dk"*
   modali çıkıyordu — bir başarısızlık gibi okunuyor, oysa iyi haber. Daha kötüsü: kod **önce
   kaydı açıp** sonra kilitleyecek iş bulamayınca kapatıyordu, yani her denemede sıfır
   saniyelik bir seans kaydı birikiyordu — **veritabanında 8 tane bulundu.** Düzeltme: önce
   bak, sonra kaydet (`has_lockable_item`; düğme iş yokken pasif ve sebebini söylüyor) +
   açılışta maddesiz seans artıklarının temizliği.
   **Ders: "hiç iş yok" bir hata hâli değil, geçerli bir sonuç — öyle karşılanmalı.**

   ⚠️ Bunu bulurken `today.rs` yeniden kullanılabilir hâle geldi (`build_today_queue(conn)`):
   seans ile ekran **aynı** kuyruğu okuyor. İki ayrı kurulum olsaydı kilitlenen iş ile
   listedeki liste ayrışabilirdi.

0ay. 🔴 **KOYU PENCERE ÇERÇEVESİ — "çağırdım, olmalı" üç kez yanılttı (2026-08-08).**
   Kullanıcı isteği basitti: koyu temada pencere çerçevesi de koyu olsun. `set_theme` çağırıp
   "oldu" dedim; **olmadı**. Uygulama çalıştırılıp ekran görüntüsü alınınca üç ayrı yanılgı
   çıktı, her biri bir öncekini çözünce göründü:

   1. **`set_theme` tek başına hiçbir şey yapmıyor.** Tanılama şaşırtıcıydı: pencere bulunuyor,
      `Ok` dönüyor, `w.theme()` geri **Dark** okuyor — ama çubuk bembeyaz. Gördüğümüz beyaz
      NSAppearance değil, **pencerenin ZEMİN rengiydi**; macOS başlığı zeminin üstüne çiziyor
      ve zemin varsayılanı beyaz. → `set_background_color`.
   2. **Zemin koyu oldu, başlık YAZISI koyu kaldı** (kullanıcı yakaladı). Tauri'nin `set_theme`i
      macOS'te **NSApp** görünümünü değiştiriyor, pencerenin kendisininkini değil. → `objc2` ile
      doğrudan `NSWindow.setAppearance(DarkAqua)`.
   3. **Görünüm doğru olduğu hâlde başlık yine koyu çizildi.** `appearance()` ve
      `effectiveAppearance()` ikisi de DarkAqua okuyor. → Zorlamak yerine soru değişti: bu yazı
      gerekli mi? **Değil** — uygulama adı kenar çubuğunun tepesinde. `hiddenTitle: true`.

   **Ders: platform API'si "Ok" döndürdü diye iş bitmiş sayılmaz; ekrana bakılmalı.** Bu
   oturumda üç tur boşa gitmesinin sebebi buydu.

0aw. 🔴 **İSİMSİZ İKON, SIRADAKİ ADIMI GİZLER — Faz B'nin gecikmiş bedeli**
   (saha geri bildirimi 2026-08-07, düzeltme **v0.11.0**).
   Kullanıcı "Satışta olmayanlar'da halef seçim modali gelmiyor, seçim yapılamıyor" dedi ve
   bunu bir kod çakışması sandı. **Modal çalışıyordu** — harness'te her iki yolda da açıldığı
   ölçüldü (öneri varsa onay modali, yoksa hedef seçme modali). Bulunamıyordu.

   Sebep: Faz B'de etiketli **"Hedef seç ve ayarla"** düğmesi isimsiz bir zincir ikonuna
   çevrildi. Eski akışta "Halef öner" düğmesi sonuç gelince YERİNİ etiketli bir sonraki-adım
   düğmesine bırakıyordu; yeni tabloda iki ikon da hep orada duruyor ve öneri geldiğinde
   İşlem sütununda **hiçbir şey değişmiyor**. Ders: ikon sütunu eylemleri *taşır*, ama
   **"şimdi şunu yap" sinyalini taşımaz**.

   Çözüm: `TableRow.subAction` — sonucun YAZDIĞI ikincil satır tıklanabilir hâle geliyor
   ("Uygun halef bulunamadı — hedef seçin"). Sonucun göründüğü yer, eylemin de durduğu yer.

   Aynı ekranda iki kusur daha çıktı, ikisi de Faz B'den eski:
   - **Tek `successorBusy` dizesi "hepsini pasifleştir" diye okunuyordu.** Ölçüldü: bir satıra
     basınca **50 düğmenin 25'i** (her satırın halef düğmesi) sönüyordu. Durum URL başına
     kaydırıldı → ölçüm 25 → **1**. Kota koruması isteğe bağlılıktır, çağrıları sıraya dizmek değil.
   - **"Halefi yeniden öner" ipucu yalan söylüyordu:** `successors[url]` doluysa fonksiyon
     sessizce dönüyordu. Artık gerçekten yeniden istiyor.

0av. ✅ **ASİSTAN BAĞLAM SEÇİMİ (v0.11.0, Faz A).** Asistan artık son gezilen ekrana kilitli değil.
   Girişin solundaki **"+"** menüsünden **7 kaynak** seçiliyor (5 SEO aracı + Sonuçlar +
   Katalog), seçim **sohbetin ortasında değiştirilebiliyor** ve seçili kaynaklar çip olarak
   görünüyor. Eski hâl kendini ekranda itiraf ediyordu: *"başka bir aracın verisini sormak
   için önce o ekrana gidin"*.

   Kaynaklar **veri-güdümlü bir kayıt tablosunda** (`src/assistantSources.ts`); `store.ts`
   içindeki `switch (lastToolPage)` kalktı. Yeni kaynak eklemek = tabloya bir girdi.

   🔬 **Bağlam bütçesi ölçüldü (gerçek veri, 2026-08-07):** beş kaynağın tamamı 50'şer
   satırla **27.000 karakter (~6.750 token)** — plandaki "kota bozulur" endişesi ölçümle
   yumuşadı. Yine de üst sınır kondu: `clamp(floor(150 / kaynakSayısı), 20, 50)`.
   Ölçülen sonuç: 1 kaynak 9.028 · 3 kaynak 20.253 · 5 kaynak 17.417 · 7 kaynak 18.731 karakter.
   ⚠️ **En kötü durum 7 kaynak DEĞİL, 3 kaynak** (~5.063 token): 1–3'te bugünkü 50 satır
   bilinçli olarak korunuyor, ancak 4'ten sonra bütçe paylaşılmaya başlıyor.

   ⚠️ **Dürüstlük ayrımı prompt'a yazıldı: "seçili değil" ≠ "veride yok".** Yüklü olmayan bir
   ekran sorulduğunda model "bu veride yok" derse kullanıcıyı yanıltır — veri duruyor, yalnızca
   yüklenmemiş. Doğru cevap eylem içeriyor: "+ menüsünden ekleyin".

   **Katalog kaynağı sayaç + eksik listesidir, rastgele dilim değil.** Ölçüm: 279 üründen
   meta'sı eksik 208, açıklaması 211, tekniği 228. 279'un rastgele 21'ini göndermek "hangi
   üründe ne eksik" sorusunu cevaplayamazdı.

   **Kalıcılık şema değiştirmeden:** `chat_sessions.tool_page` artık **virgüllü liste**.
   Gerçek veritabanındaki eski tek değerli kayıtlar (`eol`, `striking`) tek elemanlı liste
   olarak okunuyor; tanınmayan anahtar eleniyor ve varsayılana düşülüyor — dördü de denendi.
   ⚠️ `UPDATE`e de eklendi: seçim sohbet ortasında değişiyor, sadece `INSERT`te yazmak yetmezdi.

   ✅ **Dördüncü kopya-geometri hatası baştan engellendi.** `.chip`/`.chip-count` `SeoTable.vue`
   içindeki scoped tanımdan **`styles.css`'e global taşındı**; asistan kopyalamak yerine aynı
   tanımı kullanıyor. (Bu oturumda kopyalanan ölçülerin saptığı üç kez ölçülmüştü.)

   🔴 **Popover kontrol ettiği şeyin üstünü örtüyordu.** Menü "+" düğmesine çapalanınca
   seçili çipleri kapatıyordu; sabit bir kaydırma da çipler iki satıra sardığında yetmezdi.
   **Yapısal çözüm:** çipler + giriş tek `.dock` kabına alındı, menü `bottom: calc(100% + 8px)`
   ile o kaba çapalandı. Ölçüm: menü altı 602px, çipler 610px'te başlıyor.

0au. ✅ **ÖLÇÜM OMURGASI (v0.10.0, Faz Ö).** "İşe yaradı mı?" sorusu artık cevaplanıyor.
   `metric_snapshots` + `metric_page_rows` (değiştirilemez GSC anlık görüntüleri) +
   `work_events` (ne yaptık, ne zaman). Sonuç **saklanmıyor, istendiğinde hesaplanıyor**
   (`core/src/metrics.rs::outcome`) — saklanan sonuç yeni görüntü gelince bayatlar.

   🔑 **Merkezi kural: ölçülen olay, mağazaya ULAŞAN olaydır.** Yerel "tamamlandı"
   işaretlemesi Google'ın gördüğünü değiştirmiyor; yalnızca `ideasoft_push` ve
   `canonical_set` puanlanıyor. ⚠️ Yan etkisi dürüstçe söyleniyor: içeriği elle kopyalayıp
   yapıştıran kullanıcı için olay oluşmuyor → o ürün "ölçülemiyor" diyor, sessizce boş
   kalmıyor.

   🔬 **Dört ölçüm, dördü de tasarımı değiştirdi (2026-08-07):**

   1. **Yerel damgalar geriye dönük dolum için YETERSİZ.** Yol haritası "mevcut damgalardan
      doldurulabilir" diyordu; gerçek: `*_history_json` **toplam 4 girdi**, `updated_at` tek
      ve değişken alan. Gerçek olay damgası taşıyan tek alan `ideasoft_pushed_at`.
   2. ✅ **Ama geçmiş GSC'de var:** `page_stats_range` ile **17 ay** geriye veri geliyor,
      28 günlük pencere 1,3–2,2 sn. 12 pencere **30,7 saniyede** tohumlandı.
   3. **Satır eşiği ölçülerek seçildi:** `tıklama > 0 VEYA gösterim ≥ 10` → satırların %34'ü,
      tıklamaların **%100'ü**. Gerçek tohumlama: 35.062 satır, **DB 6,2 → 13,6 MB**
      (planda +2,9 MB tahmin edilmişti; eski pencerelerde satır sayısı daha yüksek çıktı).
   4. 🔴 **Çakışan pencere tuzağı:** ilk tasarım "son görüntü ≥7 günse yenisini al" diyordu.
      28 günlük pencerelerle bu **%75 çakışan** pencereler ve yılda ~18 MB demek. Pencereler
      **döşendi** (`next_window`): yılda ~5 MB ve iki pencere kıyaslaması elma-elma.

   ⚠️ **İlk çalıştırmada 72 olayın hepsi "ölçülüyor" çıktı — kusur değil, gerçek.** Gönderimlerin
   tamamı 26 Temmuz–7 Ağustos arasında yapılmış; takip penceresi için gönderimden 21 gün sonra
   BAŞLAYAN bir pencere gerekiyor, o pencere Eylül ortasında oluşacak. Eşik kalibrasyonu
   (iyileşti/geriledi sınırları) bu yüzden gerçek dağılımla değil, gerekçeli varsayılanla
   bırakıldı: **ölçülecek veri oluşunca kalibre edilmeli.**

   ⚠️ Eşikte **iki koşul birden** aranıyor (≥%20 VE ≥3 tıklama): yalnızca oran bakılsaydı
   1→2 tıklama "%100 iyileşme", yalnızca mutlak fark bakılsaydı 400→403 "iyileşme" sayılırdı.

   🔴 **SÜTUN HİZASI BOZULDU — yapısal ders (saha geri bildirimi, aynı gün).**
   Yatay taşmayı çözmek için izleri `min-content`/`max-content` tabanına çevirmiştim. Sonuç:
   tablo satırdan satıra kaydı. Sebep yapısal — **her satır KENDİ grid konteyneri**, bu yüzden
   içeriğe bağlı bir iz her satırda o satırın içeriğine göre çözülüyor. Ölçüm: başlık x=512,
   satırlar x=526 ve x=534 (üç farklı hizalama).

   **Kural (bileşende yazılı): hiçbir grid izi içeriğe bağlı olamaz** — yalnızca sabit px, %
   ve fr her satırda aynı çözülür. Rozet sütunu 132px sabit ("Dokunulmamış" 104px + 24px
   dolgu), sayısal sütun `minmax(76px, 1fr)` ("Gösterim" başlığı 75px).

   ⚠️ Bedeli bilinçli kabul edildi: 10 sütunlu Fırsatlar dar pencerede yatay kaydırma açıyor.
   **Hizasız bir tablo, kaydırmalı bir tablodan daha kötü.**

   🔴 **Faz B'de kaldırdığım `min-width` modeli geri geldi ve yine ısırdı.** Fırsatlar 10.
   sütuna (Sonuç) çıkınca elle hesaplanan toplam 972px dedi, gerçek içerik 865px'ti ve ekran
   boşuna yatay kaydırma açtı. **Kaldırıldı:** grid izleri zaten `min-content` tabanlı, ikinci
   bir model tutmak gereksiz ve yanlış. Ders: aynı kısıtı iki yerde modelleme.

0at. ✅ **ARAÇ EKRANLARINDA STİL SAPMASI — ölçüldü ve kapatıldı (Faz B, v0.9.0).**
   Kalem 1/2'de ürün kartlarında düzeltilen hastalığın aynısı, bu kez SEO araç ekranlarında:

   | Ne | Durum |
   |---|---|
   | `.tbl` kullanan | 4 ekran (Fırsatlar · Satışta olmayanlar · Yükselmeye yakın · Düşüşte) |
   | **Yapı olarak farklı** | **Yarışan sayfalar** tablo değil, `div` listesi (`.cann-list`) |
   | `.head` yerel tanım | **5 ekranda** ayrı ayrı |
   | `.note` / `.more` | 4 / 3 ekranda kopyalanmış |
   | **Sessiz sapma** | `.sku` → Düşüşte/Striking `margin-top: 2px`, Fırsatlar **1px** |

   ⚠️ Ayrıca **satır eylemi görünmez:** dört ekranda birincil eylem çıplak satır tıklaması
   (`store.openProduct`); yalnızca EOL'de görünür düğme var, o da hücreye gömülü yerel
   `.succ-btn`. Kullanıcı tespiti: *"tasarım bütünlüğü için tablo/liste ekranları aynı ya da
   birbiri ile tutarlı olmalı"* + işlem sütunu önerisi.

   **Çözüm:** `tools/SeoTable.vue` — tasarım handoff'undan (Claude Design "Tablo Sistemi")
   birebir. Sütun türleri: metin · sayı · yüzde · rozet · **değişim** (önce→sonra) · işlem.
   `<table>` değil **CSS grid**: sütun genişlikleri tür bazlı `minmax`, gruplu varyantta
   başlık satırı tüm genişliği kaplıyor — tabloyla ikisi de zor.

   🔬 **Doğrulama (tarayıcıda ölçüldü):** beş ekranda başlık hücresi `9px 12px / 11px / 620`,
   veri hücresi `9px 12px / 12.5px`, ikincil satır üst boşluğu `1px`, **işlem sütunu tam
   156px** — hepsi birebir aynı. Araç ekranlarında tablo/satır geometrisi tanımlayan yerel
   CSS kuralı **sıfır**; `styles.css`'teki global `.tbl` bloğu da silindi.

   ⚠️ **İşlem sütununa yalnızca BUGÜN VAR OLAN eylemler bağlandı** (ürünü aç · halef öner ·
   canonical). `review`/`queue`/`redirect` bileşende tanımlı ama hiçbir ekranda kullanılmıyor:
   olmayan özellik için kalıcı pasif düğme göstermek gürültüdür. Faz Ö/K/D geldiğinde ekranlar
   yalnızca `actions` dizisine bir anahtar ekleyecek.

   🔴 **Faz B sonrası saha geri bildirimi (aynı gün) — üç kusur, üçü de ölçülerek bulundu:**

   1. **Çip sayıları iki farklı birim taşıyordu.** Kategori/marka çipleri *kaçırılan tıklama*,
      iş durumu/sebep çipleri *ürün adedi* gösteriyordu — görünüşleri aynı. Kullanıcı
      "Creality 24" görüp tıklıyor, 5 satır geliyordu. Artık **hepsi ürün adedi**; kaçırılan
      tıklama ipucunda, sıralama ölçütü olarak korunuyor. Dört çipte rozet = satır sayısı
      olarak doğrulandı.
   2. 🔴 **Hayalet dikey kaydırma çubuğu — CSS tuzağı.** `overflow-x: auto` yazınca tarayıcı
      `overflow-y`'yi de `auto` yapıyor (spec: bir eksen `visible` değilse diğeri de olamaz).
      Yatay çubuğun kapladığı 17px dikey taşma doğuruyor → **ikinci çubuk**. Çözüm:
      `overflow-y: hidden` açıkça yazılmalı. Ölçüm: 28px yatay taşma → 17px dikey taşma.
   3. **Üçüncü kaydırma:** "Google'da görünmeyenler" listesi kendi `max-height: 260px`
      kaydırmasını taşıyordu (içerik 575px) ve Faz B'de bileşene alınmamıştı. Ortak bileşene
      geçirildi.

   ⚠️ Ayrıca sayısal sütunların sabit 78px alt sınırı Fırsatlar'da (9 sütun) 12px taşma
   yaratıyordu. Ölçüldü: başlıklar "CTR" 47px, "Konum" 62px, "Gösterim" 75px istiyor →
   alt sınır **`min-content`** yapıldı, `1fr` artan yeri eşit dağıtıyor. Yatay taşma 0.

   **Ders (üçüncü kez aynı):** kopyalanan geometri sapıyor. Yeni ekran açarken ortak bileşen
   yoksa önce o yazılmalı — sonra toplamak iki kat iş.

0as. ✅ **TEKNİK TABLO IDEASOFT'TAN GETİRİLİYOR (v0.8.3).** "Getir" düğmesi hedef kelimeyle
   birlikte `extraDetails` (IdeaSoft'un "Teknik Özellikler" sekmesi) içeriğini de alıyor,
   `tech_html_to_text` ile düz metne çevirip `tech_source_text`e yazıyor → "Yapılandır" ile
   düzenli tabloya dönüşüyor.

   🔬 **Ölçüm — özelliğin değerini bu sayı gösteriyor (2026-08-07):** en yeni 150 üründen
   **satışta olan 90'ının 85'inde (%94)** IdeaSoft'ta teknik tablo VAR. Uygulamada ise
   yalnızca 14 üründe tablo vardı. **Bu veri XML feed'de hiç gelmiyor** — uygulamanın
   göremediği, mağazada hazır duran gerçek içerik.

   🔴 **Kendi ölçüm hatam — ders:** ilk turda `/admin-api/products/{id}` yanıtında
   `details`/`extraDetails` anahtarlarını **üst düzeyde** aradım, bulamayınca "API bu alanları
   dönmüyor" sonucuna vardım ve ayrı bir uç (`product_details`) için kod yazdım. Gerçekte
   alanlar **`detail` (tekil) nesnesinin içinde** ve `fetch_product` bunu zaten okuyordu
   (`to_remote_reads_nested_detail` testi tam bu yüzden var). Fazladan uç ve fazladan HTTP
   isteği silindi. **Ders: "alan yok" demeden önce yanıtın TÜM yapısına bakılmalı; iç içe
   nesneler üst düzey anahtar aramasıyla kaçırılıyor.**

   ⚠️ **İki tablo biçimi var, ikisi de destekleniyor (ölçüldü):**
   - bizim ürettiğimiz: `<caption>Grup</caption>` + `<th>etiket</th><td>değer</td>`
   - mağazanın kendi yazdığı: `<thead><th>Grup</th></thead>` + `<td>etiket</td><td>değer</td>`
     — dokunmadığımız 6 üründen 6'sı bu biçimdeydi, yani **asıl yaygın olan bu**.
   Düz `html_strip` kullanılamaz: ayraç olmadan "PanelIPS" çıkar ve yapılandırıcı satırı
   ayıramaz. `tech_html_to_text` "Etiket: Değer" satırları üretiyor (5 test + canlı test).

   ⚠️ **Dolu kaynak metnin üzerine YAZILMIYOR** — orası kullanıcının elle yapıştırdığı ham
   veri olabilir. Boşsa yazılıyor, doluysa korunuyor ve hangisi olduğu kullanıcıya
   söyleniyor (mesaj arka uçta kuruluyor: "yazıldı / korundu / IdeaSoft'ta yok" ayrımını
   yalnızca orası biliyor).

0aq. ✅ **FEED DEĞİŞİKLİĞİ TESPİTİ (kod v0.8.1, yayın v0.8.2 — K2).** Senkronda ürünün *üretimi besleyen*
   alanlarından parmak izi alınıyor (`products.feed_fp`); kullanıcı "tamamlandı" dediğinde o
   anki iz damgalanıyor (`seo_status.reviewed_fp`). İkisi ayrışırsa ürün *"feed verisi
   onayınızdan sonra değişti"* diye işaretleniyor. Mantık: `core/src/fingerprint.rs`.

   🔬 **Ölçüm önce yapıldı ve tasarımı değiştirdi (254 ürünlük gerçek feed):**

   | | bayraklanan |
   |---|---|
   | ham karşılaştırma | **8** |
   | boşluk normalize | **1** |

   7'si sahte pozitifti: feed `\r\n`, veritabanı `\n` kullanıyor (ham baytta doğrulandı,
   189 tane). Normalizasyon olmasaydı **özellik ilk senkronda 7 yanlış bayrakla açılacak** ve
   kullanıcı bayrağa güvenmeyi bırakacaktı. Kalan 1 gerçek: `SW.ARB.JL686B`'nin açıklaması
   baştan yazılmış (`<section>` → `<div>`).

   ⚠️ **Hangi alanlar İZE GİRMİYOR ve neden:**
   - `quantity` (stok) — üretimi beslemiyor; girseydi her stok hareketinde tüm katalog
     bayraklanır, bayrak anlamsızlaşırdı. Ölçümde 0 değişiklik zaten.
   - `title`/`keywords`/`descriptions` — bunlar mağazadaki MEVCUT SEO alanları, yani üretimin
     girdisi değil **rakibi**. Değişmeleri "kaynak veri değişti" demek değil.

   ⚠️ **Üç tuzak, üçü de teste bağlandı:**
   1. **Yedekleme:** `feed_fp`/`reviewed_fp` yedeğe girmezse geri yüklemenin ardından ilk
      senkronda **onaylı her ürün yanlış bayraklanır** (iz yeniden hesaplanır, damga eski
      kalır). Sessiz değil, gürültülü hasar. → `yedek_parmak_izi_ve_onay_damgasini_tasiyor`.
   2. **Taban çizgisi:** özellik gelmeden önce "tamamlandı" olan ürünlerin damgası yok →
      hiç bayraklanmazlar. Senkron sonunda tek seferlik damga basılıyor (yalnızca
      `reviewed_fp IS NULL` olanlara → tekrarı zararsız). Bedeli: adaptasyon senkronunda o
      1 gerçek değişiklik yutuluyor — kaçınılmaz, çünkü "onay öncesi hâl" hiç kaydedilmemişti.
   3. **Not birikimi:** kullanıcı bakmadan iki değişiklik gelirse not ÜSTÜNE YAZILMAMALI;
      "onaydan beri değişenler" birleştiriliyor → `onaydan_beri_degisen_alanlar_birikir`.

   🔬 **Gerçek veritabanı kopyası + canlı feed ile uçtan uca doğrulandı** (`sync_fingerprint_real`,
   `--ignored`): 256 ürün, 56 damgalı, **iki turda da 0 bayrak**. Damga kasten bozulduğunda
   bayrak doğru alan listesiyle çıkıyor (`ENT.GNX.1002110098 → ad, açıklama`).

   🔴 **SAHA GERİ BİLDİRİMİ (v0.8.3'te giderildi):** uyarı yalnızca alan ADINI söylüyordu
   ("görseller değişti") ve yanında **"İçerik hâlâ doğru"** düğmesi duruyordu. Kullanıcının
   haklı sorusu: *"içeriği nasıl kontrol edeceğim hakkında bir belirsizlik var."*
   **Karar vermesi istenen kişiye kararın dayanağı da verilmeli.**

   Çözüm: onay anındaki alan değerleri `seo_status.reviewed_facts_json` alanına yazılıyor
   (`mark_reviewed`), `get_feed_diff` komutu bunu şu anki feed'le karşılaştırıyor.
   - Parmak izi **"değişti mi?"**, bu kayıt **"NE değişti?"** sorusunu cevaplıyor. İz geri
     döndürülemez bir özet olduğu için tek başına karşılaştırmaya yetmiyor.
   - Açıklamada HTML değil **metin** karşılaştırılıyor (`validation::html_strip`) — kullanıcı
     etikete değil içeriğe bakıyor.
   - Görseller metin farkı üretmiyor; çıkanlar/gelenler küçük resim olarak gösteriliyor.
   - ⚠️ **Onay kaydı olmayan ürünlerde uydurma YOK.** Özellikten önce onaylanmış ürünlerde
     eski değerler kayıtlı değil; arayüz bunu açıkça yazıyor. Tarayıcı denemesinde bir kusur
     çıktı ve düzeltildi: kayıt yokken tüm görseller "GELENLER" diye etiketleniyordu — yani
     bilmediğimiz bir şey iddia ediliyordu. Artık o durumda yalnızca "şu anki görseller".
   - 📐 **Boşluk ölçüldü:** şeridin üstünde 0, altında 16px vardı (başlığa yapışıktı).
     `margin: 16px 0 0` → iki tarafta da 16px; alttaki zaten sonraki satırın margin'inden
     geliyor, margin'ler birleşiyor. *"Boşluk da bir özelliktir"* (kullanıcı).

   Ölçüm: kullanıcının veritabanında güncelleme sonrası **14 ürün bayraklandı** (9 görsel,
   5 açıklama) — özellik sahada çalışıyor.

   Arayüz: liste satırında "Değişti" rozeti *"Tamamlandı"nın YERİNE* (ikisi yan yana çelişkili
   mesaj olurdu), sayacı sıfırken gizlenen "Feed değişti" filtresi, detayda uyarı şeridi +
   **"İçerik hâlâ doğru"** düğmesi (`mark_feed_reviewed`). O düğme olmasaydı kullanıcı bayraktan
   kurtulmak için "tamamlandı"yı kapatıp açmak zorunda kalırdı — durumu yalan söylemeye zorlayan
   bir çözüm. Harness kipi: **`?changed=1`**.

0ao. 🔴 **VARSAYILAN FEED ADRESİ TEK BİR MAĞAZAYA GÖMÜLÜYDÜ (v0.8.0'da kaldırıldı).**

   K1 (kurulum sihirbazı) keşfinde çıktı ve sihirbazdan daha önemliydi:
   ```rust
   pub const DEFAULT_FEED_URL: &str = "https://www.kurumsalit.com/output/2567783262";
   ```
   `db::feed_url()` ayar yoksa buna düşüyordu. Uygulamayı kuran **herhangi bir işletme**
   varsayılan olarak başka birinin feed'ini alıyor, "Manuel Güncelle"ye basarsa **başka bir
   mağazanın kataloğunu senkronluyordu.** Hem vizyonun açık ihlali (*"kullanıcıya özel değer
   gömmeden, ayarlanabilir olmalı"*) hem de ilk çalıştırmada somut bir veri kazası.

   Sabit kaldırıldı → `feed_url()` boş dönüyor, `sync_feed` anlamlı hata veriyor.
   **Ders: "global kullanım" kararı yalnızca yeni özellikler için değil, MEVCUT
   varsayılanlar için de geçerli.** Vizyon maddesi yazılmadan önceki kod bu gözle taranmalı.

0ap. ✅ **KURULUM SİHİRBAZI (v0.8.0).** hoş geldiniz → feed (test) → Gemini (test) →
   isteğe bağlı entegrasyonlar → ilk senkron → özet. **Yeni yetenek EKLEMİYOR**: mevcut
   `test_*` komutlarını doğru sıraya diziyor. Atlanabilir; Ayarlar'dan tekrar çalıştırılır.

   `needs_setup` **üç koşul birden** arıyor (setup_done yok + feed_url yok + ürün yok).
   ⚠️ Korunan risk yanlış pozitif: mevcut kullanıcı yükseltme yaptığında çalışan bir kuruluma
   sihirbaz teklif etmek. Şema göçü ve geriye dönük yazma YOK — durum baştan hesaplanıyor.

   ⚠️ **İki tuzak (keşifte bulundu, ikisi de doğrulandı):**
   1. **Test-önce-kaydet asimetrisi:** `test_feed_url`/`test_gemini_key` **parametre** alıyor →
      kaydetmeden test edilebiliyor. Ama `test_ideasoft`/`test_gsc_credentials` **DB'den
      okuyor** → önce kaydetmek gerekiyor. Zorunlu adımlarda yarım kayıt oluşmuyor; isteğe
      bağlı adımda "kaydet, sonra test et" ve kullanıcıya kaydedildiği söyleniyor.
   2. 🔴 **`save_settings` YEDİ alanı birden alıyor** → dokunulmayan alanlar mevcut
      değerleriyle geri yazılmazsa kullanıcının kayıtlı anahtarları **SİLİNİR**. Sihirbaz
      Ayarlar'dan tekrar çalıştırılabildiği için gerçek risk. Tarayıcıda doğrulandı.
      **Aynı tuzak `SettingsPage.persist()` için de geçerli** — oraya yeni alan eklenirse
      sihirbazın `persist()`'i de güncellenmeli.

   Harness'e **`?setup=1`** kipi eklendi: sihirbaz kullanıcının GERÇEK veritabanına
   dokunmadan uçtan uca denenebiliyor (yazan komutlar yutuluyor).

0al. 🔴 **CANONICAL HEDEFİ SATIŞTAKİ ÜRÜN OLMALI — saha hatası (v0.7.2, kullanıcı tespiti).**

   Elle hedef seçme modali IdeaSoft'un **tam kataloğunda** arıyordu (bu mağazada 10.909 ürün)
   ve satıştan kalkmış ürünleri de listeliyordu. Oysa akışın amacı tam tersi: satışta OLMAYAN
   ama trafik alan bir sayfayı **satıştaki** bir ürüne yönlendirmek. Ölü sayfayı başka bir ölü
   sayfaya işaret ettirmek sorunu taşımak olurdu.

   ⚠️ **Asıl kusur TUTARSIZLIKTI.** Yapay zekâ halef önerisi (`suggest_eol_successor`) zaten
   `products` tablosunu kullanıyordu — feed = satıştaki ürünler (kullanıcı beyanı: *"Güncel
   olan ürünler xml de gelen ve sitede satışta olan ürünlerdir"*). Aynı karara giden iki
   yoldan biri kısıtlı, diğeri değildi.
   **Ders: bir kısıt varsa o karara giden HER yol ondan geçmeli.**

   - `search_catalog` → **`search_live_products`**. Ad da düzeltildi: "katalog" IdeaSoft'un
     tümünü çağrıştırıyordu ve hata tam bu karışıklıktan çıktı. Arama artık yerel (ağsız).
   - `preview_canonical` VE `apply_canonical` hedefi doğruluyor → arayüz atlansa bile yazma
     reddediliyor.
   - ⚠️ Eşleştirme SQL `LIKE` ile YAPILMAZ: slug'daki `_` joker olur ve yanlış ürünü
     eşleştirir. Feed birkaç yüz satır → tam tarama bedava. Test sabitliyor.
   - Doğrulandı: "lenovo" → 25 satıştaki ürün · feed dışı "zenbook 17 fold" → **0 sonuç**.

0am. ✅ **SOHBET GEÇMİŞİ KALICI (v0.7.2).** `chat_sessions` tablosu; mesajlar JSON sütunda
   (projenin mevcut `*_history_json` idiomu). Her turdan sonra kaydediliyor — **hata alan tur
   da dahil**, kullanıcı ne sorduğunu ve ne olduğunu sonradan görebilmeli. Başlık ilk sorudan
   türüyor; ⚠️ başlık için modele AYRI ÇAĞRI YOK (her sohbet bir tur fazladan kota harcardı).
   "Sohbeti temizle" → **"Yeni sohbet"**: eski ad yanıltıcıydı, gerçekten siliyordu.

   🔴 **YENİ TABLO EKLERKEN YEDEKLEMEYİ UNUTMA.** `export_json`/`import_json` tabloları ELLE
   sayıyor. `chat_sessions` eklenmeseydi geri yüklemede **sessizce kaybolurdu — hata bile
   vermezdi**. Sohbet, teknik tablo gibi yeniden üretilemeyen kullanıcı emeği. İki test bunu
   koruyor: yuvarlak yolculuk + sohbet bölümü olmayan ESKİ yedeğin kırılmadan yüklenmesi.
   (`ideasoft_catalog` bilinçli olarak yedeklenmiyor — tek komutla yeniden çekiliyor.)

0an. ⚠️ **ARAÇ EKRANLARI ÖNBELLEĞİ KENDİLERİ YÜKLEMELİ — iki kez yaşandı.**
   Yükselmeye yakın / Yarışan sayfalar / Düşüşte olanlar / Satışta olmayanlar ekranlarında
   `loadOpportunityCache` çağrısı YOKTU; yalnızca Fırsatlar ve Genel Bakış'ta vardı.
   Uygulamayı açıp doğrudan o ekranlardan birine giden kullanıcı *"Henüz analiz
   çalıştırılmadı"* görüyordu — oysa analiz veritabanındaydı. Aynı hata asistan ekranında da
   çıkmıştı. Çözüm: yükleme ortak kabuğa (`ToolShell`) taşındı, beş ekranı birden çözüyor.
   **Ders: "her ekranın yapması gereken" bir şey varsa kabuğa koy, ekrana değil.**

0ai. ✅ **v0.7.0 — HER ARAÇ KENDİ EKRANINDA + ASİSTAN (5 kalemlik toparlama).**

   Kullanıcı tespiti aynen: Fırsatlar ekranı *"inanılmaz uzun ve dağınık"*. 1343 satırlık tek
   sayfada 9 bölüm + 2 modal vardı. Desen kullanıcının kendi geliştirdiği
   [QueryGSC](https://github.com/mehmetakarim/QueryGSC)'den: her analiz kendi ekranında.

   **Kalem 1 — tekrar temizliği (`449ea7b`).** `ProductContext` kurulumu 3 yerden 1'e
   (`ctx_parts` + `CtxParts::as_context`). Teknik tablonun bilinçli "araştırma verisi alma"
   kararı artık `with_insights: false` ile çağrı yerinde GÖRÜNÜR. CSS'te `.spin` (9 dosyada
   birebir aynı), `.warn` (5), `.overlay` (3), `.icon-badge` (3) global'e alındı.
   ⚠️ **İki adlandırma tuzağı çıktı:** Ayarlar'daki `.warn` aslında uyarı değil soluk ipuçtu
   (global temeli miras alsaydı üç bilgi satırı uyarı rengine bürünürdü) → `.hint`.
   `SeoResearchPanel`'in `.overlay`'ı modal değil YAN PANEL; global kural ona
   `align-items:center` ve 24px dolgu enjekte ederdi → `.panel-overlay`.
   **Ders:** scoped kural yalnızca KENDİ BİLDİRDİĞİ özellikleri geçersiz kılar; global bir
   temel eklerken "bu sınıfı kim daha kullanıyor" sorusu sorulmalı.

   **Kalem 2 — ortak iskeletler (`969ae6c`).** `ModalShell.vue` + `AnalysisSection.vue`.
   Ölçülen sapma: aynı animasyon İKİ AYRI ADLA kopyalanmış (`upd` ve `push`), üçüncüsü zayıf
   varyant; z-index 60/70/70; kapat düğmesi ikisinde yok. ⚠️ Meşguliyet kilidi bilinçli
   olarak iskelete GÖMÜLMEDİ — "ne zaman kapanamaz" her modalde farklı koşul.

   **Kalem 3 — araç ekranları (`b235414`).** `navigation.ts`'e `group`; kenar çubuğu
   Katalog / SEO Araçları / Asistan / Sistem. ⚠️ Grup adı "SEO Ayarları" DEĞİL "SEO Araçları"
   ("Ayarlar" zaten sayfa). Altı ekran: Genel Bakış (giriş noktası, araç kartları kayba göre
   okunur) + 5 analiz. ⚠️ **Veri akışı DEĞİŞMEDİ**: tek `analyze_opportunities`, tek
   `opportunity_json`; ekranlar dilimleri okuyor.

   **Kalem 4 — modül bölme (`cc5284b`).** `gemini.rs` (2119) → 5 dosya, `commands.rs` (2619)
   → 7 dosya. Yöntem: 0b'deki tuzağa karşı ayrıştırma değil **satır aralığı**, ve betikte
   güvenlik ağı — her dolu satırın TAM OLARAK BİR sahibi olmalı. Bağımsız doğrulama: her iki
   bölmede de **KAYIP=0**. ⚠️ İki ad çakışması: `commands/history.rs` ↔ `seo_core::history`
   → `versions.rs`; `commands/ideasoft.rs` ↔ `seo_core::ideasoft` → `ideasoft_cmd.rs`.

   **Kalem 5 — Yapay Zekâ Asistanı (`0dba016`).** Aşağıda 0aj'de.

0aj. 🔴 **ASİSTAN: GEMMA AKIŞTA İÇ MUHAKEMESİNİ DE YAYINLIYOR (v0.7.0).**

   **Ölçüm (2026-07-30, `gemma-4-31b-it`, tek cümlelik soru): 19 `"thought": true` parçasına
   karşılık 3 cevap parçası.** Filtrelenmeseydi kullanıcı Türkçe cevap yerine İngilizce
   düşünce zinciri görürdü — naif bir akış uygulaması TAMAMEN BOZUK görünürdü.
   `gemini/chat.rs::parse_sse_line` bu ayrımı yapıyor, üç birim testi sabitliyor.
   İkincil sonuç: akışın ~%86'sı atıldığı için "token token yazma" hissi zayıf → düşünce
   parçaları da (içeriksiz) arayüze bildiriliyor, "düşünüyor…" göstergesi çiziliyor.

   ⚠️ **Sohbet kendi zincirini kullanıyor (`CHAT_CHAIN`), `MODEL_CHAIN`'i DEĞİL.** Flash'ın
   günlük 20 istek kotası uygulamanın ASIL işine ait; birkaç sohbet turu onu bitirirdi.
   Zincir `gemma-4-31b-it` ile başlıyor (14.400/gün). Bir test bu sıralamayı koruyor.

   **Halüsinasyon sınırı hem birim hem CANLI testli:** `assistant_system_prompt` üç kural
   koyuyor (yalnızca verilen veriye dayan / veride yoksa söyle / sayı uydurma) ve canlı test
   "geçen yılın cirosu" sorusuna modelin *"Bu bilgi paylaşılan **veride yok**."* dediğini
   doğruluyor. Prompt'un doğru yazılmış olması yetmez, modelin UYDUĞU görülmeli.

   ⚠️ **Asistan HİÇBİR ŞEY YAZMAZ** — canonical/meta/gönderim kendi onaylı akışlarında.
   ⚠️ **`v-html` YOK** (`MarkdownText.vue`): metin doğrudan modelden geliyor. Tarayıcıda
   doğrulandı — `<img onerror=...>` ve `<script>` metin olarak görünüyor, DOM'a düğüm
   eklenmiyor. `marked` gibi bağımlılık eklenmedi.

   **Altyapı:** Tauri `ipc::Channel` uygulamada İLK KEZ; reqwest'e `stream` özelliği,
   `futures-util` doğrudan bağımlılık (zaten Cargo.lock'taydı).

   **Doğrulamada iki gerçek hata çıktı:** (1) mesaj satırları `:class="m.role"` ile `model`
   sınıfı alıyordu ve model rozetinin `.model` kuralına takılıyordu — **sınıf adı veriden
   türetilmemeli**; (2) asistan ekranı önbelleği kendisi yüklemiyordu.

0ak. 🧪 **GÖRSEL DOĞRULAMA YÖNTEMİ DEĞİŞTİ — `scripts/harness.py`.**
   Önceden her doğrulamada elle statik HTML yazılıyordu. İki kusuru vardı: (1) işaretleme
   elle yazıldığı için gerçek bileşenden sapabiliyordu — harness "geçer" derken uygulama
   bozuk olabilirdi; (2) scoped stiller yüzünden her parçaya `data-v-<hash>` eklemek
   gerekiyordu ve hash her derlemede değişiyordu.
   Artık `dist/index.html`in başına Tauri IPC taklidi ekleniyor: **uygulamanın KENDİ
   bundle'ı** yerel veritabanındaki gerçek veriyle çalışıyor. Kullanım:
   ```
   npm run build && python3 scripts/harness.py && npx vite preview --port 4173
   # http://localhost:4173/harness.html          → gerçek veriyle
   # http://localhost:4173/harness.html?empty=1  → "analiz hiç çalışmadı" durumu
   ```
   ⚠️ `?empty=1` kipi bir yanlış-pozitiften doğdu: boş durumu store'u dışarıdan sıfırlayarak
   test ettim ve BEŞ EKRAN DA "boş" göründü — ama test geçersizdi, sayfalar açılırken
   önbelleği yeniden yüklüyor. Doğru kiple altısı da doğru mesajı gösteriyor.

0ag. 🔴 **CANLI MAĞAZAYA İZİNSİZ YAZDIM — bir daha olmayacak (2026-07-29).**
   IdeaSoft'un hangi HTTP metotlarını desteklediğini anlamak için `PUT/POST/PATCH
   /admin-api/seo_settings/1` isteklerini **boş gövdeyle (`{}`) canlı mağazaya** attım.
   PUT 200 döndü. `jsonValue` içeriği bozulmadı ama kaydın `updatedAt` alanı değişti
   (2022-02-21 → 2026-07-29T18:08:50). Kullanıcıya derhal bildirdim.

   **Kural:** üretim verisinde metot keşfi için YAZMA isteği atılmaz. Sırasıyla: (1) `OPTIONS`
   veya `Allow` başlığı, (2) belge, (3) kullanıcıya sor. "Boş gövde zararsızdır" varsayımı
   yanlıştı — API'nin boş gövdeyi nasıl yorumladığını bilmiyordum.

0ah. ✅ **SLUG → ÜRÜN: TEK İSTEKTE, SENKRONA GEREK YOK (v0.6.5 saha düzeltmesi).**

   Kullanıcı bildirdi: "Canonical ayarla"ya basınca *"Bu sayfa IdeaSoft kataloğunda
   bulunamadı, önce katalog senkronunu çalıştırın."* Kök neden yerel DB'den okunarak bulundu:
   `ideasoft_catalog` tablosu **0 satır** — 7 dakikalık senkron hiç tamamlanmamış. Asıl kusur
   uyarı değil **tasarımdı**: tek satır yazmak için tüm katalog ön koşuldu ve senkron ilerleme
   göstermediği için donmuş görünüyordu.

   **Ölçüm yolu değiştirdi.** IdeaSoft `?s=` aramasıyla ürün tek istekte bulunuyor:
   - ⚠️ **Arama ADA göre çalışır, SLUG'a göre DEĞİL.** `s=ergotron-lx-desk-monitor-arm` → 0 sonuç;
     `s=Ergotron LX Desk Monitor Arm` → aynı ürün. Slug bu yüzden sözcüklere çevriliyor.
     (`?slug=`, `?name=`, `?seoLink=`, `filter[slug]` hepsi **sessizce yok sayılıyor** —
     filtre uygulanmamış tam liste dönüyor, bu yüzden "çalışıyor" sanmak kolay.)
   - Arama **tüm** sözcüklerin geçmesini istiyor; uzun slug'da ad ile slug örtüşmediği için boş
     dönüyor. Merdiven: **tam → 6 → 4 → 3 sözcük**, ilk birebir eşleşmede durur.
   - **Ölçüm (25 EOL sayfası, gerçek mağaza): 25/25, 1,44 istek/satır.** Rust üzerinden canlı:
     kolay vaka 1,0 sn · en zor vaka 6,2 sn (4 istek). **7 dakika → 1-6 saniye.**
   - Senkron artık yalnızca hızlandırma. Çözülen slug yerel tabloya yazılıyor → 2. çağrı bedava.

   ⚠️ **Yaklaşık eşleşme YOK** (`pick_exact_slug`): yanlış ürüne canonical yazmak geri alınması
   zor bir SEO hatası; "bulamadım" her zaman daha güvenli. Test bunu sabitliyor.

   ⚠️ **Hedef de doğrulanıyor** ve onay ekranında hedefin **adı** gösteriliyor. Var olmayan bir
   sayfaya canonical yazmak hiç yazmamaktan kötüdür: Google'a "asıl sayfa şu" der, o sayfa 404'tür.

   **İkinci saha hatası:** halef bulunamayınca buton hiç çıkmıyordu → sayfa için hiçbir şey
   yapılamıyordu. Kullanıcı: *"Uygun halef bulunamasa bile canonical ayarlama imkânı sunulmalı."*
   Haklı — modelin halef bulamaması hedefin olmadığı anlamına gelmiyor, karar operatörde.
   Buton artık her satırda; öneri yoksa arama modali açılıyor, varsa onaydan "değiştir"le
   değiştirilebiliyor. Akış hâlâ tek satır, önizlemeli, açık onaylı.

   **Yan bulgu:** ölçülen 25 EOL sayfasının **tamamı IdeaSoft'ta status=1 (aktif)**. Bu sayfalar
   mağazada pasife alınmış değil — yalnızca XML feed dışında kaldıkları için görünmüyorlardı.

0af. ✅ **CANONICAL YAZMA — IdeaSoft'ta 301 YOK, mekanizma `seo_settings` (v0.6.4).**

   Kullanıcı sordu: "IdeaSoft API'miz var, yönlendirmeyi uygulama içinde organize edemez miyiz?"
   Rota haritası çıkarıldı (**401 = rota var, 404 = rota yok** — token'sız istekle ayırt edilir):
   - `redirects`, `url_rewrites`, `seo_redirects` ve 8 aday isim daha → **hepsi 404. 301 ucu YOK.**
   - `products.canonicalUrl` alanı **kullanılmıyor** (ilk 500 üründe tamamen boş).
   - Gerçek mekanizma **`seo_settings`** kaynağı: 9.350 kayıt, ürün başına bir tane.
     `POST /admin-api/seo_settings` → **201** (kaydı olmayan ürün için oluşturur) ·
     `PUT /admin-api/seo_settings/{id}` → **200** (kısmi günceller).
     Biçim: `urun/<slug>` — başında eğik çizgi yok, alan adı yok.

   ⚠️ **Canonical bir yönlendirme DEĞİLDİR** ve arayüz bunu açıkça yazıyor: ziyaretçi yine eski
   sayfaya düşer, yalnızca Google'a "asıl sayfa şu" sinyali gider. Gerçek 301 hâlâ panelden.

   **Kullanıcı kısıtı (aynen): "toplu bir işlem olmamalı, kullanıcının onayı ile, gerektiğinde
   ve tek tek."** Tasarım bunun etrafında: `apply_canonical` bilinçli olarak **liste almıyor** —
   imza toplu kullanımı zorlaştırıyor. `preview_canonical` hiçbir şey yazmaz, farkı gösterir;
   onay modali "şu an / olacak" ve kayıt oluşturulacaksa onu da söyler.

   ⚠️ **Test yakaladı:** `normalize_canonical` ilk sürümde koşulsuz `split_once('/')` yapıyordu;
   zaten göreli olan `urun/abc` girdisinde `urun` parçasını alan adı sanıp atıyor ve `abc`
   üretiyordu — sessizce çalışmayan bir canonical. Alan adı artık YALNIZCA şema (`http`/`https`)
   varsa atılıyor. 5 girdi biçimi testte sabit.

   **Katalog senkronu:** XML feed bilinçli olarak sınırlı (bu mağazada 10.909 üründen 262'si).
   `ideasoft_catalog` tablosu tüm kataloğu slug→id olarak tutar; EOL sayfalar ancak böyle
   eşleştirilebiliyor. ⚠️ ~7 dakika sürer (ölçüm: 300 ürün 12,2 sn, 40 istek/dk limiti).
   ⚠️ Liste ucundaki `stock` GÜVENİLİR DEĞİL (300 üründe hepsi 0, detay ucu 1.0 dönüyor) —
   saklanıyor ama **arayüzde gösterilmemeli**. Token günlük döner; Ayarlar'daki bir kez eskimişti.

0ad2. ✅ **DÜŞÜŞTE OLANLAR — trend (v0.6.3).** `page_stats_offset()` ile ikinci bir GSC çağrısı
   (önceki 90 gün), `find_decay()` tıklama/gösterim/pozisyon gerilemesini karşılaştırır.
   Gerçek veride **49 sayfa / 907 tıklama kaybı**; ilk 10'un 6'sı 3D yazıcı — kayıp rastgele
   değil, kategori kümeleniyor. Arayüzde önce/sonra değerleri yan yana.

0ae. ❌ **İKİ ÖZELLİK ÖLÇÜLEREK ELENDİ — kod yazılmadan.**

   - **Ahrefs zorluk verisi (v0.6.2'de düzeltildi, sonra bırakıldı):** `parse_difficulty` Faz 4'ten
     beri sessizce **hep 0** döndürüyormuş. Ahrefs o uçtan `difficulty` alanı değil SERP verisi
     dönüyor; `unwrap_or(0)` bunu "çok kolay" diye gösteriyordu. ⚠️ **Test vardı ama HAYALİ bir
     yanıt biçimine göre yazılmıştı** — bu yüzden yeşil kalırken üretim bozuktu. Ders: dış API
     testi **gerçek yanıttan** yazılır. Düzeltildikten sonra kapsama ölçüldü: **5 sorgunun 2'si**.
     Bu kapsamayla operatör kararı desteklenemez; özellik bağlanmadı.
   - **İç link adayı:** en güçlü sinyalde **0 çift**, gevşetilmiş eşikte **1 çift** (o da
     pillar→ürün). GSC link verisi vermiyor; bu zaten bir çıkarımdı ve gerçek veri çıkarımı
     desteklemedi. Sahip olmadığımız veriyi varmış gibi sunmamak ilkesi gereği bırakıldı.

0ab. ✅ **SORGU DÜZEYİ ANALİZLER ÇALIŞIYOR (v0.6.0).**
   `gsc.rs::query_page_stats` — `dimensions:["page","query"]`, `startRow` sayfalama (GSC tek
   istekte en fazla 25.000 satır), `page contains <yol>` filtresi (yol ürünlerden türetilir).
   **Ölçüm:** 24.204 satır · 4.764 sayfa · 21.634 sorgu · **2,5 saniye**, tek istekte sığdı.

   İki analiz kuruldu (`opportunity.rs`, saf mantık, 8 test):
   - **`striking_distance()`** — pozisyon 4–20, gösterim ≥30. QueryLoom'un aralığı.
     Gerçek veride **120 sorgu / 69 ürün**. Sorgu doğrudan hedef kelime adayı →
     **döngü kapanıyor:** GSC bulur, operatör seçer, mevcut üretim yazar.
   - **`cannibalization()`** — bir sorguda ≥2 ürün sayfamız VE baskın pay yok (%70 eşiği;
     tıklama yoksa gösterim payına düşer). Gerçek veride **yalnızca 3 sorgu** (21.634 içinden)
     — gürültüsüz. ⚠️ Otomatik birleştirme ÖNERİLMİYOR (QueryLoom da önermiyor): yanlış
     birleştirme geri alınması zor bir SEO hasarı, karar operatörde.

   Doğrulanan örnekler: "çift monitör kolu" poz 6.2 / 214 gösterim / 0 tıklama ·
   "teamviewer" 4 lisans varyantı yarışıyor / 357 gösterim / 0 tıklama ·
   "akgi" aynı ürün adı iki URL'de → katalogda mükerrer kayıt şüphesi.

   **Dayanıklılık:** sorgu çağrısı hata verirse raporun GERİ KALANI YİNE DÖNER — sorgu
   katmanı ek bilgidir, yokluğu tüm analizi kaybettirmemeli.

   ~~**Sıradakiler:** trend · iç link adayı · EOL halef önerisi · Ahrefs hacim/zorluk~~
   ✅ **BU LİSTE KAPANDI (v0.6.1–v0.6.3).** Trend ve EOL halef önerisi yapıldı; iç link adayı
   ile Ahrefs zorluk verisi **ölçülerek elendi** (bkz. madde 0ae) — bekleyen iş değiller.

0ac. 📦 **ESKİ NOT — sorgu×sayfa altyapısı ilk kurulduğunda (v0.5.8'de kod var, analizler henüz yok).**
   `gsc.rs::query_page_stats` — `dimensions:["page","query"]`, `startRow` sayfalama (GSC tek
   istekte en fazla 25.000 satır), `page contains <yol>` filtresi.
   **Ölçüm:** 24.204 satır · 4.764 sayfa · 21.634 sorgu · **2,5 saniye**. Tek istekte sığdı.
   Sıradaki: striking distance (poz 4–20, sorgu bazlı), kanibalizasyon (bir sorgu birden çok
   sayfamızda, baskın pay yok), trend, iç link adayı. Referans: QueryLoom kural dosyası
   (kullanıcı araştırması). Ahrefs hacim/zorluk verisi **istek üzerine, satır bazında, önbellekli**
   bağlanacak — her CapSolver çözümü ücretli, toplu çalıştırılamaz.

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

0b5. ✅ **ÖDÜNÇ ALINMIŞ ALAN: SLUG/SKU KARIŞIKLIĞI (v0.17.2).** Saha hatası: teklifte
   katalogdan ürün seçince *"Ürün bulunamadı: Query returned no rows"*.

   🔴 **Kök sebep tek bir satırdı ve iki yıl önceki bir kolaylıktı.** `search_live_products`
   canonical hedefi araması için yazılmıştı; `CatalogMatch`in **`canonical` alanına SKU'yu**
   koyuyordu (o akışta hedef zaten SKU'ydu, ayrı alan açmaya gerek görülmemişti). Faz T ve
   Faz C aynı aramayı yeniden kullanınca `slug`ı SKU sandı — çünkü SKU adında bir alan yoktu.

   **Ders:** *bir alanı başka amaç için ödünç almak, sonraki okuyucuya kurulmuş tuzaktır.*
   Kısa vadede bir `struct` alanı kazandırır, uzun vadede tipin anlamını yalanlar. Artık
   `CatalogMatch.sku` kendi alanı; `canonical` boş bırakılıyor.

   ⚠️ **İki çağıranın biri gürültülü, biri SESSİZ hata veriyordu** — ve tehlikeli olan ikincisi:
   - Teklif: arka uç `WHERE sku = ?1` ile satır bulamayınca **bağırıyor**, kullanıcı 1 günde bildirdi.
   - Kişi-ürün bağı: `contact_products`ta yabancı anahtar YOK; slug sessizce yazılıyor, hata
     çıkmıyor, yalnızca ürün ekranındaki "bu ürünle ilgilenenler" listesi **hiç dolmuyor**.
     Kullanıcının veritabanında böyle 1 satır bulundu (ölçüldü, tahmin edilmedi).

   **Onarım göç olarak yazıldı** (`db::init`, her açılışta idempotent): slug, ürün adresinin son
   parçasıyla eşleştirilip SKU'ya çevriliyor. Eşleşme yoksa **silinmiyor** — veriyi silmektense
   bozuk bırakmak yeğdir, kullanıcı görüp karar verir. Kullanıcının gerçek veritabanının
   kopyasında koşuldu: bozuk satır doğru ürüne bağlandı.

   🔴 **Harness hatayı GİZLİYORDU, çünkü taklit arka ucun yanlışını taşıyordu.** Üç ayrı kusur:
   1. Fixture SKU'yu `canonical`a koyuyordu — üretimdeki aynı ödünç alma.
   2. `add_quote_item_from_catalog` **koşulsuz** kur hatası döndürüyordu; hangi SKU geldiğini
      hiç bakmıyordu. Artık gerçek arka uç gibi doğruluyor.
   3. `link_contact_product` her şeye `null` dönüyordu. Üretim sessiz olduğu için taklit de
      sessiz kalmalı — ama doğrulanabilir olmalı: bilinmeyen SKU'da **konsola hata** yazıyor,
      iki adımlı kontrol bunu yakalıyor. (Bekçi, hatayı bilerek yeniden vererek sınandı.)

   ⚠️ Bonus kusur: harness'ta `QT` (teklif fixture'ı) `invoke` gövdesinin **içinde** tanımlıydı,
   yani her çağrıda sıfırlanıyordu — durum değiştiren komutlar yazdıklarını bulamıyordu.
   `window.__QT__`e asıldı (sohbet stub'ı aynı sebeple `window.__CHATS__` kullanıyor).

0b6. ✅ **ÖLÇEK MODELLEMESİ (Faz G, 2026-08-12) — eşikler ayarlanabilir OLMAYACAK.**
   Bütün eşikler tek mağazada (283 ürün) kalibre edildi. Soru şuydu: 10 bin ürünlük bir
   katalogda ne kırılır, eşiklerin ayarlanabilir olması gerekir mi? Üç ölçüm yapıldı
   (`#[ignore]`'lu, sentetik, ağsız): `sync::olcek_senkron_10k`,
   `opportunity::olcek_analiz_10k`, `commands::opportunities::olcek_rapor_10k`.

   **Senkron (10.000 ürün, 22,3 MB XML):**
   | adım | süre |
   |---|---|
   | ayrıştırma | ~290 ms |
   | ilk yazma (bellek / disk) | 1.109 / 1.196 ms |
   | ikinci koşu, hiçbir şey değişmemiş | 2.018 / 2.254 ms |

   İki bulgu: **disk ile bellek arasında fark yok** (~%8) — her şey tek işlemde, maliyet
   G/Ç değil CPU. Ve **ikinci koşu ilk yazmanın iki katı**; üretimde OLAĞAN durum bu, çünkü
   her senkron 10 bin satırın parmak izini karşılaştırıyor. Yine de toplam ~2,5 sn: darboğaz değil.

   **Analiz (10.000 ürün):** satışta olmayan 15 ms, düşüşte olanlar 18 ms; sorgu düzeyinde
   1 milyon satırda bile yükselmeye yakın 347 ms, yarışan sayfalar 715 ms. **Hız hiçbir
   ölçekte sorun değil.** ⚠️ Aynı testin ürettiği *madde sayıları* okunmamalı: sentetik
   dağılım kasten geniş tutuldu, gerçek liste uzunluğunu tahmin etmez.

   🔴 **ASIL BULGU — ölçek sınırı eşiklerde değil, raporun tek parça olmasında.**
   Rapor tek bir `settings` satırında JSON. Gerçek ölçüm: **283 ürün → 520 KB.** Gerçek
   oranlar 10 bin ürüne ölçeklenince **10,4 MB**; serileştirme 353 ms, çözümleme 162 ms.
   Ölçülerek düzeltilen ilk varsayımım "her ekran açılışında çözümleniyor"du — hayır,
   yükleme `if (!store.opportunity)` ile korunuyor, **uygulama başına bir kez**. Yani
   maliyet tek seferlik yarım saniye + oturum boyunca bellekte duran 10 MB.

   **KARAR (karar kapısı cevaplandı): eşikler ayarlanabilir yapılmayacak.** Gerekçe ölçüm:
   eşiklerin ürettiği yük hiçbir ölçekte sorun çıkarmıyor, ekranlarda zaten kırpma var
   (satışta olmayanlar 25, yükselmeye yakın 40, düşüşte olanlar 30 satır + "daha fazla").
   Ayarlanabilir eşik, olmayan bir problemi çözen bir ayar olurdu. Ölçek sorunu çıkarsa
   çözülecek şey **rapor blob'u** (kovaların ayrı ayrı ve sayfalı okunması) — bu, ikinci
   mağaza/genel dağıtım geldiğinde Faz P ile birlikte bakılacak, şimdi değil.

0b7. ✅ **MODEL ZİNCİRİ AYARLARA + KULLANIM SAYACI (Faz G, v0.18.0).**

   İki kalem tek iş olarak yapıldı: sayacın anlamlı olması için hangi modellerin
   kullanıldığını bilmesi gerekiyor, o liste de artık ayarlardan geliyor. Ayrı yapılsalardı
   aynı veri iki yerde dururdu.

   **Neden:** `MODEL_CHAIN` kodda sabitti. 2026-07-28'de `gemini-1.5-flash` emekli olunca
   üretim **tamamen durdu** ve düzeltmek yeni sürüm gerektirdi (0 numaralı ders). Soru
   "tekrar olacak mı" değil, "ne zaman"dı.

   **Bayatlama kökünden çözüldü:** liste artık Google'ın `models` uç noktasından **canlı**
   geliyor; emekli bir model kullanıcının karşısına hiç çıkmıyor. Ama "var" ile "bizim
   isteğimizi kaldırıyor" farklı sorular — uç nokta `system_instruction` + `responseSchema`
   desteğini söylemiyor. Bu yüzden satır başına **Dene**: uygulamanın gerçek istek biçimiyle
   tek çağrı. ⚠️ O deneme de kotadan düşen gerçek bir istek; sayaca yazılıyor.

   🔴 **Sayaç "kalan hak" DEMİYOR — kullanıcı kararı.** *"20 hakkın var, 14 kullandın dersek
   bir beklenti oluşur"*. Sayabildiğimiz tek şey bu uygulamanın gönderdikleri; aynı anahtar
   başka yerde kullanılırsa sayı eksik. Eksik bir sayıyı kapasite gibi sunmak yanlış güven
   verir. Ekranda yazan: `gemini-3.6-flash · bugün 14 istek`, altında sınırın kendisi.

   **Geçmiş grafiği süs değil, karar girdisi.** Kullanıcının amacı: *"limitler bir darboğaz mı
   değil mi"* ve *"günde kaç üretim, üretim başına kaç istek"* — çıktısı ileride **ücretli
   API'ye geçme kararı**. Bunun için iki tasarım kararı gerekti:
   - `run_id`: bir üretim = bir istek DEĞİL. Zincir üç modele düşerse üç satır, aynı `run_id`.
     Gruplama olmasaydı ortalama hep 1,0 çıkar, yani asıl bilmek istenen şey gizlenirdi.
   - `http_code` ve 429'un **ayrı sayılması**: darboğazın göstergesi toplam istek değil,
     kotaya çarpma sayısı.

   🔴 **Ortak HTTP kapısı — sayacın doğru olmasının ön koşulu.** İstek gönderme mantığı dört
   dosyada birebir kopyalanmıştı. Sayaç kopyalardan birini atlarsa **sessizce** yanlış sayar.
   Tek `post_generate` çıkarıldı; kapının dışında kalan tek yer akış tabanlı `chat_stream` ve
   sebebi kodda yazılı. (Aynı ders beşinci kez: *kopyalanan mantık sapar, paylaşılır.*)

   ⚠️ **Kayıt DB'ye üretim sırasında yazılmıyor.** Kanal `await`'ler arasında çağrılıyor;
   oradan SQLite kilidini almak, kilidi zaten tutan bir çağıranla **kilitlenme** üretirdi.
   `CallToplayici` toplayıp üretim bitince yazıyor. Bedeli: üretimin ortasında çökülürse o
   kayıtlar kaybolur — kabul edilebilir, sayaç muhasebe defteri değil.

   ⚠️ **Gün sınırı yerel saat, Google Pasifik'e göre sıfırlıyor.** Gün dönümünde sayılar
   Google'ın gördüğünden farklı olabilir; gizlenmiyor, ekranda yazılı. "Kalan hak" iddia
   etmediğimiz için bu kayma yanıltıcı bir sonuç doğurmuyor.

   Zincir **asla boş kalmaz**: ayar boş/bozuksa `DEFAULT_MODEL_CHAIN`'e düşülüyor. Boş
   kalsaydı üretim tek istek göndermeden "tüm modeller denendi" derdi. Sohbet zinciri ayrı
   tutuldu (bilerek — asistan üretimin 20/gün kotasını yiyemez). Kullanım kaydı **yedeğe
   dahil**: taşınmasaydı sayaç her makine değişiminde sıfırlanır ve asıl soru cevaplanamazdı.

0b8. 🔬 **FAZ İ ÖN ÖLÇÜMÜ — kör nokta ve IdeaSoft blog ucu (2026-08-12).**
   Kullanıcı GSC son 12 ayı dışa aktarıp üç belge üretti; biri *"Google seni sıralıyor ama
   içerik yok"* tezine dayanan 50 maddelik içerik backlog'u. Belgeler `İndirilenler/` altında,
   repoda değil.

   🔴 **YAPISAL BULGU: uygulama sorgu→sayfa eşlemesini çekiyor ve kendi eliyle atıyor.**
   `seo_data/gsc.rs` `["page","query"]` boyutunu istiyor — yani "hangi sorguya hangi sayfa
   sıralanıyor" verisi bizde var. İki yerde çöpe gidiyor:
   1. **Çekerken:** `path_prefix` (= `/urun/`) GSC'ye `dimensionFilterGroups` olarak
      gönderiliyor → blog/kategori/marka sorgu satırları **hiç indirilmiyor.**
   2. **İşlerken:** `striking_distance` / `cannibalization` katalogda olmayan sayfayı
      `by_page.get(page)?` ile düşürüyor.

   **Kör noktanın büyüklüğü ölçüldü** (son anlık görüntü): 300 ürün-dışı sayfa — marka 77,
   blog 168, kategori 55 — **133.654 gösterim, 1.010 tık (%0,76)**, ve bunlar **hiçbir ekranda
   görünmüyor**; altı aracın hepsi ürün-merkezli. Yol haritasının "uygulamada yapılacak tek
   anlamlı kod işi JSON-LD gönderimi" değerlendirmesi bu yüzden eksikti.

   Yan ölçüm: GSC'de 3.324 ürün URL'si var, **%97,1'i katalogla eşleşmiyor** — EOL teşhisini
   bağımsız doğruluyor.

   ✅ **IdeaSoft BLOG UCU VAR — canlı doğrulandı.** `GET /admin-api/blogs` → 200.
   Kayıt yapısı ürünlerle aynı alan ailesinde, yani `ideasoft.rs` desenleri taşınır:
   `id · title · slug · excerpt · content (HTML) · pageTitle · metaDescription · metaKeywords ·
   targetKeyword · canonicalUrl · categories[] · tags[] · status · seoTotalRuleCount`.
   Sayfalama `?limit=100&page=N`. **Mağazada 252 yazı var, 250'si yayında.**
   ⚠️ Yalnızca GET denendi; POST/PUT **denenmedi** (canlı mağazaya yazmak kullanıcı onayı
   gerektirir). Yazma yolu doğrulanmadan İ5 kesinleşmiş sayılmaz.

   Ucun asıl değeri gönderim değil **ölçüm**: "bu konuda içerik var mı?" sorusu artık çıkarım
   değil, sorgu.

   🔴 **KENDİ ÖLÇÜMÜMDE HATA YAPTIM, DERSİ BURADA DURUYOR.** Backlog'u blog envanterine karşı
   sınarken ilk eşleştiricim `len > 2` filtresiyle **kısa model kodlarını atıyordu** (`k2`,
   `hi`, `a1`) — "creality k2 combo" bu yüzden *Hi Combo* yazısına eşleşti. Tam da backlog'u
   eleştirdiğim gevşek token eşleştirmesinin aynısı. *Bir yöntemi eleştirmek, aynı yöntemi
   yanlışlıkla kullanmaktan korumuyor.* Düzeltilmiş sonuç: 48 maddenin **8'inin** içeriği var,
   **40'ının yok** (275.181 gösterim) → backlog'un "yazı yok" tespiti büyük ölçüde DOĞRU.

   **Ama "yazı yok" ≠ "yazı yaz".** İki farklı soru var: *bir sayfa sıralanıyor mu* (teamviewer
   için ürün + marka sayfası var) ve *bilgi içeriği var mı* (yok). Backlog ikincisini soruyor.
   TeamViewer'da doğru cevap yine de yazı değil: 157.623 gösterim / 63 tık, sorgu
   **navigasyonel** — arayan teamviewer.com'a gidiyor. Bu yüzden Faz İ'de navigasyonel sorgu
   kuyruğa GİRMEYECEK; denetim raporunun kararı koda geçiyor.

   **Yeni kova adayı:** 250 yayında yazının 158'i GSC eşiğini geçiyor, **92'si geçmiyor.**
   ⚠️ Bu "sıfır gösterim" DEĞİL — `metric_page_rows` eşikli (`metrics::kept`). Doğru ifade:
   *ölçülebilir görünürlüğü olmayan 92 yazı.* Kesin sıfır iddiası eşiksiz sorgu gerektirir.

0b9. 🔴 **12 AY / 90 GÜN AYRIŞMASI — backlog kaybedilmiş görünürlüğün listesi (2026-08-12).**

   Faz İ mantığı gerçek veride koşturuldu (30.190 sorgu×sayfa satırı, 90 gün) ve planın
   temelini sarsan bir şey çıktı. Backlog'un ilk 15 maddesi **12 ayda 250.392 gösterim**
   almış; **son 90 günde 4.804 — %1,9.**

   | Sorgu | 12 ay (belge) | 90 gün | Şimdiki poz |
   |---|---|---|---|
   | teamviewer | 157.623 | 452 | 14,7 |
   | fortinet | 8.737 | 21 | 38,9 |
   | elegoo centauri carbon | 8.524 | 5 | 42,4 |
   | veeam | 5.213 | 16 | 25,3 |
   | creality k2 pro | 6.675 | 7 | 21,6 |
   | workstation | 5.134 | 1.939 | 40,6 |
   | firewall cihazı | 4.299 | 1.048 | 19,1 |

   Düz seyirde beklenen oran ~%25. Yani mevsimsellik değil: **o sorgulardaki sıralama
   çökmüş.** Backlog'un tezi "Google zaten sıralıyor, 9→4 çıkar"dı; ölçüm 14–42. pozisyon
   diyor — bu iyileştirme değil, sıfırdan başlamak.

   **Ders: toplu (12 aylık) dışa aktarım çöküşü GİZLER.** Ortalama, dönemin başındaki yüksek
   hacmi sonundaki sıfıra karıştırıyor. Uygulamanın 90 günlük kayan penceresi bunun için var
   ve bu, pencere seçiminin ilk kez bir stratejiyi çürüttüğü an.

   ⚠️ **Ayakta kalan iki madde:** *workstation* (%38, 1.939 gösterim) ve *firewall cihazı*
   (%24, 1.048). İçerik kararı verilecekse dayanağı bunlar, listenin tepesi değil.

   🔴 **KENDİ KODUMDA ÜÇ KUSUR — üçünü de gerçek veri gösterdi, sentetik testler değil.**
   1. **Navigasyonel tespit hiç ateşlenmedi (0 sorgu).** `NAV_MIN_IMPRESSIONS`'ı 5.000
      koymuştum — TeamViewer'ın **belgedeki** 157 bin sayısına bakarak. 90 günde 452 var.
      *Backlog'un yaptığı hatanın aynısını yaptım: belgenin sayısına güvenip kendi penceremi
      ölçmedim.* Eşik paylaşılan `MIN_IMPRESSIONS_FOR_NO_CLICK`'e (50) indi.
   2. **Kendi marka sorgumuz "açık" sayıldı.** *kurumsalit* / *kurumsal it* poz 1,0'da niyet
      uyuşmazlığı olarak işaretlendi. Kendi adımızı arayan bizi bulmuş; bu açık değil. Site
      adı `gsc_site_url`'den türetiliyor — koda gömülmüyor.
   3. **En büyük kusur: sorgunun niyeti hiç ölçülmüyordu.** Kural yalnızca *sayfa tipine*
      bakıyordu; sonuç olarak "masa standı" (poz 1,3) ve "ipad air pembe" (poz 1,4) gibi
      **ürün sorguları ürün sayfasıyla karşılandığı hâlde** açık sayıldı — oysa o isabet.
      Ayrıca poz 1–2'de beklenen TO eğrisi çok yüksek olduğundan orada her şey "eksik"
      görünüyor. Eklenen sinyaller: Türkçe bilgi belirteçleri (*nedir · nasıl · ne işe yarar ·
      farkı · hangi · mi*) + model kodu içermeyen jenerik terim + poz ≥ 3 tabanı.

   ✅ Mantığın çalıştığının kanıtı: düzeltmeden önce bile gerçek veride en tepedeki madde
   **"access point" — 4.528 gösterim, poz 7,2, 171 kaçırılan tık**. Denetim raporunun elle
   bulduğu şeyi kod kendisi buldu.

0c0. ✅ **FAZ İ — İÇERİK AÇIĞI VE MAĞAZA SAYFALARI (v0.19.0).**
   Ön ölçümler 0b8'de, 12ay/90gün çöküşü ve düzeltilen üç kusur 0b9'da. Bu girdi **inşa
   edilen şeyi** anlatıyor.

   **Kapatılan kör nokta.** Altı araç ekranının hepsi ürün-merkezliydi. Artık:
   `page_kind` (sayfa tipi) · `content_gap` (üç kova + navigasyonel ayıklama) ·
   `query_rows` (sorgu satırları ilk kez saklanıyor) · `store_pages` (kategori/marka/blog
   envanteri) · İçerik açığı ekranı · sayfa iyileştirme paneli · `Bucket::Content`.

   🔴 **Sayfa tipi TAHMİN değil ÖLÇÜM.** İlk tasarım kullanıcıya segment etiketletiyordu.
   IdeaSoft'un `blogs`/`categories`/`brands` uçlarının okunabildiği görülünce envanterden
   ölçmeye çevrildi. Farkı gerçek veri gösterdi: `/blog/etiket/fortinet-nedir` yola göre
   "blog" ama 252 blog slug'ı arasında yok — etiket sayfası. Tahmin bunu ayıramıyordu.
   ⚠️ Segment yolu YEDEK olarak duruyor: IdeaSoft modülü opsiyonel.

   🔴 **Navigasyonel ayıklama kovalanamaz hacmi listeden çıkarıyor.** İki AYRI kural, çünkü
   iki ayrı durum — ve bunu test düşürerek öğrendim: kendi marka sorgumuz İYİ TO alıyor
   (%48), o yüzden "TO çökmüş" koşuluna hiç takılmıyordu. Kendi adımız koşulsuz eleniyor
   (trafik zaten bizde), başka üreticinin markası yalnızca TO beklenenin onda birinden azsa.
   Marka listesi katalogdan, site adı GSC adresinden — koda hiçbir mağaza adı gömülü değil.

   **Üretim hattı** (`gemini::store_page`): kategori/marka/blog için meta + tanıtım metni.
   Bağlam ölçülmüş veriden iki kaynakla kuruluyor — o sayfanın GSC'de aldığı gerçek sorgular
   ve katalogdaki gerçek ürün adları. Model "bu sayfada ne var" sorusunu tahmin etmiyor.
   ⚠️ **Halüsinasyon kalkanı burada ürün tarafından ZAYIF ve kodda yazılı.** Teknik tabloda
   her sayı kaynağa karşı doğrulanıyor; kategori metninde doğrulanacak kaynak yok — sayfanın
   boş olması zaten üretme sebebimiz. `verify_no_invented_numbers` kaynakta olmayan sayı
   bulursa metni **reddediyor** (sessizce silmiyor: silinen sayı cümleyi bozar, kullanıcı
   fark etmez). Sayısız yanlış iddia yakalanmıyor — kabul edilen sınır, panelde de yazılı.

   **IdeaSoft yazma yolu canlı doğrulandı** (kullanıcı izniyle, `status=0` taslak kayıtta):
   `PUT /admin-api/blogs/{id}` → 200, **birleştirme mantığında** — gönderilmeyen `title`,
   `content`, `categories`, `status` alanlarına dokunulmuyor. Test sonrası kayıt ilk hâline
   döndürüldü. ⚠️ **Ölçülen kısıt: `metaDescription` 160 karakterde SESSİZCE kırpılıyor**
   (170 gönderildi, 160 saklandı, uyarı yok). Artık kendimiz kırpıyoruz — ne gönderdiğimizi
   bilmek için. Kategorilerde 193 karakterlik değer duruyor, yani sınır varlık tipine göre
   değişiyor olabilir; blog PUT'u dışında doğrulanmadı.

   **Kova:** `Bucket::Content` + `ItemRef::Query`, `TOTAL` 13 → 14 (yeni kova mevcutlardan
   slot çalmıyor — Faz C'deki kararın aynısı). Kimlik SORGU çünkü aynı sayfa birden çok
   sorguda açık veriyor. Ağırlık 0,6 (`Leak` ile aynı sınıf: çözümün bir kısmı dışarıda).
   🔴 **Yalnızca eylemi olan madde kuyruğa giriyor** — sıralanan sayfa envanterde yoksa
   panel açılamaz, eylemsiz madde kullanıcıyı "Yapıldı"dan başka bir şey yapamayacağı işe
   yönlendirir.

   **Kopyalar paylaşıma çevrildi:** meta uzunluk kuralları (`20..=60`, `50..=155`) ZATEN iki
   yerde kopyalanmıştı; üçüncüsünü eklemek yerine `validation::TITLE_MIN/MAX`,
   `DESC_MIN/MAX` yapıldı ve mevcut ikisi de ona bağlandı. `DESC_MAX` bilerek 155:
   IdeaSoft'un kestiği 160'ın kenarına yaklaşmamak için.

   ⚠️ **Aynı hatayı iki kez yaptım:** harness stub'ını `?setup=1` bloğunun içine koydum ve
   ikisinde de sessizce çalışmadı. `test_ideasoft` / `test_feed_url` çapaları o bloğun içinde
   ve genel seviyede sanılıyor. Yeni stub eklerken **girinti seviyesi kontrol edilmeli.**

   Ekranda görülüp düzeltilenler: kova adı satırda iki kez · envanter tablosunda satır
   sarması (23/26/26/26 px) · odak bağı hiç kurulmamış · süzgeç açıkken odaklanan satır
   listede olmadığı için odağın sessizce başarısız olması.

0c1. 🔴 **İÇERİK GÖNDERİMİ ÖLÇÜLMÜYORDU (v0.19.1) — "ölçülüyor" sandığım şey ölçülmüyordu.**

   Kullanıcı "sıradaki iş: içerik pilotu ne demek?" diye sordu; cevabı yazarken bir tur önce
   söylediğim cümleyi doğruladım ve **yanlış çıktı**. "İçerik gönderimleri `work_events`'e
   yazılıyor, kalibrasyonda dağılıma girecek" demiştim. Olay yazılıyordu ama **hiçbir yer
   okumuyordu.**

   İki kapı da SKU anahtarlıydı:
   - `today::store_events` → `WHERE reaches_store = 1 AND sku IS NOT NULL`
   - `metrics_cmd::get_outcome_badges` → `let Some(sku) = ev.sku else { continue }`

   İçerik gönderimi `sku = NULL` yazıyor → sonuç kontrolü kovası maddeyi geri getirmiyor,
   rozet hiç çıkmıyordu. **Ders: bir olayı yazmak, onu ölçmek değildir.** Yazma tarafını
   kurup okuma tarafını kontrol etmeden "ölçülüyor" demek, en kolay yalan söyleme biçimi.

   🔴 **İkinci hata, birincisinden beter: URL yerine SLUG yazıyordum.** `metrics::outcome`
   zaten URL anahtarlı çalışıyor (yani ölçüm katmanı hazırdı) ama olaya `access-point`
   yazılıyordu, `metric_page_rows.url` ise `https://…/kategori/access-point` tutuyor. İki
   kapı açılsa bile eşleşme olmayacaktı.

   **Hata kullanıcının GERÇEK verisinde bulundu.** Ölçüm için veritabanı kopyasına bakarken
   beklemediğim bir satır çıktı: `store_page_push · url=access-point · 2026-08-13T00:48`.
   Kullanıcı v0.19.0'ı kurup gerçekten bir kategori sayfası göndermiş — yani pilot fiilen
   başlamış ve o gönderim ölçülemez durumdaydı.

   **Düzeltmeler:**
   - `store_events` artık `ItemRef` döndürüyor: ürün → `Product(sku)`, sayfa → `Page(slug)`.
     ⚠️ Kimlik olarak URL değil **slug**: `ItemRef::Page` kaçak kovasında da slug taşıyor,
     aynı uzayda iki biçim sonradan aranacak bir tuzak olurdu. Tam adres olayın kendisinde.
   - `page_url_of`: gönderim anında gerçek adres **ölçüm satırlarından okunuyor**, kurulmuyor
     — yol deseni mağazadan mağazaya değişir ve kurulan adres eşleşmezse yine sessizce
     ölçülemez olurdu.
   - Göç: mevcut slug'lı olaylar tam adrese çevriliyor. Eşleşme yoksa dokunulmuyor.
   - `outcomeBadges` sözlüğü `sku || url` ile anahtarlanıyor — yalnızca sku ile
     anahtarlansaydı bütün içerik rozetleri boş anahtarda çakışırdı.
   - İçerik açığı ekranına **Sonuç** sütunu.

   ⚠️ İçerik maddesi (`ItemRef::Query`) ile gönderim kimliği (`ItemRef::Page`) **bilerek
   eşleşmiyor**: bir sayfaya metin göndermek o sayfayla sıralanan HER sorguyu çözmez.
   İçerik maddesinin susması "Yapıldı" işaretiyle oluyor.

0c2. 🔴 **SESSİZ ŞEMA UYUŞMAZLIĞI — sorgu satırları hiç yazılmamış (v0.19.2, 2026-08-13).**

   Kullanıcı "diğer üç sayfayı da gönderelim" dedi. Göndermeden önce üretimin bağlamını
   ölçtüm ve üç sayfada da **0 sorgu** çıktı. Sebep aranınca:

   `query_rows` kullanıcının makinesinde **ilk tasarımdaki `snapshot_id` şemasıyla** duruyor.
   Tabloyu sonradan `captured_at`/`window_days` ile yeniden tanımladım ama
   `CREATE TABLE IF NOT EXISTS` var olan tabloyu **değiştirmez**. Sonuç zinciri:
   INSERT hazırlanamıyor → `kaydet_query_rows` hata dönüyor → çağrı `let _ =` ile
   yazılmış → **hata yutuluyor** → sorgu satırları hiç yazılmıyor → içerik üretimi
   bağlamsız çalışıyor → kimse fark etmiyor.

   🔴 **Bu tuzağa ÜÇÜNCÜ kez düşüldü** (`queue_dismissals`, `store_pages`, `query_rows`) ve
   iki kezinde uyarıyı kendi elimle koda yazmıştım. Uyarı yazmak, tuzaktan korumuyor.
   Kalıcı çözüm: `refresh_schema` — `init`'in EN BAŞINDA, yeniden tanımlanmış atılabilir
   tabloyu düşürüyor. ⚠️ Yalnızca **atılabilir** tablolar için: `query_rows` tasarımı gereği
   son çekimi tutuyor. Kalıcı veri taşıyan tabloda `ALTER TABLE` yolu geçerli (`migrate`).

   **İkinci ders, birincisinden önemli: `let _ =` bir karar, ihmal değil.** Sayaç yazımında
   bilinçli kullanılmıştı ("kayıt başarısız olursa üretim düşmesin" — 0b7) ve orada doğru.
   Burada yanlıştı: veri yolunun kendisi sessizce koptu. Artık hata `eprintln!` ile yüzeye
   çıkıyor.

   **Yan bulgu — pilotun bağlamı zayıfmış.** Access Point metni 11 gerçek ürünle ama
   **sorgu bağlamı olmadan** üretilmiş. Ayrıca ölçüldü: *Güvenlik Duvarı (Firewall)* ve
   *Video Konferans Ürünleri* kategorilerinde katalogda **hiç ürün yok** — XML feed 283 ürün
   taşıyor ve o kategoriler feed'de değil. Yani o iki sayfa için üretim yalnızca sayfa adı +
   sorgulara dayanacak. *Stand Ve Dock Station*'da 13 ürün var.

0c3. 🔴 **YORUM YALAN SÖYLÜYORDU — paragraflar sayfada birleşiyordu (v0.19.3, 2026-08-13).**

   Kullanıcı düzeltmeyi kurup analizi çalıştırdı ve iki sayfa gönderdi. Sorgu bağlamının
   işe yaradığı görüldü: Access Point başlığı *"Access Point Modelleri ve Çeşitleri |
   TP-Link ve Aruba"* oldu ve metne katalogdaki gerçek markalar girdi. Ama üretilen metni
   ölçünce iki kusur çıktı.

   🔴 **Birincisi ve utandırıcı olanı:** `GeneratedStorePage.showcase` alanının yorumunda
   *"Paragraflar gönderim anında sarmalanıyor"* yazıyordu. **Sarmalanmıyordu.** Kod ham
   metni gönderiyordu. Canlı veride ölçüldü: gönderilen iki metinde de **0 satır sonu,
   0 HTML etiketi** — `showcaseContent` HTML olarak render edildiği için iki paragraf tek
   blok hâlinde birleşiyordu, birinde cümleler bitişikti (*"…yapar.İşletmeler…"*).
   *Ders: yorumun anlattığı davranış kodda yoksa, yorum yalandır — ve yalan yorum, yorum
   olmamasından kötüdür, çünkü kontrol etmeyi engeller.*

   **İkinci kusur, birincinin sebebi:** prompt "2 kısa paragraf" diyordu ama şema tek
   `STRING` istiyordu; model ayırıcıyı koymadı. *Ayırıcıyı ummak yerine yapıyı şemaya
   taşımak gerekiyordu* — `showcase` artık `ARRAY`. Tek paragraf dönerse `violations`
   yakalıyor ve retry devreye giriyor.

   Sarmalama gerçekten yapılıyor (`ideasoft::wrap_paragraphs`): dizi ya da boş satırla
   ayrılmış metin kabul ediyor (model dizi verir, operatör panelde boş satır bırakır),
   tek satır sonunu `<br>` sayıyor ve **HTML kaçışı** yapıyor — operatörün yazdığı `<`
   sayfa düzenini bozmasın. Taslak düz metin saklanıyor ki panelde düzenlenebilsin.

   ⚠️ **Zaten gönderilmiş iki sayfa birleşik metinle duruyor.** Düzeltme geriye dönük
   çalışmıyor (mağazadaki HTML'i uygulama bilmiyor); o iki sayfa yeniden üretilip
   gönderilmeli.

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

0b. ✅ **FAZ 2B TAMAMLANDI (v0.7.0, kalem 4) — aşağısı tarihsel kayıt.**
   `gemini.rs` (2119) → 5 dosya, `commands.rs` (2619) → 7 dosya; her iki bölmede de satır
   KAYBI=0 doğrulandı. Nasıl yapıldığı ve çıkan üç tuzak madde 0ai'de. Aşağıdaki plan
   uygulandığı hâliyle duruyor — benzer bir bölme gerekirse yöntem oradan okunabilir.

   **Bu iş TAMAMEN KOZMETİKTİ.** Rust'ta derleme birimi dosya değil crate'tir;
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

⚠️ **Bu vizyonun pratik sonucu: ilk kurulum deneyimi bir "nice to have" değil.** Uygulamayı
şu an tek kişi kullanıyor olması onu tek kullanıcılı bir araç yapmaz; hedef kitle IdeaSoft
altyapısı kullanan herhangi bir işletme. Yeni kullanıcının gördüğü ilk ekran boş bir liste
ve "Ayarlar'a git" sezgisiyse, kodun kalitesi bir işe yaramıyor. (Kuyrukta **K1**.)

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

⚠️ Bu liste **2026-08-01'de kodda tek tek doğrulandı** — eski hâli bayattı ve "atladığımız iş
var mı?" sorusunun bir kez gereksiz sorulmasına yol açtı. Bitmiş maddeler aşağıda işaretli.

- ~~Toplu üretim + ilerleme çubuğu~~ → 🔴 **YAPILMAYACAK**, gerekçe kuyruk denetiminde
  (halüsinasyon kısıtı + ölçülen kota gerçeği). Yeniden karar gerektirir.
- ~~IdeaSoft'a doğrudan yazma~~ → ✅ **Faz 9'da yapıldı**, gönderim modülü çalışıyor.
- ~~details üretiminde yoğunluk retry'ı~~ → ✅ **var** (`gemini/details.rs`, üç ayrı üretim
  yolunda da "yoğunluk aralık dışıysa tek retry, hedefe yakın olanı seç").
- **Gemini kota/kullanım göstergesi** → hâlâ yok. Kısmen karşılanıyor: `ModelTag` hangi modelin
  ürettiğini gösterdiği için alt modele düşüş görülebiliyor; ama "bugün kaç hakkım kaldı"
  bilgisi yok. Google bu sayacı API'den vermiyor → yerel sayım gerekir (kesin olmaz).
- **Kod imzalama + notarization** → ⏸️ kod işi değil, **maliyet kararı** (Apple ~$99/yıl).
  İmzasız kurulumda macOS "geliştirici doğrulanamadı" uyarısı veriyor; global dağıtımda
  ilk izlenimi doğrudan etkileyen tek açık kalem bu.
- **JSON-LD'yi IdeaSoft gönderimine eklemek** → 📌 ertelendi, koşulları 0ar'de.

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

⚠️ Bu bölüm 2026-08-07'de tazelendi; "repo push bekliyor" gibi aylar önce bitmiş maddeler
duruyordu (repo 2026-07-26'dan beri public).

- **Testler:** `cd src-tauri && cargo test` (Tauri katmanı, 19) ·
  `cargo test -p seo-core` (197) · canlı testler `-- --ignored` ile ve env değişkenleriyle
  (ör. `SEO_DB_COPY=... cargo test sync_fingerprint_real -- --ignored --nocapture`)
- **Çalıştır:** `npm run tauri dev`
- **Görsel doğrulama:** `npm run build && python3 scripts/harness.py` → `npx vite preview`
  `dist/harness.html` · kipler: `?empty=1` `?setup=1` `?changed=1` `?nosnap=1` `?nosonuc=1`
  `?nav=<ekran>` `&tema=koyu` · `?halefyok=1` (halef bulunamadı) · `?boskuyruk=1` (Bugün boş) · `?seans=1` (odak seansı
  sürüyor) · `?hepsiyapildi=1` (günün işi bitmiş) · `?kararlar=1` (EOL'de karar verilmiş satırlar) ·
  `?musteri=0` (CRM boş) · `?sessizlik=1` (eşik önerisi hazır) ·
  `?nav=quotes` (teklif düzenleyici, marj uyarıları dahil)
  ⚠️ `npm run build` `dist/`i siler → harness HER ZAMAN derlemeden sonra üretilir.
  🔴 **Ekran görüntüsü YETMİYOR, konsol da okunmalı** (`read_console_messages`). Ölçüldü
  (2026-08-10): harness ekranı doğru çiziyordu ama konsolda kurulum hatası vardı ve aynı kod
  `npm run tauri dev`'de ekranı bomboş açıyordu. ⚠️ Konsol tamponu yenilemede temizlenmiyor →
  `console.log("ISARET")` bırakıp işaretten SONRASINA bak.
  🔴 **`file://` ile AÇILMAZ** — derlenen `index.html` varlıkları `/assets/...` mutlak yolundan
  istiyor ve dosya protokolünde bunlar bulunamıyor, sayfa sessizce boş açılıyor. HTTP üzerinden
  servis edilmeli (`npx vite preview --port 4173`). Bir kez bu tuzağa düşüldü (2026-08-07).

### Yayın (sürüm çıkarma) — sırayla
1. CHANGELOG'a `## vX.Y.Z` bölümü (CI **yalnızca etiketle eşleşen** bölümü çıkarır)
2. Sürüm numarası **5 dosyada**: `package.json`, `package-lock.json` (2 yer),
   `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `src-tauri/core/Cargo.toml`
3. Commit → `git push origin main` → `git tag -a vX.Y.Z` → `git push origin vX.Y.Z`
4. CI'ın çıkaracağı notu **etiketlemeden önce** yerelde doğrula:
   `awk -v tag="## vX.Y.Z" '$0==tag{f=1;next} f&&/^## v/{exit} f{print}' CHANGELOG.md`
5. Bitince `latest.json`'ı kontrol et: sürüm doğru mu, 7 platformun imzası dolu mu, notlar tam mı

⚠️ **İki sürümü arka arkaya etiketleme.** İki CI koşusu `latest.json`'ı yarıştırır; önce
biten geç bitene ezdirir ve güncelleyici geriye işaret edebilir. v0.8.1 bu yüzden hiç
yayınlanmadı, notları v0.8.2'ye taşındı.

🔴 **Etiket push'u tetiklemeyebiliyor (v0.8.3'te yaşandı, 2026-08-07).** Etiket sunucuya
gitti, Actions açıktı, iş akışı etkindi — ama koşu hiç oluşmadı. Çözüm:
`gh workflow run release --ref vX.Y.Z`. İş akışı her yerde `github.ref_name` kullandığı için
sonuç push tetikleyicisiyle **birebir aynı** (notlar, Release adı, varlıklar). Etiketledikten
sonra koşunun gerçekten başladığını doğrula: `gh run list --limit 1`.
   ℹ️ v0.9.0 · v0.10.0 · v0.11.0 etiketleri **normal tetikledi** — sorun kalıcı değil, ama
   doğrulama adımı yine de atlanmamalı.

- **Bu dosyayı güncelle:** her faz/önemli karar sonrası "Son güncelleme" + ilgili bölüm

## 🧩 Açık sorular / kullanıcı kararları bekleyen
- [x] Kart 2 "Açıklamayı Tamamlandı" aktif olsun mu? → EVET, Faz 2'de aktifleştirildi.
- [x] Gemini model tercihi? → Kademeli fallback zinciri (2.0-flash → 2.5-flash → 1.5-flash).
- [x] Faz 3: details üretiminde hangi model? → **Aynı `MODEL_CHAIN`**; kota dolunca alt modele
      düşüyor, üreten model `ModelTag` ile kart başlığında görünüyor.
- [x] "Açıklama Bekliyor" filtresi → **çalışıyor** (`store.ts` sayaçları, `overall === "bekliyor"`).

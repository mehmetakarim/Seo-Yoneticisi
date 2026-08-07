# 🗺️ yol-haritasi.md — E-ticaret Operasyon Masası

> Bu dosya **nereye gittiğimizi** tutar. `brain.md` ise **ne olduğunu ve neden öyle olduğunu**
> tutar. İkisi karışmamalı: faz tanımları yalnızca burada, ölçüm sonuçları yalnızca brain.md'de.
> Aynı bilgi iki yerde durursa zamanla ayrışır ve ikisine de güven biter.

**Oluşturuldu:** 2026-08-07 · **Temel alınan sürüm:** v0.8.3
**Kaynak:** kullanıcının iki not belgesi (vizyon + kod değerlendirmesi) ve bu belgedeki
iddiaların kodda tek tek doğrulanması.

**Faz B ✅ tamamlandı (v0.9.0).** Sıradaki faz: **Ö (Ölçüm omurgası).**

---

## 1. Vizyon

> **E-ticaret Operasyon Masası; vitrin (katalog/SEO), satış (müşteri/teklif) ve söz
> (stok/taahhüt) işlerini aynı yerelde, aynı kuyrukta ve aynı "ne yaptık → ne oldu" dilinde
> yürüten masaüstü çalışma alanıdır.**

Bu cümle bir **özellik filtresi**: yeni bir fikir bu üç işten birine hizmet etmiyorsa ya da
kuyruk/olay/ölçüm diline bağlanmıyorsa, iyi bir fikir olsa bile bu ürüne ait değildir.

**SEO = wedge.** Bugünün olgun modülü SEO; ama ürünün tamamı SEO olmak zorunda değil.
Katalog (SKU) her modülün ortak omurgası — müşteri, teklif, stok ve SEO aynı ürüne bağlanır.

⚠️ **Bilinen risk: iki yarım ürün.** Vizyon geniş; tehlike SEO'yu yarım bırakıp CRM'e
atlamak. Panzehir, bu belgedeki **"bitti sayılır"** satırlarıdır — bir faz ölçülebilir biçimde
bitmeden sonraki faz açılmaz.

---

## 2. Değişmeyen ilkeler

Hepsi yaşanmış kararlardan çıktı; gerekçeleri `brain.md`'de numaralı maddelerde.

1. **Toplu kör üretim yok.** Üretim tek tek, operatör onaylı. Gerekçe iki katmanlı:
   halüsinasyon riski (kullanıcı kısıtı) + ölçülen kota gerçeği (flash katmanı 20 istek/gün).
2. **Veri uydurulmaz.** Kaynakta yoksa yazılmaz — teknik tabloda satır, JSON-LD'de fiyat,
   üretilmiş ürün fotoğrafı; hepsi aynı sınıf hata.
3. **Ölçmeden inşa edilmez.** Ölçüm tasarımı değiştirebilir ve bu projede defalarca değiştirdi
   (deterministik halef eşleştirme elendi, `offers` alanı çürütüldü, normalizasyon 8 bayrağı
   1'e indirdi).
4. **Yerel-first.** Veri kullanıcının makinesinde. Anahtarlar hiçbir koşulda bulutta düz metin
   olmaz.
5. **Mağazaya özel değer koda gömülmez.** (0ao: gömülü varsayılan feed adresi kusuru.)
6. **Tek tasarım dili.** Aynı iş aynı bileşenle çizilir; yeni ekran mevcut bileşenlerden
   kurulur. *"Boşluk da bir özelliktir"* — geometri kopyalanmaz, paylaşılır. Kopyalanan her
   stil zamanla sapıyor; kanıtı iki kez ölçüldü (kartlarda ikon 26/30px, araç ekranlarında
   `.sku` 1/2px).
7. **Her özellik üç soruya cevap vermeli:** kuyruğa iş düşürüyor mu · olaya yazılıyor mu ·
   sonra ölçülüyor mu?

---

## 3. Bugünkü durum (v0.8.3)

**Olgun olanlar:** feed senkronu + taslak yönetimi, Gemini ile meta/açıklama/teknik tablo
üretimi (model zinciri + sürüm geçmişi), GSC tabanlı altı analiz ekranı (Genel Bakış,
Fırsatlar, Yükselmeye yakın, Yarışan sayfalar, Düşüşte olanlar, Satışta olmayanlar), EOL halef
önerisi + canonical yazma, IdeaSoft gönderimi ve teknik tablo getirme, görsel skoru, JSON-LD,
feed değişikliği tespiti + karşılaştırma, kurulum sihirbazı, otomatik güncelleme, AI Asistanı.

**Bilinen boşluklar — kodda doğrulandı (2026-08-07):**

| Boşluk | Kanıt | Hangi faz kapatıyor |
|---|---|---|
| **Geçmiş tutulmuyor**, her analiz bir öncekini siliyor | `opportunity_json` tek `settings` anahtarı, `set_setting` ile üzerine yazılıyor — `src-tauri/src/commands/opportunities.rs:279` | **Ö** |
| Araç ekranlarında tablo/liste tutarsız | `.tbl` 4 ekranda; Yarışan sayfalar `div` listesi (`.cann-list`). `.head` 5, `.note` 4, `.more` 3 ekranda ayrı tanımlı; `.sku` 2px vs **1px** sapmış | **B** |
| Satır eylemi görünmez | 4 ekranda birincil eylem çıplak satır tıklaması (`store.openProduct`); yalnızca EOL'de görünür düğme, o da hücreye gömülü yerel `.succ-btn` | **B** |
| Asistan tek ekrana kilitli | `assistantContext()` bir `switch (lastToolPage)` — `src/store.ts:405`; `chat_sessions.tool_page` tek metin sütunu | **A** |
| `MODEL_CHAIN` kodda sabit | `src-tauri/core/src/gemini/mod.rs:49`, dört çağrı yerinde doğrudan kullanılıyor | **G** |
| Eşikler tek mağazada kalibre | `src-tauri/core/src/opportunity.rs` — 7 sabit (`SD_MIN_POSITION`, `CANNIBAL_DOMINANT_SHARE`, `DECAY_CLICK_RATIO`…) | **G** |
| Çoklu mağaza yok | tek `db_path` — `src-tauri/src/lib.rs:31` | **P** |
| Kota görünmez | yerel sayaç yok; yalnızca `ModelTag` alt modele düşüşü gösteriyor | **G** |

**Elimizdeki hazır varlık:** olay günlüğü sıfırdan kurulmayacak. `meta_history_json`,
`details_history_json`, `tech_history_json` girdilerinde **zaten `at` zaman damgası var**;
ayrıca `ideasoft_pushed_at`, `reviewed_fp`, `last_synced_at`. Yani `work_events` **geriye
dönük doldurulabilir** — "işe yaradı mı?" sorusunun ilk cevabı haftalar öne çekilebilir.

---

## 4. Fazlar

**Sıra: B → Ö → A → K → S → D → C → T → P.**
**G** fazlara bağlı değil; küçük kalemler halinde araya girer.
**P**, koşulu erken oluşursa öne alınır.

> ⚠️ **B neden Ö'den önce:** Faz Ö sonuç rozetlerini bu listelerin **hepsine** ekleyecek.
> Ortak bileşen önce çıkarsa rozet tek yere eklenir; sonra çıkarsa beş ekrana ayrı ayrı
> eklenip sonra tekrar toplanır. Aynı gerekçe Faz K'nin "Bugün" listesi için de geçerli.

---

### Faz B — Tasarım bütünlüğü ✅ (v0.9.0)

**Amaç.** Araç ekranlarının tablo/liste yapısını tek ortak bileşende toplamak ve satır
eylemlerini görünür, genişletilebilir bir yere oturtmak. Premium hissin sürmesi, ekrandan
ekrana aynı ritmin korunmasına bağlı.

**Kapsam.**
- Ortak `SeoTable` (gruplu varyant dahil): sütun tanımı veri-güdümlü, sayısal
  hizalama (`tabular-nums`), boş/yükleniyor/hata durumları, "daha fazla" davranışı.
- **Sabit "İşlem" sütunu.** Eylemler veri-güdümlü tanımlanır (ikon + ipucu + görünürlük
  koşulu); satıra göre gizlenir veya pasifleşir; yıkıcı eylem onay ister; **sütun genişliği
  eylem sayısından bağımsız sabit** kalır ki satır ritmi ekrandan ekrana kaymasın.
- Yarışan sayfalar `div` listesinden ortak yapıya taşınır.
- `.head` / `.note` / `.more` / `.sku` yerel kopyaları silinir, paylaşılan tanıma bağlanır.

**Başlama koşulu.** Yok — hemen başlanabilir.

**Bitti sayılır.**
- Altı araç ekranının tamamı ortak bileşeni kullanıyor; `src/components/tools/*.vue` içinde
  tablo/satır geometrisi tanımlayan **yerel CSS kuralı kalmadı** (grep ile doğrulanır).
- Her ekranda birincil eylem **görünür bir düğme** olarak İşlem sütununda; çıplak satır
  tıklaması artık tek affordance değil.
- Harness'te altı ekran açık ve koyu temada yan yana karşılaştırıldı; satır yüksekliği,
  hücre dolgusu ve başlık tipografisi ekranlar arasında **birebir aynı**.

**Ekran gerekiyor mu.** Evet — mevcut ekranların yeniden tasarımı değil, **ortak bileşenin
tasarım kararı** (satır ritmi, İşlem sütunu yerleşimi, ikon dili, boş durum). Claude Design
promptu mevcut ekran görüntüleriyle birlikte verilecek.

---

### Faz Ö — Ölçüm omurgası

**Amaç.** "Bugün 60 fırsat var" diyebiliyoruz; "3 hafta önce 80'di, 20'sini kapattık, 12'si
gerçekten iyileşti" diyemiyoruz. Anlık fotoğraf var, film yok.

**Kapsam (kavramsal — kesin şema faz planında).**
- `metric_snapshots` + `metric_page_rows`: **değiştirilemez** GSC anlık görüntüleri.
  Snapshot politikası **hibrit** — her manuel analiz bir snapshot; son snapshot ≥7 gün ise
  kullanıcıya önerilir. GSC gecikmeli olduğu için günlük snapshot şart değil.
- `work_events`: `meta_done · details_done · tech_done · ideasoft_push · keyword_set ·
  canonical_set · feed_ack · eol_successor_set`. **Mevcut zaman damgalarından geriye dönük
  doldurulur.**
- `outcome_links`: olay → sonraki snapshot deltası. Etki penceresi **21–45 gün**.
- Sonuç rozetleri, uygulama genelinde **aynı dil**: ⏳ Ölçülüyor · ↑ İyileşti · → Değişmedi ·
  ↓ Geriledi · ? Veri yetersiz.

**Dil kuralı.** Kesin nedensellik iddia edilmez. "Bu iş sayesinde arttı" değil, "işlemden
sonraki 28 günde tıklama arttı". Yetersiz veri saklanmaz, açıkça söylenir.

**Başlama koşulu.** Faz B bitmiş olmalı (rozetler ortak bileşene tek yerden eklenecek).

**Bitti sayılır.**
- İki farklı tarihte alınmış snapshot arasında sayfa bazında delta üretiliyor.
- Geriye dönük dolum çalıştırıldığında mevcut veriden **kaç olay üretildiği ölçülüp**
  brain.md'ye yazıldı.
- En az bir gerçek olay için rozet "⏳ ölçülüyor"dan çıkıp sonuç gösterdi.
- Yedekleme (`export_json`/`import_json`) yeni tabloları taşıyor ve **testi var**.

**Ekran gerekiyor mu.** Kısmen — rozetler mevcut listelere girer (Faz B bileşeni sayesinde tek
yerden). Genel Bakış'a "sonuçlar" bölümü eklenir → küçük bir design promptu.

---

### Faz A — Asistan bağlam seçimi

**Amaç.** Bugün asistan **son gezilen ekrana** kilitli; aynı sohbette A ekranını konuşup
B ekranına geçmek mümkün değil. Kullanıcı hangi verinin gönderildiğini de göremiyor.

**Kapsam.**
- Girişin soluna **"+" düğmesi** → açılır menüden konuşulacak veri kaynakları seçilir (çoklu).
- Seçim **sohbet ortasında değiştirilebilir**; aynı pencerede önce Fırsatlar, sonra EOL.
- Seçili kaynaklar girişin üstünde **çip** olarak görünür — model neyi görüyor, kullanıcı bilir.
- `switch (lastToolPage)` kalkar; `chat_sessions` çoklu kaynak tutar (mevcut tek değerli
  kayıtlarla geriye dönük uyumlu).

**Ölçüm önce.** Bugün tek ekran için **50 satır** gönderiliyor; beş kaynak seçilirse 250 satır
olur, hem kota hem cevabın odağı bozulur. Faz A bir bütçe ölçümüyle başlar: toplam üst sınır +
kaynak başına pay.

**Dürüstlük kuralı.** Seçilmemiş bir ekran sorulduğunda asistan *"bu ekranın verisi seçili
değil"* der — mevcut "veride yoksa uydurma" kuralının doğal devamı.

**Başlama koşulu.** Yok; Ö'den bağımsız. (Ö bitmişse asistan sonuç verisini de konuşabilir.)

**Bitti sayılır.**
- Tek sohbet penceresinde en az iki farklı kaynak arka arkaya konuşuldu.
- Seçilmemiş kaynak sorulduğunda model uydurmuyor, seçili olmadığını söylüyor.
- Bağlam bütçesi ölçüldü ve üst sınır kodda **tek yerde** tanımlı.

**Ekran gerekiyor mu.** Evet — giriş şeridi + açılır menü + çip alanı → design promptu.

---

### Faz K — Bugün + iş kuyruğu

**Amaç.** Sabah uygulamayı açan kişi 5–10 net aksiyon görsün; hangi ekrana gideceğine karar
vermek zorunda kalmasın.

**Kapsam.** Beş kova: acil operasyon (feed değişti, push yansımadı) · yüksek kaldıraç (GSC
fırsat) · kaçak trafik (EOL, canonical bekleyen) · sonuç kontrolü (28g+ önce işlendi) ·
bakım (decay, kanibalizasyon). Şeffaf skor (formül kullanıcıya gösterilir). Kuyruk maddesi =
**ne** + **neden** (tek cümle metrik) + tahmini süre + derin link.

**Başlama koşulu.** Faz Ö bitmiş olmalı — "sonuç kontrolü" kovası ve "yakın zamanda işlendi,
ölçülüyor" cezası ölçüm verisine dayanıyor.

**Bitti sayılır.** Gerçek veriyle açıldığında beş kovadan en az üçü dolu; her maddenin "neden"
cümlesi gerçek bir metrikten türetiliyor; madde tıklanınca doğru ekranda doğru satıra gidiyor.

**Ekran gerekiyor mu.** Evet — yeni ana ekran ("Bugün"), menünün en üstüne gelir.

---

### Faz S — Odak seansı

**Amaç.** Kuyruğu sürdürülebilir bir tempoda tüketmek. **Oyunlaştırma değil**, sakin bir
çalışma ritmi.

**Kapsam.** 25/5 (ayarlanabilir), seans başında kuyruktan **tek iş kilitlenir**, iş
biter/atlanır/ertelenir, mola önerilir (zorunlu değil), sakin seans özeti.
`focus_sessions` + `focus_session_items` → ortalama iş süresi kalibrasyonu.

**Açık yasak.** XP, lig, streak cezası, konfeti, "Harikasın! 🔥" bildirimleri. Tek seferde tek
ürün / tek aksiyon — toplu üretim yasağıyla uyumlu.

**Başlama koşulu.** Faz K bitmiş olmalı.

**Bitti sayılır.** Bir seans baştan sona yürütüldü; seans geçmişinden gerçek ortalama iş süresi
hesaplandı ve kuyruk tahminleri bu ölçüme göre güncellendi.

**Ekran gerekiyor mu.** Evet — seans paneli/çubuğu → design promptu.

---

### Faz D — Katalog derinliği + SEO güçlendirme

**Amaç.** Ölçülen SEO fırsatını işe çevirmek.

**Kapsam.** Ürün sağlık skoru (meta+açıklama+görsel+teknik+IdeaSoft skoru tek göstergede) ·
içerik borç listesi · **EOL → 301 iş listesi + CSV export** (uygulama yönlendirme yapamaz, iş
listesi üretir) · **canonical karar-kuyruğu** · stok eşiği uyarısı · varyant/aile gruplama.

> **Canonical karar-kuyruğu — kuralı bozmayan orta yol.** Üretimdeki "toplu işlem yok" kuralının
> gerekçesi halüsinasyon; canonical'da ise risk **hedef seçimi**, yazma işleminin kendisi
> deterministik. Bu yüzden: **karar tek tek verilir** (öneri + onay), onaylanan kararlar bir
> kuyrukta birikir, **yazma toplu yürütülür**. Operatör kontrolü kaybolmaz, oturum sayısı düşer.

**Başlama koşulu.** Faz K bitmiş olmalı (iş listeleri kuyruğa bağlanacak).

**Bitti sayılır.** EOL listesinden üretilen CSV gerçek veriyle açıldı ve panelde kullanılabilir
durumda; canonical karar-kuyruğunda biriken kararlar tek oturumda yazıldı ve her satır
`work_events`'e düştü.

**Ekran gerekiyor mu.** Kısmen — sağlık skoru ve borç listesi mevcut ekranlara girer; karar
kuyruğu için küçük bir onay ekranı.

---

### Faz P — Çoklu mağaza profili

**Amaç.** Aynı uygulamada birden fazla mağazayı veri karıştırmadan yönetmek.

**Kapsam.** Profil başına DB yolu, ayar ad alanı ve yedekleme; profil değiştirici; sihirbazın
"yeni profil" akışı.

**Başlama koşulu.** ⚠️ **İkinci mağaza gerçekten geldiğinde** ya da uygulama genel dağıtıma
açılmadan önce. Tek mağazayla çalışırken bu faz erken — tek `db_path` varsayımı kodun birçok
yerinde ve refactor küçük değil.

**Bitti sayılır.** İki profil arasında geçiş yapıldığında ürün, ayar, sohbet ve ölçüm verisi
**hiçbir noktada karışmıyor**; yedek alma/geri yükleme profil bazında çalışıyor ve testi var.

**Ekran gerekiyor mu.** Evet — profil değiştirici + profil yönetimi.

---

### Faz C — CRM ince dilim

**Amaç.** Site vitrin; asıl satış mail/telefon/teklif üzerinden yürüyor. SEO ziyaretçi
getiriyor, ama ne olduğu uygulamanın dışında kalıyor.

**Kapsam.** Kişi/firma kartı · kanal etiketi (mail/telefon/IG/fuar) · ilgi etiketleri ·
**son temas + sonraki adım tarihi** (CRM'in %80'i) · zaman çizelgesi · "bu ürünle ilgilendi"
SKU bağı · sessiz müşteri uyarısı · CSV içe aktarma. Kuyruğa `crm.followup` düşürür.

**Ertelenenler.** Gmail gelen kutusu entegrasyonu, otomatik mail okuma, ekip yetkileri.

**Başlama koşulu.** Faz K + S bitmiş olmalı — CRM işleri kuyruğa ve seansa bağlanmazsa ayrı
bir ada olur, "iki yarım ürün" riski tam olarak burada doğar.

**Bitti sayılır.** Bir müşteri kaydı üzerinden "sonraki adım" tarihi kuyruğa iş düşürüyor ve
tamamlandığında olay günlüğüne yazılıyor.

**Ekran gerekiyor mu.** Evet — müşteri listesi + kartı.

---

### Faz T — Teklif ve offline satış

**Amaç.** Teklifi katalog gerçeğine bağlamak ve takibini kuyruğa taşımak.

**Kapsam.** Teklif taslağı (satır = SKU, elle satır da mümkün) · durum makinesi (taslak →
gönderildi → … → kazandı/kaybetti) · PDF/HTML kopyala (mail'e yapıştırılır, entegrasyon yok) ·
versiyon (v1/v2 fiyat geçmişi) · kazanma/kaybetme nedeni · tekliften görev üretme · marj notu.

**Başlama koşulu.** Faz C bitmiş olmalı.

**Bitti sayılır.** Bir teklif taslaktan sonuca kadar yürütüldü; her durum değişimi olay
günlüğüne düştü; kayıp nedeni raporlanabiliyor.

**Ekran gerekiyor mu.** Evet — teklif listesi + teklif düzenleyici.

---

### Faz G — Global sağlamlık (araya giren küçük kalemler)

⚠️ **G diğerleri gibi bir faz değil**, o yüzden beş başlığı yok: birbirinden bağımsız küçük
kalemlerden oluşuyor ve her biri uygun bir aralıkta tek başına alınabilir. Bir kalem işe
alındığında kendi "bitti sayılır" tanımı o anda yazılır.

| Kalem | Neden |
|---|---|
| `MODEL_CHAIN` → ayarlar | Model emekliliği bugün **yeni sürüm** gerektiriyor. 0ao dersi birebir: kodda yalnızca varsayılan, gerçek değer `settings`'te |
| Gemini kota/kullanım sayacı (yerel) | Ücretsiz katmanla çalışan bir üretim aracının kapasitesi bugün görünmez. Google kesin sayaç vermiyor → yerel sayım yeterli |
| Ölçek modellemesi | Eşikler tek mağazada kalibre; 10k ürünlü sentetik feed ile senkron süresi, fırsat sayısı ve liste boyu ölçülecek. Eşiklerin ayarlanabilir olması **gerekip gerekmediğini** bu ölçüm belirler |
| i18n | Global dağıtım hedefiyle Türkçe arayüz çelişiyor |
| Kod imzalama / notarization | Kod işi değil, **maliyet kararı** (~$99/yıl). İmzasız kurulumda macOS uyarı veriyor |

---

## 5. Erken ölçümler (kod yazmadan önce)

**1. Embedding kıyası.** 254 ürün embed edilir; en çok trafik alan 12 EOL sayfası için mevcut
IDF yöntemiyle embedding yöntemi yan yana konur. **Taban çizgisi hazır:** brain.md 0ad — 12
sayfanın 3'ünde güçlü eşleşme, eşik 0.45'te tıklamaların %23'ü.

Sonuç, tek başına şu maddelerin **hepsini birden** açar veya kapatır: semantik ürün arama ·
EOL halef adayları · mükerrer kayıt tespiti · sorgu→ürün eşleştirme · iç link önerisi (sözcük
örtüşmesiyle 0 çift vermişti, alet değişince sonuç değişebilir) · semantik kanibalizasyon ·
hedef kelime çakışması · kategori denetimi.

> ⚠️ **Embedding 1 ve 2 aynı uzayda değil.** Vektörle birlikte **model adı ve boyut**
> saklanmalı. **Fallback zinciri kurulmayacak** — düşülen model sessizce anlamsız benzerlikler
> üretir; bu, üretim zincirindeki mantığın tam tersi. Tek sabit model + açık migration.
> `feed_fp` yeniden-embed koşulu için hazır: parmak izi değişmediyse vektör geçerli.

**2. Ölçek modellemesi.** Sentetik 10k ürünlü feed ile senkron süresi, üretilen fırsat sayısı
ve EOL liste uzunluğu ölçülür.

**3. Ücretsiz katman limitleri** konsoldan doğrulanır (0a dersi: ölçmeden zincire güvenme).

---

## 6. Değerlendirildi — girmeyenler ve gerekçeleri

Bu bölüm, aynı soruların tekrar açılmaması için var.

| Fikir | Neden hayır |
|---|---|
| Sınırsız toplu Gemini üretimi | Halüsinasyon kısıtı + ölçülen kota (flash 20/gün) |
| Otomatik toplu IdeaSoft push | Operatör kontrolü kaybolur |
| **Ürün fotoğrafı üretimi** | Yanlış port/renk/aksesuar gösteren görsel **iade sebebi**; mevzuat tarafı var. Teknik tabloya uydurma sayı yazmakla aynı sınıf hata. "<3 görsel" kapısının doğru çözümü gerçek görsel temin etmek |
| JSON-LD'de fiyat/stok | Feed'de fiyat yok, stok anlık değişiyor; mağaza zaten canlı basıyor. Yanlış fiyat, fiyat olmamasından kötü (0ar) |
| `-preview` sınıfı Live / native-audio modeller | Habersiz kayboluyorlar; zincire preview model konmaz (0 numaralı ders) |
| Full cloud / login | Ertelendi — asıl fatura para değil; kimlik, senkron, conflict, secret vault, migration ve offline UX |
| Marketplace merkezi | Farklı bir operasyon |

**Koşullu — kapı açılırsa yeniden değerlendirilir:**

| Fikir | Koşulu |
|---|---|
| Şartlı batch üretim | Max 5–10 ürün, satır satır onay, kota göstergesi. **Ürün kararı gerekir**, teknik iş değil |
| JSON-LD'nin IdeaSoft'a gönderilmesi | Saha testi + tema kontrolü + **`<script>` kırpılıyor mu ölçümü** (0ar'de kayıtlı, henüz ölçülmedi) |
| Sesli brifing (TTS) | Faz Ö bitmeden anlamsız — okunacak bir "geçen haftaya göre" verisi yok |
| Imagen (kategori/OG görseli, uygulama materyali) | Ürün fotoğrafı hariç; içerik tarafı açılırsa |

---

## 7. Çalışma yöntemi

1. **Faz başında plan modu** — kapsam ve ölçüm tasarımı konuşulur.
2. **Gerekiyorsa önce ölçüm** — sonuç tasarımı değiştirebilir; bu projede sık oldu.
3. **Yeni ekran veya ortak bileşen varsa Claude Design promptu.** Prompt şunları içerir:
   amaç · gösterilecek veri · durumlar (boş/yükleniyor/hata/çok satır) · mevcut tasarım dili
   (token'lar, `cubic-bezier(.32,.72,0,1)`, açık+koyu tema, kompakt kart geometrisi).
   ⚠️ **Mevcut ekran görüntüleriyle birlikte** verilir ki tasarım sıfırdan değil, var olan
   dilin devamı olsun. Kullanıcı tasarlar → çıktı `design/` klasörüne (handoff zip + PNG).
4. **Uygulama** → harness ile görsel doğrulama (`?empty=1`, `?setup=1`, `?changed=1` gibi
   kipler) → testler.
5. **Saha testi** → sürüm: CHANGELOG bölümü → 5 dosyada sürüm → etiket → `latest.json`
   kontrolü. (Ayrıntı ve tuzaklar brain.md "Süreç / operasyon" bölümünde.)
6. **Faz kaydı brain.md'ye** numaralı madde olarak; burada faz ✅ işaretlenir.

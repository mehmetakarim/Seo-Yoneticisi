# Claude Design promptu — Faz B: Ortak tablo/liste bileşeni

> **Bu bir devam promptudur.** Uygulamanın tasarımı zaten Claude Design'da yapılmıştı
> (`SEO Yöneticisi.dc.html`); **aynı sohbetten devam edilecek.** Yani görsel dil, tipografi
> ve renk paleti orada kurulu — burada **yeniden tanımlanmayacak, üzerine inşa edilecek.**
>
> Eklenecek görseller: `design/faz-b-mevcut/` — o tasarımdan **sonra** eklenen altı SEO araç
> ekranının bugünkü hâli (1440×1000; `opportunities` ve `cannibal` için koyu tema da var).
> Bu ekranlar tasarım sohbetinde yok; ayrışma tam olarak orada başladı.

---

## Ne değişti (devam noktası)

İlk tasarımdan bu yana uygulama büyüdü: ürün ekranının yanına **altı SEO araç ekranı** geldi
(Genel Bakış · Fırsatlar · Yükselmeye yakın · Yarışan sayfalar · Düşüşte olanlar · Satışta
olmayanlar). Bunlar tasarım masasından geçmeden, ihtiyaç doğdukça eklendi — ve her biri kendi
tablo/liste yapısını kurdu.

Kart dili (ürün ekranı) tutarlı kaldı; **tablo dili ayrıştı.** Bu fazın işi tam olarak bu
boşluğu kapatmak: kartlar için yapılanın aynısını tablolar için yapmak.

## Çözülecek sorun

SEO araçları grubunda **altı ekran** var ve tablo/liste yapıları birbirinden ayrışmış:

- Dördü `<table>` kullanıyor, biri (**Yarışan sayfalar**) `div` listesi
- Başlık/dipnot/"daha fazla" stilleri her ekranda ayrı tanımlı (3–5 kopya)
- Satır yüksekliği ve ikincil metin ölçüleri sapmış (bir ekranda 1px, diğerinde 2px)
- **Satır eylemi görünmez:** dört ekranda tek eylem, satırın kendisine tıklamak. Yalnızca
  "Satışta olmayanlar" ekranında görünür düğmeler var, onlar da hücre içine serpiştirilmiş

Sonuç: ekrandan ekrana geçerken ritim bozuluyor ve kullanıcı bir satırla ne yapabileceğini
göremiyor.

## İstenen

**Tek bir tablo/liste bileşeni** ve varyantları. Altı ekranın hepsi bunu kullanacak; ileride
eklenecek ekranlar da (iş kuyruğu, müşteri listesi, teklif listesi) aynı bileşenden kurulacak.

### 1. Temel tablo

Bugünkü ölçüler (**korunmalı, sadece tek yerde tanımlanmalı**):

| Öğe | Değer |
|---|---|
| Başlık hücresi | dolgu `9px 12px` · `11px` · ağırlık `620` · renk `--c-soft` · sola dayalı · satır kaymaz |
| Veri hücresi | dolgu `9px 12px` · `12.5px` · renk `--c-text` |
| Ayırıcı | alt kenarlık `1px solid var(--c-border-soft)` |
| Ad sütunu | genişlik `%36` |
| Sayısal sütun | sağa dayalı, `font-variant-numeric: tabular-nums` |
| İkincil satır (SKU) | `10.5px`, renk `--c-faint`, ada bitişik |

Sütun türleri: **metin** (ad + altında SKU) · **sayı** (sağa dayalı) · **yüzde** · **rozet** ·
**işlem**.

### 2. "İşlem" sütunu — bu fazın asıl yeniliği

En sağda **sabit genişlikte** bir sütun. Kurallar:

- Eylemler ikon düğme; üzerine gelince ipucu (tooltip) çıkar
- **Satıra göre değişir:** bir eylem o satır için geçerli değilse **gizlenir veya pasifleşir**
- **Sütun genişliği eylem sayısından bağımsız sabit** — satır ritmi ekrandan ekrana kaymasın
- Yıkıcı/dışarıya yazan eylem (ör. mağazaya canonical yazma) ayırt edilebilir olmalı ve onay
  ekranı açar
- Bugünkü "satıra tıkla" davranışı kaybolmayacak, ama artık **tek** affordance olmayacak

Bugün ve yakın gelecekte bu sütuna girecek eylemler:

| Eylem | Nerede |
|---|---|
| Ürünü aç | tüm ekranlar (bugün gizli satır tıklaması) |
| Halef öner | Satışta olmayanlar |
| Canonical ayarla | Satışta olmayanlar |
| Sonucu incele | (yakında) ölçüm fazı — "bu işi yaptık, ne oldu?" |
| Kuyruğa ekle | (yakında) iş kuyruğu fazı |
| 301 listesine ekle | (yakında) katalog fazı |

⚠️ Tasarım **3 eylemi rahat**, **5 eylemi sıkışmadan** taşıyabilmeli. Taşma durumunda ne
olacağına karar verilmeli (ör. üç nokta menüsü).

### 3. Gruplu varyant

**Yarışan sayfalar** ekranının verisi gerçekten hiyerarşik: *bir arama sorgusu → o sorguda
yarışan N ürün sayfası*. Bunu düz tabloya zorlamak veriyi yanlış anlatır.

Gerekli: **grup başlığı** (sorgu + toplam gösterim/tıklama) + altında girintili satırlar.
Aynı hücre/sütun dilini kullanmalı ki iki varyant kardeş görünsün.

### 4. Üst şerit ve filtreler

**Fırsatlar** ekranı en zengin olan: özet satırı ("51 fırsat · yaklaşık 322 tıklama
kaçırılıyor"), kategori/marka çipleri, durum filtresi çipleri. Diğer ekranlarda yalnızca özet
satırı var.

İstenen: bu parçaların **ortak bir üst şerit** olarak tanımlanması — bir ekran filtre
kullanmıyorsa şerit sadece özet satırını gösterir, düzen değişmez.

### 5. Durumlar

Hepsi tasarlanmalı, bugün bazıları eksik veya her ekranda farklı:

| Durum | Not |
|---|---|
| **Normal** | 10–15 satır görünür |
| **Boş** | "Bu görünümde kayıt yok" — sakin, suçlayıcı değil |
| **Yükleniyor** | analiz sürerken |
| **Hata** | analiz başarısız (kalıcı şerit, kaybolmayan) |
| **Kısmi liste** | çok satır varsa alt kısımda "daha fazla göster" (bugün `10px 16px`, `12px`) |
| **Uzun metin** | ürün adları iki satıra taşabiliyor — satır yüksekliği buna göre |

### 6. Renk ve tema

⚠️ Bunlar tasarım sohbetinde zaten kurulu; **hatırlatma olarak** buradalar çünkü kodda
kullanılan kesin değerler bunlar. Yeni renk **icat edilmemeli**:

```
--accent: #0071e3        --green: #1a8a4a       --red: #c0392b       --amber: #b7791f
--c-text: #1d1d1f        --c-mid: #5a5a5e       --c-soft: #8a8a8e    --c-faint: #a0a0a6
--c-bg: #ffffff          --c-panel: #f5f5f7     --c-list: #fbfbfc    --c-card: #fdfdfd
--c-border: #e8e8ea      --c-border-soft: #ebebee                    --c-hover: #f2f2f4
```

Durum rozetleri çift olarak tanımlı (metin + zemin):
`eksik` kırmızı · `hatali` kehribar · `bekliyor` mavi · `uygun` yeşil · `tamamlandi` gri.

**Açık ve koyu tema birlikte** tasarlanmalı — uygulamada ikisi de birinci sınıf.
Geçiş/animasyon easing'i: `cubic-bezier(.32, .72, 0, 1)`.

### 7. Ekranların gerçek sütunları

Tasarımın gerçek veriyle sınanması için:

| Ekran | Sütunlar |
|---|---|
| **Fırsatlar** | Ürün · Durum (rozet) · Gösterim · Tıklama · CTR · Konum · Kaçırılan · Sebep (rozet) |
| **Satışta olmayanlar** | Sayfa · Tıklama · Gösterim · Konum · *(+ halef/canonical eylemleri)* |
| **Yükselmeye yakın** | Sorgu / Ürün · Gösterim · Tıklama · Konum · Kaçırılan |
| **Düşüşte olanlar** | Ürün · Tıklama (önce→sonra) · Konum (önce→sonra) · Kayıp |
| **Yarışan sayfalar** | *gruplu:* sorgu başlığı → konum · sayfa adı · tıklama |
| **Genel Bakış** | tablo yok; araç kartları — bu bileşenin kapsamı dışında |

⚠️ **Düşüşte olanlar**'da bir hücre iki değer taşıyor (önceki → şimdiki). Bu "değişim"
sütun türü de tanımlanmalı; ölçüm fazında daha çok kullanılacak.

---

## Beklenen çıktı

1. **Tablo bileşeni** — normal hâli, tüm sütun türleriyle, İşlem sütunu dahil
2. **Gruplu varyant**
3. **Üst şerit** — filtreli ve filtresiz hâli
4. **Durumlar** — boş · yükleniyor · hata · kısmi liste
5. **Açık + koyu tema**
6. İkon dili: hangi eylem hangi ikon (mevcut ikon seti Lucide çizgisinde, `stroke-width` 1.9)

Handoff çıktısı `design/` klasörüne bırakılacak; uygulama ondan sonra başlayacak.

## Sınırlar

- **Var olan tasarımın devamı** — ilk sohbette kurulan kart dili, tipografi ve palet korunur;
  tablo dili onunla kardeş görünmeli, ayrı bir sistem gibi durmamalı
- Yeni renk, yeni gölge, yeni yuvarlaklık ölçüsü **icat edilmeyecek** — var olan token'lar
- Satır yüksekliği bugünkünden belirgin biçimde artmamalı; ekranlarda 50+ satır görülüyor
- Süsleme yok: gradyan, illüstrasyon, dekoratif ikon yok
- Bu bileşen **ileride müşteri ve teklif listelerinde de** kullanılacak; yalnızca SEO
  metriklerine özel bir yapı olmamalı

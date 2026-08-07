# Claude Design promptu — "Bugün" ekranı (Faz K)

> Aşağıdaki metni Claude Design'a olduğu gibi yapıştırın. Çıkan tasarımı `design/faz-k-bugun/`
> klasörüne (veya handoff zip olarak `design/` içine) bırakmanız yeterli; entegrasyonu ben yaparım.
>
> Mevcut ekranın ekran görüntülerini de yükleyin (açık + koyu tema) — "neyi iyileştiriyoruz"
> sorusunun cevabı görsel olarak da elinde olsun.

---

## Prompt

Bir masaüstü uygulaması için **tek bir ekranın** tasarımını yapmanı istiyorum: bir e-ticaret
SEO yönetim aracının **"Bugün"** ekranı.

### Ürün bağlamı

Kullanıcı bir e-ticaret mağazasının SEO'sunu yönetiyor. Uygulamada dokuz ekran var ve her biri
farklı bir analizi listeliyor (fırsatlar, satışta olmayan sayfalar, düşüşte olanlar…). Sorun
şuydu: sabah uygulamayı açan kişi **hangi ekrana gideceğine kendisi karar vermek zorundaydı.**

"Bugün" ekranı bunu çözüyor: 2.226 aday satırdan **10 iş** seçiyor ve sıraya diziyor.
Kullanıcının tek bir sorusu var: **"bugün neye dokunayım?"**

### Ekranın içeriği (veri gerçek, uydurma değil)

**Üst şerit:**
- "10 iş bugün için seçildi" · "16 dakika (tahmini)"
- "Analiz 1 gün önce çalıştı · her kovadan en fazla 3 madde"
- Sağda: "Skor nasıl hesaplanıyor?" düğmesi (bir açıklama paneli açıyor)
- Bazen: "2 madde kuyruktan çıkarıldı — geri getir"

**Kuyruk maddeleri (en fazla 10 tane).** Her maddede şunlar var:

| Alan | Örnek |
|---|---|
| Kova rozeti | `Acil` · `Yüksek kaldıraç` · `Kaçak trafik` · `Sonuç kontrolü` · `Bakım` |
| Başlık | ürün adı ya da sayfa adresi (uzun olabilir, 60+ karakter) |
| Skor | `309` — üstüne gelince formülü söylüyor ("515 tıklama × 0,6 = 309") |
| Süre | `≈2 dk` (tahmin) |
| Neden | tek cümle, **her zaman gerçek bir metrikten**: "515 tıklama satın alınamayan bir sayfaya gidiyor (konum 6.6)" |
| "Ayrıca" | 0–2 ek satır: aynı ürünün başka sebepleri ("ayrıca: tıklama 141→11") |
| Eylemler | **Aç** (birincil) · **Yapıldı** · **Bugünlük ertele** · **Gizle** |

**Alt bilgi:** "Şu an aday çıkarmayan kova: Sonuç kontrolü. Gönderimlerin üstünden 28 gün
geçmesi gerekiyor — en erken 2026-09-15."

**Boş durumlar:** (a) analiz hiç çalışmamış, (b) bugün seçilecek iş yok.

### Çözmeni istediğim tasarım problemleri

1. **Hiyerarşi.** Bir maddede yedi bilgi parçası var (kova, başlık, skor, süre, neden, ayrıca,
   dört eylem). Şu anki hâlinde hepsi neredeyse aynı ağırlıkta ve liste "yoğun" görünüyor.
   Göz önce **ne yapacağını**, sonra **neden**ini görmeli.
2. **Dört eylemin dengesi.** "Aç" birincil eylem (işi başka ekranda yapıyorsunuz). "Yapıldı"
   olumlu ama ikincil. "Bugünlük ertele" ve "Gizle" nadiren kullanılacak — her satırda dört
   düğme durması gürültü yaratıyor olabilir. Bunları nasıl yerleştirirdin?
3. **Kova ayrımı.** Beş kova beş farklı iş türü. Düz bir liste yerine kovalara göre gruplama
   daha mı iyi olur, yoksa tek sıralı liste mi? (Sıralama skora göre; gruplama sıralamayı
   bozmamalı.) İki seçeneği de görmek isterim.
4. **Skorun anlaşılırlığı.** Sayı tek başına anlamsız (309 mu iyi, 37 mi?). Görece büyüklüğü
   sezdirecek bir görsel dil olabilir mi — çubuk, nokta, kalınlık?
5. **Günün tamamlanma hissi.** Kullanıcı 10 işten 4'ünü bitirdiğinde bunu hissetmeli. Bir
   ilerleme göstergesi ekrana ne katardı, nereye konurdu?

### Kısıtlar (bunlara uymak zorunlu)

- **Masaüstü.** Pencere en az 1200px genişlik, 720px yükseklik. Solda 190px sabit bir kenar
  çubuğu var (tasarıma dahil etme, sadece kalan alanı kullan).
- **Açık ve koyu tema**, ikisi de.
- **Apple'ın masaüstü uygulamalarına yakın bir dil:** sakin, az renk, çok boşluk, ince
  kenarlıklar, yumuşak köşeler (10–13px), gölge neredeyse yok.
  **Boşluk bir özelliktir** — sıkıştırma.
- Geçiş/animasyon kullanacaksan `cubic-bezier(.32,.72,0,1)`, 140–200ms.
- **Emoji ve dekoratif illüstrasyon yok.** İkon kullanacaksan Lucide çizgi ikonları (1.9
  kalınlık) — mevcut uygulamanın dili bu.
- Yazı boyutları küçük ve yoğun bir uygulamanın parçası: gövde 11,5–13px, başlık 15–19px.
- Renk paletinde vurgu rengi **#0071e3** (açık ve koyu temada aynı). Rozet renkleri:
  kırmızı/amber/mavi/yeşil/gri aileleri — ama **saf doygun renk değil**, düşük doygunluklu
  arka plan + koyu metin (açık temada), koyu temada tersi.
- Türkçe metin. Yukarıdaki örnek metinleri aynen kullan, uydurma İngilizce yerleştirme metni
  koyma. Uzun ürün adları gerçek: *"Lenovo ThinkPad E16 G3 21SR006RTX U7-255H 16GB 512GB 16'' DOS"*.

### Teslim

- Açık ve koyu tema için birer tam ekran.
- Bir maddenin yakın plan varyasyonları (normal · üstüne gelince · "yapıldı" işaretlenmiş).
- 3. maddedeki iki seçenek (düz liste ↔ kovalara göre gruplu) yan yana.
- Boş durum ekranı ("bugün seçilecek iş yok").
- Kullandığın ölçüleri (dolgu, köşe yarıçapı, yazı boyutları, renk değerleri) yazılı olarak ver
  — koda birebir aktaracağım.

Tasarımı yaparken şunu aklında tut: bu ekran **her sabah** açılıyor. Etkileyici olmasından çok
**yormayan** olması önemli.

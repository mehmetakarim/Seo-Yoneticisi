# Değişiklik Günlüğü

Bu dosya **tek doğruluk kaynağıdır**: yayın sırasında CI, etikete karşılık gelen bölümü buradan
alıp GitHub Release gövdesine ve `latest.json`'ın `notes` alanına yazar. Yani buraya yazdığınız
metin, kullanıcının **uygulama içindeki güncelleme ekranında** gördüğü metindir.

**Yazım kuralları**
- Kullanıcıya dönük yazın, commit mesajı gibi değil: *ne değişti* ve *kullanıcıya ne katıyor*.
- Her madde tek satır, `- ` ile başlar. Güncelleme ekranı satır bazlı render ediyor.
- Teknik iç ayrıntı (crate ayrımı, derleme ayarı vb.) buraya girmez — o bilgi commit'lerde ve
  `brain.md`'de duruyor. Buraya yalnızca kullanıcının fark edeceği şeyler yazılır.
- Başlık biçimi tam olarak `## vX.Y.Z` olmalı; CI bölümü bu satırla eşleştiriyor.

---

## v0.6.4
- Yeni: satıştan kalkmış bir sayfanın canonical'ını, halef ürüne doğrudan uygulamadan yazabilirsiniz
- Yazma öncesi onay ekranı "şu an" ve "olacak" değerini yan yana gösterir; toplu işlem yoktur, her satır tek tek onaylanır
- Onay ekranı bunun bir 301 yönlendirme olmadığını açıkça belirtir — ziyaretçi yine eski sayfaya düşer, yalnızca Google'a asıl sayfa sinyali gider
- "Katalogla eşleştir" ile mağazanızın tüm ürün listesi çekilir; feed dışında kalan ürünler de artık eşleştirilebilir

## v0.6.3
- Yeni: "Düşüşte olanlar" — önceki döneme göre trafik veya sıra kaybeden ürünleri gösterir
- Tıklama ve konumun önce/sonra değerleri yan yana; nerede ne kadar kaybettiğiniz tek bakışta görünür

## v0.6.2
- "SEO Araştır" panelindeki anahtar kelime zorluğu düzeltildi: veri gelmediğinde 0 (yani "çok kolay") görünüyordu, artık veri yoksa hiç gösterilmiyor

## v0.6.1
- Satışta olmayan sayfalar için "Halef öner": güncel neslin hangisi olduğunu yapay zekâ önerir
- Uygun bir halef yoksa bunu açıkça söyler — yanlış yönlendirme önerilmez
- Öneri tek tek alınır, en çok trafik alan sayfalardan başlayın

## v0.6.0
- Yeni: "Yükselmeye yakın sorgular" — hangi aramada kaçıncı sırada olduğunuzu gösterir, o arama doğrudan hedef kelime adayıdır
- Yeni: "Birbiriyle yarışan sayfalar" — aynı aramada çakışan kendi ürünlerinizi bulur
- Fırsatlar ekranını yenilediğinizde bu bölümler dolar

## v0.5.9
- Fırsatlar ekranının boş açılmasına yol açan hata giderildi
- Yeni bölümleri görmek için Fırsatlar ekranında "Yenile" deyip analizi bir kez çalıştırın

## v0.5.8
- Yeni bölüm: satışta olmayan ama Google'dan trafik alan sayfalar — ürün trafiğinizin büyük kısmı buraya gidiyor olabilir
- Fırsat listesinde artık her ürünün SEO durumu görünüyor: hiç dokunulmamış mı, çalışılmış ama sonuç alınamamış mı
- Kategori ve marka kesitleri: kaybın nerede toplandığını gösterir, tıklayınca listeyi filtreler
- Fırsatları iş durumuna ve sebebe göre filtreleyebilirsiniz

## v0.5.7
- Meta ve açıklama için sürüm geçmişi: yeniden üretmeden önceki hâl saklanıyor, beğenmezseniz geri yükleyebilirsiniz
- Geri yüklerken mevcut hâl de saklanıyor — geri yükleme de geri alınabilir
- Her sürümün hangi modelle üretildiği listede görünüyor

## v0.5.6
- Yeni "Fırsatlar" sayfası: Google Search Console verisiyle hangi ürüne öncelik vermeniz gerektiğini sıralar
- Her ürün için kaç tıklama kaçırdığınız ve nedeni gösteriliyor — ikinci sayfada mı, meta mı çekmiyor
- Google'da hiç görünmeyen ürünler ayrı listeleniyor
- Bu ekran artık her sürümde o sürümün gerçek yeniliklerini gösteriyor

## v0.5.5
- Üretimi yapan yapay zekâ modeli artık kart başlığında görünüyor
- Kart düzeni düzeltildi: model bilgisi butonların hizasını bozmuyor

## v0.5.4
- Model listesi güncellendi ve günlük kullanım limitlerine göre sıralandı — bir model dolduğunda
  üretim kesintisiz devam ediyor
- Hangi içeriğin hangi modelle üretildiği kaydediliyor

## v0.5.3
- Üretimi tamamen durduran hata giderildi: kullanımdan kaldırılan bir model artık sıradakini engellemiyor

## v0.5.2
- Butonların üzerine gelince açıklama baloncuğu çıkıyor
- Açıklama kartındaki buton hizası düzeltildi

## v0.5.1
- Otomatik güncelleme altyapısı tamamlandı

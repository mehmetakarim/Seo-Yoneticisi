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

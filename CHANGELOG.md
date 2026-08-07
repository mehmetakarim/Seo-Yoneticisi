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

## v0.9.0
- Yeni: SEO araç ekranlarındaki tüm tablolar **tek tasarıma** kavuştu — satır ritmi, sütun hizası ve başlık tipografisi artık ekrandan ekrana aynı
- Yeni: her satırda **İşlem sütunu** — daha önce yalnızca satıra tıklayarak ulaşılan eylemler artık görünür düğmeler; mağazaya yazan eylemler (canonical) kırmızı
- Satışta olmayanlar ekranında halef önerisi ve canonical ayarlama satırın sağındaki düğmelere taşındı; halef sonucu sayfa adının altında görünüyor
- Fırsatlar ekranındaki filtreler etiketlendi (Kategori · Marka · İş durumu · Sebep) — hangi filtrenin ne olduğu artık belli
- Yarışan sayfalar listesi diğer ekranlarla aynı tablo dilini kullanıyor; sorgu başlıkları altında yarışan sayfalar girintili
- Uzun listelerde "daha fazla göster", boş filtre sonucu ve yükleniyor görünümü de aynı bileşenden geliyor

## v0.8.3
- Yeni: feed değişikliği uyarısında **"Neler değişti?"** — onayladığınız hâl ile şu anki feed verisi yan yana gösteriliyor
- Ad ve açıklama için eski/yeni metin, görseller için çıkan ve gelen küçük resimler; kaç karakterden kaça gittiği de yazıyor
- Karşılaştırma son senkrona değil **onay anınıza** göre yapılıyor: arada iki değişiklik olduysa ikisini de görürsünüz
- Uygulamayı bu sürümden önce tamamladığınız ürünlerde önceki değerler kayıtlı olmadığı için yalnızca değişen alan adı görünür; bundan sonraki değişikliklerde karşılaştırma tam çalışır
- Yeni: hedef kelime **"Getir"** artık teknik tablo verisini de getiriyor — IdeaSoft'un "Teknik Özellikler" sekmesindeki veri XML feed'de gelmiyordu, uygulama bu veriyi ilk kez görüyor
- Gelen tablo düz metne çevrilip teknik tablo kaynağına yazılıyor; oradan **"Yapılandır"** ile düzenli tabloya dönüştürebilirsiniz
- Kaynak alanınız doluysa üzerine yazılmaz — ne yapıldığı (getirildi / korundu / IdeaSoft'ta yok) her seferinde açıkça bildirilir
- Hedef kelime satırındaki "iki kart da bu kelimeyi kullanır" açıklaması kaldırıldı
- Düzeltme: uyarı şeridinin üst boşluğu yoktu, başlığa yapışık duruyordu

## v0.8.2
- Yeni: **feed değişikliği uyarısı** — tedarikçi bir ürünün adını veya açıklamasını değiştirdiğinde, tamamladığınız ürün "Değişti" olarak işaretlenir
- Uyarı hangi alanların değiştiğini yazar; içerik hâlâ doğruysa tek tıkla "İçerik hâlâ doğru" diyip bayrağı kaldırabilirsiniz
- Ürün listesine "Feed değişti" filtresi eklendi — yalnızca değişiklik varken görünür
- Yalnızca gerçek içerik değişiklikleri sayılır: stok hareketi veya biçim/satır sonu farkı uyarı üretmez
- Yeni: her ürün için **Schema.org (JSON-LD)** kartı — ürünü arama motorlarına ve yapay zekâ araçlarına yapılandırılmış veri olarak anlatan kod, tek tıkla kopyalanır
- Teknik özellik tablonuz da bu koda giriyor: mağaza yazılımının kendi çıktısında ürün özellikleri yok, asıl kazanç burada
- Kod yeni içerik üretmez — mevcut ad, marka, kategori, görseller ve teknik tablodan derlenir
- Fiyat ve stok bilinçle dışarıda: mağazanız bunları sayfada zaten canlı basıyor, kopyalanan koda yazılsa bir gün sonra yanlış olurdu

## v0.8.0
- Yeni: **ilk kurulum sihirbazı** — feed adresi, Gemini anahtarı ve isteğe bağlı entegrasyonlar adım adım, her adımda "test et" ile
- Gemini anahtarınız yoksa o adımı atlayabilirsiniz; nereden alacağınız ekranda yazıyor
- Sihirbazı istediğiniz zaman Ayarlar'dan tekrar çalıştırabilirsiniz
- Düzeltme: feed adresi girilmemişken uygulama artık örnek bir mağazanın feed'ine düşmüyor — kendi adresinizi girmeden senkron yapılmıyor

## v0.7.2
- Yeni: AI Asistanı sohbetleri artık kaydediliyor — uygulamayı kapatıp açtığınızda geçmiş sohbetlerinize dönebilirsiniz
- Yeni: geçmiş sohbetler tek tek veya toplu olarak silinebiliyor; silme öncesi onay isteniyor
- Düzeltme: canonical hedefi araması artık yalnızca **satıştaki** ürünleri listeliyor — daha önce satıştan kalkmış ürünler de çıkıyordu ve ölü bir sayfa başka bir ölü sayfaya yönlendirilebiliyordu
- Düzeltme: Yükselmeye yakın, Yarışan sayfalar, Düşüşte olanlar ve Satışta olmayanlar ekranlarına doğrudan girildiğinde analiz verisi görünmüyordu

## v0.7.1
- Düzeltme: kenar çubuğunda asistan sekmesi "AI Asistanı" olarak kısaldı — uzun ad iki satıra kırılıp menü hizasını bozuyordu

## v0.7.0
- Yeni: her SEO aracı artık kendi ekranında — Fırsatlar, Yükselmeye yakın, Yarışan sayfalar, Düşüşte olanlar, Satışta olmayanlar
- Yeni: "Genel Bakış" ekranı — analizi buradan çalıştırın, hangi araçta ne kadar kayıp olduğunu tek bakışta görün
- Yeni: kenar çubuğu gruplandı (Katalog · SEO Araçları · Asistan · Sistem); tek uzun kaydırma yerine her iş kendi yerinde
- Yeni: Yapay Zekâ Asistanı — bulunduğunuz ekranın verisini konuşarak sorgulayın, yanıt yazılırken akar
- Asistan yalnızca ekrandaki veriye bakar; veride olmayan bir şey sorulursa bilmediğini söyler ve hiçbir değişiklik yapmaz

## v0.6.5
- Düzeltme: canonical ayarlamak için artık katalog senkronu beklemeniz gerekmiyor — uygulama ilgili ürünü saniyeler içinde buluyor
- Yeni: yapay zekâ uygun halef bulamadığında hedefi kendiniz arayıp seçebilirsiniz; öneri geldiğinde de onay ekranından değiştirebilirsiniz
- Onay ekranı artık hedef ürünün adını da gösteriyor; hedef mağazanızda bulunamazsa yazma adımına geçilmiyor

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

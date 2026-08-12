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

## v0.19.0
- Yeni: **İçerik açığı** ekranı (SEO Araçları'nın yedincisi). Google'ın sizi sıraladığı ama çıkan sayfanın aramanın niyetine uymadığı aramaları gösteriyor — kategori, marka ve blog sayfalarınız ilk kez uygulamada görünür oldu
- Üç durum ayrı ayrı: **niyet uyuşmuyor** (vitrin sayfası bilgi aramasına çıkıyor) · **sayfa yok** (aramayı anasayfa karşılıyor) · **yanlış eşleşme** (Google alakasız bir ürün sayfasını seçmiş)
- ⚠️ Kurtarılamayacak aramalar listeye alınmıyor: bir markanın adı arandığında kişi üreticinin sitesine gitmek istiyor ve sıralamayı yükseltmek bunu değiştirmiyor. Hangi aramaların neden elendiği ekranda yazıyor
- Yeni: **Sayfa iyileştirme paneli.** Kategori/marka/blog sayfanız için başlık, açıklama, hedef kelime ve tanıtım metni üretiliyor; mağazadaki hâliyle yan yana görüp düzenleyip gönderiyorsunuz
- Üretim, o sayfanın Google'da aldığı **gerçek aramalara** ve kataloğunuzdaki **gerçek ürünlere** dayanıyor. Ölçüm verisi yoksa panel bunu söylüyor
- ⚠️ Taslak metin doğrulanmıyor: sayısal iddialar engelleniyor ama diğer bilgiler kontrol edilemiyor — göndermeden önce okuyun
- Yeni: Ayarlar'da **sayfa envanteri** — kategori, marka ve blog kayıtlarınız çekiliyor; hangilerinde başlık/açıklama eksik olduğunu gösteriyor
- Bugün listesine **İçerik** kovası eklendi. Günlük iş sayısı 13'ten 14'e çıktı: yeni kova mevcut işlerin yerini almıyor

## v0.18.0
- Yeni: **Yapay Zekâ Modelleri** ayarı. Üretimin hangi modelleri hangi sırayla deneyeceğini artık siz belirliyorsunuz. Bir model emekliye ayrıldığında yeni sürüm beklemeden listeyi düzeltebilirsiniz — daha önce bu, üretimi tamamen durduran bir sorundu
- Model listesi Google'dan **canlı** geliyor: "Modelleri getir" deyince o an sunulan modelleri görürsünüz, emekli olanlar listede hiç çıkmaz
- Her modelin yanında **Dene** düğmesi: uygulamanın gerçek istek biçimiyle tek çağrı yapar, model destekliyor mu anında söyler
- Asistan için **ayrı zincir**: sohbet, üretimin dar günlük kotasını tüketmesin diye
- Yeni: **Gemini Kullanımı** kartı. Bugün hangi modele kaç istek gitti, kaç kez kota doldu, son 14 günün dağılımı ve üretim başına ortalama istek sayısı
- ⚠️ Sayaç yalnızca bu uygulamanın gönderdiği istekleri sayar; aynı anahtarı başka yerde kullanıyorsanız gerçek toplam daha yüksektir. Bu yüzden "kalan hakkınız" gösterilmiyor

## v0.17.2
- Düzeltme: teklifte **katalogdan ürün eklenemiyordu** — *"Ürün bulunamadı"* hatası veriyordu. Artık ekleniyor
- Düzeltme: kişi kartında bir ürünü "ilgilendiği ürünler"e bağladığınızda bağ **yanlış kaydediliyordu**. Hata görünmüyordu ama ürün ekranındaki "bu ürünle ilgilenenler" listesi hiç dolmuyordu
- Daha önce bu şekilde kaydedilmiş ürün bağlarınız güncellemede **kendiliğinden onarılıyor**

## v0.17.1
- Düzeltme: **PDF olarak kaydet** çalışmıyordu — *"Not allowed to open path…"* hatası veriyordu. Belge artık düzgün açılıyor
- Düzeltme: teklif tablosundaki sütun kayması. Pencere büyüdükçe sayılar başlıklarından uzaklaşıyordu; sütun genişlikleri sabitlendi
- Yeni: satırda yaptığınız son değişikliği **geri alma** düğmesi (satır sonunda beliriyor)

## v0.17.0
- Yeni: **Teklif belgesi.** Teklif ekranındaki "Belge" düğmesi müşteriye gidecek hâli gösteriyor — maliyet ve marj bu belgede yer almaz
- **Panoya kopyala** biçimli kopyalıyor: mail'e tablo olarak yapışıyor, kaynak kod olarak değil
- **PDF olarak kaydet**: belge varsayılan tarayıcınızda açılıyor, oradan yazdırma penceresiyle PDF'e kaydediyorsunuz. Yazıcı gerekmiyor
- Belge Outlook ve Gmail'de düzgün görünecek biçimde kuruldu (sabit genişlik, tablo düzeni) — daha önce mailde dağılıyordu
- Yeni: teklifi **gönderildi** işaretlediğinizde, müşteriye ne zaman döneceğiniz soruluyor. Kabul ederseniz o gün Bugün listenizde çıkıyor. ⚠️ Kişinin zaten bir randevusu varsa üzerine yazılmıyor
- Yeni: Teklifler ekranında **kazanma/kaybetme özeti** ve kayıp nedenleri ("fiyat 2 · termin 1")
- Kişi kartında o kişiye verilen **teklifler** listeleniyor
- Düzeltme: teklif tablosunda sayı sütunları başlıklarıyla hizalanmıyordu
- Yeni: **Vazgeç** düğmesi — teklif başlığında kaydedilmemiş değişiklikleri geri alır (satır değişiklikleri anında kaydedilir)
- Ayarlar'da teklif belgesinin **firma adı** ve **alt notu** (ödeme koşulu, teslim süresi)
- Yedekleme teklifleri de taşıyor

## v0.16.0
- Yeni: **Teklifler ekranı.** Satırları kataloğunuzdan seçiyorsunuz; fiyat, KDV ve maliyet ürünün kendi verisinden geliyor. Katalogda olmayan kalemler (montaj, nakliye) elle yazılıyor
- ⚠️ Bu özellik feed'inizde **fiyat alanları** ister: `buyingPrice`, `price1`, `tax`, `currencyAbbr`, `priceTaxWithCur`. Alanlar yoksa teklif satırları boş fiyatla gelir
- **KDV oranı ürün başına** uygulanıyor — kataloğunuzda %20 ve %10 birlikte varsa teklifte ikisi ayrı satırda görünüyor
- Teklif **USD veya TL** olabilir. TL teklifte kur sorulmuyor: mağazanızın kendi TL fiyatı kullanılıyor. USD teklifte yalnızca USD olmayan ürünler (EUR, TL fiyatlı) için kur giriyorsunuz
- 🔴 **Marj yalnızca sizde.** Her satırda ve teklif toplamında kâr yüzdesi görünüyor; %10 altı sarı, zararına olan kırmızı uyarı veriyor. Maliyet bilgisi müşteriye giden hiçbir yere yazılmıyor
- Teklif durumu izleniyor: taslak → gönderildi → kazanıldı/kaybedildi. Her değişiklik, teklif bir müşteriye bağlıysa **o kişinin zaman çizelgesine** düşüyor
- Kaybedilen teklifte neden kaydediliyor (fiyat, termin, rakip…) — sonradan raporlanabilmesi için
- Listede açık tekliflerin toplamı, süresi geçenler ayrı rozetle görünüyor
- Ayarlar'da teklif varsayılanları: elle satırların KDV oranı ve geçerlilik süresi

## v0.15.0
- Yeni: **Müşteriler ekranı.** Site vitrin, asıl satış mail ve telefonla yürüyor — bu taraf bugüne kadar uygulamanın dışındaydı. Kişi/firma kartı, kanal, ilgi etiketleri ve temas geçmişi artık burada
- **Sonraki adım tarihi** fazın kalbi: bir kişiye tarih verdiğinizde o gün Bugün listesinde çıkıyor, dönüş yapınca temizleniyor. Odak seansı da müşteri işini kilitleyebiliyor
- Günlük liste **10'dan 13 maddeye** çıktı: müşteri işleri SEO işlerinin yerini almasın diye
- Yeni: **CSV'den kişi aktarma.** Excel listenizi yükleyip sütunları kendiniz eşliyorsunuz — Türkçe Excel'in noktalı virgüllü biçimi de okunuyor. Aktarmadan önce ilk 5 satırın nasıl geleceğini gösteriyor
- Aynı e-posta veya telefona sahip kişi varsa yeni kayıt açılmıyor, mevcut kayıt güncelleniyor. CSV'de boş bıraktığınız sütunlar uygulamadaki bilgiyi silmiyor
- Yeni: **"Bu ürünle ilgilendi"** bağı — kişiyi kataloğunuzdaki ürüne bağlıyorsunuz, ürün detayında "3 kişi bu ürünle ilgilendi" görünüyor
- Yeni: **sessiz müşteri uyarısı**, varsayılan olarak kapalı. Yeterli temas biriktiğinde uygulama sizin kendi ritminizi ölçüp bir eşik öneriyor ("ortalama 25 günde bir dönüyorsunuz") — kabul etmek sizin kararınız
- ⚠️ Müşteri bilgileri **AI asistanına gönderilmiyor**; asistan bağlamı Google'a gidiyor, kişisel veri oraya girmez. Yedek dosyası kişi kayıtlarını içeriyor ve Ayarlar ekranı bunu artık söylüyor
- Düzeltme: **"Yapıldı" dediğiniz iş artık ertesi gün geri gelmiyor.** İki sebebi vardı: liste güne değil son analize bağlıydı, ayrıca "yapıldı" acil maddelerin sebebini (feed değişiklik notu) temizlemiyordu
- Düzeltme: içeriği mağazaya elle yapıştırdığınızda ürün "hiç gönderilmemiş" olarak listede kalıyordu; artık "Yapıldı" işareti gönderim sayılıyor
- Düzeltme: bilgi satırlarındaki kalın yazılar cümleyi üç parçaya bölüyordu

## v0.14.1
- Düzeltme: **Yaptığınız iş artık kuyruğa geri gelmiyor.** "Yapıldı" işareti analiz yenilenince düşüyordu; oysa bir düzeltmenin sonucu Google Search Console'da haftalar sonra görünüyor (90 günlük pencere). Yani işi yapmanız maddeyi listeden düşürmüyordu
- Mağazaya ulaşan bir iş yaptıysanız o madde **28 gün boyunca** yüksek kaldıraç, kaçak trafik ve bakım kovalarında yeniden çıkmıyor. Kaybolmuyor: süre dolunca **sonuç kontrolü** kovasında, bu kez "sonucuna bakılabilir" diye geliyor
- Acil kovası bundan etkilenmiyor — kanıtı Search Console değil, mağazanızın kendi verisi: dün gönderdiğiniz üründe bugün metin değiştiyse bu bugün bilinen bir gerçek
- Bugün ekranı kaç işin beklediğini söylüyor: **"81 iş ölçüm bekliyor"** — üstüne gelince neden beklediğini açıklıyor
- Satışta olmayan sayfalarda **kararını verdiğiniz sayfalar** artık günlük listede çıkmıyor: 301 yönlendirmesini panelde tanımladıysanız da, sayfayı bilinçli tutuyorsanız da bu bir karar; kuyruk onu tekrar önermiyor

## v0.14.0
- Yeni: **Satışta olmayan sayfalar için kalıcı karar deposu.** Verdiğiniz halef kararları artık uygulama kapansa da duruyor — daha önce yalnızca o oturum boyunca hatırlanıyordu
- Her sayfa için üç karar: **301 yönlendir** (hedefi siz seçiyorsunuz) · **canonical** · **bilinçli tutuyorum**. Karar verilen satırlar listede rozetle işaretli, aynı sayfa her analizde yeniden karşınıza çıkmıyor
- Yeni: **301 CSV çıktısı** — kararlarınız IdeaSoft paneline elle girilebilecek bir listeye dönüşüyor: kaynak yol, hedef yol, tıklama, konum, karar tarihi
- ⚠️ Karar vermediğiniz satırlarda **hedef sütunu bilerek boş**. En iyi aday ayrı bir bilgi sütununda duruyor ama hedefe yazılmıyor: otomatik eşleştirme güvenilir değil ve yanlış yönlendirme, yönlendirmemekten kötüdür
- CSV'de doldurduğunuz hedefler uygulamaya geri dönmüyor — dosya panele girmek için, uygulama yalnızca kendi içinde verdiğiniz kararları bilir
- Yeni: **Ürün sağlık skoru** — Ürünler listesinde 0–100 arası bir rozet; üstüne gelince neyin eksik olduğunu puanlarıyla söylüyor (meta 25 · açıklama 25 · teknik tablo 20 · görsel 15 · mağazaya gönderim 15)
- Skor, durum rozetinin yerini almıyor yanına geliyor. Farkı şu: durum rozeti içeriğin **mağazaya ulaşıp ulaşmadığına bakmıyor**. Yerelde "Tamamlandı" görünen bir ürün Google için hiç yapılmamış olabilir — skor bunu 85 puanla ve "mağazaya gönderim" eksiğiyle söylüyor

## v0.13.0
- Yeni: **Odak seansı** — Bugün ekranındaki "Odak seansı başlat" düğmesi kuyruktan **tek iş** kilitler ve üst şeridin altında ince bir çubuk açar. Çubuk tüm ekranlarda durur, çünkü işi başka ekranlarda yapıyorsunuz
- Çubukta: kalan süre, kilitli işin adı, bu işte ne kadardır çalıştığınız ve dört düğme — **Aç · Bitti · Atla · Ertele**. "Atla" yalnızca o seans için; iş kuyrukta kalır
- Süre dolunca mola **önerilir**, kendiliğinden başlamaz. Kuyruk biterse seans erken biter ve sakin bir özet gelir: kaç iş, ne kadar sürdü, hangi kovalar
- **Kuyruktaki süreler artık ölçülüyor.** Bugüne kadar "≈2 dk" gibi tahminlerdi; seans gerçek süreyi ölçüyor ve bir kovada 5 iş bitirdiğinizde tahminin yerini alıyor. Ekran ikisini ayırt ediyor: "≈2 dk" tahmin, "4 dk" ölçüm
- Ayarlar'da **seans ve mola süresi** (varsayılan 25/5) ile hangi kovada kaç ölçüm biriktiği görünüyor
- Seans puan, rozet veya seri tutmuyor — amaç sakin bir çalışma ritmi
- Düzeltme: günün işlerinin tamamı bitmişken "Odak seansı başlat"a basıldığında "Seans bitti · 0 iş" penceresi çıkıyordu. Artık düğme pasif ve sebebini söylüyor; boşuna seans kaydı da oluşmuyor

## v0.12.0
- Yeni: **Bugün ekranı** — menünün en üstünde, uygulama artık burada açılıyor. Sabah "hangi ekrana gideyim?" diye düşünmeden, o gün dokunulacak 10 işi sıralı olarak görüyorsunuz
- İşler beş kovadan seçiliyor: acil (canlıda bayat içerik) · yüksek kaldıraç (kaçırılan tıklama) · kaçak trafik (satışta olmayan sayfalar) · sonuç kontrolü · bakım. Her kovadan en fazla 3 madde, böylece liste tek bir konuya saplanmıyor
- Her maddede **neden** yazıyor ve bu cümle her zaman gerçek bir ölçümden geliyor: "515 tıklama satın alınamayan bir sayfaya gidiyor (konum 6.6)"
- **Skor şeffaf**: yanındaki çubuk maddenin görece ağırlığını gösteriyor, üstüne gelince formülü çıkıyor, "Skor nasıl hesaplanıyor?" düğmesi ağırlıkları gerekçeleriyle açıklıyor
- Maddeye tıklayınca doğru ekranda **doğru satıra** gidiyorsunuz — satır seçili ve görünür oluyor, listenin altlarında kalsa bile
- **Yapıldı** düğmesi: madde listeden kaybolmuyor, üstü çizilip "3 / 10 bitti" ilerlemesine ekleniyor. Yanlışlıkla bastıysanız "geri al"
- Yaptığınız işi mağazada elle yaptıysanız (örneğin 301 yönlendirme) "Yapıldı" artık sonuç takibini de başlatıyor — bu işler daha önce hiç ölçülemiyordu
- Üzerinde çalışmayacağınız maddeler için **Bugünlük ertele** ve **Gizle**; gizlenenler sayaçta görünüyor ve tek tıkla geri geliyor
- Koyu temaya geçtiğinizde **pencere çerçevesi de koyuluyor** — başlık çubuğu artık uygulamanın geri kalanıyla uyumlu

## v0.11.0
- Yeni: **AI Asistanı artık hangi verilerle konuşacağınızı size sorduruyor** — giriş kutusunun solundaki **+** düğmesinden veri kaynağı seçiliyor. Beş SEO aracı, Sonuçlar ve Katalog listede
- Seçim **sohbetin ortasında değiştirilebiliyor**: aynı pencerede önce Fırsatlar'ı, sonra Satışta olmayanlar'ı konuşabilirsiniz. Yeni sohbet açmaya gerek yok
- Seçili kaynaklar girişin üstünde **çip** olarak duruyor — asistanın neyi gördüğünü görüyorsunuz. Eski "önce o ekrana gidin" uyarısı kalktı
- Asistan, seçili olmayan bir ekranın verisi sorulduğunda artık "bu veride yok" demiyor; o kaynağın seçili olmadığını söyleyip **+** menüsünü gösteriyor
- Kaydedilen sohbetler kaynak seçimini de hatırlıyor; eski sohbetleriniz olduğu gibi açılıyor
- Düzeltme (**Satışta olmayanlar**): bir sayfa için "Halef öner"e basıldığında listedeki **tüm** satırların düğmesi pasifleşiyordu. Artık yalnızca o satır bekliyor, diğer satırlarla aynı anda çalışabiliyorsunuz
- Düzeltme: halef sonucu geldiğinde sıradaki adım görünmüyordu. Sonuç yazısı artık tıklanabilir — "Uygun halef bulunamadı — hedef seçin" doğrudan seçim ekranını açıyor
- Düzeltme: "Halefi yeniden öner" düğmesi hiçbir şey yapmıyordu; artık gerçekten yeniden öneriyor

## v0.10.0
- Yeni: **sonuç takibi** — "yaptığım iş işe yaradı mı?" sorusu artık cevaplanıyor. Search Console geçmişi bir kez getiriliyor, mağazaya yaptığınız gönderimlerin öncesi ve sonrası karşılaştırılıyor
- Genel Bakış'ta **Sonuçlar şeridi**: kaç gönderim izleniyor, kaçı iyileşti/değişmedi/geriledi, net kaç tıklama
- Fırsatlar tablosuna **Sonuç sütunu** — hangi üründe gönderim sonrası ne olduğu satırda görünüyor
- Ürün ekranında **Geçmiş ve sonuç** kartı: ne yaptınız, ne zaman, sonra ne oldu
- Ölçülen olay **mağazaya ulaşan** olaydır: yerel "tamamlandı" işaretlemesi Google'ın gördüğünü değiştirmediği için puanlanmıyor, yalnızca geçmişte görünüyor
- İçeriği elle kopyalayıp yapıştırdığınız ürünler **ölçülemiyor** ve bu açıkça yazılıyor — gönderim düğmesini kullanırsanız sonraki değişikliğin etkisi ölçülür
- Düzeltme: Fırsatlar tablosunda sütunlar satırdan satıra kayıyordu (rozet uzunluğu sütun genişliğini değiştiriyordu); başlıklar ve değerler artık her satırda aynı hizada
- Nedensellik iddia edilmiyor: "bu iş sayesinde arttı" değil, "gönderimden sonraki dönemde arttı". Etkinin görünmesi en az 21 gün alıyor

## v0.9.0
- Yeni: SEO araç ekranlarındaki tüm tablolar **tek tasarıma** kavuştu — satır ritmi, sütun hizası ve başlık tipografisi artık ekrandan ekrana aynı
- Yeni: her satırda **İşlem sütunu** — daha önce yalnızca satıra tıklayarak ulaşılan eylemler artık görünür düğmeler; mağazaya yazan eylemler (canonical) kırmızı
- Satışta olmayanlar ekranında halef önerisi ve canonical ayarlama satırın sağındaki düğmelere taşındı; halef sonucu sayfa adının altında görünüyor
- Fırsatlar ekranındaki filtreler etiketlendi (Kategori · Marka · İş durumu · Sebep) — hangi filtrenin ne olduğu artık belli
- Yarışan sayfalar listesi diğer ekranlarla aynı tablo dilini kullanıyor; sorgu başlıkları altında yarışan sayfalar girintili
- Uzun listelerde "daha fazla göster", boş filtre sonucu ve yükleniyor görünümü de aynı bileşenden geliyor
- Yeni: **Genel Bakış** ekranı da aynı tasarıma alındı — özet şeridi kart diline uydu, araç kartları 2+3 düzeninde tam genişlikte
- Düzeltme: Fırsatlar ekranındaki kategori ve marka filtrelerinin yanındaki sayı **kaçırılan tıklamayı** gösteriyordu, ürün sayısı sanılıyordu. Artık her filtrede sayı **ürün adedi**; kaçırılan tıklama bilgisi üzerine gelince çıkıyor
- Düzeltme: Fırsatlar özet satırı iki farklı kümeden sayı okuyordu ("51 fırsat · 24 tıklama"); filtre etkinken artık ikisi de aynı kümeden ve toplam ayrıca yazıyor
- Düzeltme: Fırsatlar ekranında sayfanın içinde iki ayrı kaydırma çubuğu daha vardı — tablonun kendi dikey çubuğu ve "Google'da görünmeyenler" listesinin çubuğu. İkisi de kaldırıldı; kaydırma yalnızca sayfada
- Düzeltme: geniş tablo dar pencerede gereksiz yatay kaydırma açıyordu; sütun genişlikleri başlık içeriğine göre hesaplanıyor

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

export type MetaBadge = "eksik" | "hatali" | "uygun" | "tamamlandi";
export type OverallStatus =
  | "eksik"
  | "hatali"
  | "bekliyor"
  | "uygun"
  | "tamamlandi";

export interface SyncSummary {
  run_at: string;
  active: number;
  added: number;
  updated: number;
  deleted: number;
  duplicate_skipped: number;
}

export interface ProductRow {
  sku: string;
  name: string;
  brand: string | null;
  img_url: string | null;
  meta_badge: MetaBadge;
  details_badge: MetaBadge;
  overall: OverallStatus;
  meta_done: boolean;
  details_done: boolean;
  tech_done: boolean;
  image_count: number;
  /** Doluysa: onaydan sonra feed verisi değişti; değişen alanların adı ("ad, açıklama"). */
  feed_changed: string | null;
  /** SEO sağlık skoru 0–100 (Faz D). ⚠️ `overall`ın yerine değil, yanına. */
  health: number;
  /** Skoru düşüren bileşenler — baloncukta gösteriliyor. */
  health_missing: HealthMissing[];
}

export interface ProductDetail {
  sku: string;
  name: string;
  brand: string | null;
  main_category: string | null;
  category: string | null;
  quantity: number | null;
  url: string | null;
  img_url: string | null;
  title: string | null;
  descriptions: string | null;
  keywords: string | null;
  search_keywords: string | null;
  details: string | null;
  meta_status: string;
  details_status: string;
  target_keyword: string | null;
  draft_title: string | null;
  draft_descriptions: string | null;
  draft_keywords: string | null;
  draft_search_keywords: string | null;
  draft_details: string | null;
  badge: MetaBadge;
  details_badge: MetaBadge;
  overall: OverallStatus;
  // Faz 7: galeri görselleri + skoru
  gallery: string[];
  image_count: number;
  image_badge: MetaBadge;
  image_check: ImageCheck[] | null;
  // Faz 8: teknik özellik tablosu
  tech_source_text: string | null;
  tech_specs: TechGroup[] | null;
  tech_status: string;
  tech_badge: MetaBadge;
  tech_history: TechVersionMeta[];
  ideasoft_pushed_at: string | null;
  ideasoft_seo_rule: number | null;
  /** İçeriği hangi Gemini modelinin ürettiği. Zincir kotaya takıldıkça alt modellere
   *  düşüyor; kullanıcı bunu görüp limitler yenilendiğinde yeniden üretebilir. */
  meta_model: string | null;
  details_model: string | null;
  tech_model: string | null;
  /** Yeniden üretimden önceki hâller (en yeni başta). */
  meta_history: MetaVersionMeta[];
  details_history: DetailsVersionMeta[];
  /** Doluysa: onaydan sonra feed verisi değişti — bkz. ProductRow.feed_changed. */
  feed_changed: string | null;
}

export interface MetaVersionMeta {
  at: string;
  title: string;
  model: string;
}
export interface DetailsVersionMeta {
  at: string;
  words: number;
  model: string;
}

/** IdeaSoft gönderim öncesi fark önizlemesi. */
export interface IdeasoftRemote {
  id: number;
  sku: string;
  name: string;
  page_title: string;
  meta_description: string;
  meta_keywords: string;
  search_keywords: string;
  target_keyword: string;
  details: string;
  extra_details: string;
  seo_rule_count: number | null;
}
export interface IdeasoftPreview {
  id: number;
  remote: IdeasoftRemote;
  local: Record<string, unknown>;
}

/** Önceki teknik tablo sürümünün özeti (en yeni başta). */
export interface TechVersionMeta {
  at: string;
  rows: number;
  groups: number;
}

export interface TechRow {
  label: string;
  value: string;
}
export interface TechGroup {
  group: string;
  rows: TechRow[];
}
export interface TechSpecsResult {
  groups: TechGroup[];
  /** Kaynakta doğrulanamadığı için atılan satırların etiketleri. */
  dropped: string[];
}

export interface ImageCheck {
  url: string;
  width: number;
  height: number;
  is_square: boolean;
  meets_min: boolean;
  ok: boolean;
  error: string | null;
}

export interface Settings {
  feed_url: string;
  gemini_api_key: string;
  capsolver_api_key: string;
  seo_country: string;
  gsc_site_url: string;
  gsc_client_email: string;
  ideasoft_domain: string;
  ideasoft_token: string;
  ideasoft_active: boolean;
  theme: string | null;
  last_backup_at: string | null;
}

// Faz 4: gerçek SEO araştırma verisi (Rust SeoInsights ile birebir).
export interface KeywordCand {
  keyword: string;
  difficulty: number;
  volume: number;
  kind: string; // "idea" | "question"
}
export interface KeywordDifficulty {
  keyword: string;
  difficulty: number;
  shortage: number;
  last_update: string;
}
export interface GscQuery {
  query: string;
  clicks: number;
  impressions: number;
  ctr: number;
  position: number;
}
export interface TrendTerm {
  term: string;
  volume: number;
}
export interface DomainOverview {
  domain: string;
  domain_rating: number;
  backlinks: number;
  ref_domains: number;
}
export interface SeoInsights {
  seed: string;
  target_candidates: KeywordCand[];
  seed_difficulty: KeywordDifficulty | null;
  gsc_queries: GscQuery[];
  trends: TrendTerm[];
  domain: DomainOverview | null;
  fetched_at: string;
  notes: string[];
}

export type FilterKey =
  | "eksik"
  | "hatali"
  | "bekliyor"
  | "uygun"
  | "tamamlandi"
  | "degisti"
  | "tumu";

/** Fırsat analizi: bir ürünün neden listede olduğu. */
export type OpportunityReason = "second_page" | "no_clicks" | "low_ctr";

export interface Opportunity {
  sku: string;
  name: string;
  url: string;
  clicks: number;
  impressions: number;
  ctr: number;
  position: number;
  /** Konumunun getirmesi gereken tıklamanın kaçını alamıyor — sıralama buna göre. */
  missed_clicks: number;
  reason: OpportunityReason;
  /** Bağlam alanları — eski önbellekte olmayabilir, bu yüzden boş string dönebilir. */
  category: string;
  brand: string;
  meta_status: string;
  details_status: string;
}

/** Ürünün SEO iş durumu — "üret" ile "neden işe yaramadı?" farklı işler. */
export type WorkState = "untouched" | "partial" | "worked";

export function workState(o: Opportunity): WorkState {
  const m = o.meta_status === "done";
  const d = o.details_status === "done";
  if (m && d) return "worked";
  if (!m && !d) return "untouched";
  return "partial";
}

export interface InvisibleProduct {
  sku: string;
  name: string;
  url: string;
}

/** Satışta olmayan ama trafik alan sayfa — katalogda yok, Google'da hâlâ sıralanıyor. */
export interface EolPage {
  url: string;
  slug: string;
  clicks: number;
  impressions: number;
  position: number;
}

/** Bir ürünün BELİRLİ BİR SORGUDAKİ fırsatı — "ne yazmalıyım" katmanı. */
export interface QueryOpportunity {
  sku: string;
  name: string;
  query: string;
  clicks: number;
  impressions: number;
  ctr: number;
  position: number;
  missed_clicks: number;
}

export interface CannibalPage {
  sku: string;
  name: string;
  clicks: number;
  impressions: number;
  position: number;
}

/** Aynı sorguda yarışan kendi sayfalarımız. Tespit; birleştirme kararı operatörde. */
export interface Cannibalization {
  query: string;
  clicks: number;
  impressions: number;
  pages: CannibalPage[];
}

/** Önceki döneme göre gerileyen sayfa. */
export interface Decay {
  sku: string;
  name: string;
  clicks_now: number;
  clicks_before: number;
  position_now: number;
  position_before: number;
  clicks_lost: number;
}

export interface OpportunityReport {
  analyzed_at: string;
  days: number;
  opportunities: Opportunity[];
  /** GSC'de hiç satırı olmayanlar — farklı bir iş (indeksleme), ayrı listede. */
  invisible: InvisibleProduct[];
  total_products: number;
  matched: number;
  /** En çok tıklama alan başta. */
  eol: EolPage[];
  eol_clicks: number;
  striking: QueryOpportunity[];
  cannibalization: Cannibalization[];
  decay: Decay[];
}

/** EOL sayfa için halef ürün adayı (deterministik sıralama — öneri değil, aday). */
export interface SuccessorCandidate {
  sku: string;
  name: string;
  score: number;
}

/** Yapay zekânın halef kararı. `sku` boşsa "uygun halef yok" demektir. */
export interface SuccessorSuggestion {
  sku: string | null;
  name: string | null;
  url: string | null;
  reason: string;
  model: string;
  candidates: SuccessorCandidate[];
}

export interface CatalogSyncResult {
  fetched: number;
  synced_at: string;
  matched_eol: number;
}

/** Canonical yazmadan önce gösterilen fark. */
export interface CanonicalPreview {
  product_id: number;
  product_name: string;
  /** Canonical'ın işaret edeceği ürünün adı. */
  target_name: string;
  current: string;
  proposed: string;
  will_create: boolean;
}

/** IdeaSoft'ta bulunan ürün — canonical hedefi seçerken listelenir. */
export interface CatalogMatch {
  slug: string;
  id: number;
  name: string;
  status: number;
  stock: number;
  canonical: string;
}

/** Asistan sohbetindeki tek mesaj. `role` Gemini'nin beklediği adlandırma. */
export interface ChatMessage {
  role: "user" | "model";
  text: string;
}

/**
 * Akış olayı. `thinking` gerçek bir sinyal: Gemma akışta iç muhakemesini de yayınlıyor,
 * onu filtreliyoruz ama "model çalışıyor" bilgisini arayüze taşıyoruz.
 */
export type AssistantEvent = { kind: "thinking" } | { kind: "chunk"; text: string };

/** Kaydedilmiş bir sohbetin liste görünümü (mesaj gövdeleri taşınmaz). */
export interface ChatSessionMeta {
  id: number;
  title: string;
  /** Hangi araç ekranının verisiyle konuşulduğu; boş olabilir. */
  tool_page: string;
  messages: number;
  model: string;
  updated_at: string;
}

/** Onay anındaki hâl ile şu anki feed verisi arasındaki tek alanlık fark. */
export interface FeedFieldDiff {
  field: string;
  old: string;
  new: string;
}

/** "Ne değişti?" cevabı — bkz. `get_feed_diff` komutu. */
export interface FeedDiff {
  /** false ise önceki değerler kayıtlı değil (ürün, özellik eklenmeden önce onaylanmış). */
  has_snapshot: boolean;
  changed_fields: string[];
  fields: FeedFieldDiff[];
  images_old: string[];
  images_new: string[];
}

/** IdeaSoft "Getir" sonucu — mesaj arka uçta kuruluyor (bkz. `IdeasoftPull`). */
export interface IdeasoftPull {
  detail: ProductDetail;
  message: string;
}

/** Genel Bakış'taki "Sonuçlar" şeridi — bkz. `get_outcome_summary`. */
export interface OutcomeSummary {
  snapshots: number;
  oldest_window: string;
  measured_events: number;
  improved: number;
  flat: number;
  worse: number;
  measuring: number;
  insufficient: number;
  net_delta_clicks: number;
}

/** Fırsatlar tablosundaki "Sonuç" rozeti. */
export interface OutcomeBadge {
  sku: string;
  label: string;
  tone: string;
  tip: string;
}

export interface TimelineItem {
  at: string;
  kind: string;
  label: string;
  outcome_label: string | null;
  outcome_tone: string | null;
  outcome_tip: string | null;
}

/** Ürün detayındaki olay/sonuç zaman çizelgesi. */
export interface ProductTimeline {
  items: TimelineItem[];
  /** false ise ürün ölçülemiyor — mağazaya gönderim kaydı yok. */
  has_store_event: boolean;
}

/** Geçmiş tohumlama sonucu. */
export interface SeedResult {
  snapshots_added: number;
  rows_written: number;
  events_backfilled: number;
  skipped_existing: number;
}

// ---------------------------------------------------------------------------
// Bugün kuyruğu (Faz K)
// ---------------------------------------------------------------------------

/** İş türü kovası. Rust `seo_core::queue::Bucket` ile birebir. */
export type Bucket = "urgent" | "leverage" | "leak" | "review" | "upkeep" | "contact";

/** Madde kimliği: ürün maddeleri sku, satışta olmayan sayfalar slug, müşteriler kişi
 *  kimliği taşır.
 *  ⚠️ EOL satırlarında sku YOK — bu yüzden ayrı kimlik uzayları var. */
export interface ItemRef {
  kind: "product" | "page" | "contact";
  ref: string;
}

export interface QueueItem {
  reference: ItemRef;
  bucket: Bucket;
  title: string;
  /** Tek cümlelik gerekçe — her zaman gerçek bir metrikten türer. */
  reason: string;
  clicks: number;
  score: number;
  /** Maddeyi açacak ekran. */
  page: string;
  /** O ekranda odaklanacak satırın kimliği. */
  focus_id: string;
  minutes: number;
  /** Süre ölçüldü mü? false ise elle yazılmış tahmin (Faz S ölçüyor). */
  minutes_measured: boolean;
  /** Aynı ürünün diğer kovalardaki sebepleri. */
  also: string[];
  /** Bugün "yapıldı" işaretlendi mi — madde listeden düşmez, üstü çizili kalır. */
  done: boolean;
}

export interface BucketCount {
  bucket: Bucket;
  label: string;
  candidates: number;
}

export interface TodayQueue {
  items: QueueItem[];
  analyzed_at: string;
  hidden: number;
  /** Listede kaç madde "yapıldı" — ilerleme çubuğu bunu gösteriyor. */
  done_count: number;
  bucket_counts: BucketCount[];
  /** Yapıldı ama sonucu henüz ölçülemeyen iş sayısı (28 günlük pencere). */
  in_flight: number;
  /** Sonuç kontrolü kovası boşsa en erken dolacağı tarih; doluysa "". */
  review_ready_at: string;
}

// ---------------------------------------------------------------------------
// Odak seansı (Faz S)
// ---------------------------------------------------------------------------

/** Seansta o an kilitli iş — aynı anda YALNIZCA BİR tane. */
export interface LockedItem {
  kind: string;
  reference: string;
  bucket: Bucket;
  title: string;
  reason: string;
  page: string;
  focus_id: string;
  /** Bu iş ne zaman kilitlendi — "bu işte 4 dakikadır" için. */
  started_at: string;
}

export interface FocusState {
  /** null ise seans yok. */
  session_id: number | null;
  started_at: string;
  planned_minutes: number;
  break_minutes: number;
  locked: LockedItem | null;
  done_count: number;
  skipped_count: number;
}

/** Seans özeti — sakin bir bilanço, kutlama değil. */
export interface FocusSummary {
  done_count: number;
  skipped_count: number;
  minutes: number;
  /** "queue_empty" | "time_up" | "stopped" */
  ended_reason: string;
  /** [kova, adet] — bitirilen işlerin dağılımı. */
  buckets: [string, number][];
}

/** Ayarlar ekranı için: kova başına kaç ölçüm birikti. */
export interface CalibrationRow {
  /** ⚠️ `string` DEĞİL: gevşek tip yüzünden Faz C'nin 6. kovası Ayarlar'da ham anahtar
   *  olarak görünecekti (bkz. SettingsPage.vue'daki not). */
  bucket: Bucket;
  samples: number;
  /** Yeterli örnek yoksa null — "henüz ölçülmedi". */
  minutes: number | null;
}

// ---------------------------------------------------------------------------
// EOL karar deposu + sağlık skoru (Faz D)
// ---------------------------------------------------------------------------

/** Bir bileşenin eksikliği: etiket + kaybedilen puan. */
export interface HealthMissing {
  label: string;
  points: number;
}

/** Satışta olmayan bir sayfa için verilmiş karar. */
export interface EolDecision {
  slug: string;
  url: string;
  /** "redirect_301" | "canonical" | "keep" (bilinçli tutuluyor) */
  action: string;
  target_slug: string | null;
  target_sku: string | null;
  /** "ai" | "manual" — hedefi model mi önerdi, siz mi seçtiniz. */
  source: string;
  decided_at: string;
  /** CSV'ye çıkmış mı — "bunu panele girdim mi?" sorusu. */
  exported_at: string | null;
}

/** CSV üretim özeti. */
export interface ExportSummary {
  decided_rows: number;
  undecided_rows: number;
  bytes: number;
  path: string;
}

// ---------------------------------------------------------------------------
// CRM ince dilim (Faz C)
// ---------------------------------------------------------------------------

/** Müşteri kaydı. ⚠️ Kişisel veri — asistan bağlamına GİRMİYOR (bkz. assistantSources.ts). */
export interface Contact {
  id: number;
  name: string;
  company: string;
  email: string;
  phone: string;
  /** "mail" | "telefon" | "instagram" | "fuar" | "referans" | "diğer" — boş olabilir. */
  channel: string;
  note: string;
  last_contact_at: string | null;
  /** Fazın kalbi: tarih verildiği gün kuyruğa iş düşer, dönüş yapılınca temizlenir. */
  next_step_at: string | null;
  next_step_note: string;
  archived: boolean;
  event_count: number;
  /** İlgi etiketleri — tek sorguda geliyor, satır başına ek istek yok. */
  tags: string[];
}

/** Kişi ↔ ürün bağı. İki yön de aynı tabloyu sorguluyor, kopya kayıt yok. */
export interface ContactProduct {
  sku: string;
  /** Kişi kartında ürün adı, ürün detayında kişi adı ("Ahmet Yılmaz · Kurumsal BT"). */
  name: string;
  contact_id: number;
  at: string;
}

/** CSV önizlemesi — hiçbir şey yazılmadan önce gösterilen zorunlu adım. */
export interface CsvPreview {
  headers: string[];
  rows: string[][];
  total_rows: number;
  delimiter: string;
  /** Uygulamanın tahmini: `fields` sırasıyla sütun indeksleri. */
  mapping: (number | null)[];
  /** [anahtar, okunur ad] — eşleştirme satırları buradan çiziliyor. */
  fields: [string, string][];
}

export interface ImportSummary {
  added: number;
  updated: number;
  skipped: number;
  skip_reason: string;
}

/** Sessizlik eşiği. ⚠️ `days: 0` = KAPALI; öneri yalnızca yeterli veride dolu. */
export interface SilenceState {
  days: number;
  suggestion: number | null;
  sample_contacts: number;
}

/** Temas kaydı — CRM'in kendi olay günlüğü (`work_events` DEĞİL). */
export interface ContactEvent {
  id: number;
  at: string;
  /** "call" | "email" | "meeting" | "note" | "followup_done" */
  kind: string;
  note: string;
}

// ---------------------------------------------------------------------------
// Teklif (Faz T)
// ---------------------------------------------------------------------------

export type MarginState = "ok" | "low" | "negative";

/** Marj — **teklifin para biriminde**. Çevrim satır eklenirken bir kez yapıldı. */
export interface Margin {
  amount: number;
  pct: number;
  state: MarginState;
}

export interface TaxRow {
  rate: number;
  base: number;
  amount: number;
}

export interface QuoteItem {
  id: number;
  /** null = elle satır (montaj, nakliye). */
  sku: string | null;
  name: string;
  qty: number;
  unit_price: number;
  tax_rate: number;
  /** 🔴 Yalnızca uygulama içi — müşteriye giden çıktıya ASLA girmez. */
  cost: number | null;
  net: number;
  margin: Margin | null;
}

export interface Quote {
  id: number;
  no: string;
  contact_id: number | null;
  contact_name: string;
  /** "draft" | "sent" | "won" | "lost" | "expired" */
  status: string;
  status_label: string;
  /** "USD" | "TRY" */
  currency: string;
  /** Yalnızca USD teklifte USD OLMAYAN ürün varsa gerekiyor. */
  fx_rate: number | null;
  fx_date: string | null;
  valid_until: string | null;
  note: string;
  close_reason: string;
  created_at: string;
  sent_at: string | null;
  items: QuoteItem[];
  subtotal: number;
  /** Orana göre kırılım — katalogda iki KDV oranı var (%20 ve %10). */
  taxes: TaxRow[];
  tax_total: number;
  grand_total: number;
  margin: Margin | null;
  version_count: number;
}

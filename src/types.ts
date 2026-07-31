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

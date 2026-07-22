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
  draft_search_keywords: string | null;
  draft_details: string | null;
  badge: MetaBadge;
  details_badge: MetaBadge;
  overall: OverallStatus;
}

export interface Settings {
  feed_url: string;
  gemini_api_key: string;
  capsolver_api_key: string;
  seo_country: string;
  gsc_site_url: string;
  gsc_client_email: string;
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
  | "tumu";

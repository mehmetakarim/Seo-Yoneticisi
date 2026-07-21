export type MetaBadge = "eksik" | "hatali" | "uygun" | "tamamlandi";

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
  badge: MetaBadge;
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
  badge: MetaBadge;
}

export interface Settings {
  feed_url: string;
  gemini_api_key: string;
  theme: string | null;
  last_backup_at: string | null;
}

export type FilterKey =
  | "eksik"
  | "hatali"
  | "bekliyor"
  | "uygun"
  | "tamamlandi"
  | "tumu";

import { invoke } from "@tauri-apps/api/core";
import type {
  ProductDetail,
  ProductRow,
  SeoInsights,
  Settings,
  SyncSummary,
} from "./types";

export const api = {
  syncFeed: () => invoke<SyncSummary>("sync_feed"),
  getLastSync: () => invoke<SyncSummary | null>("get_last_sync"),
  listProducts: (filter: string, search: string) =>
    invoke<ProductRow[]>("list_products", { filter, search }),
  getProduct: (sku: string) => invoke<ProductDetail>("get_product", { sku }),
  setTargetKeyword: (sku: string, kw: string) =>
    invoke<void>("set_target_keyword", { sku, kw }),
  saveMetaDraft: (
    sku: string,
    title: string,
    descriptions: string,
    searchKeywords: string,
  ) =>
    invoke<void>("save_meta_draft", {
      sku,
      title,
      descriptions,
      searchKeywords,
    }),
  markMetaDone: (sku: string) => invoke<string>("mark_meta_done", { sku }),
  markDetailsDone: (sku: string) => invoke<string>("mark_details_done", { sku }),
  generateMeta: (sku: string) => invoke<ProductDetail>("generate_meta", { sku }),
  generateDetails: (sku: string) =>
    invoke<ProductDetail>("generate_details", { sku }),
  researchSeo: (sku: string, seed?: string) =>
    invoke<SeoInsights>("research_seo", { sku, seed: seed ?? null }),
  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (
    feedUrl: string,
    geminiApiKey: string,
    capsolverApiKey: string,
    seoCountry: string,
    gscSiteUrl: string,
  ) =>
    invoke<void>("save_settings", {
      feedUrl,
      geminiApiKey,
      capsolverApiKey,
      seoCountry,
      gscSiteUrl,
    }),
  setGscServiceAccount: (path: string) =>
    invoke<string>("set_gsc_service_account", { path }),
  clearGscServiceAccount: () => invoke<void>("clear_gsc_service_account"),
  testGscCredentials: () => invoke<string>("test_gsc_credentials"),
  setTheme: (theme: string) => invoke<void>("set_theme", { theme }),
  testFeedUrl: (url: string) => invoke<number>("test_feed_url", { url }),
  testGeminiKey: (key: string) => invoke<string>("test_gemini_key", { key }),
  testCapsolverKey: (key: string) =>
    invoke<string>("test_capsolver_key", { key }),
  exportDb: (path: string, format: "db" | "json") =>
    invoke<void>("export_db", { path, format }),
  importDb: (path: string) => invoke<void>("import_db", { path }),
};

import { Channel, invoke } from "@tauri-apps/api/core";
import type {
  ImageCheck,
  ProductDetail,
  ProductRow,
  SeoInsights,
  Settings,
  IdeasoftPreview,
  SyncSummary,
  TechGroup,
  TechSpecsResult,
  OpportunityReport,
  SuccessorSuggestion,
  CatalogSyncResult,
  CatalogMatch,
  CanonicalPreview,
  ChatMessage,
  AssistantEvent,
  ChatSessionMeta,
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
  checkImages: (sku: string) => invoke<ImageCheck[]>("check_images", { sku }),
  restoreMetaVersion: (sku: string, index: number) =>
    invoke<ProductDetail>("restore_meta_version", { sku, index }),
  restoreDetailsVersion: (sku: string, index: number) =>
    invoke<ProductDetail>("restore_details_version", { sku, index }),
  analyzeOpportunities: () => invoke<OpportunityReport>("analyze_opportunities"),
  getOpportunityCache: () => invoke<OpportunityReport | null>("get_opportunity_cache"),
  suggestEolSuccessor: (url: string) =>
    invoke<SuccessorSuggestion>("suggest_eol_successor", { url }),
  syncIdeasoftCatalog: () => invoke<CatalogSyncResult>("sync_ideasoft_catalog"),
  /** ⚠️ Yalnızca SATIŞTAKİ (feed) ürünlerde arar — bkz. search_live_products. */
  searchLiveProducts: (term: string) =>
    invoke<CatalogMatch[]>("search_live_products", { term }),
  /**
   * Asistan turu. Yanıt `onEvent` ile parça parça gelir; söz verilen değer kullanılan
   * modelin adı. Uygulamada Tauri kanalının ilk kullanımı.
   */
  assistantAsk: (
    history: ChatMessage[],
    context: string,
    onEvent: (e: AssistantEvent) => void,
  ) => {
    const ch = new Channel<AssistantEvent>();
    ch.onmessage = onEvent;
    return invoke<string>("assistant_ask", { history, context, onEvent: ch });
  },
  listChatSessions: () => invoke<ChatSessionMeta[]>("list_chat_sessions"),
  getChatSession: (id: number) => invoke<ChatMessage[]>("get_chat_session", { id }),
  saveChatSession: (
    id: number | null,
    messages: ChatMessage[],
    toolPage: string,
    model: string,
  ) => invoke<number>("save_chat_session", { id, messages, toolPage, model }),
  deleteChatSession: (id: number) => invoke<void>("delete_chat_session", { id }),
  deleteAllChatSessions: () => invoke<void>("delete_all_chat_sessions"),
  previewCanonical: (eolSlug: string, targetSlug: string) =>
    invoke<CanonicalPreview>("preview_canonical", { eolSlug, targetSlug }),
  applyCanonical: (eolSlug: string, targetSlug: string) =>
    invoke<string>("apply_canonical", { eolSlug, targetSlug }),
  saveTechSource: (sku: string, text: string) =>
    invoke<void>("save_tech_source", { sku, text }),
  structureTechSpecs: (sku: string) =>
    invoke<TechSpecsResult>("structure_tech_specs", { sku }),
  saveTechSpecs: (sku: string, specs: TechGroup[]) =>
    invoke<void>("save_tech_specs", { sku, specs }),
  techTableHtml: (sku: string) => invoke<string>("tech_table_html", { sku }),
  markTechDone: (sku: string) => invoke<string>("mark_tech_done", { sku }),
  restoreTechVersion: (sku: string, index: number) =>
    invoke<ProductDetail>("restore_tech_version", { sku, index }),
  testIdeasoft: () => invoke<string>("test_ideasoft"),
  ideasoftPreview: (sku: string, parts: string[]) =>
    invoke<IdeasoftPreview>("ideasoft_preview", { sku, parts }),
  ideasoftPush: (sku: string, parts: string[]) =>
    invoke<ProductDetail>("ideasoft_push", { sku, parts }),
  ideasoftPullKeyword: (sku: string) =>
    invoke<ProductDetail>("ideasoft_pull_keyword", { sku }),
  needsSetup: () => invoke<boolean>("needs_setup"),
  markSetupDone: () => invoke<void>("mark_setup_done"),
  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (
    feedUrl: string,
    geminiApiKey: string,
    capsolverApiKey: string,
    seoCountry: string,
    gscSiteUrl: string,
    ideasoftDomain: string,
    ideasoftToken: string,
  ) =>
    invoke<void>("save_settings", {
      feedUrl,
      geminiApiKey,
      capsolverApiKey,
      seoCountry,
      gscSiteUrl,
      ideasoftDomain,
      ideasoftToken,
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

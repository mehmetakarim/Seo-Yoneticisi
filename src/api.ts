import { teamUser, type TeamFunctionSession } from "./teamConnection";
import { teamQuotes } from "./teamQuotes";
import { teamContacts, type ContactDraft } from "./teamContacts";
import type { TeamTodayInputs } from "./teamToday";
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
  FeedDiff,
  IdeasoftPull,
  OutcomeSummary,
  OutcomeBadge,
  ProductTimeline,
  QueryComparison,
  SeedResult,
  TodayQueue,
  QueueItem,
  FocusState,
  FocusSummary,
  CalibrationRow,
  EolDecision,
  ExportSummary,
  Contact,
  ContactId,
  ContactEvent,
  ContactProduct,
  CsvPreview,
  ImportSummary,
  SilenceState,
  Quote,
  QuoteDoc,
  QuoteSummary,
  ModelChains,
  GeminiKullanim,
  StorePageSyncResult,
  StorePageDetail,
  NewStorePage,
  HealthCheck,
} from "./types";

export const api = {
  syncFeed: () => invoke<SyncSummary>("sync_feed"),
  getLastSync: () => invoke<SyncSummary | null>("get_last_sync"),
  seedMetricHistory: (measurementScope?: string) => invoke<SeedResult>("seed_metric_history", { measurementScope }),
  getOutcomeSummary: (shared?: object | null) => invoke<OutcomeSummary>("get_outcome_summary", { shared }),
  getOutcomeBadges: (shared?: object | null) => invoke<OutcomeBadge[]>("get_outcome_badges", { shared }),
  getProductTimeline: (sku: string, shared?: object | null) => invoke<ProductTimeline>("get_product_timeline", { sku, shared }),
  compareProductQuery: (sku: string, query: string) =>
    invoke<QueryComparison>("compare_product_query", { sku, query }),
  summarizeTeamQuery: (eventDay: string, rows: object[]) =>
    invoke<QueryComparison>("summarize_team_query", { eventDay, rows }),
  getFeedDiff: (sku: string) => invoke<FeedDiff>("get_feed_diff", { sku }),
  getJsonld: (sku: string) => invoke<string>("get_jsonld", { sku }),
  markFeedReviewed: (sku: string) => invoke<void>("mark_feed_reviewed", { sku }),
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
  analyzeOpportunities: (measurementScope?: string, shared?: object | null) =>
    invoke<OpportunityReport>("analyze_opportunities", { measurementScope, shared }),
  gscSnapshotWindow: (measurementScope: string, sharedWindowEnd?: string) =>
    invoke<[string, string] | null>("gsc_snapshot_window", { measurementScope, sharedWindowEnd }),
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
  /** `sources`: yüklü kaynakların okunur adları ("Fırsatlar, Katalog") — prompt'ta
   *  "seçili değil" ile "veride yok" ayrımını kurmak için. */
  assistantAsk: (
    history: ChatMessage[],
    context: string,
    sources: string,
    onEvent: (e: AssistantEvent) => void,
  ) => {
    const ch = new Channel<AssistantEvent>();
    ch.onmessage = onEvent;
    return invoke<string>("assistant_ask", { history, context, sources, onEvent: ch });
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
  testAndSaveIdeasoftConnection: (domain: string, token: string) =>
    invoke<string>("test_and_save_ideasoft_connection", { domain, token }),
  ideasoftPreview: (sku: string, parts: string[]) =>
    invoke<IdeasoftPreview>("ideasoft_preview", { sku, parts }),
  ideasoftPush: (sku: string, parts: string[]) =>
    invoke<ProductDetail>("ideasoft_push", { sku, parts }),
  ideasoftPullKeyword: (sku: string) =>
    invoke<IdeasoftPull>("ideasoft_pull_keyword", { sku }),
  needsSetup: () => invoke<boolean>("needs_setup"),
  markSetupDone: () => invoke<void>("mark_setup_done"),
  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (
    feedUrl: string,
    geminiApiKey: string,
    capsolverApiKey: string,
    seoCountry: string,
    gscSiteUrl: string,
    ideasoftDomain?: string,
    ideasoftToken?: string,
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
  /** Okunan kontroller — ağ yok, anında döner. */
  localHealth: () => invoke<HealthCheck[]>("local_health"),
  /** Ucuz ağ kontrolleri: token geçerli mi, model listesi erişilebilir mi. */
  remoteHealth: () => invoke<HealthCheck[]>("remote_health"),
  getStorePage: (kind: string, id: number) =>
    invoke<StorePageDetail>("get_store_page", { kind, id }),
  listStorePages: (kind: string, fetchedAt?: string) =>
    invoke<StorePageDetail[]>("list_store_pages", { kind, fetchedAt }),
  createStorePage: (kind: string, draft: NewStorePage) =>
    invoke<StorePageDetail>("create_store_page", { kind, draft }),
  /** Meta + tanıtım metni üretir; yalnızca taslağa yazar, mağazaya dokunmaz. */
  generateStorePage: (kind: string, id: number, shared?: { page: StorePageDetail; session: TeamFunctionSession; products: string[] }) =>
    invoke<StorePageDetail>("generate_store_page", { kind, id, shared }),
  saveStorePageDraft: (
    kind: string, id: number, pageTitle: string, metaDescription: string,
    targetKeyword: string, showcase: string,
  ) =>
    invoke<StorePageDetail>("save_store_page_draft", {
      kind, id, pageTitle, metaDescription, targetKeyword, showcase,
    }),
  /** ⚠️ Mağazayı DEĞİŞTİRİR. Yalnızca operatör onayıyla çağrılmalı. */
  pushStorePage: (kind: string, id: number, shared?: StorePageDetail) =>
    invoke<StorePageDetail>("push_store_page", { kind, id, shared }),
  publishStorePage: (kind: string, id: number, shared?: StorePageDetail) =>
    invoke<void>("publish_store_page", { kind, id, shared }),
  /** Kategori · marka · blog envanterini IdeaSoft'tan çeker (~7 istek). */
  syncStorePages: () => invoke<StorePageSyncResult>("sync_store_pages"),
  getModelChains: () => invoke<ModelChains>("get_model_chains"),
  setModelChains: (uretim: string[], sohbet: string[]) =>
    invoke<void>("set_model_chains", { uretim, sohbet }),
  listGeminiModels: () => invoke<string[]>("list_gemini_models"),
  /** ⚠️ Gerçek bir istek gönderir — kotadan düşer ve sayaca yazılır. */
  probeGeminiModel: (model: string) =>
    invoke<string>("probe_gemini_model", { model }),
  previewTeamGeminiMeta: (session: TeamFunctionSession) =>
    invoke<{ model: string; title: string; description: string }>("preview_team_gemini_meta", { session }),
  geminiUsage: () => invoke<GeminiKullanim>("gemini_usage"),
  testCapsolverKey: (key: string) =>
    invoke<string>("test_capsolver_key", { key }),
  exportDb: (path: string, format: "db" | "json") =>
    invoke<void>("export_db", { path, format }),
  importDb: (path: string) => invoke<void>("import_db", { path }),

  // --- Bugün kuyruğu (Faz K) ---
  getTodayQueue: (shared?: TeamTodayInputs) => invoke<TodayQueue>("get_today_queue", { shared }),
  /** `until` yoksa kalıcı gizleme, varsa (YYYY-AA-GG) o tarihe kadar erteleme. */
  dismissQueueItem: (kind: string, reference: string, until: string | null) =>
    invoke<void>("dismiss_queue_item", { kind, reference, until }),
  /** "Yapıldı": maddeyi sonraki analize kadar çıkarır ve ürünse ölçüm olayı yazar. */
  completeQueueItem: (kind: string, reference: string, recordStoreChange?: boolean) =>
    invoke<void>("complete_queue_item", { kind, reference, recordStoreChange }),
  /** Tek maddenin "yapıldı"/gizleme kararını geri alır. Ölçüm olayı silinmez. */
  restoreQueueItem: (kind: string, reference: string) =>
    invoke<void>("restore_queue_item", { kind, reference }),
  restoreQueueItems: () => invoke<void>("restore_queue_items"),

  // --- Odak seansı (Faz S) ---
  startFocusSession: (queueScope?: string, queueItems?: QueueItem[]) =>
    invoke<FocusState>("start_focus_session", { queueScope, queueItems }),
  /** `outcome`: "done" | "skipped" | "dismissed" */
  resolveFocusItem: (outcome: string, queueScope?: string, queueItems?: QueueItem[], expectedSessionId?: number, expectedItemId?: number) =>
    invoke<FocusState>("resolve_focus_item", { outcome, queueScope, queueItems, expectedSessionId, expectedItemId }),
  endFocusSession: (reason: string, queueScope?: string, expectedSessionId?: number) =>
    invoke<FocusSummary | null>("end_focus_session", { reason, queueScope, expectedSessionId }),
  getFocusState: (queueScope?: string, queueItems?: QueueItem[]) =>
    invoke<FocusState>("get_focus_state", { queueScope, queueItems }),
  getFocusCalibration: (queueScope?: string) => invoke<CalibrationRow[]>("get_focus_calibration", { queueScope }),
  setFocusDurations: (work: number, brk: number) =>
    invoke<void>("set_focus_durations", { work, brk }),
  /** Kuyrukta seansa kilitlenebilecek iş var mı — düğme buna göre açılıyor. */
  hasLockableItem: (queueScope?: string, queueItems?: QueueItem[]) =>
    invoke<boolean>("has_lockable_item", { queueScope, queueItems }),

  // --- EOL karar deposu + 301 CSV (Faz D) ---
  getEolDecisions: () => invoke<EolDecision[]>("get_eol_decisions"),
  /** `action`: "redirect_301" | "canonical" | "keep". `keep`te hedef gönderilmez. */
  saveEolDecision: (
    slug: string,
    url: string,
    action: string,
    targetSlug: string | null,
    targetSku: string | null,
    source: string,
  ) => invoke<void>("save_eol_decision", { slug, url, action, targetSlug, targetSku, source }),
  deleteEolDecision: (slug: string) => invoke<void>("delete_eol_decision", { slug }),
  /** Kararsız satırlarda hedef sütunu BOŞ kalır — bkz. decisions.rs. */
  exportRedirectCsv: (path: string, minClicks: number) =>
    invoke<ExportSummary>("export_redirect_csv", { path, minClicks }),

  // --- CRM ince dilim (Faz C) ---
  // ⚠️ Kişisel veri. Bu uçlardan gelen hiçbir alan asistan bağlamına konmuyor.
  listContacts: (search: string, includeArchived: boolean, channel: string, tag: string) =>
    teamUser.value ? teamContacts.list(search, includeArchived, channel, tag)
      : invoke<Contact[]>("list_contacts", { search, includeArchived, channel, tag }),
  listLocalContacts: () => invoke<Contact[]>("list_contacts", { search: "", includeArchived: false, channel: "", tag: "" }),
  getContact: (id: ContactId) =>
    teamUser.value ? teamContacts.get(id) : invoke<Contact>("get_contact", { id }),
  saveContact: ({ revision, ...draft }: ContactDraft) =>
    teamUser.value ? teamContacts.save({ ...draft, revision }) : invoke<ContactId>("save_contact", draft),
  archiveContact: (id: ContactId, archived: boolean, revision?: number) =>
    teamUser.value ? teamContacts.archive(id, archived, revision) : invoke<void>("archive_contact", { id, archived }),
  getContactEvents: (contactId: ContactId) =>
    teamUser.value ? teamContacts.events(contactId) : invoke<ContactEvent[]>("get_contact_events", { contactId }),
  addContactEvent: (
    contactId: ContactId, kind: string, note: string, nextStepAt: string | null,
    nextStepNote: string | null, revision?: number,
  ) => typeof contactId === "string"
    ? teamContacts.addEvent(contactId, revision, { kind, note, next_step_at: nextStepAt, next_step_note: nextStepNote })
    : invoke<void>("add_contact_event", { contactId, kind, note, nextStepAt, nextStepNote }).then(() => "synced" as const),
  listContactTags: () =>
    teamUser.value ? teamContacts.tags() : invoke<string[]>("list_contact_tags"),
  setContactTags: (contactId: ContactId, tags: string[], revision?: number) =>
    teamUser.value ? teamContacts.act(contactId, revision, "tags", { tags })
      : invoke<void>("set_contact_tags", { contactId, tags }),
  getContactProducts: (contactId: ContactId) =>
    teamUser.value ? teamContacts.products(contactId) : invoke<ContactProduct[]>("get_contact_products", { contactId }),
  contactsOfProduct: (sku: string) =>
    teamUser.value ? teamContacts.contactsOfProduct(sku) : invoke<ContactProduct[]>("contacts_of_product", { sku }),
  linkContactProduct: (contactId: ContactId, sku: string, revision?: number) =>
    teamUser.value ? teamContacts.act(contactId, revision, "link", { sku })
      : invoke<void>("link_contact_product", { contactId, sku }),
  unlinkContactProduct: (contactId: ContactId, sku: string, revision?: number) =>
    teamUser.value ? teamContacts.act(contactId, revision, "unlink", { sku })
      : invoke<void>("unlink_contact_product", { contactId, sku }),
  previewContactCsv: (path: string) => invoke<CsvPreview>("preview_contact_csv", { path }),
  importContactsCsv: (path: string, mapping: (number | null)[]) =>
    teamUser.value ? Promise.reject(new Error("Ekip kayıtlarını içe aktarmak için birleştirme akışı gerekir."))
      : invoke<ImportSummary>("import_contacts_csv", { path, mapping }),

  getSilenceState: () => invoke<SilenceState>("get_silence_state"),
  /** `0` = kapalı. Öneri kendiliğinden yazılmıyor; bu yalnızca kullanıcı onayıyla çağrılır. */
  setSilenceDays: (days: number) => invoke<void>("set_silence_days", { days }),

  // --- Teklif (Faz T) ---
  listQuotes: (status: string) => invoke<Quote[]>("list_quotes", { status }),
  getQuote: (id: number) => invoke<Quote>("get_quote", { id }),
  createQuote: (contactId: number | null, currency: string) =>
    invoke<number>("create_quote", { contactId, currency }),
  saveQuote: (q: {
    id: number;
    contactId: number | null;
    currency: string;
    fxRate: number | null;
    fxDate: string | null;
    validUntil: string | null;
    note: string;
  }) => invoke<void>("save_quote", q),
  deleteQuote: (id: number) => invoke<void>("delete_quote", { id }),
  /** Fiyat ve maliyet teklifin para birimine ÇEVRİLEREK ekleniyor (bir kez, donuyor). */
  addQuoteItemFromCatalog: (quoteId: number, sku: string, qty: number, pricing?: {
    currency: string;
    fxRate: number | null;
    fxDate: string | null;
    expectedCurrency: string;
    expectedFxRate: number | null;
  }) => invoke<void>("add_quote_item_from_catalog", { quoteId, sku, qty, pricing }),
  addQuoteItemManual: (quoteId: number, name: string) =>
    invoke<void>("add_quote_item_manual", { quoteId, name }),
  updateQuoteItem: (
    itemId: number,
    name: string,
    qty: number,
    unitPrice: number,
    taxRate: number,
  ) => invoke<void>("update_quote_item", { itemId, name, qty, unitPrice, taxRate }),
  deleteQuoteItem: (itemId: number) => invoke<void>("delete_quote_item", { itemId }),
  /** Geçiş kuralları arka uçta; geçersiz geçiş hata döner. */
  setQuoteStatus: (id: number, status: string, reason: string) =>
    invoke<void>("set_quote_status", { id, status, reason }),
  snapshotQuote: (id: number) => invoke<number>("snapshot_quote", { id }),
  /** 🔴 Maliyet İÇERMEZ: arka uçtaki dönüşüm kayıplı (bkz. quotes.rs `to_out`). */
  renderQuote: async (id: Quote["id"]) => typeof id === "string"
    ? invoke<QuoteDoc>("render_team_quote", { quote: await teamQuotes.get(id) })
    : invoke<QuoteDoc>("render_quote", { id }),
  /** Belgeyi geçici dosyaya yazar **ve tarayıcıda açar** (açma işi Rust'ta: JS'in yol
   *  açma kapsamı geçici klasörü reddediyor). Dönen değer bilgi amaçlı dosya yolu. */
  exportQuoteHtml: async (id: Quote["id"]) => typeof id === "string"
    ? invoke<string>("export_team_quote_html", { quote: await teamQuotes.get(id) })
    : invoke<string>("export_quote_html", { id }),
  quoteSummary: () => invoke<QuoteSummary>("quote_summary"),
  quotesOfContact: async (contactId: Quote["contact_id"]) => typeof contactId === "string"
    ? (await teamQuotes.list()).filter((quote) => quote.contact_id === contactId)
    : invoke<Quote[]>("quotes_of_contact", { contactId }),
  getQuoteDefaults: () =>
    invoke<{ tax_rate: number; valid_days: number; seller: string; footer: string }>(
      "get_quote_defaults",
    ),
  setQuoteDefaults: (taxRate: number, validDays: number, seller: string, footer: string) =>
    invoke<void>("set_quote_defaults", { taxRate, validDays, seller, footer }),
};

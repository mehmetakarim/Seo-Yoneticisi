import { defineStore } from "pinia";
import { api } from "./api";
import { flushTeamContacts, teamContactProgress } from "./teamContacts";
import { teamAccessIssue, teamReachable, teamRestoring, teamUser, teamWorkspace } from "./teamConnection";
import { getTeamProduct, listTeamProducts, prepareTeamCatalogLine, readOnlineTeamCatalogRevision } from "./teamCatalog";
import { flushTeamGscSnapshots, seedTeamGscHistory, teamGscAnalysis, teamGscMode, teamMeasurementScope, teamOutcomeInputs } from "./teamGsc";
import { listTeamStorePages } from "./teamStorePages";
import { fetchTeamIdeasoftProduct, previewTeamIdeasoftProduct, pushTeamIdeasoftProduct } from "./teamIdeasoft";
import { flushGscReports, loadGscReport, queueGscReport } from "./teamGscReport";
import { flushTeamProductSeo, getTeamProductSeo, patchTeamProductSeo, readTeamProductSeoForGeneration, teamProductSeoScope, verifyTeamProductSeoRevision, type TeamSeoRecord } from "./teamProductSeo";
import { assembleTeamProductTech, restoreTeamProductTech, structureTeamProductTech, teamTechSourceChanges } from "./teamProductTech";
import { generateTeamProductDetails, restoreTeamProductDetails } from "./teamProductDetails";
import { generateTeamProductMeta, restoreTeamProductMeta } from "./teamProductMeta";
import { summarizeTeamQuotes, teamQuoteQueue, teamQuotes } from "./teamQuotes";
import { SOURCES, buildContext, sourceByKey } from "./assistantSources";
import type { ContextSource, SourceData } from "./assistantSources";
import type {
  FilterKey,
  ImageCheck,
  ProductDetail,
  ProductRow,
  SeoInsights,
  Settings,
  IdeasoftPreview,
  SyncSummary,
  TechGroup,
  OpportunityReport,
  SuccessorSuggestion,
  CanonicalPreview,
  CatalogMatch,
  ChatMessage,
  ChatSessionMeta,
  OutcomeSummary,
  OutcomeBadge,
  TodayQueue,
  FocusState,
  FocusSummary,
  EolDecision,
  Contact,
  ContactId,
  ContactEvent,
  ContactProduct,
  Quote,
  QuoteSummary,
  StorePageDetail,
} from "./types";
import type { Page } from "./navigation";
import { KIND_BY_PAGE, PAGE_BY_KIND } from "./catalog";
import { acceptTodayConflict, flushTeamToday, loadTeamTodayInputs, queueTodayDecision, refreshTodayDecisions,
  teamTodayMode, teamTodayScope, teamTodaySync, todayLocalDay, type TeamTodayInputs } from "./teamToday";
import type { ItemRef } from "./types";

type Theme = "light" | "dark";
let contactsRequest = 0;
let contactRequest = 0;
let quotesRequest = 0;
let quoteRequest = 0;
let productsRequest = 0;
let productRequest = 0;
let storePagesRequest = 0;
let todayRequest = 0;
let todayInputs: TeamTodayInputs | undefined;
const todaySource = () => teamTodayMode() ? teamTodayScope() ?? "" : undefined;

type ProductMetaEdits = Partial<Record<"draft_title" | "draft_descriptions" | "draft_search_keywords" | "target_keyword", string>>;
type MetaAutosave = { changes: ProductMetaEdits; timer?: number; saving?: Promise<boolean> };
const metaAutosaves = new WeakMap<ProductDetail, MetaAutosave>();

/**
 * Katalog listesi süzgeci.
 *
 * ⚠️ "görünen" ölçülmüş bir ayrım, kozmetik değil: markaların yalnızca %20'si Google'da
 * görünüyor (ölçüldü 2026-08-14, 265 kayıttan 54). Görünmeyen bir sayfanın meta'sını
 * düzeltmek ölçülebilir sonuç üretmiyor — süzgeç bu farkı görünür kılıyor.
 */
export type StorePageFilter = "hepsi" | "eksik" | "gorunen" | "taslak";

interface Toast {
  id: number;
  text: string;
  kind: "info" | "error" | "ok";
}

interface State {
  page: Page;
  theme: Theme;
  allRows: ProductRow[];
  productCatalogRevision: number | null;
  filter: FilterKey;
  search: string;
  selectedSku: string | null;
  detail: ProductDetail | null;
  syncing: boolean;
  generating: boolean;
  generatingDetails: boolean;
  researching: boolean;
  research: SeoInsights | null;
  imageChecking: boolean;
  imageCheck: ImageCheck[] | null;
  opportunity: OpportunityReport | null;
  opportunityBusy: boolean;
  opportunityError: string;
  /** EOL url → halef önerisi. İstek üzerine dolar, önbelleklenir. */
  successors: Record<string, SuccessorSuggestion>;
  /** Halef önerisi beklenen EOL url'leri. ⚠️ Tek dize DEĞİL, URL başına: tek bir dize
   *  ekranda "hepsini pasifleştir" diye okunuyordu (saha hatası, 2026-08-07). */
  successorBusy: Record<string, boolean>;
  catalogBusy: boolean;
  /** Açık onay bekleyen canonical işlemi. Tek seferde YALNIZCA BİR tane —
   *  kullanıcı kararı: toplu değil, gerektiğinde ve tek tek. */
  canonicalPending: (CanonicalPreview & { eolSlug: string; targetSlug: string }) | null;
  canonicalBusy: boolean;
  /** Hedef seçme adımı: yapay zekâ halef bulamadığında (ya da öneriyi değiştirmek
   *  istediğinde) kullanıcı hedefi kendisi arar. Yine tek satır için. */
  canonicalPicker: { eolSlug: string; suggested: string } | null;
  canonicalQuery: string;
  canonicalResults: CatalogMatch[];
  canonicalSearching: boolean;
  /** Kurulum sihirbazı açık mı. İlk çalıştırmada kendiliğinden açılır; Ayarlar'dan da
   *  elle açılabilir. */
  setupOpen: boolean;
  /** Kullanıcı asistana geçmeden ÖNCE hangi araç ekranındaydı. `page` asistana geçince
   *  değiştiği için ayrı tutuluyor. App.vue güncelliyor. Faz A'dan beri yalnızca
   *  VARSAYILAN kaynak seçimini belirliyor; bağlamı artık `assistantSources` belirliyor. */
  lastToolPage: Page | "";
  /** Asistanın konuşacağı veri kaynakları ("+" menüsünden seçiliyor, Faz A).
   *  Anahtarlar `assistantSources.ts`'teki `SOURCES` kaydından. */
  assistantSources: string[];
  /** Açık sohbetin mesajları. v0.7.2'den beri her turdan sonra `chat_sessions` tablosuna
   *  kaydediliyor — uygulama kapanınca kaybolmuyor. */
  chat: ChatMessage[];
  chatBusy: boolean;
  /** Model düşünüyor ama henüz cevap parçası gelmedi (Gemma akışta muhakemesini de
   *  yayınlıyor, onu filtreliyoruz). Kör döner ikon yerine gerçek sinyal. */
  chatThinking: boolean;
  /** Son turu hangi model cevapladı — rozet olarak gösteriliyor. */
  chatModel: string;
  /** Açık sohbetin veritabanı kimliği; yeni sohbette null. İlk yanıt kaydedilince dolar. */
  chatId: number | null;
  /** Kaydedilmiş sohbetlerin listesi (yalnızca üstveri). */
  chatSessions: ChatSessionMeta[];
  techStructuring: boolean;
  techTableSaving: boolean;
  techDropped: string[];
  /** Bugünün iş kuyruğu (Faz K). Saklanmıyor, her açılışta hesaplanıyor. */
  today: TodayQueue | null;
  todayBusy: boolean;
  todayError: string;
  /**
   * Kuyruktan bir maddeye tıklanınca hedef araç ekranında **hangi satırın** öne
   * çıkarılacağı. Ekran onu çizdikten sonra temizleniyor.
   *
   * ⚠️ Faz K'ye kadar ekranlar arası satır hedefleme YOKTU; yalnızca `openProduct(sku)`
   * vardı ve o da ürün ekranına gidiyordu. Üstelik üç araç ekranı listeyi kırpıyor
   * (EOL 25, Düşüşte 30, Yükselmeye yakın 40) — hedef satır render edilmemiş olabiliyor.
   */
  focus: { page: string; id: string } | null;
  /** Odak seansı (Faz S). `session_id` null ise seans yok.
   *  ⚠️ `focus` ADI ALINMIŞ — o, Faz K'nin satır odağı. Bu ayrı bir kavram. */
  session: FocusState | null;
  /** Seans başlangıcından beri geçen saniye — ekrandaki sayaç. ⚠️ Yalnızca GÖSTERİM;
   *  ölçüm arka uçtaki zaman damgalarından çıkıyor, uygulama uyusa da bozulmuyor. */
  sessionElapsed: number;
  /** Süre dolunca gösterilen mola önerisi. Otomatik başlamaz — kullanıcı kararı. */
  sessionBreakOffered: boolean;
  /** Seans bitince gösterilen sakin bilanço. */
  sessionSummary: FocusSummary | null;
  /** Kuyrukta kilitlenebilecek iş var mı — "Odak seansı başlat" düğmesi buna bakıyor. */
  sessionCanStart: boolean;
  sessionBusy: boolean;
  /** EOL karar deposu (Faz D): slug → karar. ⚠️ Halef ÖNERİLERİ hâlâ bellekte
   *  (`successors`); burada yalnızca ONAYLANMIŞ kararlar var ve bunlar kalıcı. */
  eolDecisions: Record<string, EolDecision>;
  /** Hedef seçme modali 301 KARARI için mi açıldı? Doluysa slug; boşsa canonical akışı.
   *  ⚠️ Aynı modal iki amaca hizmet ediyor — hangisi olduğunu bu alan söylüyor. */
  redirectPicking: string;
  /** CRM (Faz C). ⚠️ Kişisel veri — asistan bağlamına HİÇBİR koşulda girmiyor. */
  contacts: Contact[];
  contactsBusy: boolean;
  contactSearch: string;
  contactArchived: boolean;
  /** Açık olan kişi kartı; kimse seçili değilse null. */
  contact: Contact | null;
  contactEvents: ContactEvent[];
  /** Açık kartın ilgilendiği ürünler (Faz C2). */
  contactProducts: ContactProduct[];
  /** Kullanılan bütün etiketler — süzgeç ve öneri kaynağı. */
  contactTags: string[];
  contactChannel: string;
  contactTag: string;
  /** Teklifler (Faz T). ⚠️ `quote.items[].cost` yalnızca ekranda; çıktıya girmiyor. */
  quotes: Quote[];
  quotesBusy: boolean;
  quoteStatusFilter: string;
  quote: Quote | null;
  /** Kazanma/kaybetme özeti — kayıp nedenleri raporlanabilsin diye (fazın bitiş şartı). */
  quoteSummary: QuoteSummary | null;
  /* --- Faz İ2: katalog sayfaları (kategori · marka · içerik) ---
     ⚠️ Tip başına AYRI durum tutulmuyor: ekran `:key` ile yeniden kuruluyor ve açılışta
     kendi listesini yüklüyor. Üç kopya durum, üç kopya sıfırlama hatası demekti.
     ⚠️ Ad öneki `storePage` — `catalogBusy` YUKARIDA başka bir şey (EOL halef araması).
     Aynı adı ikinci bir anlamda kullanmak derleyiciyi de yanılttı (çakışma yakalandı). */
  storePages: StorePageDetail[];
  storePagesBusy: boolean;
  storePageSearch: string;
  storePageFilter: StorePageFilter;
  /** Seçili sayfanın tam kaydı; seçim kimliği bunun `remote_id`'si. */
  storePage: StorePageDetail | null;
  /** İçerik açığından gelindiğinde açılacak sayfa — ekran kurulunca tüketiliyor. */
  storePagePending: { kind: string; id: number } | null;

  /** Ölçüm omurgası (Faz Ö) — sonuç özeti ve satır rozetleri. */
  outcomeSummary: OutcomeSummary | null;
  outcomeBadges: Record<string, OutcomeBadge>;
  seedBusy: boolean;
  ideasoftBusy: boolean;
  ideasoftPreview: IdeasoftPreview | null;
  ideasoftPreviewTarget: { sku: string; scope: string; revision: number | null } | null;
  ideasoftParts: string[];
  appVersion: string;
  updateInfo: { version: string; notes: string } | null;
  updating: boolean;
  updateDownloaded: number;
  updateTotal: number;
  updateChecking: boolean;
  lastSync: SyncSummary | null;
  showSummary: boolean;
  settings: Settings | null;
  toasts: Toast[];
  loading: boolean;
}

let toastSeq = 1;

export const useStore = defineStore("app", {
  state: (): State => ({
    // ⚠️ Açılış ekranı Faz K'de "products"tan "today"e alındı: fazın amacı sabah açan
    // kişinin karar vermek zorunda kalmadan işe başlaması. Analiz yoksa ekran bunu
    // söylüyor ve Genel Bakış'a yönlendiriyor, boş kalmıyor.
    page: "today",
    theme: "light",
    allRows: [],
    productCatalogRevision: null,
    filter: "tumu",
    search: "",
    selectedSku: null,
    detail: null,
    syncing: false,
    generating: false,
    generatingDetails: false,
    researching: false,
    research: null,
    imageChecking: false,
    imageCheck: null,
    opportunity: null,
    opportunityBusy: false,
    opportunityError: "",
    successors: {},
    successorBusy: {},
    catalogBusy: false,
    canonicalPending: null,
    canonicalBusy: false,
    canonicalPicker: null,
    canonicalQuery: "",
    canonicalResults: [],
    canonicalSearching: false,
    setupOpen: false,
    lastToolPage: "",
    assistantSources: [],
    chat: [],
    chatBusy: false,
    chatThinking: false,
    chatModel: "",
    chatId: null,
    chatSessions: [],
    techStructuring: false,
    techTableSaving: false,
    techDropped: [],
    today: null,
    todayBusy: false,
    todayError: "",
    contacts: [],
    contactsBusy: false,
    contactSearch: "",
    contactArchived: false,
    contact: null,
    contactEvents: [],
    contactProducts: [],
    contactTags: [],
    contactChannel: "",
    contactTag: "",
    quotes: [],
    quotesBusy: false,
    quoteStatusFilter: "",
    quote: null,
    quoteSummary: null,
    focus: null,
    session: null,
    sessionElapsed: 0,
    sessionBreakOffered: false,
    sessionSummary: null,
    sessionCanStart: false,
    sessionBusy: false,
    eolDecisions: {},
    redirectPicking: "",
    storePages: [],
    storePagesBusy: false,
    storePageSearch: "",
    storePageFilter: "hepsi",
    storePage: null,
    storePagePending: null,

    outcomeSummary: null,
    outcomeBadges: {},
    seedBusy: false,
    ideasoftBusy: false,
    ideasoftPreview: null,
    ideasoftPreviewTarget: null,
    ideasoftParts: [],
    appVersion: "",
    updateInfo: null,
    updating: false,
    updateDownloaded: 0,
    updateTotal: 0,
    updateChecking: false,
    lastSync: null,
    showSummary: false,
    settings: null,
    toasts: [],
    loading: false,
  }),

  getters: {
    productIdeasoftAvailable(state): boolean {
      if (teamWorkspace.value || teamUser.value) return !!state.detail?.team_seo_scope;
      return !!state.settings?.ideasoft_active;
    },
    counts(state): Record<FilterKey, number> {
      const c: Record<FilterKey, number> = {
        eksik: 0,
        hatali: 0,
        bekliyor: 0,
        uygun: 0,
        tamamlandi: 0,
        degisti: 0,
        tumu: 0,
      };
      for (const r of state.allRows) {
        // "Değişti" diğerlerinden bağımsız: bayraklı ürün aynı anda "tamamlandı"dır da.
        if (r.feed_changed) c.degisti++;
        if (r.overall === "tamamlandi") c.tamamlandi++;
        else {
          c.tumu++;
          if (r.overall === "eksik") c.eksik++;
          else if (r.overall === "hatali") c.hatali++;
          else if (r.overall === "bekliyor") c.bekliyor++;
          else if (r.overall === "uygun") c.uygun++;
        }
      }
      return c;
    },
    rows(state): ProductRow[] {
      const q = state.search.trim().toLowerCase();
      return state.allRows.filter((r) => {
        if (
          q &&
          !r.name.toLowerCase().includes(q) &&
          !r.sku.toLowerCase().includes(q)
        )
          return false;
        switch (state.filter) {
          case "tumu":
            return r.overall !== "tamamlandi";
          case "tamamlandi":
            return r.overall === "tamamlandi";
          case "degisti":
            return !!r.feed_changed;
          default:
            return r.overall === state.filter;
        }
      });
    },
    visibleSkus(): string[] {
      return this.rows.map((r) => r.sku);
    },
  },

  actions: {
    toast(text: string, kind: Toast["kind"] = "info") {
      const id = toastSeq++;
      this.toasts.push({ id, text, kind });
      setTimeout(() => {
        this.toasts = this.toasts.filter((t) => t.id !== id);
      }, 2600);
    },

    applyTheme() {
      document.documentElement.setAttribute("data-theme", this.theme);
    },

    async setTheme(theme: Theme) {
      this.theme = theme;
      this.applyTheme();
      try {
        await api.setTheme(theme);
      } catch (e) {
        /* ayar kaydı kritik değil */
      }
    },

    // ---- Faz 10: otomatik güncelleme ----
    /** Yeni sürüm var mı? `silent` ise bulunamayınca kullanıcıyı rahatsız etmez. */
    async checkUpdate(silent = true) {
      if (this.updateChecking || this.updating) return;
      this.updateChecking = true;
      try {
        const { check } = await import("@tauri-apps/plugin-updater");
        const up = await check();
        if (up) {
          this.updateInfo = { version: up.version, notes: up.body ?? "" };
        } else if (!silent) {
          this.toast("Uygulama güncel ✓", "ok");
        }
      } catch (e) {
        // Ağ yoksa veya endpoint erişilemezse açılışta sessiz kal
        if (!silent) this.toast(String(e), "error");
      } finally {
        this.updateChecking = false;
      }
    },

    dismissUpdate() {
      if (this.updating) return;
      this.updateInfo = null;
    },

    /** İndir + kur + yeniden başlat. */
    async runUpdate() {
      if (!this.updateInfo || this.updating) return;
      this.updating = true;
      this.updateDownloaded = 0;
      this.updateTotal = 0;
      try {
        const { check } = await import("@tauri-apps/plugin-updater");
        const up = await check();
        if (!up) {
          this.toast("Güncelleme bulunamadı.", "error");
          this.updateInfo = null;
          return;
        }
        await up.downloadAndInstall((ev: any) => {
          if (ev.event === "Started") this.updateTotal = ev.data?.contentLength ?? 0;
          else if (ev.event === "Progress") this.updateDownloaded += ev.data?.chunkLength ?? 0;
        });
        const { relaunch } = await import("@tauri-apps/plugin-process");
        await relaunch();
      } catch (e) {
        this.toast(`Güncelleme başarısız: ${e}`, "error");
      } finally {
        this.updating = false;
      }
    },

    async loadAppearance() {
      if (window.matchMedia?.("(prefers-color-scheme: dark)").matches) this.theme = "dark";
      this.applyTheme();
      try {
        this.settings = await api.getSettings();
        if (this.settings?.theme === "dark" || this.settings?.theme === "light") {
          this.theme = this.settings.theme;
          this.applyTheme();
        }
      } catch {
        // Cihaz tercihi okunamadığında sistem temasını kullan.
      }
    },

    async init() {
      this.loading = true;
      try {
        this.lastSync = await api.getLastSync();
        // Sürüm bilgisi + sessiz güncelleme kontrolü (açılışta)
        try {
          const { getVersion } = await import("@tauri-apps/api/app");
          this.appVersion = await getVersion();
        } catch {
          /* sürüm okunamazsa önemli değil */
        }
        void this.checkUpdate(true);
        await this.reload();
        if (this.rows.length && !this.selectedSku) {
          await this.select(this.rows[0].sku);
        }
      } catch (e) {
        this.toast(String(e), "error");
      } finally {
        this.loading = false;
      }
    },

    async refreshOpenCatalog() {
      const request = productsRequest;
      const revision = await readOnlineTeamCatalogRevision();
      if (revision !== null && this.hasDifferentCatalogRevision(revision) && request === productsRequest &&
          this.page === "products" && !this.syncing) await this.reload(true);
    },

    hasDifferentCatalogRevision(revision: number) {
      return revision !== this.productCatalogRevision ||
        (this.detail?.team_catalog_revision !== undefined && revision !== this.detail.team_catalog_revision);
    },

    async reload(backgroundCatalog = false) {
      const request = ++productsRequest;
      const inWorkspace = Boolean(teamUser.value && teamWorkspace.value);
      const shared = inWorkspace ? await listTeamProducts() : null;
      const rows = inWorkspace ? shared?.rows ?? [] : await api.listProducts("hepsi", "");
      const current = () => request === productsRequest && (!backgroundCatalog ||
        (this.page === "products" && teamReachable.value === true && !teamAccessIssue.value && !this.syncing));
      if (!current()) return;
      if (backgroundCatalog) {
        if (!shared || !this.hasDifferentCatalogRevision(shared.revision)) return;
        const sku = this.selectedSku;
        const detail = this.detail;
        if (sku && !rows.some((row) => row.sku === sku)) {
          const pending = detail && metaAutosaves.get(detail);
          if (pending?.saving || Object.keys(pending?.changes ?? {}).length ||
              this.generating || this.generatingDetails || this.techStructuring || this.techTableSaving ||
              this.ideasoftBusy || this.ideasoftPreview ||
              document.activeElement?.matches("input, textarea, [contenteditable=true]")) return;
        } else if (sku && detail?.sku === sku) {
          const fresh = await getTeamProduct(sku);
          if (!current() || this.selectedSku !== sku || this.detail !== detail) return;
          // Keep the detail object and the editor's unsaved drafts while refreshing the feed fields.
          Object.assign(detail, {
            name: fresh.name, brand: fresh.brand, main_category: fresh.main_category, category: fresh.category,
            quantity: fresh.quantity, url: fresh.url, img_url: fresh.img_url, gallery: fresh.gallery,
            image_count: fresh.image_count, image_badge: fresh.image_badge,
            title: fresh.title, descriptions: fresh.descriptions, keywords: fresh.keywords,
            search_keywords: fresh.search_keywords, details: fresh.details, feed_changed: fresh.feed_changed,
            team_catalog_revision: fresh.team_catalog_revision,
          });
        }
      }
      this.allRows = rows;
      this.productCatalogRevision = shared?.revision ?? null;
      if (this.selectedSku && !rows.some((row) => row.sku === this.selectedSku)) {
        productRequest++;
        this.selectedSku = null;
        this.detail = null;
      }
    },

    resetProductsSource() {
      productsRequest++;
      productRequest++;
      this.allRows = [];
      this.selectedSku = null;
      this.detail = null;
      this.productCatalogRevision = null;
    },

    async readSelectedProduct(sku: string) {
      return teamUser.value && teamWorkspace.value ? getTeamProduct(sku) : api.getProduct(sku);
    },

    async applyProductEdit(updated: ProductDetail) {
      if (this.selectedSku !== updated.sku) return;
      const request = productRequest;
      const detail = this.productCatalogRevision !== null
        ? await this.readSelectedProduct(updated.sku)
        : updated;
      if (request === productRequest) this.detail = detail;
    },

    async saveSharedSeo(detail: ProductDetail, changes: Record<string, unknown>, strict = false) {
      try {
        if (detail.team_seo_scope !== teamProductSeoScope()) throw new Error("Çalışma alanı değişti.");
        const revision = detail.seo_revision ?? null;
        const saved = await patchTeamProductSeo(detail.sku, revision, changes, strict);
        void flushTeamProductSeo().catch((error) => this.toast(String(error), "error"));
        if ((detail.seo_revision ?? 0) < saved.revision) detail.seo_revision = saved.revision;
        return true;
      } catch (error) {
        this.toast(String(error), "error");
        return false;
      }
    },

    needsSharedIntegration() {
      if (this.detail?.seo_revision == null) return false;
      this.toast("Bu işlem ortak SEO taslağına henüz bağlanmadı.", "info");
      return true;
    },

    async refreshProductsSource() {
      const previous = this.productCatalogRevision;
      await this.reload();
      const selectedRevision = this.detail?.team_catalog_revision ?? previous;
      if (this.selectedSku && selectedRevision !== this.productCatalogRevision) {
        await this.select(this.selectedSku);
      }
    },

    setFilter(f: FilterKey) {
      this.filter = f;
    },

    setSearch(q: string) {
      this.search = q;
    },

    /** Önbellekteki son analizi yükler (API'ye gitmez). Sayfa açılışında çağrılır. */
    async loadOpportunityCache() {
      try {
        const shared = teamGscMode();
        const scope = teamMeasurementScope();
        const report = await loadGscReport();
        if (shared !== teamGscMode() || scope !== teamMeasurementScope()) return;
        if (!this.opportunity || !report || report.analyzed_at >= this.opportunity.analyzed_at) this.opportunity = report;
      } catch {
        // Önbellek okunamazsa sessiz kal — kullanıcı yine "Analizi çalıştır" diyebilir.
      }
    },

    /** GSC'ye gidip analizi yeniler. Tek API çağrısı, birkaç saniye sürer. */
    async runOpportunityAnalysis() {
      if (this.opportunityBusy) return;
      this.opportunityBusy = true;
      this.opportunityError = "";
      try {
        const inTeam = teamGscMode();
        const scope = teamMeasurementScope();
        const inputs = scope ? await teamOutcomeInputs().catch(() => null) : null;
        const sharedWindowEnd = inputs?.snapshots.map((snapshot) => snapshot.window_end).sort().pop();
        if (inTeam !== teamGscMode() || scope !== teamMeasurementScope()) throw new Error("Çalışma alanı değişti.");
        const shared = inTeam ? await teamGscAnalysis(sharedWindowEnd) : null;
        if (inTeam !== teamGscMode() || scope !== teamMeasurementScope()) throw new Error("Çalışma alanı değişti.");
        const report = await api.analyzeOpportunities(scope, shared);
        if (inTeam !== teamGscMode() || scope !== teamMeasurementScope()) throw new Error("Çalışma alanı değişti.");
        if (shared && scope) {
          await queueGscReport(report, scope);
          void flushGscReports().catch(console.warn);
        }
        this.opportunity = report;
        void flushTeamGscSnapshots().then(() => this.loadOutcomes()).catch(console.warn);
        const n = this.opportunity.opportunities.length;
        this.toast(n ? `${n} fırsat bulundu` : "Fırsat bulunamadı — tablo temiz", "ok");
      } catch (e) {
        // Hata mesajı sayfada kalıcı gösterilir; toast kaybolur, kullanıcı sebebi göremez.
        this.opportunityError = String(e);
      } finally {
        this.opportunityBusy = false;
      }
    },

    /**
     * Satışta olmayan bir sayfa için halef önerisi al.
     *
     * İSTEK ÜZERİNE, tek sayfa için: 1.073 EOL sayfanın tamamı için model çağırmak günlük
     * kotayı (flash modellerde 20/gün) anında tüketirdi. Kota koruması **isteğe bağlılıktır**,
     * çağrıları sıraya dizmek değil.
     *
     * 🔴 Saha geri bildirimi (2026-08-07): eskiden tek bir `successorBusy` dizesi vardı ve
     * ekran onu "herhangi biri meşgulse HEPSİNİ pasifleştir" diye okuyordu — bir satıra
     * basınca 25 satırın düğmesi birden sönüyordu. Ayrıca `successors[url]` doluysa fonksiyon
     * sessizce dönüyordu; oysa düğmenin ipucu "Halefi yeniden öner" diyordu. İkisi de düzeldi:
     * durum artık **URL başına**, ve mevcut öneri yeniden istenebiliyor.
     */
    async suggestSuccessor(url: string) {
      if (this.successorBusy[url]) return;
      this.successorBusy[url] = true;
      try {
        this.successors[url] = await api.suggestEolSuccessor(url);
      } catch (e) {
        this.toast(String(e), "error");
      } finally {
        delete this.successorBusy[url];
      }
    },

    /** Bağlam kaynaklarının okuyacağı veri — tek yerden toplanıyor. */
    sourceData(): SourceData {
      return {
        report: this.opportunity,
        products: this.allRows,
        outcomeSummary: this.outcomeSummary,
        outcomeBadges: this.outcomeBadges,
      };
    },

    /**
     * Asistanın göreceği bağlam: SEÇİLİ kaynakların satırları + rapor özeti.
     *
     * ⚠️ v0.11.0'a (Faz A) kadar burada `switch (lastToolPage)` vardı; asistan son gezilen
     * ekrana kilitliydi. Artık kullanıcı "+" menüsünden birden çok kaynak seçebiliyor;
     * satır üretimi `assistantSources.ts` kayıt tablosunda.
     *
     * Satır sayısı hâlâ bilinçli olarak sınırlı (toplam bütçe paylaştırılıyor) ve asistan
     * "listenin tamamı" iddiasında bulunmasın diye kaç satır gördüğü bağlama yazılıyor.
     */
    assistantContext(): string {
      return buildContext(this.sourceData(), this.assistantSources);
    },

    /** Seçili kaynaklardan yalnızca verisi olanlar — menü ve şerit bunu gösteriyor. */
    activeSources(): ContextSource[] {
      const d = this.sourceData();
      return this.assistantSources
        .map(sourceByKey)
        .filter((s): s is ContextSource => !!s && s.available(d));
    },

    /** Bir kaynağı ekler/çıkarır. Verisi olmayan kaynak seçilemez. */
    toggleSource(key: string) {
      const s = sourceByKey(key);
      if (!s || !s.available(this.sourceData())) return;
      const i = this.assistantSources.indexOf(key);
      if (i >= 0) this.assistantSources.splice(i, 1);
      else this.assistantSources.push(key);
    },

    /**
     * Varsayılan seçim: geldiğiniz araç ekranı; o da yoksa verisi olan ilk kaynak.
     * Böylece "ekrandan gelip soru sor" alışkanlığı Faz A'dan sonra da aynı sonucu veriyor.
     */
    defaultSources(): string[] {
      const d = this.sourceData();
      const gelinen = sourceByKey(this.lastToolPage);
      if (gelinen && gelinen.available(d)) return [gelinen.key];
      const ilk = SOURCES.find((s) => s.available(d));
      return ilk ? [ilk.key] : [];
    },

    /** Asistana bir soru gönderir; yanıt akarken `chat`teki son mesaj büyür. */
    async askAssistant(question: string) {
      const q = question.trim();
      if (!q || this.chatBusy) return;
      this.chat.push({ role: "user", text: q });
      // Cevap balonu ÖNCEDEN eklenir ve parçalar ona akar; böylece kullanıcı yazının
      // gerçek zamanlı büyüdüğünü görür.
      this.chat.push({ role: "model", text: "" });
      const slot = this.chat.length - 1;
      this.chatBusy = true;
      this.chatThinking = true;
      this.chatModel = "";
      try {
        // Boş cevap balonu geçmişe gönderilmez — model onu kendi turu sanır.
        const history = this.chat.slice(0, -1);
        const yuklu = this.activeSources()
          .map((s) => s.label)
          .join(", ");
        const model = await api.assistantAsk(history, this.assistantContext(), yuklu, (e) => {
          if (e.kind === "thinking") return;
          this.chatThinking = false;
          this.chat[slot].text += e.text;
        });
        this.chatModel = model;
        if (!this.chat[slot].text.trim()) {
          this.chat[slot].text = "Yanıt alınamadı, tekrar deneyin.";
        }
      } catch (e) {
        // Hata balonun İÇİNDE gösterilir: toast kaybolur ve kullanıcı boş balonla kalırdı.
        this.chat[slot].text = String(e);
      } finally {
        this.chatBusy = false;
        this.chatThinking = false;
        // Her turdan sonra kaydet — hata alan tur da dahil. Kullanıcı uygulamayı kapatıp
        // açtığında ne sorduğunu ve ne cevap geldiğini (hatayı da) görebilmeli.
        await this.persistChat();
      }
    },

    /** Açık sohbeti veritabanına yazar; yeni sohbetse kimliğini alır. */
    async persistChat() {
      if (!this.chat.length) return;
      try {
        this.chatId = await api.saveChatSession(
          this.chatId,
          this.chat,
          // ⚠️ Faz A'dan beri VİRGÜLLE AYRILMIŞ LİSTE. Şema değişmedi: tek değerli eski
          // kayıtlar tek elemanlı liste olarak okunuyor (bkz. openChatSession).
          this.assistantSources.join(","),
          this.chatModel,
        );
        await this.loadChatSessions();
      } catch (e) {
        // Kaydedilememesi sohbeti bozmasın; kullanıcı yine de cevabını görüyor.
        this.toast(`Sohbet kaydedilemedi: ${e}`, "error");
      }
    },

    /** Sihirbazı elle aç (Ayarlar'daki düğme). */
    openSetup() {
      this.setupOpen = true;
    },

    /**
     * Sihirbazı kapat ve bir daha kendiliğinden açılmasın diye işaretle.
     * ⚠️ ATLANDIĞINDA DA işaretleniyor: her açılışta sihirbazla karşılaşmak, atlama
     * seçeneğini anlamsız kılardı. Kullanıcı Ayarlar'dan istediğinde tekrar açabiliyor.
     */
    async finishSetup() {
      this.setupOpen = false;
      try {
        await api.markSetupDone();
        this.settings = await api.getSettings();
      } catch (e) {
        this.toast(String(e), "error");
      }
    },

    /** Kaydedilmiş sohbetleri tazeler. */
    async loadChatSessions() {
      try {
        this.chatSessions = await api.listChatSessions();
      } catch (e) {
        // Geçmiş yardımcı bir özellik; okunamaması sohbeti engellememeli.
        this.chatSessions = [];
      }
    },

    /** Geçmişten bir sohbeti açar; devam edilen mesajlar aynı kayda yazılır. */
    async openChatSession(id: number) {
      if (this.chatBusy) return;
      try {
        this.chat = await api.getChatSession(id);
        this.chatId = id;
        const oturum = this.chatSessions.find((s) => s.id === id);
        this.chatModel = oturum?.model ?? "";
        // Kaynak seçimi de geri geliyor. Tek değerli eski kayıt ("opportunities") tek
        // elemanlı listeye çözülüyor; artık tanınmayan anahtarlar sessizce eleniyor.
        const kayitli = (oturum?.tool_page ?? "")
          .split(",")
          .map((k) => k.trim())
          .filter((k) => !!sourceByKey(k));
        this.assistantSources = kayitli.length ? kayitli : this.defaultSources();
      } catch (e) {
        this.toast(String(e), "error");
        await this.loadChatSessions();
      }
    },

    /**
     * Yeni sohbet başlatır. ⚠️ Eskisini SİLMEZ — kaydedilmişse geçmişte duruyor.
     * (Eskiden bu "temizle" idi ve sohbet gerçekten kayboluyordu.)
     */
    newChat() {
      if (this.chatBusy) return;
      this.chat = [];
      this.chatModel = "";
      this.chatId = null;
      this.assistantSources = this.defaultSources();
    },

    /** Tek bir sohbeti siler — kullanıcı eylemi. */
    async deleteChatSession(id: number) {
      try {
        await api.deleteChatSession(id);
        if (this.chatId === id) this.newChat();
        await this.loadChatSessions();
      } catch (e) {
        this.toast(String(e), "error");
      }
    },

    /** Tüm geçmişi siler — arayüz açık onayla çağırıyor. */
    async deleteAllChatSessions() {
      try {
        await api.deleteAllChatSessions();
        this.newChat();
        await this.loadChatSessions();
      } catch (e) {
        this.toast(String(e), "error");
      }
    },

    /** IdeaSoft kataloğunu çeker (~7 dk, 10.909 ürün). Elle tetiklenir. */
    async syncCatalog() {
      if (this.catalogBusy) return;
      this.catalogBusy = true;
      try {
        const r = await api.syncIdeasoftCatalog();
        this.toast(`${r.fetched} ürün alındı · ${r.matched_eol} sayfa eşleşti`, "ok");
      } catch (e) {
        this.toast(String(e), "error");
      } finally {
        this.catalogBusy = false;
      }
    },

    /**
     * Canonical akışını başlatır. Hedef biliniyorsa doğrudan önizlemeye, bilinmiyorsa
     * hedef seçme adımına gider.
     *
     * ⚠️ Kullanıcı geri bildirimi (2026-07-30): halef önerisi boş çıktığında buton hiç
     * görünmüyordu ve sayfa için hiçbir şey yapılamıyordu. Yapay zekânın halef bulamaması
     * "hedef yok" demek değil — karar zaten operatörde.
     */
    async startCanonical(eolSlug: string, suggested = "") {
      if (this.canonicalBusy) return;
      if (suggested) return this.askCanonical(eolSlug, suggested);
      this.canonicalPicker = { eolSlug, suggested };
      this.canonicalQuery = "";
      this.canonicalResults = [];
    },

    /** Hedef ararken IdeaSoft'ta ürün arar (ad üzerinden). */
    async searchCanonicalTarget() {
      const term = this.canonicalQuery.trim();
      if (term.length < 3 || this.canonicalSearching) return;
      this.canonicalSearching = true;
      try {
        this.canonicalResults = await api.searchLiveProducts(term);
      } catch (e) {
        this.toast(String(e), "error");
      } finally {
        this.canonicalSearching = false;
      }
    },

    /** Listeden hedef seçildi → önizlemeye geç. */
    async pickCanonicalTarget(targetSlug: string) {
      const p = this.canonicalPicker;
      if (!p) return;
      this.canonicalPicker = null;
      // ⚠️ Aynı modal iki amaca hizmet ediyor: 301 kararı mı, canonical yazma mı?
      // 301'de mağazaya HİÇBİR ŞEY yazılmıyor — yalnızca karar kaydediliyor.
      if (this.redirectPicking) {
        const slug = this.redirectPicking;
        this.redirectPicking = "";
        const url = this.opportunity?.eol.find((e) => e.slug === slug)?.url ?? slug;
        await this.decideEol(slug, url, "redirect_301", targetSlug, null, "manual");
        return;
      }
      await this.askCanonical(p.eolSlug, targetSlug);
    },

    /** Önizlemeden hedef seçmeye dön — yanlış halef önerisini düzeltebilmek için. */
    changeCanonicalTarget() {
      const p = this.canonicalPending;
      if (!p || this.canonicalBusy) return;
      this.canonicalPending = null;
      this.canonicalPicker = { eolSlug: p.eolSlug, suggested: "" };
      this.canonicalQuery = "";
      this.canonicalResults = [];
    },

    cancelCanonicalPicker() {
      this.redirectPicking = "";
      if (!this.canonicalSearching) this.canonicalPicker = null;
    },

    /** Canonical önizlemesi aç — YAZMAZ, yalnızca ne olacağını gösterir. */
    async askCanonical(eolSlug: string, targetSlug: string) {
      if (this.canonicalBusy) return;
      this.canonicalBusy = true;
      try {
        const p = await api.previewCanonical(eolSlug, targetSlug);
        this.canonicalPending = { ...p, eolSlug, targetSlug };
      } catch (e) {
        this.toast(String(e), "error");
      } finally {
        this.canonicalBusy = false;
      }
    },

    cancelCanonical() {
      if (!this.canonicalBusy) this.canonicalPending = null;
    },

    /** Onaylanan canonical'ı yazar. Canlı mağazayı değiştirir. */
    async confirmCanonical() {
      const p = this.canonicalPending;
      if (!p || this.canonicalBusy) return;
      this.canonicalBusy = true;
      try {
        await api.applyCanonical(p.eolSlug, p.targetSlug);
        this.toast("Canonical ayarlandı", "ok");
        this.canonicalPending = null;
      } catch (e) {
        this.toast(String(e), "error");
      } finally {
        this.canonicalBusy = false;
      }
    },

    /** Fırsat satırından ürüne atla. */
    async openProduct(sku: string) {
      this.page = "products";
      await this.select(sku);
    },

    async select(sku: string) {
      const request = ++productRequest;
      if (this.selectedSku !== sku) this.detail = null;
      this.selectedSku = sku;
      this.research = null; // araştırma ürüne özel — seçim değişince sıfırla
      this.imageCheck = null;
      this.techDropped = [];
      try {
        const detail = await this.readSelectedProduct(sku);
        if (request !== productRequest) return;
        this.detail = detail;
        // Cache'lenmiş boyut sonucu varsa anında göster, sonra tazele
        this.imageCheck = this.detail?.image_check ?? null;
        if (!detail.team_seo_scope) void this.checkImages(sku);
      } catch (e) {
        if (request === productRequest) this.toast(String(e), "error");
      }
    },

    // ---- Faz 8: teknik özellik tablosu ----
    async saveTechSource(detail: ProductDetail, text: string): Promise<boolean> {
      try {
        if (detail.team_seo_scope || teamWorkspace.value || teamUser.value || teamRestoring.value) {
          if (detail.team_seo_scope !== teamProductSeoScope()) throw new Error("Ortak ürün taslağını yeniden açın.");
          if (!await this.flushProductMetaEdits(detail)) return false;
          const revision = detail.seo_revision ?? null;
          const record = await getTeamProductSeo(detail.sku);
          if ((detail.seo_revision ?? null) !== revision || (record?.revision ?? null) !== revision) {
            throw new Error("Ortak taslak değişti. Teknik metni yeniden kaydedin.");
          }
          if (record?.draft.tech_source_text !== text && !await this.saveSharedSeo(detail, teamTechSourceChanges(record, text), true)) return false;
        } else await api.saveTechSource(detail.sku, text);
        detail.tech_source_text = text;
        return true;
      } catch (e) {
        this.toast(String(e), "error");
        return false;
      }
    },

    async applySharedSeoResult(detail: ProductDetail, saved: TeamSeoRecord) {
      const request = productRequest;
      const revision = detail.seo_revision ?? null;
      const current = () => this.detail === detail && this.selectedSku === detail.sku &&
        request === productRequest && detail.team_seo_scope === teamProductSeoScope() &&
        (detail.seo_revision ?? null) === revision;
      if (!current()) return;
      const updated = await getTeamProduct(detail.sku, saved);
      if (current()) Object.assign(detail, updated);
    },

    async structureTech(source: string) {
      const sku = this.selectedSku;
      const detail = this.detail;
      if (!sku || !detail || detail.sku !== sku || this.techStructuring) return;
      this.techStructuring = true;
      this.techDropped = [];
      try {
        if (!await this.flushProductMetaEdits(detail) || this.detail !== detail) return;
        let res;
        if (detail.team_seo_scope || teamWorkspace.value || teamUser.value || teamRestoring.value) {
          const scope = detail.team_seo_scope;
          let revision = detail.seo_revision ?? null;
          const current = () => this.detail === detail && this.selectedSku === sku && scope === teamProductSeoScope() &&
            (detail.seo_revision ?? null) === revision;
          const assertCurrent = () => {
            if (!current()) throw new Error("Ürün veya taslak değişti. Teknik tablo üretimini yeniden başlatın.");
          };
          assertCurrent();
          if (!source.trim()) throw new Error("Önce üreticinin teknik özellik metnini yapıştırın.");
          let draft = await readTeamProductSeoForGeneration(sku, revision);
          assertCurrent();
          if (draft?.draft.tech_source_text !== source) {
            const saved = await patchTeamProductSeo(sku, revision, teamTechSourceChanges(draft, source), true);
            void flushTeamProductSeo().catch(console.warn);
            assertCurrent();
            revision = saved.revision;
            detail.seo_revision = revision;
            detail.tech_source_text = source;
            draft = await readTeamProductSeoForGeneration(sku, revision);
          }
          assertCurrent();
          if (!draft) throw new Error("Önce üreticinin teknik özellik metnini yapıştırın.");
          const generated = await structureTeamProductTech(detail, draft);
          await verifyTeamProductSeoRevision(sku, revision);
          assertCurrent();
          const saved = await patchTeamProductSeo(sku, revision, generated.changes, true);
          void flushTeamProductSeo().catch(console.warn);
          await this.applySharedSeoResult(detail, saved);
          res = generated.result;
        } else {
          await api.saveTechSource(sku, source);
          res = await api.structureTechSpecs(sku);
          if (this.detail === detail) {
            detail.tech_source_text = source;
            detail.tech_specs = res.groups;
          }
        }
        if (this.selectedSku !== sku || (detail.team_seo_scope && detail.team_seo_scope !== teamProductSeoScope())) return;
        this.techDropped = res.dropped;
        const n = res.groups.reduce((a, g) => a + g.rows.length, 0);
        this.toast(`Teknik tablo hazır · ${n} satır`, "ok");
        await this.reload();
      } catch (e) {
        this.toast(String(e), "error");
      } finally {
        this.techStructuring = false;
      }
    },

    async saveTechSpecs(specs: TechGroup[]) {
      const sku = this.selectedSku;
      const detail = this.detail;
      if (!sku || !detail || detail.sku !== sku || this.techTableSaving) return;
      this.techTableSaving = true;
      try {
        if (detail.team_seo_scope || teamWorkspace.value || teamUser.value || teamRestoring.value) {
          if (!await this.flushProductMetaEdits(detail)) return;
          if (!await this.saveSharedSeo(detail, { tech_specs_json: specs }, true)) return;
        } else await api.saveTechSpecs(sku, specs);
        if (this.detail === detail) detail.tech_specs = specs;
      } catch (e) {
        this.toast(String(e), "error");
      } finally {
        this.techTableSaving = false;
      }
    },

    async techHtml() {
      const sku = this.selectedSku;
      const detail = this.detail;
      if (!sku || !detail || detail.sku !== sku) throw new Error("Önce bir ürün açın.");
      if (detail.team_seo_scope || teamWorkspace.value || teamUser.value || teamRestoring.value) {
        if (detail.team_seo_scope !== teamProductSeoScope() || detail.seo_revision == null) throw new Error("Önce ortak teknik tabloyu yapılandırın.");
        return assembleTeamProductTech(detail.tech_specs ?? []);
      }
      return api.techTableHtml(sku);
    },

    // ---- Faz 9: IdeaSoft gönderimi ----
    /** Fark önizlemesini açar (gönderim öncesi zorunlu adım). */
    async openIdeasoftPreview(parts: string[]) {
      if (!this.selectedSku || this.ideasoftBusy) return;
      this.closeIdeasoftPreview();
      this.ideasoftBusy = true;
      try {
        const sku = this.selectedSku;
        const selected = this.detail;
        if (selected && (parts.includes("meta") || parts.includes("keyword"))) {
          if (!await this.flushProductMetaEdits(selected)) return;
          if (selected !== this.detail || sku !== this.selectedSku) return;
        }
        if (teamWorkspace.value || teamUser.value) {
          const detail = this.detail;
          const scope = teamProductSeoScope();
          if (!detail || detail.sku !== sku || detail.team_seo_scope !== scope) throw new Error("Ortak ürün taslağını yeniden açın.");
          const revision = detail.seo_revision ?? null;
          if (revision === null) throw new Error("Mağazaya göndermek için önce ortak SEO taslağını kaydedin.");
          await verifyTeamProductSeoRevision(sku, revision);
          const preview = await previewTeamIdeasoftProduct(detail, parts);
          if (this.detail !== detail || this.selectedSku !== sku || teamProductSeoScope() !== scope ||
              (detail.seo_revision ?? null) !== revision) throw new Error("Ürün veya taslak değişti. Önizlemeyi yeniden açın.");
          this.ideasoftPreviewTarget = { sku, scope, revision };
          this.ideasoftPreview = preview;
        } else {
          const preview = await api.ideasoftPreview(sku, parts);
          if (sku !== this.selectedSku || teamWorkspace.value || teamUser.value) throw new Error("Ürün veya çalışma alanı değişti.");
          this.ideasoftPreview = preview;
        }
        this.ideasoftParts = parts;
      } catch (e) {
        this.toast(String(e), "error");
        this.ideasoftParts = [];
      } finally {
        this.ideasoftBusy = false;
      }
    },

    /** Hedef kelimeyi IdeaSoft'tan çeker (yerel alan boşken faydalı). */
    async pullIdeasoftKeyword() {
      const sku = this.selectedSku;
      const detail = this.detail;
      if (!sku || !detail || detail.sku !== sku || this.ideasoftBusy) return;
      this.ideasoftBusy = true;
      try {
        if (!await this.flushProductMetaEdits(detail)) return;
        if (this.detail !== detail || this.selectedSku !== sku) return;
        if (detail.team_seo_scope || teamWorkspace.value || teamUser.value || teamRestoring.value) {
          if (detail.team_seo_scope !== teamProductSeoScope()) throw new Error("Ortak ürün taslağını yeniden açın.");
          const revision = detail.seo_revision;
          const remote = await fetchTeamIdeasoftProduct(sku);
          if (this.detail !== detail || this.selectedSku !== sku || !await this.flushProductMetaEdits(detail)) return;
          if (this.detail !== detail || detail.seo_revision !== revision || detail.team_seo_scope !== teamProductSeoScope()) {
            throw new Error("Ürün veya taslak değişti. Getir işlemini yeniden başlatın.");
          }
          const changes: Record<string, string> = {};
          if (remote.keyword && remote.keyword !== detail.target_keyword?.trim()) changes.target_keyword = remote.keyword;
          if (remote.techText && !detail.tech_source_text?.trim()) changes.tech_source_text = remote.techText;
          if (!Object.keys(changes).length) {
            const message = remote.techText ? "Dolu teknik kaynak metni korundu."
              : remote.keyword ? "Ortak taslaktaki hedef kelime IdeaSoft ile eşleşiyor."
              : "IdeaSoft’ta getirilecek hedef kelime veya teknik tablo yok.";
            this.toast(message, "info");
            return;
          }
          if (!await this.saveSharedSeo(detail, changes, true)) return;
          Object.assign(detail, changes);
          this.toast(remote.techText && !("tech_source_text" in changes)
            ? "Hedef kelime getirildi. Dolu teknik kaynak metni korundu."
            : "IdeaSoft verileri ortak taslağa alındı.", "ok");
          return;
        }
        // Ne getirildiği/neyin korunduğu ayrımını arka uç biliyor; mesaj oradan geliyor.
        const r = await api.ideasoftPullKeyword(sku);
        await this.applyProductEdit(r.detail);
        this.toast(r.message, "ok");
      } catch (e) {
        this.toast(String(e), "error");
      } finally {
        this.ideasoftBusy = false;
      }
    },

    closeIdeasoftPreview() {
      this.ideasoftPreview = null;
      this.ideasoftPreviewTarget = null;
      this.ideasoftParts = [];
    },

    /** Onaylanan parçaları IdeaSoft'a yazar. */
    async confirmIdeasoftPush() {
      if (!this.selectedSku || this.ideasoftBusy || !this.ideasoftParts.length || !this.ideasoftPreview) return;
      this.ideasoftBusy = true;
      try {
        const target = this.ideasoftPreviewTarget;
        if (target) {
          if (target.sku !== this.selectedSku || target.scope !== teamProductSeoScope() ||
              this.detail?.team_seo_scope !== target.scope || (this.detail.seo_revision ?? null) !== target.revision) {
            this.closeIdeasoftPreview();
            throw new Error("Ürün veya taslak değişti. Önizlemeyi yeniden açın.");
          }
          const preview = this.ideasoftPreview;
          this.closeIdeasoftPreview();
          await verifyTeamProductSeoRevision(target.sku, target.revision);
          if (this.selectedSku !== target.sku || teamProductSeoScope() !== target.scope ||
              (this.detail?.seo_revision ?? null) !== target.revision) throw new Error("Ürün veya taslak değişti. Önizlemeyi yeniden açın.");
          const result = await pushTeamIdeasoftProduct(target.sku, preview);
          if (result.updated && this.detail?.sku === target.sku && this.detail.team_seo_scope === target.scope && result.pushed_at) {
            this.detail.ideasoft_pushed_at = result.pushed_at;
          }
          if (!result.updated) this.toast("Mağazadaki değerler önizlemeyle eşleşiyor.", "ok");
          else if (result.event_recorded) this.toast("IdeaSoft’a gönderildi ve geçmişe kaydedildi.", "ok");
          else if (result.event_pending) this.toast("IdeaSoft’a gönderildi. Geçmiş kaydı eşitlenmeyi bekliyor.", "info");
          else this.toast("IdeaSoft’a gönderildi. Geçmiş kaydı doğrulanamadı.", "info");
          return;
        }
        if (teamWorkspace.value || teamUser.value || this.ideasoftPreview.remote.sku.trim().toLowerCase() !== this.selectedSku.trim().toLowerCase()) {
          this.closeIdeasoftPreview();
          throw new Error("Ürün veya çalışma alanı değişti. Önizlemeyi yeniden açın.");
        }
        await this.applyProductEdit(await api.ideasoftPush(this.selectedSku, this.ideasoftParts));
        this.toast("IdeaSoft'a gönderildi ✓", "ok");
        this.closeIdeasoftPreview();
      } catch (e) {
        this.toast(String(e), "error");
      } finally {
        this.ideasoftBusy = false;
      }
    },

    async restoreMetaVersion(index: number) {
      const sku = this.selectedSku;
      const detail = this.detail;
      if (!sku || !detail || detail.sku !== sku || this.generating) return;
      try {
        if (!await this.flushProductMetaEdits(detail)) return;
        if (this.detail !== detail || this.selectedSku !== sku) return;
        if (detail.team_seo_scope || teamWorkspace.value || teamUser.value || teamRestoring.value) {
          const revision = detail.seo_revision ?? null;
          const current = () => this.detail === detail && this.selectedSku === sku &&
            detail.team_seo_scope === teamProductSeoScope() && (detail.seo_revision ?? null) === revision;
          if (!current()) throw new Error("Ortak ürün taslağını yeniden açın.");
          const record = await getTeamProductSeo(sku);
          if (!current() || !record || record.revision !== revision) throw new Error("Taslak değişti. Meta sürüm listesini yeniden açın.");
          const changes = await restoreTeamProductMeta(record, index);
          if (!current()) throw new Error("Taslak değişti. Meta sürüm listesini yeniden açın.");
          const saved = await patchTeamProductSeo(sku, revision, changes, true);
          void flushTeamProductSeo().catch(console.warn);
          await this.applySharedSeoResult(detail, saved);
          await this.reload();
        } else await this.applyProductEdit(await api.restoreMetaVersion(sku, index));
        this.toast("Önceki meta geri yüklendi", "ok");
      } catch (e) {
        this.toast(String(e), "error");
      }
    },

    async restoreDetailsVersion(index: number) {
      const sku = this.selectedSku;
      const detail = this.detail;
      if (!sku || !detail || detail.sku !== sku) return;
      try {
        if (detail.team_seo_scope || teamWorkspace.value || teamUser.value || teamRestoring.value) {
          const sameProduct = () => this.detail === detail && this.selectedSku === sku &&
            detail.team_seo_scope === teamProductSeoScope();
          if (!sameProduct()) throw new Error("Ortak ürün taslağını yeniden açın.");
          if (!await this.flushProductMetaEdits(detail)) return;
          const revision = detail.seo_revision ?? null;
          const current = () => sameProduct() && (detail.seo_revision ?? null) === revision;
          if (!current()) throw new Error("Ortak ürün taslağını yeniden açın.");
          const record = await getTeamProductSeo(sku);
          if (!current() || !record || record.revision !== revision) {
            throw new Error("Taslak değişti. Açıklama sürümlerini yeniden açın.");
          }
          const changes = await restoreTeamProductDetails(record, index);
          if (!current()) throw new Error("Taslak değişti. Açıklama sürümlerini yeniden açın.");
          const saved = await patchTeamProductSeo(sku, revision, changes, true);
          void flushTeamProductSeo().catch(console.warn);
          await this.applySharedSeoResult(detail, saved);
        } else await this.applyProductEdit(await api.restoreDetailsVersion(sku, index));
        this.toast("Önceki açıklama geri yüklendi", "ok");
      } catch (e) {
        this.toast(String(e), "error");
      }
    },

    async restoreTechVersion(index: number) {
      const sku = this.selectedSku;
      const detail = this.detail;
      if (!sku || !detail || detail.sku !== sku) return;
      try {
        if (detail.team_seo_scope || teamWorkspace.value || teamUser.value || teamRestoring.value) {
          const revision = detail.seo_revision ?? null;
          const current = () => this.detail === detail && this.selectedSku === sku &&
            detail.team_seo_scope === teamProductSeoScope() && (detail.seo_revision ?? null) === revision;
          if (!current()) throw new Error("Ortak ürün taslağını yeniden açın.");
          const record = await getTeamProductSeo(sku);
          if (!current() || !record || record.revision !== revision) throw new Error("Taslak değişti. Teknik sürüm listesini yeniden açın.");
          const changes = await restoreTeamProductTech(record, index);
          if (!current()) throw new Error("Taslak değişti. Teknik sürüm listesini yeniden açın.");
          const saved = await patchTeamProductSeo(sku, revision, changes, true);
          void flushTeamProductSeo().catch(console.warn);
          await this.applySharedSeoResult(detail, saved);
        } else await this.applyProductEdit(await api.restoreTechVersion(sku, index));
        this.techDropped = [];
        this.toast("Önceki sürüm geri yüklendi", "ok");
      } catch (e) {
        this.toast(String(e), "error");
      }
    },

    async toggleTechDone() {
      if (!this.selectedSku || !this.detail) return;
      const detail = this.detail;
      const next = detail.tech_status === "done" ? "pending" : "done";
      if (detail.team_seo_scope || teamWorkspace.value || teamUser.value || teamRestoring.value) {
        if (!await this.saveSharedSeo(detail, { tech_status: next })) return;
        detail.tech_status = next;
      } else detail.tech_status = await api.markTechDone(this.selectedSku);
      await this.reload();
    },

    async checkImages(sku: string) {
      if (this.imageChecking) return;
      this.imageChecking = true;
      try {
        const res = await api.checkImages(sku);
        if (this.selectedSku === sku) this.imageCheck = res;
      } catch (e) {
        /* görsel kontrolü kritik değil — sessizce geç */
      } finally {
        this.imageChecking = false;
      }
    },

    async runResearch(seed?: string) {
      if (!this.selectedSku || this.researching) return;
      if (this.needsSharedIntegration()) return;
      this.researching = true;
      try {
        this.research = await api.researchSeo(this.selectedSku, seed);
        const n = this.research.target_candidates.length;
        this.toast(
          n
            ? `Araştırma tamam · ${n} anahtar kelime fikri`
            : "Araştırma tamam",
          "ok",
        );
      } catch (e) {
        this.toast(String(e), "error");
      } finally {
        this.researching = false;
      }
    },

    async refreshDetailBadgeInList() {
      // Detaydaki değişiklikler sonrası liste rozetlerini tazele
      await this.reload();
    },

    async sync() {
      if (this.syncing) return;
      this.syncing = true;
      try {
        if (teamUser.value && teamWorkspace.value) {
          await this.refreshProductsSource();
          if (this.productCatalogRevision === null) throw new Error("Bu çalışma alanında ortak katalog henüz yayımlanmadı.");
          this.toast(`Ortak katalog · ${this.allRows.length} etkin ürün`, "ok");
          return;
        }
        this.lastSync = await api.syncFeed();
        this.showSummary = true;
        await this.reload();
        if (!this.selectedSku && this.rows.length) {
          await this.select(this.rows[0].sku);
        } else if (this.selectedSku) {
          await this.select(this.selectedSku);
        }
        this.toast(
          `Feed güncellendi · ${this.lastSync.active} aktif ürün`,
          "ok",
        );
      } catch (e) {
        this.toast(String(e), "error");
      } finally {
        this.syncing = false;
      }
    },

    scheduleProductMetaEdits(detail: ProductDetail, changes: ProductMetaEdits) {
      const pending = metaAutosaves.get(detail) ?? { changes: {} };
      metaAutosaves.set(detail, pending);
      Object.assign(pending.changes, changes);
      window.clearTimeout(pending.timer);
      pending.timer = window.setTimeout(async () => {
        if (!await this.flushProductMetaEdits(detail)) return;
        await this.reload().catch((error) => this.toast(String(error), "error"));
      }, 600);
    },

    async flushProductMetaEdits(detail: ProductDetail): Promise<boolean> {
      const pending = metaAutosaves.get(detail);
      if (!pending) return true;
      window.clearTimeout(pending.timer);
      pending.timer = undefined;
      const save = async (previous: boolean) => {
        const changes = pending.changes;
        if (!Object.keys(changes).length) return previous;
        pending.changes = {};
        try {
          if (detail.team_seo_scope || teamWorkspace.value || teamUser.value || teamRestoring.value) {
            if (!await this.saveSharedSeo(detail, changes, true)) {
              pending.changes = { ...changes, ...pending.changes };
              return false;
            }
          } else {
            if (changes.draft_title !== undefined || changes.draft_descriptions !== undefined || changes.draft_search_keywords !== undefined) {
              await api.saveMetaDraft(detail.sku,
                changes.draft_title ?? detail.draft_title ?? detail.title ?? "",
                changes.draft_descriptions ?? detail.draft_descriptions ?? detail.descriptions ?? "",
                changes.draft_search_keywords ?? detail.draft_search_keywords ?? detail.search_keywords ?? "");
            }
            if (changes.target_keyword !== undefined) await api.setTargetKeyword(detail.sku, changes.target_keyword);
          }
          Object.assign(detail, changes);
          return true;
        } catch (error) {
          pending.changes = { ...changes, ...pending.changes };
          this.toast(String(error), "error");
          return false;
        }
      };
      const saving = (pending.saving ?? Promise.resolve(true)).then(save);
      pending.saving = saving;
      const saved = await saving;
      if (pending.saving === saving) pending.saving = undefined;
      return saved;
    },

    async toggleMetaDone() {
      if (!this.selectedSku || !this.detail || this.generating) return;
      const detail = this.detail;
      if (!await this.flushProductMetaEdits(detail) || this.detail !== detail) return;
      const shared = Boolean(detail.team_seo_scope || teamWorkspace.value || teamUser.value || teamRestoring.value);
      const next = (!shared || detail.seo_revision != null) && detail.meta_status === "done" ? "pending" : "done";
      if (shared) {
        if (!await this.saveSharedSeo(detail, { meta_status: next }, true)) return;
        detail.meta_status = next;
      } else detail.meta_status = await api.markMetaDone(this.selectedSku);
      await this.reload();
    },

    /// "Baktım, içerik hâlâ doğru" — bayrağı düşürür, içeriğe dokunmaz.
    /**
     * Sonuç verisini yükler. Ölçüm omurgası yerel veritabanından okunuyor — GSC çağrısı yok,
     * bu yüzden ekran açılışında çağrılması ucuz.
     */
    // --- Odak seansı (Faz S) ---------------------------------------------------------
    //
    // 🚫 Oyunlaştırma yok: XP, lig, seri cezası, konfeti yok. Sakin bir çalışma ritmi.

    /** Seans durumunu okur (açılışta ve her eylemden sonra). */
    async loadSession() {
      const source = todaySource();
      if (source === "") return;
      try {
        const [session, canStart] = await Promise.all([
          api.getFocusState(source, source ? this.today?.items ?? [] : undefined),
          api.hasLockableItem(source, source ? this.today?.items ?? [] : undefined),
        ]);
        if (source !== todaySource() || this.sessionBusy) return;
        this.session = session;
        this.sessionCanStart = canStart;
        this.tickSession();
      } catch (e) {
        /* seans yardımcı bir özellik; okunamaması uygulamayı engellemesin */
      }
    },

    /** Seansı başlatır ve ilk işi kilitler. */
    async startSession() {
      const source = todaySource();
      if (source === "" || this.sessionBusy || (source && !teamTodaySync.value.canEdit)) return;
      this.sessionBusy = true;
      try {
        if (source && !this.today) await this.loadToday();
        if (source !== todaySource()) return;
        const d = await api.startFocusSession(source, source ? this.today?.items ?? [] : undefined);
        if (source !== todaySource()) return;
        // ⚠️ Kilitlenecek iş yoksa arka uç seans AÇMIYOR. Eskiden açıp hemen kapatıyordu ve
        // kullanıcı "Seans bitti · 0 iş" modaliyle karşılaşıyordu — başarısızlık gibi
        // okunuyordu, oysa günün işi bitmiş demek (saha hatası, 2026-08-08).
        if (!d.session_id) {
          this.sessionCanStart = false;
          this.toast("Bugünün işleri bitmiş — kilitlenecek iş kalmadı.", "ok");
          return;
        }
        this.session = d;
        this.sessionSummary = null;
        this.sessionBreakOffered = false;
        this.sessionCanStart = true;
        this.tickSession();
      } catch (e) {
        this.toast(String(e), "error");
      } finally {
        if (source === todaySource()) this.sessionBusy = false;
      }
    },

    /**
     * Sayacı günceller. ⚠️ Sayaç yalnızca GÖSTERİM: gerçek süre arka uçtaki
     * `started_at`/`ended_at` farkından çıkıyor, bu yüzden uygulama uyusa bile ölçüm bozulmaz.
     */
    tickSession() {
      const s = this.session;
      if (!s?.session_id) {
        this.sessionElapsed = 0;
        return;
      }
      const bas = new Date(s.started_at).getTime();
      this.sessionElapsed = Math.max(0, Math.floor((Date.now() - bas) / 1000));
      // Süre dolduğunda mola ÖNERİLİR, otomatik başlamaz.
      if (this.sessionElapsed >= s.planned_minutes * 60) this.sessionBreakOffered = true;
    },

    /**
     * Kilitli işi sonuçlandırır ve sıradakini kilitler.
     *
     * ⚠️ "Bitti" Faz K'nin `completeQueueItem`ini çağırıyor — yeni bir "tamamlandı" kavramı
     * YOK. "Atla" ise **yalnızca bu seans için**: kuyrukta kalır, kalıcı karar yazılmaz.
     */
    async resolveSessionItem(outcome: "done" | "skipped" | "dismissed", until: string | null = null) {
      const source = todaySource();
      const id = this.session?.session_id;
      const it = this.session?.locked;
      if (source === "" || !id || !it || this.sessionBusy) return;
      this.sessionBusy = true;
      try {
        if (outcome === "done") {
          await this.recordTodayDecision(it.kind as ItemRef["kind"], it.reference, "done", null,
            it.kind === "product" && it.bucket !== "review");
        } else if (outcome === "dismissed") {
          await this.recordTodayDecision(it.kind as ItemRef["kind"], it.reference, until ? "snooze" : "dismiss", until);
        }
        if (source !== todaySource()) return;
        const session = await api.resolveFocusItem(outcome, source, source ? this.today?.items ?? [] : undefined, id, it.item_id);
        if (source !== todaySource()) return;
        this.session = session;
        if (!session.session_id || !session.locked) {
          await this.showSessionSummary("queue_empty", source, id);
        }
        if (!source) await this.loadToday();
      } catch (e) {
        this.toast(String(e), "error");
      } finally {
        if (source === todaySource()) this.sessionBusy = false;
      }
    },

    /** Seansı bitirir ve sakin özeti gösterir. */
    async endSession(reason: "time_up" | "stopped" | "queue_empty") {
      const source = todaySource();
      const id = this.session?.session_id;
      if (source === "" || !id || this.sessionBusy) return;
      this.sessionBusy = true;
      try {
        await this.showSessionSummary(reason, source, id);
        await this.loadToday();
      } catch (e) {
        this.toast(String(e), "error");
      } finally {
        if (source === todaySource()) this.sessionBusy = false;
      }
    },

    /** Arka uç seansı kendi kapattığında (kuyruk tükendi) özeti çeker. */
    async showSessionSummary(reason: string, source: string | undefined, id: number) {
      const summary = await api.endFocusSession(reason, source, id);
      if (source !== todaySource()) return;
      this.sessionSummary = summary;
      this.session = null;
      this.sessionElapsed = 0;
      this.sessionBreakOffered = false;
    },

    // --- EOL karar deposu + 301 CSV (Faz D) -------------------------------------------

    /** Verilmiş kararları okur (EOL ekranı satırları rozetlemek için kullanıyor). */
    async loadEolDecisions() {
      try {
        const list = await api.getEolDecisions();
        this.eolDecisions = Object.fromEntries(list.map((d) => [d.slug, d]));
      } catch (e) {
        /* karar listesi okunamazsa ekran yine çalışsın */
      }
    },

    /**
     * Karar verir. `action`: `redirect_301` | `canonical` | `keep`.
     *
     * ⚠️ Karar ≠ öneri: buraya yalnızca kullanıcı onayladığında geliniyor. Sebebi ölçülmüş —
     * deterministik eşleştirici tek başına güvenilir değil ve yanlış yönlendirme,
     * yönlendirmemekten kötüdür (bkz. `decisions.rs`).
     */
    async decideEol(
      slug: string,
      url: string,
      action: string,
      targetSlug: string | null,
      targetSku: string | null,
      source: string,
    ) {
      try {
        await api.saveEolDecision(slug, url, action, targetSlug, targetSku, source);
        await this.loadEolDecisions();
        this.toast(
          action === "keep" ? "Sayfa bilinçli tutuluyor olarak işaretlendi." : "301 listesine eklendi.",
          "ok",
        );
      } catch (e) {
        this.toast(String(e), "error");
      }
    },

    /**
     * 301 iş listesini CSV'ye yazar.
     *
     * ⚠️ CSV bir **çıktı**, senkron kanalı değil: Excel'de doldurulan hedefler uygulamaya
     * geri dönmüyor. Kararsız satırların hedef sütunu bilerek boş (bkz. `decisions.rs`).
     */
    async exportRedirectCsv(path: string, minClicks: number) {
      try {
        const o = await api.exportRedirectCsv(path, minClicks);
        await this.loadEolDecisions();
        return o;
      } catch (e) {
        this.toast(String(e), "error");
        return null;
      }
    },

    /** Kararı geri alır — sayfa yeniden "kararsız" olur. */
    async undecideEol(slug: string) {
      try {
        await api.deleteEolDecision(slug);
        await this.loadEolDecisions();
      } catch (e) {
        this.toast(String(e), "error");
      }
    },

    resetTodaySource() {
      todayRequest++;
      todayInputs = undefined;
      this.today = null;
      this.todayBusy = false;
      this.todayError = "";
      this.session = null;
      this.sessionSummary = null;
      this.sessionCanStart = false;
      this.sessionBusy = false;
      this.sessionElapsed = 0;
      this.sessionBreakOffered = false;
    },

    /** Ekip ekranı, aynı çalışma alanının önbelleğiyle çevrimdışı da hesaplanır. */
    async loadToday(reuseInputs = false, refreshDecisions = false) {
      const source = todaySource();
      const request = ++todayRequest;
      this.todayError = "";
      if (source === "") {
        this.today = null;
        this.todayBusy = false;
        if (!teamRestoring.value) this.todayError = "İş kuyruğu için ekip oturumunu ve çalışma alanını kontrol edin.";
        return;
      }
      this.todayBusy = true;
      try {
        let shared = source && reuseInputs && todayInputs?.focus_scope === source ? todayInputs : undefined;
        if (source) {
          shared = shared
            ? { ...shared, decisions: await refreshTodayDecisions(source, refreshDecisions) }
            : await loadTeamTodayInputs(source);
        }
        const queue = await api.getTodayQueue(shared);
        if (request !== todayRequest || source !== todaySource()) return;
        todayInputs = shared;
        this.today = queue;
        this.sessionCanStart = queue.items.some((item) => !item.done);
      } catch (e) {
        if (request === todayRequest && source === todaySource()) {
          this.todayError = e instanceof Error ? e.message : String(e);
        }
      } finally {
        if (request === todayRequest && source === todaySource()) this.todayBusy = false;
      }
    },

    /**
     * Kuyruk maddesini açar: doğru ekrana gider ve o satırı öne çıkarır.
     *
     * Ürün maddeleri ürün ekranında seçiliyor (mevcut `openProduct`); araç ekranı
     * maddeleri `focus` üzerinden hedefleniyor.
     */
    async openQueueItem(page: string, id: string) {
      if (page === "products") {
        await this.openProduct(id);
        return;
      }
      // Müşteri maddesi kişi kartını açıyor — araç ekranlarındaki satır odağı gibi değil,
      // burada "satır" zaten kartın kendisi.
      if (page === "contacts") {
        this.page = "contacts";
        await this.openContact(teamTodayMode() ? id : Number(id));
        return;
      }
      const kind = KIND_BY_PAGE[page as Page];
      if (kind) {
        await this.openStorePage(kind, Number(id));
        return;
      }
      this.focus = { page, id };
      this.page = page as Page;
    },

    // --- Faz İ2: katalog sayfaları (kategori · marka · içerik) ---

    /**
     * Bir tipin tüm sayfalarını yükler. Sıralama arka uçta, **gösterime göre**.
     *
     * ⚠️ Seçim korunuyor ama tazeleniyor: gönderim sonrası liste yeniden yüklendiğinde
     * seçili satır kaybolursa operatör yerini kaybeder.
     */
    resetStorePagesSource() {
      storePagesRequest++;
      this.storePages = [];
      this.storePage = null;
      this.storePagesBusy = false;
    },

    async loadStorePages(kind: string) {
      const request = ++storePagesRequest;
      this.storePagesBusy = true;
      try {
        const rows = await listTeamStorePages(kind);
        if (request !== storePagesRequest || this.page !== PAGE_BY_KIND[kind]) return;
        this.storePages = rows;
        const secili = this.storePage?.remote_id;
        if (secili != null) {
          this.storePage =
            this.storePages.find((r) => r.remote_id === secili) ?? null;
        }
      } catch (e) {
        if (request === storePagesRequest) this.toast(String(e), "error");
      } finally {
        if (request === storePagesRequest) this.storePagesBusy = false;
      }
    },

    /**
     * Satır seçimi. Listedeki kayıt zaten tam `StorePageDetail` — yeniden çekmiyoruz.
     *
     * ⚠️ Liste satırı ile detay AYNI nesne olmalı: ayrı kopya tutulsaydı taslak
     * kaydedildikten sonra listedeki rozet eski hâlde kalırdı.
     */
    selectStorePage(id: number) {
      this.storePage = this.storePages.find((r) => r.remote_id === id) ?? null;
    },

    /** Detay tazelendi (üretim/kayıt/gönderim) → listedeki satırı da yerinde güncelle. */
    patchStorePage(d: StorePageDetail) {
      this.storePage = d;
      const i = this.storePages.findIndex((r) => r.remote_id === d.remote_id);
      if (i >= 0) this.storePages[i] = d;
    },

    /**
     * Başka bir ekrandan bir katalog sayfasına gider (İçerik açığı → "düzenle").
     *
     * ⚠️ Seçim doğrudan yapılamıyor: hedef ekran henüz kurulmadı ve listesi boş.
     * `storePagePending` bırakılıyor, ekran kurulunca tüketiyor — kuyruk maddelerindeki
     * `focus` mekanizmasının aynısı.
     */
    openStorePage(kind: string, id: number) {
      const hedef = PAGE_BY_KIND[kind];
      if (!hedef) return;
      this.storePagePending = { kind, id };
      this.storePage = null;
      this.storePageSearch = "";
      this.storePageFilter = "hepsi";
      this.page = hedef;
    },

    // --- Teklif (Faz T) ---

    resetQuoteSource() {
      quotesRequest++;
      quoteRequest++;
      this.quotes = [];
      this.quote = null;
      this.quoteSummary = null;
      this.quotesBusy = false;
    },

    async loadQuotes() {
      const request = ++quotesRequest;
      this.quotesBusy = true;
      try {
        const status = this.quoteStatusFilter;
        const central = !!teamUser.value;
        const quotes = central ? await teamQuotes.list() : await api.listQuotes(status);
        const summary = central ? summarizeTeamQuotes(quotes) : await api.quoteSummary();
        if (request !== quotesRequest) return;
        this.quotes = central && status ? quotes.filter((q) => q.status === status) : quotes;
        this.quoteSummary = summary;
      } catch (e) {
        if (request === quotesRequest) this.toast(String(e), "error");
      } finally {
        if (request === quotesRequest) this.quotesBusy = false;
      }
    },

    async openQuote(id: Quote["id"]) {
      const request = ++quoteRequest;
      try {
        const quote = typeof id === "string" ? await teamQuotes.get(id) : await api.getQuote(id);
        if (request === quoteRequest) this.quote = quote;
      } catch (e) {
        if (request === quoteRequest) this.toast(String(e), "error");
      }
    },

    async createQuote(contactId: number | null, currency: string) {
      try {
        if (teamUser.value) {
          const defaults = await api.getQuoteDefaults();
          const now = new Date();
          const valid = new Date(now);
          valid.setDate(valid.getDate() + defaults.valid_days);
          const id = crypto.randomUUID();
          await teamQuotes.save({
            id, no: `T-${now.getFullYear()}-${id.replace(/-/g, "").slice(0, 12).toUpperCase()}`,
            contact_id: null, contact_name: "", currency, status: "draft", status_label: "Taslak",
            fx_rate: null, fx_date: null, valid_until: valid.toISOString().slice(0, 10), note: "",
            close_reason: "", created_at: now.toISOString(), sent_at: null, items: [], subtotal: 0,
            taxes: [], tax_total: 0, grand_total: 0, margin: null, version_count: 0,
          }, null);
          await this.loadQuotes();
          await this.openQuote(id);
          return id;
        }
        const id = await api.createQuote(contactId, currency);
        await this.loadQuotes();
        await this.openQuote(id);
        return id;
      } catch (e) {
        this.toast(String(e), "error");
        return null;
      }
    },

    async saveQuote(q: Omit<Parameters<typeof api.saveQuote>[0], "id" | "contactId"> & {
      id: Quote["id"]; contactId: ContactId | null; contactName?: string;
    }) {
      try {
        if (typeof q.id === "string") {
          const quote = this.quote;
          if (!quote || quote.id !== q.id || quote.revision == null) return;
          await teamQuotes.save({
            ...quote, contact_id: q.contactId,
            contact_name: q.contactName ?? (quote.contact_id === q.contactId ? quote.contact_name : ""),
            currency: q.currency, fx_rate: q.fxRate, fx_date: q.fxDate,
            valid_until: q.validUntil, note: q.note,
          }, quote.revision);
        } else {
          const { contactName: _, ...draft } = q;
          await api.saveQuote({ ...draft, id: q.id, contactId: typeof q.contactId === "number" ? q.contactId : null });
        }
        await this.openQuote(q.id);
        await this.loadQuotes();
        this.toast(typeof q.id === "string" && (teamQuoteQueue.value.pending || teamQuoteQueue.value.conflicts)
          ? "Teklif bu cihazda kaydedildi. Eşitleme durumunu listeden izleyebilirsiniz."
          : "Teklif kaydedildi.", "ok");
      } catch (e) {
        this.toast(String(e), "error");
      }
    },

    async deleteQuote(id: Quote["id"]) {
      if (typeof id === "string") return;
      try {
        await api.deleteQuote(id);
        if (this.quote?.id === id) this.quote = null;
        await this.loadQuotes();
        this.toast("Teklif silindi.", "ok");
      } catch (e) {
        this.toast(String(e), "error");
      }
    },

    /** ⚠️ Hata mesajı olduğu gibi gösteriliyor: "kur girin" gibi eyleme dönük bilgi taşıyor. */
    async addQuoteLine(sku: string | null, name: string) {
      const q = this.quote;
      if (!q) return;
      try {
        if (typeof q.id === "string") {
          if (q.revision == null) return;
          const item = sku
            ? await prepareTeamCatalogLine(sku, q.currency, q.fx_rate)
            : { sku: null, name: name.trim(), qty: 1, unit_price: 0,
              tax_rate: (await api.getQuoteDefaults()).tax_rate, cost: null };
          if (!item.name) return;
          await teamQuotes.save({ ...q, items: [...q.items, { ...item, id: crypto.randomUUID(), net: 0, margin: null }] }, q.revision);
        } else if (sku) await api.addQuoteItemFromCatalog(q.id, sku, 1);
        else await api.addQuoteItemManual(q.id, name);
        await this.openQuote(q.id);
      } catch (e) {
        this.toast(String(e), "error");
      }
    },

    async updateQuoteLine(
      itemId: Quote["items"][number]["id"],
      name: string,
      qty: number,
      unitPrice: number,
      taxRate: number,
    ) {
      const q = this.quote;
      if (!q) return;
      try {
        if (typeof itemId === "string" && q.revision != null) {
          await teamQuotes.save({ ...q, items: q.items.map((item) => item.id === itemId
            ? { ...item, name, qty, unit_price: unitPrice, tax_rate: taxRate } : item) }, q.revision);
        } else if (typeof itemId === "number") await api.updateQuoteItem(itemId, name, qty, unitPrice, taxRate);
        await this.openQuote(q.id);
      } catch (e) {
        this.toast(String(e), "error");
      }
    },

    async deleteQuoteLine(itemId: Quote["items"][number]["id"]) {
      const q = this.quote;
      if (!q) return;
      try {
        if (typeof itemId === "string" && q.revision != null) {
          await teamQuotes.save({ ...q, items: q.items.filter((item) => item.id !== itemId) }, q.revision);
        } else if (typeof itemId === "number") await api.deleteQuoteItem(itemId);
        await this.openQuote(q.id);
      } catch (e) {
        this.toast(String(e), "error");
      }
    },

    /** Durum geçişi. Kişi bağlıysa zaman çizelgesine de yazılıyor (arka uçta). */
    async setQuoteStatus(status: string, reason = "") {
      const q = this.quote;
      if (!q) return;
      try {
        if (typeof q.id === "string") await teamQuotes.transition(q, status, reason);
        else await api.setQuoteStatus(q.id, status, reason);
        await this.openQuote(q.id);
        await this.loadQuotes();
        if (q.contact_id) {
          void this.loadContacts();
          if (this.contact?.id === q.contact_id) void this.openContact(q.contact_id);
        }
        return true;
      } catch (e) {
        this.toast(String(e), "error");
      }
    },

    // --- CRM (Faz C) ---

    resetContactsSource() {
      contactsRequest++;
      this.contacts = [];
      this.contactTags = [];
      this.contactTag = "";
      this.contactsBusy = false;
      this.closeContact();
    },

    async loadContacts() {
      const request = ++contactsRequest;
      this.contactsBusy = true;
      try {
        const rows = await api.listContacts(this.contactSearch, this.contactArchived, this.contactChannel, this.contactTag);
        if (request !== contactsRequest) return;
        const tags = await api.listContactTags();
        if (request !== contactsRequest) return;
        this.contacts = rows;
        this.contactTags = tags;
      } catch (e) {
        if (request === contactsRequest) this.toast(String(e), "error");
      } finally {
        if (request === contactsRequest) this.contactsBusy = false;
      }
    },

    setContactSearch(v: string) {
      this.contactSearch = v;
      void this.loadContacts();
    },

    async openContact(id: ContactId) {
      const request = ++contactRequest;
      try {
        const [contact, events, products] = await Promise.all([
          api.getContact(id), api.getContactEvents(id), api.getContactProducts(id),
        ]);
        if (request !== contactRequest) return;
        this.contact = contact;
        this.contactEvents = events;
        this.contactProducts = products;
      } catch (e) {
        if (request === contactRequest) this.toast(String(e), "error");
      }
    },

    closeContact() {
      contactRequest++;
      this.contact = null;
      this.contactEvents = [];
      this.contactProducts = [];
    },

    /** Etiketleri toptan yazar — ekle/çıkar ayrı değil (ekranla veri ayrışmasın). */
    async setContactTags(id: ContactId, tags: string[]) {
      try {
        await api.setContactTags(id, tags, this.contact?.id === id ? this.contact.revision : undefined);
        await this.openContact(id);
        await this.loadContacts();
      } catch (e) {
        this.toast(String(e), "error");
      }
    },

    async linkContactProduct(id: ContactId, sku: string) {
      try {
        await api.linkContactProduct(id, sku, this.contact?.id === id ? this.contact.revision : undefined);
        await this.openContact(id);
      } catch (e) {
        this.toast(String(e), "error");
      }
    },

    async unlinkContactProduct(id: ContactId, sku: string) {
      try {
        await api.unlinkContactProduct(id, sku, this.contact?.id === id ? this.contact.revision : undefined);
        await this.openContact(id);
      } catch (e) {
        this.toast(String(e), "error");
      }
    },

    /** Kişiyi kaydeder ve kartı açık tutar — kaydettikten sonra listeye atmak sinir bozucu. */
    async saveContact(c: Parameters<typeof api.saveContact>[0]) {
      try {
        const id = await api.saveContact(c);
        await this.loadContacts();
        await this.openContact(id);
        const progress = typeof id === "string" ? await teamContactProgress(id) : "synced";
        if (typeof id === "number") void this.loadToday();
        this.toast(progress === "conflict" ? "Yerel düzenleme korundu. Sunucudaki kayıtla çakışıyor."
          : progress === "pending" ? "Kişi bu cihazda kaydedildi. Eşitleme bekliyor."
          : "Kişi kaydedildi.", progress === "conflict" ? "error" : "ok");
        return id;
      } catch (e) {
        this.toast(String(e), "error");
        return null;
      }
    },

    async archiveContact(id: ContactId, archived: boolean) {
      try {
        await api.archiveContact(id, archived, this.contact?.id === id ? this.contact.revision : undefined);
        await this.loadContacts();
        if (this.contact?.id === id) await this.openContact(id);
        void this.loadToday();
        this.toast(archived ? "Kişi arşivlendi." : "Kişi arşivden çıkarıldı.", "ok");
      } catch (e) {
        this.toast(String(e), "error");
      }
    },

    /** Temas + (varsa) yeni randevu. ⚠️ Kuyruk da tazeleniyor: sonraki adım kuyruğu besliyor. */
    async addContactEvent(
      contactId: ContactId,
      kind: string,
      note: string,
      nextStepAt: string | null,
      nextStepNote: string | null,
      revision?: number,
    ) {
      try {
        await api.addContactEvent(contactId, kind, note, nextStepAt, nextStepNote,
          revision ?? (this.contact?.id === contactId ? this.contact.revision : undefined));
        if (this.contact?.id === contactId) await this.openContact(contactId);
        if (this.page === "contacts") await this.loadContacts();
        void this.loadToday();
        this.toast("Temas kaydedildi.", "ok");
      } catch (e) {
        this.toast(String(e), "error");
      }
    },

    /** Hedef satır çizildikten sonra ekran bunu çağırır — odak yapışkan olmamalı. */
    clearFocus() {
      this.focus = null;
    },

    async recordTodayDecision(kind: ItemRef["kind"], reference: string, action: "done" | "dismiss" | "snooze" | "restore", until: string | null = null, recordStoreChange = false) {
      const source = todaySource();
      if (source === "") throw new Error("İş kuyruğu için çalışma alanını açın.");
      if (!source) {
        if (action === "done") await api.completeQueueItem(kind, reference, recordStoreChange);
        else if (action === "restore") await api.restoreQueueItem(kind, reference);
        else await api.dismissQueueItem(kind, reference, until);
        return;
      }
      const shared = todayInputs;
      if (shared?.focus_scope !== source) throw new Error("İş kuyruğunu yenileyip yeniden deneyin.");
      await queueTodayDecision(source, shared, kind, reference, action, until, recordStoreChange);
      if (source !== todaySource()) return;
      await this.loadToday(true);
      this.syncTodayDecisions(source);
    },

    syncTodayDecisions(source: string) {
      void flushTeamContacts().then(flushTeamToday).then((changed) => {
        if (!changed || source !== todaySource()) return;
        void this.loadToday();
        if (this.page === "contacts") {
          void this.loadContacts();
          if (typeof this.contact?.id === "string") void this.openContact(this.contact.id);
        }
        if (this.outcomeSummary) void this.loadOutcomes();
      }).catch(console.warn);
    },

    /** Maddeyi kuyruktan çıkarır. `until` yoksa kalıcı, varsa o tarihe kadar. */
    async dismissQueueItem(kind: ItemRef["kind"], reference: string, until: string | null) {
      const source = todaySource();
      try {
        await this.recordTodayDecision(kind, reference, until ? "snooze" : "dismiss", until);
        if (source !== todaySource()) return;
        if (!source) await this.loadToday();
        this.toast(source ? "İş kararı eşitleme kuyruğuna alındı." : until ? "Madde yarına ertelendi." : "Madde kuyruktan gizlendi.", "ok");
      } catch (e) {
        this.toast(String(e), "error");
      }
    },

    /**
     * Maddeyi "yapıldı" işaretler: kuyruktan çıkar + ölçüm olayı yaz.
     *
     * ⚠️ Gizlemeden farkı **kalıcı olmaması**: işaret sonraki analize kadar geçerli. İş işe
     * yaramadıysa madde geri gelir. Ürün maddelerinde ayrıca sonuç takibi başlar — Faz Ö'nün
     * "elle yapılan iş ölçülemiyor" boşluğu bu düğmeyle kapanıyor.
     */
    async completeQueueItem(kind: ItemRef["kind"], reference: string, recordStoreChange = false) {
      const source = todaySource();
      try {
        await this.recordTodayDecision(kind, reference, "done", null, recordStoreChange);
        if (source !== todaySource()) return;
        if (source) {
          this.toast("Yapıldı işareti eşitleme kuyruğuna alındı.", "ok");
          return;
        }
        await this.loadToday();
        // Ürün maddelerinde sonuç rozetleri de değişiyor; özet tazelensin.
        if (kind === "product") void this.loadOutcomes();
        // Müşteri maddesinde kart da tazelensin: sonraki adım temizlendi, temas eklendi.
        if (kind === "contact") {
          void this.loadContacts();
          if (this.contact?.id === Number(reference)) void this.openContact(Number(reference));
        }
        this.toast(
          kind === "product"
            ? "Yapıldı — sonucu 28 gün sonra ölçülecek."
            : kind === "contact"
              ? "Dönüş yapıldı olarak kaydedildi."
              : "Yapıldı olarak işaretlendi.",
          "ok",
        );
      } catch (e) {
        this.toast(String(e), "error");
      }
    },

    /** Tek maddenin kararını geri alır ("geri al" bağlantısı). */
    async restoreQueueItem(kind: ItemRef["kind"], reference: string) {
      try {
        const source = todaySource();
        await this.recordTodayDecision(kind, reference, "restore");
        if (!source && source === todaySource()) await this.loadToday();
      } catch (e) {
        this.toast(String(e), "error");
      }
    },

    /** Gizlenen/ertelenen maddelerin tamamını geri getirir. */
    async restoreQueueItems() {
      try {
        const source = todaySource();
        if (source === "") return;
        if (!source) {
          await api.restoreQueueItems();
          await this.loadToday();
          return;
        }
        const shared = todayInputs;
        if (shared?.focus_scope !== source) return;
        const day = todayLocalDay();
        const hidden = shared.decisions.filter((row) => row.action === "dismiss" || (row.until_day && row.until_day > day));
        try {
          for (const row of hidden) {
            if (teamTodaySync.value.conflicts.some(({ id }) => id === row.id)) continue;
            await queueTodayDecision(source, shared, row.kind, row.reference, "restore");
          }
        } finally {
          if (source === todaySource()) {
            await this.loadToday(true);
            this.syncTodayDecisions(source);
          }
        }
      } catch (e) {
        this.toast(String(e), "error");
      }
    },

    async resolveTodayConflict(id: string) {
      const source = teamTodayScope();
      if (!source) return;
      try {
        await acceptTodayConflict(source, id);
        if (source === teamTodayScope()) await this.loadToday();
      } catch (e) {
        this.toast(String(e), "error");
      }
    },

    async loadOutcomes() {
      try {
        const inTeam = teamGscMode();
        const scope = teamMeasurementScope();
        const shared = inTeam ? await teamOutcomeInputs() : null;
        if (inTeam !== teamGscMode() || scope !== teamMeasurementScope()) return;
        if (inTeam && !shared) {
          this.outcomeSummary = null;
          this.outcomeBadges = {};
          return;
        }
        const [ozet, rozetler] = await Promise.all([
          api.getOutcomeSummary(shared),
          api.getOutcomeBadges(shared),
        ]);
        if (inTeam !== teamGscMode() || scope !== teamMeasurementScope()) return;
        this.outcomeSummary = ozet;
        // ⚠️ Anahtar `sku || url`: içerik rozetlerinin sku'su boş, yalnızca sku ile
        // anahtarlansaydı hepsi tek bir boş anahtara çakışırdı (Faz İ).
        this.outcomeBadges = Object.fromEntries(rozetler.map((b) => [b.sku || b.url, b]));
      } catch (e) {
        // Sessiz: sonuç verisi yardımcı bir katman, ekranı düşürmemeli.
        console.warn("sonuç verisi okunamadı", e);
      }
    },

    /** Geçmişi GSC'den tohumlar (bir kez; tekrar çalıştırmak zararsız). */
    async seedMetricHistory() {
      if (this.seedBusy) return;
      this.seedBusy = true;
      try {
        const inTeam = teamGscMode();
        const scope = teamMeasurementScope();
        const message = inTeam
          ? await seedTeamGscHistory().then((count) => count
            ? `${count} GSC dönemi ortak geçmişe eşitlemek için hazırlandı.`
            : "Ortak GSC geçmişinde kuyruğa alınacak yeni dönem bulunamadı.")
          : await api.seedMetricHistory().then((result) => `${result.snapshots_added} dönem eklendi · ${result.events_backfilled} geçmiş gönderim işlendi`);
        if (inTeam !== teamGscMode() || scope !== teamMeasurementScope()) return;
        this.toast(message, "ok");
        await this.loadOutcomes();
      } catch (e) {
        this.toast(String(e), "error");
      } finally {
        this.seedBusy = false;
      }
    },

    async dismissFeedChange() {
      if (!this.selectedSku) return;
      if (this.needsSharedIntegration()) return;
      await api.markFeedReviewed(this.selectedSku);
      if (this.detail) this.detail.feed_changed = null;
      await this.reload();
      this.toast("Ürün gözden geçirildi olarak işaretlendi.");
    },

    async toggleDetailsDone() {
      const sku = this.selectedSku;
      const detail = this.detail;
      if (!sku || !detail || detail.sku !== sku || this.generatingDetails) return;
      if (!await this.flushProductMetaEdits(detail) || this.detail !== detail || this.selectedSku !== sku) return;
      if (detail.team_seo_scope || teamWorkspace.value || teamUser.value || teamRestoring.value) {
        const next = detail.seo_revision != null && detail.details_status === "done" ? "pending" : "done";
        if (!await this.saveSharedSeo(detail, { details_status: next }, true)) return;
        detail.details_status = next;
      } else detail.details_status = await api.markDetailsDone(sku);
      await this.reload();
    },

    async generateMeta() {
      const sku = this.selectedSku;
      const detail = this.detail;
      if (!sku || !detail || detail.sku !== sku || this.generating) return;
      const shared = Boolean(detail.team_seo_scope || teamWorkspace.value || teamUser.value || teamRestoring.value);
      if (!shared && detail.meta_status === "done") {
        this.toast("Bu ürün zaten tamamlandı işaretli.", "info");
        return;
      }
      this.generating = true;
      try {
        if (!await this.flushProductMetaEdits(detail)) return;
        if (this.detail !== detail || this.selectedSku !== sku) return;
        if (shared) {
          const revision = detail.seo_revision ?? null;
          const current = () => this.detail === detail && this.selectedSku === sku &&
            detail.team_seo_scope === teamProductSeoScope() && (detail.seo_revision ?? null) === revision;
          if (!current()) throw new Error("Ortak ürün taslağını yeniden açın.");
          const record = await readTeamProductSeoForGeneration(sku, revision);
          if (!current()) throw new Error("Taslak değişti. Meta üretimini yeniden başlatın.");
          if (record?.draft.meta_status === "done") {
            this.toast("Bu ürün zaten tamamlandı işaretli.", "info");
            return;
          }
          const changes = await generateTeamProductMeta(detail, record);
          if (!await this.flushProductMetaEdits(detail)) return;
          await verifyTeamProductSeoRevision(sku, revision);
          if (!current()) throw new Error("Taslak değişti. Meta üretimini yeniden başlatın.");
          const saved = await patchTeamProductSeo(sku, revision, changes, true);
          void flushTeamProductSeo().catch(console.warn);
          await this.applySharedSeoResult(detail, saved);
        } else if (!teamRestoring.value && !teamUser.value && !teamWorkspace.value) {
          await this.applyProductEdit(await api.generateMeta(sku));
        } else return;
        await this.reload();
        this.toast("Meta üretildi · alanlar dolduruldu", "ok");
      } catch (e) {
        this.toast(String(e), "error");
      } finally {
        this.generating = false;
      }
    },

    async generateDetails() {
      const sku = this.selectedSku;
      const detail = this.detail;
      if (!sku || !detail || detail.sku !== sku || this.generatingDetails) return;
      const shared = !!(detail.team_seo_scope || teamWorkspace.value || teamUser.value || teamRestoring.value);
      if (!shared && detail.details_status === "done") {
        this.toast("Bu ürünün açıklaması zaten tamamlandı işaretli.", "info");
        return;
      }
      if (detail.image_count < 3) {
        this.toast(
          `En az 3 ürün görseli gerekli — şu an ${detail.image_count}/4.`,
          "error",
        );
        return;
      }
      this.generatingDetails = true;
      try {
        if (!await this.flushProductMetaEdits(detail)) return;
        if (this.detail !== detail || this.selectedSku !== sku) return;
        if (shared) {
          const sameProduct = () => this.detail === detail && this.selectedSku === sku &&
            detail.team_seo_scope === teamProductSeoScope();
          const revision = detail.seo_revision ?? null;
          const current = () => sameProduct() && (detail.seo_revision ?? null) === revision;
          if (!current()) throw new Error("Ortak ürün taslağını yeniden açın.");
          const record = await readTeamProductSeoForGeneration(sku, revision);
          if (!current()) throw new Error("Üretim sırasında ürün taslağı değişti.");
          if (record?.draft.details_status === "done") {
            this.toast("Bu ürünün açıklaması zaten tamamlandı işaretli.", "info");
            return;
          }
          const changes = await generateTeamProductDetails(detail, record);
          if (!current()) throw new Error("Üretim sırasında ürün taslağı değişti.");
          if (!await this.flushProductMetaEdits(detail)) return;
          if (!current()) throw new Error("Üretim sırasında ürün taslağı değişti.");
          await verifyTeamProductSeoRevision(sku, revision);
          if (!current()) throw new Error("Üretim sırasında ürün taslağı değişti.");
          const saved = await patchTeamProductSeo(sku, revision, changes, true);
          void flushTeamProductSeo().catch(console.warn);
          await this.applySharedSeoResult(detail, saved);
        } else if (!teamRestoring.value && !teamUser.value && !teamWorkspace.value) {
          await this.applyProductEdit(await api.generateDetails(sku));
        } else return;
        await this.reload();
        this.toast("Açıklama taslağı üretildi", "ok");
      } catch (e) {
        this.toast(String(e), "error");
      } finally {
        this.generatingDetails = false;
      }
    },

    navigate(dir: 1 | -1) {
      const skus = this.visibleSkus;
      if (!skus.length) return;
      const cur = this.selectedSku ? skus.indexOf(this.selectedSku) : -1;
      const nextIdx =
        cur < 0
          ? 0
          : Math.min(Math.max(cur + dir, 0), skus.length - 1);
      this.select(skus[nextIdx]);
    },
  },
});

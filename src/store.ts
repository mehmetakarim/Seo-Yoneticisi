import { defineStore } from "pinia";
import { api } from "./api";
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
} from "./types";
import type { Page } from "./navigation";

type Theme = "light" | "dark";

interface Toast {
  id: number;
  text: string;
  kind: "info" | "error" | "ok";
}

interface State {
  page: Page;
  theme: Theme;
  allRows: ProductRow[];
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
  successorBusy: string;
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
  /** Asistanın bağlam kaynağı: kullanıcı asistana geçmeden ÖNCE hangi araç ekranındaydı.
   *  `page` asistana geçince değiştiği için ayrı tutuluyor. App.vue güncelliyor. */
  lastToolPage: Page | "";
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
  techDropped: string[];
  ideasoftBusy: boolean;
  ideasoftPreview: IdeasoftPreview | null;
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
    page: "products",
    theme: "light",
    allRows: [],
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
    successorBusy: "",
    catalogBusy: false,
    canonicalPending: null,
    canonicalBusy: false,
    canonicalPicker: null,
    canonicalQuery: "",
    canonicalResults: [],
    canonicalSearching: false,
    setupOpen: false,
    lastToolPage: "",
    chat: [],
    chatBusy: false,
    chatThinking: false,
    chatModel: "",
    chatId: null,
    chatSessions: [],
    techStructuring: false,
    techDropped: [],
    ideasoftBusy: false,
    ideasoftPreview: null,
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

    async init() {
      // Tema: kayıtlı tercih yoksa sistem tercihine düş
      this.loading = true;
      try {
        this.settings = await api.getSettings();
        if (this.settings?.theme === "dark" || this.settings?.theme === "light") {
          this.theme = this.settings.theme;
        } else if (window.matchMedia?.("(prefers-color-scheme: dark)").matches) {
          this.theme = "dark";
        }
        this.applyTheme();
        this.lastSync = await api.getLastSync();
        // İlk çalıştırma mı? Karar backend'de (üç koşul birden, bkz. db::needs_setup) —
        // ön yüz yalnızca sonucu gösteriyor.
        try {
          this.setupOpen = await api.needsSetup();
        } catch {
          /* sihirbaz sorulamazsa uygulama normal açılsın */
        }
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

    async reload() {
      // Tüm ürünleri (tamamlananlar dahil) çek; filtre/arama istemcide.
      this.allRows = await api.listProducts("hepsi", "");
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
        this.opportunity = await api.getOpportunityCache();
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
        this.opportunity = await api.analyzeOpportunities();
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
     * kotayı (flash modellerde 20/gün) anında tüketirdi. Sonuç bellekte tutulur, aynı sayfa
     * için ikinci kez çağrılmaz.
     */
    async suggestSuccessor(url: string) {
      if (this.successorBusy || this.successors[url]) return;
      this.successorBusy = url;
      try {
        this.successors[url] = await api.suggestEolSuccessor(url);
      } catch (e) {
        this.toast(String(e), "error");
      } finally {
        this.successorBusy = "";
      }
    },

    /**
     * Asistanın göreceği bağlam: kullanıcının O AN baktığı ekranın satırları + rapor özeti.
     *
     * ⚠️ Kullanıcı kararı buydu — tüm raporu (2.190 EOL satırı dahil) her mesajda göndermek
     * gecikmeyi artırır ve uzun bağlamda model detayı karıştırmaya daha yatkın olur.
     * Satır sayısı bilinçli olarak sınırlı; asistan "listenin tamamı" iddiasında bulunmasın
     * diye kaç satır gördüğü de bağlama yazılıyor.
     */
    assistantContext(): string {
      const r = this.opportunity;
      if (!r) return "Henüz analiz çalıştırılmamış — elimizde Search Console verisi yok.";
      const N = 50;
      const n = (x: number) => Math.round(x);
      const lines: string[] = [
        `Analiz tarihi: ${r.analyzed_at} · son ${r.days} gün`,
        `Katalog: ${r.total_products} ürün, ${r.matched} tanesi Google'da bulundu`,
        `Fırsat: ${r.opportunities?.length ?? 0} · Satışta olmayan sayfa: ${r.eol?.length ?? 0} ` +
          `(${n(r.eol_clicks ?? 0)} tıklama) · Yükselmeye yakın sorgu: ${r.striking?.length ?? 0} ` +
          `· Yarışan arama: ${r.cannibalization?.length ?? 0} · Düşüşte: ${r.decay?.length ?? 0}`,
        "",
      ];

      const take = <T>(a: T[] | undefined) => (a ?? []).slice(0, N);
      const more = (a: unknown[] | undefined) =>
        (a?.length ?? 0) > N ? ` (ilk ${N} satır; toplam ${a!.length})` : "";

      // Aktif ekranın verisi ayrıntılı, diğerleri yalnızca yukarıdaki özet satırında.
      switch (this.lastToolPage) {
        case "opportunities":
          lines.push(`FIRSATLAR${more(r.opportunities)}:`);
          for (const o of take(r.opportunities))
            lines.push(
              `- ${o.name} [${o.sku}] gösterim=${n(o.impressions)} tıklama=${n(o.clicks)} ` +
                `konum=${o.position.toFixed(1)} kaçırılan=${n(o.missed_clicks)} sebep=${o.reason} ` +
                `kategori=${o.category || "-"} marka=${o.brand || "-"}`,
            );
          break;
        case "eol":
          lines.push(`SATIŞTA OLMAYAN AMA TRAFİK ALAN SAYFALAR${more(r.eol)}:`);
          for (const e of take(r.eol))
            lines.push(
              `- ${e.slug} tıklama=${n(e.clicks)} gösterim=${n(e.impressions)} konum=${e.position.toFixed(1)}`,
            );
          break;
        case "striking":
          lines.push(`YÜKSELMEYE YAKIN SORGULAR${more(r.striking)}:`);
          for (const q of take(r.striking))
            lines.push(
              `- "${q.query}" → ${q.name} gösterim=${n(q.impressions)} tıklama=${n(q.clicks)} ` +
                `konum=${q.position.toFixed(1)} kaçırılan=${n(q.missed_clicks)}`,
            );
          break;
        case "cannibal":
          lines.push("BİRBİRİYLE YARIŞAN SAYFALAR:");
          for (const c of take(r.cannibalization)) {
            lines.push(`- "${c.query}" gösterim=${n(c.impressions)} tıklama=${n(c.clicks)}`);
            for (const pg of c.pages)
              lines.push(`    · ${pg.name} konum=${pg.position.toFixed(1)} tıklama=${n(pg.clicks)}`);
          }
          break;
        case "decay":
          lines.push(`DÜŞÜŞTE OLANLAR${more(r.decay)}:`);
          for (const d of take(r.decay))
            lines.push(
              `- ${d.name} [${d.sku}] tıklama ${n(d.clicks_before)}→${n(d.clicks_now)} ` +
                `konum ${d.position_before.toFixed(1)}→${d.position_now.toFixed(1)} kayıp=${n(d.clicks_lost)}`,
            );
          break;
        default:
          lines.push(
            "Kullanıcı şu an bir araç ekranında değil; yalnızca yukarıdaki özet elimizde.",
          );
      }
      return lines.join("\n");
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
        const model = await api.assistantAsk(history, this.assistantContext(), (e) => {
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
          this.lastToolPage || "",
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
        this.chatModel = this.chatSessions.find((s) => s.id === id)?.model ?? "";
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
      this.selectedSku = sku;
      this.research = null; // araştırma ürüne özel — seçim değişince sıfırla
      this.imageCheck = null;
      this.techDropped = [];
      try {
        this.detail = await api.getProduct(sku);
        // Cache'lenmiş boyut sonucu varsa anında göster, sonra tazele
        this.imageCheck = this.detail?.image_check ?? null;
        void this.checkImages(sku);
      } catch (e) {
        this.toast(String(e), "error");
      }
    },

    // ---- Faz 8: teknik özellik tablosu ----
    async saveTechSource(text: string) {
      if (!this.selectedSku) return;
      await api.saveTechSource(this.selectedSku, text);
      if (this.detail) this.detail.tech_source_text = text;
    },

    async structureTech() {
      if (!this.selectedSku || this.techStructuring) return;
      this.techStructuring = true;
      this.techDropped = [];
      try {
        const res = await api.structureTechSpecs(this.selectedSku);
        this.techDropped = res.dropped;
        if (this.detail) this.detail.tech_specs = res.groups;
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
      if (!this.selectedSku) return;
      try {
        await api.saveTechSpecs(this.selectedSku, specs);
        if (this.detail) this.detail.tech_specs = specs;
      } catch (e) {
        this.toast(String(e), "error");
      }
    },

    // ---- Faz 9: IdeaSoft gönderimi ----
    /** Fark önizlemesini açar (gönderim öncesi zorunlu adım). */
    async openIdeasoftPreview(parts: string[]) {
      if (!this.selectedSku || this.ideasoftBusy) return;
      this.ideasoftBusy = true;
      this.ideasoftParts = parts;
      try {
        this.ideasoftPreview = await api.ideasoftPreview(this.selectedSku, parts);
      } catch (e) {
        this.toast(String(e), "error");
        this.ideasoftParts = [];
      } finally {
        this.ideasoftBusy = false;
      }
    },

    /** Hedef kelimeyi IdeaSoft'tan çeker (yerel alan boşken faydalı). */
    async pullIdeasoftKeyword() {
      if (!this.selectedSku || this.ideasoftBusy) return;
      this.ideasoftBusy = true;
      try {
        // Ne getirildiği/neyin korunduğu ayrımını arka uç biliyor; mesaj oradan geliyor.
        const r = await api.ideasoftPullKeyword(this.selectedSku);
        this.detail = r.detail;
        this.toast(r.message, "ok");
      } catch (e) {
        this.toast(String(e), "error");
      } finally {
        this.ideasoftBusy = false;
      }
    },

    closeIdeasoftPreview() {
      this.ideasoftPreview = null;
      this.ideasoftParts = [];
    },

    /** Onaylanan parçaları IdeaSoft'a yazar. */
    async confirmIdeasoftPush() {
      if (!this.selectedSku || this.ideasoftBusy || !this.ideasoftParts.length) return;
      this.ideasoftBusy = true;
      try {
        this.detail = await api.ideasoftPush(this.selectedSku, this.ideasoftParts);
        this.toast("IdeaSoft'a gönderildi ✓", "ok");
        this.closeIdeasoftPreview();
      } catch (e) {
        this.toast(String(e), "error");
      } finally {
        this.ideasoftBusy = false;
      }
    },

    async restoreMetaVersion(index: number) {
      if (!this.selectedSku) return;
      try {
        this.detail = await api.restoreMetaVersion(this.selectedSku, index);
        this.toast("Önceki meta geri yüklendi", "ok");
      } catch (e) {
        this.toast(String(e), "error");
      }
    },

    async restoreDetailsVersion(index: number) {
      if (!this.selectedSku) return;
      try {
        this.detail = await api.restoreDetailsVersion(this.selectedSku, index);
        this.toast("Önceki açıklama geri yüklendi", "ok");
      } catch (e) {
        this.toast(String(e), "error");
      }
    },

    async restoreTechVersion(index: number) {
      if (!this.selectedSku) return;
      try {
        this.detail = await api.restoreTechVersion(this.selectedSku, index);
        this.techDropped = [];
        this.toast("Önceki sürüm geri yüklendi", "ok");
      } catch (e) {
        this.toast(String(e), "error");
      }
    },

    async toggleTechDone() {
      if (!this.selectedSku || !this.detail) return;
      const next = await api.markTechDone(this.selectedSku);
      this.detail.tech_status = next;
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

    async saveDraft(title: string, descriptions: string, searchKeywords: string) {
      if (!this.selectedSku) return;
      await api.saveMetaDraft(this.selectedSku, title, descriptions, searchKeywords);
    },

    async saveKeyword(kw: string) {
      if (!this.selectedSku) return;
      await api.setTargetKeyword(this.selectedSku, kw);
    },

    async toggleMetaDone() {
      if (!this.selectedSku || !this.detail) return;
      const next = await api.markMetaDone(this.selectedSku);
      this.detail.meta_status = next;
      await this.reload();
    },

    /// "Baktım, içerik hâlâ doğru" — bayrağı düşürür, içeriğe dokunmaz.
    async dismissFeedChange() {
      if (!this.selectedSku) return;
      await api.markFeedReviewed(this.selectedSku);
      if (this.detail) this.detail.feed_changed = null;
      await this.reload();
      this.toast("Ürün gözden geçirildi olarak işaretlendi.");
    },

    async toggleDetailsDone() {
      if (!this.selectedSku || !this.detail) return;
      const next = await api.markDetailsDone(this.selectedSku);
      this.detail.details_status = next;
      await this.reload();
    },

    async generateMeta() {
      if (!this.selectedSku || this.generating) return;
      if (this.detail?.meta_status === "done") {
        this.toast("Bu ürün zaten tamamlandı işaretli.", "info");
        return;
      }
      this.generating = true;
      try {
        this.detail = await api.generateMeta(this.selectedSku);
        await this.reload();
        this.toast("Meta üretildi · alanlar dolduruldu", "ok");
      } catch (e) {
        this.toast(String(e), "error");
      } finally {
        this.generating = false;
      }
    },

    async generateDetails() {
      if (!this.selectedSku || this.generatingDetails) return;
      if (this.detail?.details_status === "done") {
        this.toast("Bu ürünün açıklaması zaten tamamlandı işaretli.", "info");
        return;
      }
      if ((this.detail?.image_count ?? 0) < 3) {
        this.toast(
          `En az 3 ürün görseli gerekli — şu an ${this.detail?.image_count ?? 0}/4.`,
          "error",
        );
        return;
      }
      this.generatingDetails = true;
      try {
        this.detail = await api.generateDetails(this.selectedSku);
        await this.reload();
        this.toast("Açıklama üretildi · yapı korundu", "ok");
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

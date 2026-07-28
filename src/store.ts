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
        tumu: 0,
      };
      for (const r of state.allRows) {
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
        this.detail = await api.ideasoftPullKeyword(this.selectedSku);
        this.toast(`Hedef kelime getirildi: "${this.detail.target_keyword ?? ""}"`, "ok");
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

import { defineStore } from "pinia";
import { api } from "./api";
import type {
  FilterKey,
  ProductDetail,
  ProductRow,
  SeoInsights,
  Settings,
  SyncSummary,
} from "./types";

type Page = "products" | "settings";
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

    async select(sku: string) {
      this.selectedSku = sku;
      this.research = null; // araştırma ürüne özel — seçim değişince sıfırla
      try {
        this.detail = await api.getProduct(sku);
      } catch (e) {
        this.toast(String(e), "error");
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

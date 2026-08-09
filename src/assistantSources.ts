/**
 * Asistanın konuşabileceği veri kaynakları — kayıt tablosu (Faz A).
 *
 * ⚠️ **Neden var:** bağlam üretimi `store.ts` içinde bir `switch (lastToolPage)` idi, yani
 * asistan **en son gezilen ekrana kilitliydi**; aynı sohbette önce Fırsatlar'ı sonra EOL'ü
 * konuşmak mümkün değildi. Ekrandaki uyarı bunu itiraf ediyordu: *"başka bir aracın verisini
 * sormak için önce o ekrana gidin"*.
 *
 * Artık kullanıcı girişteki **"+"** menüsünden kaynak seçiyor. Yeni bir kaynak eklemek =
 * bu tabloya bir girdi; ekran, store ve prompt değişmiyor.
 */
import type { OpportunityReport, OutcomeBadge, OutcomeSummary, ProductRow } from "./types";

/** Asistana gönderilecek bağlamın toplam satır bütçesi. */
export const TOTAL_LINE_BUDGET = 150;
const MIN_PER_SOURCE = 20;
const MAX_PER_SOURCE = 50;

/**
 * Kaynak başına satır sayısı.
 *
 * 🔬 Ölçüldü (2026-08-07, gerçek veri): beş kaynağın tamamı 50'şer satırla ~27.000 karakter
 * (~6.750 token) ediyor — bugün tek ekranla giden ~5.400 karakterin beş katı. Bütçe
 * paylaştırılınca **1–3 kaynakta bugünkü 50 satır korunuyor**, 7 kaynakta 21'e iniyor.
 */
export function linesPerSource(count: number): number {
  if (count <= 0) return 0;
  const pay = Math.floor(TOTAL_LINE_BUDGET / count);
  return Math.min(MAX_PER_SOURCE, Math.max(MIN_PER_SOURCE, pay));
}

/** Bağlam üretimi için gereken veriler — store'dan geçiliyor, kaynaklar store'u tanımıyor. */
export interface SourceData {
  report: OpportunityReport | null;
  products: ProductRow[];
  outcomeSummary: OutcomeSummary | null;
  outcomeBadges: Record<string, OutcomeBadge>;
}

export interface ContextSource {
  key: string;
  /** Menü ve çip etiketi — kenar çubuğundaki ekran adlarıyla aynı kelimeler. */
  label: string;
  /** Menüde gösterilen tek satırlık açıklama (baloncuk). */
  hint: string;
  /** Verisi var mı? Yoksa menüde soluk ve seçilemez. */
  available(d: SourceData): boolean;
  /** Toplam satır — "ilk N / toplam M" diyebilmek için. */
  total(d: SourceData): number;
  /** Bağlam bloğu: başlık + en fazla `n` satır. */
  lines(d: SourceData, n: number): string[];
}

const n0 = (x: number) => Math.round(x);
/** "(ilk N satır; toplam M)" — asistan listenin tamamını gördüğünü SANMAMALI. */
const more = (shown: number, total: number) =>
  total > shown ? ` (ilk ${shown} satır; toplam ${total})` : "";

export const SOURCES: ContextSource[] = [
  {
    key: "opportunities",
    label: "Fırsatlar",
    hint: "Konumunun getirmesi gereken tıklamayı alamayan ürünler",
    available: (d) => !!d.report?.opportunities?.length,
    total: (d) => d.report?.opportunities?.length ?? 0,
    lines: (d, n) => {
      const rows = (d.report?.opportunities ?? []).slice(0, n);
      const out = [`FIRSATLAR${more(rows.length, d.report?.opportunities?.length ?? 0)}:`];
      for (const o of rows)
        out.push(
          `- ${o.name} [${o.sku}] gösterim=${n0(o.impressions)} tıklama=${n0(o.clicks)} ` +
            `konum=${o.position.toFixed(1)} kaçırılan=${n0(o.missed_clicks)} sebep=${o.reason} ` +
            `kategori=${o.category || "-"} marka=${o.brand || "-"}`,
        );
      return out;
    },
  },
  {
    key: "eol",
    label: "Satışta olmayanlar",
    hint: "Google'da sıralanan ama katalogda olmayan sayfalar",
    available: (d) => !!d.report?.eol?.length,
    total: (d) => d.report?.eol?.length ?? 0,
    lines: (d, n) => {
      const rows = (d.report?.eol ?? []).slice(0, n);
      const out = [
        `SATIŞTA OLMAYAN AMA TRAFİK ALAN SAYFALAR${more(rows.length, d.report?.eol?.length ?? 0)}:`,
      ];
      for (const e of rows)
        out.push(
          `- ${e.slug} tıklama=${n0(e.clicks)} gösterim=${n0(e.impressions)} ` +
            `konum=${e.position.toFixed(1)}`,
        );
      return out;
    },
  },
  {
    key: "striking",
    label: "Yükselmeye yakın",
    hint: "4–20. sıradaki aramalar; hedef kelime adayları",
    available: (d) => !!d.report?.striking?.length,
    total: (d) => d.report?.striking?.length ?? 0,
    lines: (d, n) => {
      const rows = (d.report?.striking ?? []).slice(0, n);
      const out = [`YÜKSELMEYE YAKIN SORGULAR${more(rows.length, d.report?.striking?.length ?? 0)}:`];
      for (const q of rows)
        out.push(
          `- "${q.query}" → ${q.name} gösterim=${n0(q.impressions)} tıklama=${n0(q.clicks)} ` +
            `konum=${q.position.toFixed(1)} kaçırılan=${n0(q.missed_clicks)}`,
        );
      return out;
    },
  },
  {
    key: "cannibal",
    label: "Yarışan sayfalar",
    hint: "Aynı aramada birbiriyle yarışan kendi sayfalarınız",
    available: (d) => !!d.report?.cannibalization?.length,
    total: (d) => d.report?.cannibalization?.length ?? 0,
    lines: (d, n) => {
      // ⚠️ Bu kaynakta bir "satır" bir sorgu + altındaki sayfalar; bütçe sorgu başına sayılıyor.
      const rows = (d.report?.cannibalization ?? []).slice(0, n);
      const out = [
        `BİRBİRİYLE YARIŞAN SAYFALAR${more(rows.length, d.report?.cannibalization?.length ?? 0)}:`,
      ];
      for (const c of rows) {
        out.push(`- "${c.query}" gösterim=${n0(c.impressions)} tıklama=${n0(c.clicks)}`);
        for (const pg of c.pages)
          out.push(`    · ${pg.name} konum=${pg.position.toFixed(1)} tıklama=${n0(pg.clicks)}`);
      }
      return out;
    },
  },
  {
    key: "decay",
    label: "Düşüşte olanlar",
    hint: "Önceki döneme göre tıklama veya sıra kaybeden sayfalar",
    available: (d) => !!d.report?.decay?.length,
    total: (d) => d.report?.decay?.length ?? 0,
    lines: (d, n) => {
      const rows = (d.report?.decay ?? []).slice(0, n);
      const out = [`DÜŞÜŞTE OLANLAR${more(rows.length, d.report?.decay?.length ?? 0)}:`];
      for (const x of rows)
        out.push(
          `- ${x.name} [${x.sku}] tıklama ${n0(x.clicks_before)}→${n0(x.clicks_now)} ` +
            `konum ${x.position_before.toFixed(1)}→${x.position_now.toFixed(1)} ` +
            `kayıp=${n0(x.clicks_lost)}`,
        );
      return out;
    },
  },
  {
    key: "outcomes",
    label: "Sonuçlar",
    hint: "Mağazaya yapılan gönderimlerden sonra ne oldu (Faz Ö)",
    available: (d) => (d.outcomeSummary?.measured_events ?? 0) > 0,
    total: (d) => Object.keys(d.outcomeBadges).length,
    lines: (d, n) => {
      const s = d.outcomeSummary;
      const out = ["GÖNDERİM SONUÇLARI:"];
      if (s)
        out.push(
          `- Özet: ${s.measured_events} gönderim izleniyor · ${s.improved} iyileşti · ` +
            `${s.flat} değişmedi · ${s.worse} geriledi · ${s.measuring} hâlâ ölçülüyor · ` +
            `net ${s.net_delta_clicks >= 0 ? "+" : ""}${n0(s.net_delta_clicks)} tıklama · ` +
            `${s.snapshots} dönem kayıtlı`,
        );
      // ⚠️ Önce SONUCU BELLİ olanlar: "ölçülüyor" satırları henüz bilgi taşımıyor.
      const hepsi = Object.values(d.outcomeBadges);
      const oncelik = (b: OutcomeBadge) =>
        b.label === "İyileşti" || b.label === "Geriledi" ? 0 : b.label === "Değişmedi" ? 1 : 2;
      const rows = [...hepsi].sort((a, b) => oncelik(a) - oncelik(b)).slice(0, n);
      for (const b of rows) out.push(`- [${b.sku}] ${b.label} — ${b.tip}`);
      return out;
    },
  },
  {
    key: "catalog",
    label: "Katalog",
    hint: "Ürünler ve SEO iş durumu — hangi üründe ne eksik",
    available: (d) => d.products.length > 0,
    total: (d) => d.products.length,
    lines: (d, n) => {
      const p = d.products;
      const eksik = (r: ProductRow) => !r.meta_done || !r.details_done || !r.tech_done;
      const bekleyen = p.filter(eksik);
      const degisen = p.filter((r) => !!r.feed_changed);
      const out = [
        `KATALOG: ${p.length} ürün · meta tamamlanan ${p.filter((r) => r.meta_done).length} · ` +
          `açıklama ${p.filter((r) => r.details_done).length} · ` +
          `teknik ${p.filter((r) => r.tech_done).length} · ` +
          `feed değişmiş ${degisen.length}`,
      ];
      // ⚠️ 279 ürünün rastgele bir dilimi anlamsız olurdu; EKSİĞİ OLANLAR listeleniyor.
      const rows = bekleyen.slice(0, n);
      out.push(`EKSİĞİ OLAN ÜRÜNLER${more(rows.length, bekleyen.length)}:`);
      for (const r of rows) {
        const yok = [
          r.meta_done ? "" : "meta",
          r.details_done ? "" : "açıklama",
          r.tech_done ? "" : "teknik",
        ]
          .filter(Boolean)
          .join("+");
        // ⚠️ `workState()` KULLANILAMAZ: o `Opportunity` alıyor (meta_status/details_status),
        // `ProductRow` ise done bayrakları taşıyor. Aynı üç durum burada kendi alanlarından.
        const durum = r.meta_done && r.details_done
          ? "islendi"
          : !r.meta_done && !r.details_done
            ? "dokunulmamis"
            : "kismi";
        out.push(
          `- ${r.name} [${r.sku}] marka=${r.brand || "-"} durum=${durum} ` +
            `eksik=${yok}${r.feed_changed ? ` feed-değişti=${r.feed_changed}` : ""}`,
        );
      }
      return out;
    },
  },
];

export const sourceByKey = (key: string) => SOURCES.find((s) => s.key === key);

/**
 * Seçili kaynaklardan asistan bağlamını kurar.
 *
 * İlk satırlar her zaman rapor özeti: kullanıcı hangi kaynağı seçerse seçsin, asistanın
 * "katalogda kaç ürün var, kaç fırsat var" gibi çerçeveyi bilmesi gerekiyor.
 */
/**
 * 🔴 **GİZLİLİK KİLİDİ (Faz C).** Bu kaynak anahtarları asistana AÇILAMAZ.
 *
 * Asistan bağlamı Gemini'ye gönderiliyor. `contacts` tabloları müşteri adı, telefonu,
 * e-postası ve görüşme notları tutuyor — bunlar Google'a gitmez. Kural bir yorumla
 * bırakılmadı çünkü kayıt tablosuna girdi eklemek tek satırlık bir iş: aşağıdaki kontrol,
 * yasaklı bir anahtar eklenirse uygulamayı geliştirme kipinde **hemen** durduruyor.
 *
 * Kişi verisini asistana vermek istenirse önce şu soru cevaplanmalı: kullanıcı bunu açıkça
 * onayladı mı ve hangi alanlar maskeleniyor? O gün geldiğinde bu liste bilerek değiştirilir.
 */
const YASAKLI_KAYNAKLAR = ["contacts", "contact", "kisiler"];

if (import.meta.env.DEV) {
  const sizinti = SOURCES.filter((s) => YASAKLI_KAYNAKLAR.includes(s.key));
  if (sizinti.length) {
    throw new Error(
      `Gizlilik kilidi: kişisel veri kaynağı asistana açılamaz (${sizinti
        .map((s) => s.key)
        .join(", ")}). Bkz. assistantSources.ts — YASAKLI_KAYNAKLAR.`,
    );
  }
}

export function buildContext(d: SourceData, selected: string[]): string {
  const r = d.report;
  if (!r) return "Henüz analiz çalıştırılmamış — elimizde Search Console verisi yok.";

  const secili = selected.map(sourceByKey).filter((s): s is ContextSource => !!s);
  const n = linesPerSource(secili.length);

  const lines: string[] = [
    `Analiz tarihi: ${r.analyzed_at} · son ${r.days} gün`,
    `Katalog: ${r.total_products} ürün, ${r.matched} tanesi Google'da bulundu`,
    `Fırsat: ${r.opportunities?.length ?? 0} · Satışta olmayan sayfa: ${r.eol?.length ?? 0} ` +
      `(${n0(r.eol_clicks ?? 0)} tıklama) · Yükselmeye yakın sorgu: ${r.striking?.length ?? 0} ` +
      `· Yarışan arama: ${r.cannibalization?.length ?? 0} · Düşüşte: ${r.decay?.length ?? 0}`,
    "",
  ];

  if (!secili.length) {
    lines.push("Hiçbir veri kaynağı seçilmemiş; yalnızca yukarıdaki özet elimizde.");
    return lines.join("\n");
  }

  for (const s of secili) {
    lines.push(...s.lines(d, n), "");
  }
  return lines.join("\n");
}

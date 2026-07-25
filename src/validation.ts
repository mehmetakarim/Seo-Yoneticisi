import type { MetaBadge } from "./types";

/** Türkçe karakterler için grapheme bazlı sayım (backend ile tutarlı). */
export function glen(s: string): number {
  return [...(s ?? "")].length;
}

export const BADGE_LABEL: Record<MetaBadge | "bekliyor", string> = {
  eksik: "Eksik",
  hatali: "Hatalı",
  bekliyor: "Açıklama Bekliyor",
  uygun: "Uygun",
  tamamlandi: "Tamamlandı",
};

export interface RuleCheck {
  label: string;
  ok: boolean;
}

function keywordRule(text: string, kw: string): boolean {
  const k = kw.trim().toLowerCase();
  // Belirsiz (kelime yok) durumda geçmiş sayılır — göstergede yeşil olur.
  if (!k) return true;
  return text.toLowerCase().includes(k);
}

export function titleChecks(title: string, kw: string): RuleCheck[] {
  const t = title.trim();
  const len = glen(t);
  return [
    { label: "Boş değil", ok: len > 0 },
    { label: "20–60 karakter", ok: len >= 20 && len <= 60 },
    { label: "Hedef kelime içeriyor", ok: !!kw.trim() && keywordRule(t, kw) },
  ];
}

export function descChecks(desc: string, kw: string): RuleCheck[] {
  const d = desc.trim();
  const len = glen(d);
  return [
    { label: "Boş değil", ok: len > 0 },
    { label: "50–155 karakter", ok: len >= 50 && len <= 155 },
    { label: "Hedef kelime içeriyor", ok: !!kw.trim() && keywordRule(d, kw) },
  ];
}

/** Canlı rozet — backend meta_badge ile aynı mantık. */
export function computeBadge(
  title: string,
  desc: string,
  kw: string,
  metaDone: boolean,
): MetaBadge {
  if (metaDone) return "tamamlandi";
  const t = title.trim();
  const d = desc.trim();
  if (!t || !d) return "eksik";
  const tl = glen(t);
  const dl = glen(d);
  const tLenOk = tl >= 20 && tl <= 60;
  const dLenOk = dl >= 50 && dl <= 155;
  const tKw = keywordRule(t, kw);
  const dKw = keywordRule(d, kw);
  return tLenOk && dLenOk && tKw && dKw ? "uygun" : "hatali";
}

// ---- Kart 2 (details, salt gösterim) ----

export function stripHtml(html: string): string {
  return (html ?? "")
    .replace(/<[^>]*>/g, " ")
    .replace(/&nbsp;/gi, " ")
    .replace(/&amp;/gi, "&")
    .replace(/&[a-z]+;/gi, " ")
    .replace(/\s+/g, " ")
    .trim();
}

export function wordCount(html: string): number {
  const t = stripHtml(html);
  return t ? t.split(/\s+/).length : 0;
}

export function density(html: string, kw: string): number {
  const words = wordCount(html);
  const k = kw.trim().toLowerCase();
  if (!words || !k) return 0;
  const text = stripHtml(html).toLowerCase();
  let occ = 0;
  let i = 0;
  while ((i = text.indexOf(k, i)) !== -1) {
    occ++;
    i += k.length;
  }
  // Öbek-bazlı: geçiş / toplam kelime (öbek kelime sayısıyla çarpılmaz — Rust ile birebir).
  return (occ / words) * 100;
}

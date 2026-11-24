import type { Quote } from "./types";

type ActivityQuote = Pick<Quote, "id" | "updated_at" | "sent_at" | "created_at">;

const timestamp = /^(\d{4}-\d{2}-\d{2})[T ](\d{2}:\d{2}:\d{2})(?:\.(\d+))?(Z|[+-]\d{2}(?::?\d{2})?)?$/;

function instant(value?: string | null) {
  if (typeof value !== "string") return -Infinity;
  const input = value.trim();
  const match = timestamp.exec(input);
  if (!match) {
    const day = /^\d{4}-\d{2}-\d{2}$/.test(input) ? Date.parse(input) : NaN;
    return Number.isFinite(day) ? day : -Infinity;
  }
  // SQLite's older wall-clock timestamps have no offset. Compare them as written on every device.
  const zone = (match[4] ?? "Z").replace(/^([+-]\d{2})(\d{2})?$/, (_, hours, minutes) => `${hours}:${minutes ?? "00"}`);
  const wholeSeconds = Date.parse(`${match[1]}T${match[2]}${zone}`);
  return Number.isFinite(wholeSeconds) ? wholeSeconds + Number(`0.${match[3] ?? "0"}`) * 1000 : -Infinity;
}

export function quoteActivity(quote: ActivityQuote) {
  return Math.max(instant(quote.updated_at), instant(quote.sent_at), instant(quote.created_at));
}

export function sortQuotesByActivity<T extends ActivityQuote>(quotes: readonly T[]): T[] {
  return quotes.map((quote) => ({ quote, activity: quoteActivity(quote) })).sort((a, b) => {
    if (a.activity !== b.activity) return a.activity > b.activity ? -1 : 1;
    const left = a.quote.id, right = b.quote.id;
    if (typeof left === "number" && typeof right === "number") return right - left;
    const leftKey = String(left), rightKey = String(right);
    return leftKey < rightKey ? -1 : leftKey > rightKey ? 1 : 0;
  }).map(({ quote }) => quote);
}

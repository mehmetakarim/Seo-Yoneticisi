import { invoke } from "@tauri-apps/api/core";
import type { SupabaseClient } from "@supabase/supabase-js";
import { teamAccessIssue, teamClient, teamProject, teamReachable, teamSession, teamUser, teamWorkspace } from "./teamConnection";
import type { CatalogMatch, ProductDetail, ProductRow } from "./types";
import { getTeamProductSeo, listTeamProductSeo, teamProductSeoScope, type TeamSeoRecord } from "./teamProductSeo";

type CatalogFacts = {
  id: string | null;
  status: string | null;
  quantity: number | null;
  url: string | null;
  category?: string | null;
  main_category?: string | null;
  brand?: string | null;
  currency_abbr: string | null;
  price1: number | null;
  buying_price: number | null;
  tax_rate: number | null;
  price_tl: number | null;
};
type CatalogProduct = { sku: string; name: string; facts: CatalogFacts };
type CatalogSnapshot = { revision: number; products: CatalogProduct[] };

const scopeKey = () => JSON.stringify([teamProject.value, teamUser.value, teamWorkspace.value, "catalog"]);
const cache = <T>(scope: string, request: object) => invoke<T>("team_cache", { scope, request });
const catalogRevision = (client: SupabaseClient, workspace: string) =>
  client.from("team_catalog_state").select("revision").eq("workspace_id", workspace).maybeSingle();
let inFlight: { scope: string; promise: Promise<CatalogSnapshot | null> } | undefined;

export async function readOnlineTeamCatalogRevision(): Promise<number | null> {
  const scope = scopeKey();
  const client = teamClient.value;
  const workspace = teamWorkspace.value;
  if (!client || !workspace || !teamSession.value || teamReachable.value !== true || teamAccessIssue.value) return null;
  const result = await catalogRevision(client, workspace).retry(false);
  if (scope !== scopeKey() || client !== teamClient.value || !teamSession.value ||
      teamReachable.value !== true || teamAccessIssue.value || result.error) return null;
  const revision = result.data?.revision;
  return Number.isSafeInteger(revision) && revision > 0 ? revision : null;
}

export function refreshTeamCatalog(): Promise<CatalogSnapshot | null> {
  const scope = scopeKey();
  if (!teamProject.value || !teamUser.value || !teamWorkspace.value) return Promise.resolve(null);
  if (inFlight?.scope === scope) return inFlight.promise;
  const promise = readSnapshot(scope, teamWorkspace.value).finally(() => {
    if (inFlight?.promise === promise) inFlight = undefined;
  });
  inFlight = { scope, promise };
  return promise;
}

async function readSnapshot(scope: string, workspace: string): Promise<CatalogSnapshot | null> {
  const client = teamSession.value && (teamReachable.value ?? navigator.onLine) ? teamClient.value : null;
  const saved = await cache<CatalogSnapshot | null>(scope, { action: "read", key: "catalog" });
  if (!client) return saved;
  const revision = () => catalogRevision(client, workspace);
  const unavailable = (status: number) => status === 0 || status >= 500;
  for (let attempt = 0; attempt < 2; attempt++) {
    const before = await revision();
    if (before.error) {
      if (unavailable(before.status)) return saved;
      throw new Error("Ortak kataloğa erişilemiyor.");
    }
    if (!before.data) return null;
    if (before.data.revision === saved?.revision) return saved;
    const products: CatalogProduct[] = [];
    const pageSize = 100;
    for (let offset = 0; ; offset += pageSize) {
      const page = await client.from("team_catalog").select("sku,name,facts").eq("workspace_id", workspace)
        .eq("active", true).order("sku").range(offset, offset + pageSize - 1);
      if (page.error) {
        if (unavailable(page.status)) return saved;
        throw new Error("Ortak katalog ürünleri okunamadı.");
      }
      products.push(...(page.data as unknown as CatalogProduct[]));
      if (page.data.length < pageSize) break;
    }
    const after = await revision();
    if (after.error) {
      if (unavailable(after.status)) return saved;
      throw new Error("Ortak katalog sürümü doğrulanamadı.");
    }
    if (after.data?.revision !== before.data.revision) continue;
    if (scope !== scopeKey()) return null;
    const snapshot = { revision: before.data.revision as number, products };
    await cache(scope, { action: "write", key: "catalog", data: snapshot });
    return snapshot;
  }
  throw new Error("Katalog güncelleniyor. Ürün aramasını yeniden deneyin.");
}

async function availableCatalog() {
  const scope = scopeKey();
  if (teamAccessIssue.value) throw new Error("Ortak katalog için çalışma alanına erişim gerekli.");
  const catalog = await refreshTeamCatalog();
  if (!catalog || scope !== scopeKey()) throw new Error("Bu çalışma alanındaki ortak katalog henüz alınamadı.");
  return catalog;
}

export async function listTeamProducts() {
  const scope = scopeKey();
  if (teamAccessIssue.value) throw new Error("Ortak katalog için çalışma alanına erişim gerekli.");
  const snapshot = await refreshTeamCatalog();
  if (scope !== scopeKey()) throw new Error("Çalışma alanı değişti.");
  if (!snapshot) return null;
  const seo = await listTeamProductSeo();
  const rows = await invoke<ProductRow[]>("list_team_products", { products: snapshot.products, seoRecords: [...seo.values()] });
  return { rows, revision: snapshot.revision };
}

export async function getTeamProduct(sku: string, knownSeo?: TeamSeoRecord) {
  const scope = teamProductSeoScope();
  const catalog = await availableCatalog();
  const product = catalog.products.find((row) => row.sku === sku);
  if (!product) throw new Error("Ürün ortak katalogda bulunamadı.");
  const [seo, lastPush] = await Promise.all([
    knownSeo ?? getTeamProductSeo(sku),
    lastTeamIdeasoftPush(scope, sku).catch(() => null),
  ]);
  const detail = await invoke<ProductDetail>("get_team_product", { product, seoRecords: seo ? [seo] : [] });
  if (scope !== teamProductSeoScope()) throw new Error("Çalışma alanı değişti.");
  return { ...detail, ideasoft_pushed_at: lastPush ?? detail.ideasoft_pushed_at, team_seo_scope: scope,
    team_catalog_revision: catalog.revision };
}

export async function getTeamProductJsonld(sku: string, scope: string) {
  if (scope !== teamProductSeoScope()) throw new Error("Çalışma alanı değişti.");
  const [catalog, seo] = await Promise.all([availableCatalog(), getTeamProductSeo(sku)]);
  const product = catalog.products.find((row) => row.sku === sku);
  if (!product) throw new Error("Ürün ortak katalogda bulunamadı.");
  if (scope !== teamProductSeoScope() || teamAccessIssue.value) throw new Error("Çalışma alanı değişti.");
  return invoke<string>("get_team_jsonld", { product, seoRecords: seo ? [seo] : [] });
}

async function lastTeamIdeasoftPush(scope: string, sku: string): Promise<string | null> {
  const slot = `ideasoft-last-push:${sku}`;
  const client = teamSession.value && (teamReachable.value ?? navigator.onLine) ? teamClient.value : null;
  if (client && teamWorkspace.value && !teamAccessIssue.value) {
    const result = await client.from("team_seo_work_events").select("occurred_at")
      .eq("workspace_id", teamWorkspace.value).eq("sku", sku).eq("kind", "ideasoft_push").eq("reaches_store", true)
      .order("occurred_at", { ascending: false }).limit(1).maybeSingle().retry(false);
    if (scope !== teamProductSeoScope() || teamAccessIssue.value) return null;
    if (!result.error) {
      const at = result.data?.occurred_at as string | undefined;
      await cache(scope, { action: "write", key: slot, data: at ?? null });
      return at ?? null;
    }
    if (result.status === 401 || result.status === 403) return null;
  }
  if (scope !== teamProductSeoScope() || teamAccessIssue.value) return null;
  return cache<string | null>(scope, { action: "read", key: slot });
}

export async function searchTeamCatalog(term: string): Promise<CatalogMatch[]> {
  const query = term.trim().toLocaleLowerCase("tr");
  if (query.length < 3) return [];
  const { products } = await availableCatalog();
  return products.filter((item) => [item.name, item.sku].some((value) => value.toLocaleLowerCase("tr").includes(query)))
    .slice(0, 15).map(({ sku, name, facts }) => ({
      sku, name, slug: sku, id: Number(facts.id) || 0, stock: facts.quantity ?? 0,
      status: Number(facts.status) || 0, canonical: facts.url ?? "",
    }));
}

export async function prepareTeamCatalogLine(sku: string, currency: string, fx: number | null) {
  const product = (await availableCatalog()).products.find((item) => item.sku === sku);
  if (!product) throw new Error("Ürün ortak katalogda bulunamadı.");
  return invoke<Omit<import("./types").QuoteItem, "id" | "net" | "margin">>(
    "prepare_team_catalog_line", { sku, currency, fx, product: { ...product.facts, name: product.name } },
  );
}

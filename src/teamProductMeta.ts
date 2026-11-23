import { invoke } from "@tauri-apps/api/core";
import { getTeamFunctionSession } from "./teamConnection";
import type { TeamSeoRecord } from "./teamProductSeo";
import type { ProductDetail } from "./types";

export function generateTeamProductMeta(product: ProductDetail, record: TeamSeoRecord | null) {
  return invoke<Record<string, unknown>>("generate_shared_product_meta", {
    product: { name: product.name, brand: product.brand, category: product.category, main_category: product.main_category },
    draft: record?.draft ?? {},
    session: getTeamFunctionSession(),
  });
}

export function restoreTeamProductMeta(record: TeamSeoRecord, index: number) {
  return invoke<Record<string, unknown>>("restore_shared_product_meta", { draft: record.draft, index });
}

import { invoke } from "@tauri-apps/api/core";
import { getTeamFunctionSession } from "./teamConnection";
import type { TeamSeoRecord } from "./teamProductSeo";
import type { ProductDetail } from "./types";

export function generateTeamProductDetails(product: ProductDetail, record: TeamSeoRecord | null) {
  return invoke<Record<string, unknown>>("generate_shared_details", {
    product: { name: product.name, brand: product.brand, category: product.category, main_category: product.main_category },
    content: { html: product.details, gallery: product.gallery },
    draft: record?.draft ?? {},
    session: getTeamFunctionSession(),
  });
}

export function restoreTeamProductDetails(record: TeamSeoRecord, index: number) {
  return invoke<Record<string, unknown>>("restore_shared_details_version", { draft: record.draft, index });
}

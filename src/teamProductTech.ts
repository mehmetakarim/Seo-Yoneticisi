import { invoke } from "@tauri-apps/api/core";
import { getTeamFunctionSession } from "./teamConnection";
import type { TeamSeoRecord } from "./teamProductSeo";
import type { ProductDetail, TechGroup, TechSpecsResult } from "./types";

export function teamTechSourceChanges(record: TeamSeoRecord | null, text: string) {
  const draft = record?.draft;
  if (Array.isArray(draft?.tech_specs_json) && draft.tech_specs_json.length && draft.tech_table_source_text == null) {
    return { tech_source_text: text, tech_table_source_text: draft.tech_source_text ?? "" };
  }
  return { tech_source_text: text };
}

export function structureTeamProductTech(product: ProductDetail, record: TeamSeoRecord) {
  return invoke<{ result: TechSpecsResult; changes: Record<string, unknown> }>("structure_shared_tech_specs", {
    product: { name: product.name, brand: product.brand, category: product.category, main_category: product.main_category },
    draft: record.draft,
    session: getTeamFunctionSession(),
  });
}

export function restoreTeamProductTech(record: TeamSeoRecord, index: number) {
  return invoke<Record<string, unknown>>("restore_shared_tech_version", { draft: record.draft, index });
}

export function assembleTeamProductTech(groups: TechGroup[]) {
  return invoke<string>("shared_tech_table_html", { groups });
}

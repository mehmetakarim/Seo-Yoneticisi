export const storeKinds = ["category", "brand", "blog"] as const;
export type StoreKind = typeof storeKinds[number];
export type RemoteStorePage = {
  kind: StoreKind; id: number; name: string; slug: string; status: number;
  page_title: string; meta_description: string; meta_keywords?: string; target_keyword: string; showcase_content: string | null;
};
type Batch = { pages?: RemoteStorePage[]; has_more?: boolean };

/** Finish every page before making a new inventory available for publication. */
export async function readIdeasoftInventory(
  read: (kind: StoreKind, page: number) => Promise<Batch>,
  current: () => boolean,
  wait: () => Promise<void> = () => new Promise((resolve) => setTimeout(resolve, 1600)),
): Promise<RemoteStorePage[]> {
  const rows: RemoteStorePage[] = [];
  const seen = new Set<string>();
  for (const kind of storeKinds) {
    for (let page = 1; ; page++) {
      if (!current()) throw new Error("Çalışma alanı değişti.");
      const result = await read(kind, page);
      if (!current()) throw new Error("Çalışma alanı değişti.");
      if (!Array.isArray(result.pages) || typeof result.has_more !== "boolean" || result.pages.length > 100 ||
          result.has_more !== (result.pages.length === 100)) throw new Error("Mağaza envanteri yanıtı doğrulanamadı.");
      for (const row of result.pages) {
        const id = `${kind}:${row.id}`;
        if (row.kind !== kind || !Number.isSafeInteger(row.id) || row.id <= 0 || seen.has(id) ||
            ![0, 1].includes(row.status) || typeof row.name !== "string" || !row.name.trim() ||
            [row.slug, row.page_title, row.meta_description, row.target_keyword].some((v) => typeof v !== "string") ||
            (row.showcase_content !== null && typeof row.showcase_content !== "string")) {
          throw new Error("Mağaza envanterinde geçersiz veya yinelenen kayıt var. Önceki envanter korundu.");
        }
        seen.add(id);
        rows.push(row);
        if (rows.length > 5000) throw new Error("Envanter 5000 kayıt sınırını aşıyor. Önceki envanter korundu.");
      }
      if (!result.has_more) break;
      await wait();
    }
    if (kind !== "blog") await wait();
  }
  return rows;
}

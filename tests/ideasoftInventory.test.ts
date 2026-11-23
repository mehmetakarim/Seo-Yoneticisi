import { test } from "node:test";
import assert from "node:assert/strict";
import { readIdeasoftInventory, type RemoteStorePage, type StoreKind } from "../src/ideasoftInventory.ts";

const row = (kind: StoreKind, id: number): RemoteStorePage => ({ kind, id, name: `Page ${id}`, slug: `page-${id}`, status: 0,
  page_title: "Title", meta_description: "Description", target_keyword: "", showcase_content: null });
const noWait = async () => {};

test("all categories, brands and blogs are read before returning inventory", async () => {
  const calls: string[] = [];
  const rows = await readIdeasoftInventory(async (kind, page) => {
    calls.push(`${kind}:${page}`);
    if (kind === "brand" && page === 1) return { pages: Array.from({ length: 100 }, (_, i) => row(kind, i + 1)), has_more: true };
    return { pages: [row(kind, kind === "brand" ? 101 : 1)], has_more: false };
  }, () => true, noWait);
  assert.deepEqual(calls, ["category:1", "brand:1", "brand:2", "blog:1"]);
  assert.equal(rows.length, 103);
});

test("a partial upstream failure rejects the entire new inventory", async () => {
  await assert.rejects(readIdeasoftInventory(async (kind) => {
    if (kind === "brand") throw new Error("upstream unavailable");
    return { pages: [row(kind, 1)], has_more: false };
  }, () => true, noWait), /upstream unavailable/);
});

test("workspace changes discard the response and stop further requests", async () => {
  let current = true, calls = 0;
  await assert.rejects(readIdeasoftInventory(async (kind) => {
    calls++; current = false;
    return { pages: [row(kind, 1)], has_more: false };
  }, () => current, noWait), /Çalışma alanı değişti/);
  assert.equal(calls, 1);
});

test("duplicates, invalid status and malformed rows cannot replace the inventory", async () => {
  for (const pages of [[row("category", 1), row("category", 1)], [{ ...row("category", 1), status: 3 }], [{ ...row("category", 1), name: "" }]]) {
    await assert.rejects(readIdeasoftInventory(async () => ({ pages, has_more: false }), () => true, noWait), /geçersiz veya yinelenen/);
  }
});

test("inconsistent paging fails instead of silently losing rows", async () => {
  await assert.rejects(readIdeasoftInventory(async () => ({ pages: [], has_more: true }), () => true, noWait), /doğrulanamadı/);
});

test("an inventory beyond the server limit is rejected before publication", async () => {
  await assert.rejects(readIdeasoftInventory(async (kind, page) => ({
    pages: Array.from({ length: 100 }, (_, i) => row(kind, (page - 1) * 100 + i + 1)), has_more: true,
  }), () => true, noWait), /5000 kayıt sınırını aşıyor/);
});

test("exactly 5000 rows are accepted after checking the terminating empty page", async () => {
  const rows = await readIdeasoftInventory(async (kind, page) => {
    if (kind !== "category" || page === 51) return { pages: [], has_more: false };
    return { pages: Array.from({ length: 100 }, (_, i) => row(kind, (page - 1) * 100 + i + 1)), has_more: true };
  }, () => true, noWait);
  assert.equal(rows.length, 5000);
});

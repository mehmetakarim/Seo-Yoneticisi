import assert from "node:assert/strict";
import { test } from "node:test";
import { quoteActivity, sortQuotesByActivity } from "../src/quoteOrder.ts";

test("orders all quote statuses by the latest valid activity time", () => {
  const rows = [
    { id: "draft", status: "draft", created_at: "2026-11-24T07:00:00Z", updated_at: "2026-11-24T11:00:00+03:00", sent_at: null },
    { id: "sent", status: "sent", created_at: "2026-11-22T12:00:00Z", updated_at: "", sent_at: "2026-11-24 09:30:00+00" },
    { id: "won", status: "won", created_at: "2026-11-22T08:00:00Z", updated_at: "2026-11-24T11:00:00Z", sent_at: "2026-11-23T08:00:00Z" },
    { id: "older", status: "lost", created_at: "2026-11-24T07:00:00", sent_at: null },
  ];
  assert.deepEqual(sortQuotesByActivity(rows).map(({ id }) => id), ["won", "sent", "draft", "older"]);
  assert.equal(rows[0].id, "draft");

  rows[0] = { ...rows[0], updated_at: "2026-11-24T12:00:00.125Z" };
  assert.equal(sortQuotesByActivity(rows)[0].id, "draft");
});

test("distinguishes sub-millisecond timestamps and gives legacy ties a stable order", () => {
  const one = { id: "a", created_at: "invalid", sent_at: null, updated_at: "2026-11-24T10:00:00.000001+00:00" };
  const two = { ...one, id: "z", updated_at: "2026-11-24T10:00:00.000002Z" };
  assert.deepEqual(sortQuotesByActivity([one, two]).map(({ id }) => id), ["z", "a"]);
  const legacy = { created_at: "1970-01-01", sent_at: null };
  assert.equal(quoteActivity({ ...legacy, id: 1 }), 0);
  assert.deepEqual(sortQuotesByActivity([{ ...legacy, id: 1 }, { ...legacy, id: 10 }]).map(({ id }) => id), [10, 1]);
  assert.deepEqual(sortQuotesByActivity([{ ...one, id: "b" }, one]).map(({ id }) => id), ["a", "b"]);
});

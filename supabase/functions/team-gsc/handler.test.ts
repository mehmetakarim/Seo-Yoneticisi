import { test } from "node:test";
import assert from "node:assert/strict";
import { handleGsc } from "./handler.ts";

const workspace = "11111111-1111-4111-8111-111111111111";
const user = "22222222-2222-4222-8222-222222222222";
const property = "https://store.example.test/";
const project = "https://project.example.test";
const day = (daysAgo: number) => new Date(Date.now() - daysAgo * 86400000).toISOString().slice(0, 10);
const snapshot = { workspace, mode: "snapshot", start: day(50), end: day(22) };
const queryRequest = (body: object) => new Request(project + "/functions/v1/team-gsc", {
  method: "POST",
  headers: { Authorization: "Bearer user-session" },
  body: JSON.stringify(body),
});

test("only editors and owners can request an exact, finalized historical snapshot", async () => {
  const key = await crypto.subtle.generateKey({
    name: "RSASSA-PKCS1-v1_5", modulusLength: 2048, publicExponent: new Uint8Array([1, 0, 1]), hash: "SHA-256",
  }, true, ["sign", "verify"]);
  const privateKey = Buffer.from(await crypto.subtle.exportKey("pkcs8", key.privateKey)).toString("base64");
  const env = (name: string) => ({
    SUPABASE_URL: project,
    SUPABASE_ANON_KEY: "publishable-key",
    TEAM_GSC_11111111_1111_4111_8111_111111111111: JSON.stringify({
      property, client_email: "gsc@example.test", private_key: privateKey,
    }),
  })[name];

  for (const role of ["viewer", "editor", "owner"]) {
    const calls: string[] = [];
    const fetchFixture: typeof fetch = async (input, init) => {
      const address = new URL(String(input));
      calls.push(address.origin);
      if (address.pathname === "/auth/v1/user") return Response.json({ id: user });
      if (address.pathname === "/rest/v1/workspace_members") {
        assert.equal(address.searchParams.get("workspace_id"), `eq.${workspace}`);
        return Response.json([{ role }]);
      }
      if (address.origin === "https://oauth2.googleapis.com") return Response.json({ access_token: "test-google-token" });
      assert.equal(address.pathname, `/webmasters/v3/sites/${encodeURIComponent(property)}/searchAnalytics/query`);
      assert.deepEqual(JSON.parse(String(init?.body)), {
        startDate: snapshot.start, endDate: snapshot.end, rowLimit: 10000, dataState: "final", dimensions: ["page"], startRow: 0,
      });
      return Response.json({ rows: [{
        keys: [property + "sample"], clicks: 4, impressions: 50, ctr: 0.08, position: 6,
      }] });
    };
    const response = await handleGsc(queryRequest(snapshot), env, fetchFixture);
    if (role === "viewer") {
      assert.equal(response.status, 403);
      assert.deepEqual(calls, [project, project]);
    } else {
      assert.equal(response.status, 200);
      assert.deepEqual(await response.json(), {
        configured: true, rows: [{ page: property + "sample", clicks: 4, impressions: 50, ctr: 0.08, position: 6 }], next: null,
      });
    }
  }
});

test("an incomplete or wrongly sized historical window is rejected before contacting a service", async () => {
  const env = () => "set";
  const noNetwork: typeof fetch = async () => { assert.fail("Invalid windows must not reach an external service"); };
  for (const window of [{ start: day(30), end: day(2) }, { start: day(51), end: day(22) }]) {
    const response = await handleGsc(queryRequest({ ...snapshot, ...window }), env, noNetwork);
    assert.equal(response.status, 400);
    assert.deepEqual(await response.json(), { error: "invalid_window" });
  }
});

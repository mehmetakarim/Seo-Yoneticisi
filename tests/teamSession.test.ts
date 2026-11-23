import assert from "node:assert/strict";
import { test, type TestContext } from "node:test";
import { nextTick } from "vue";

const project = "https://iwtygnaqgjcmnmygjequ.supabase.co";
const member = "11111111-1111-4111-8111-111111111111";
const workspace = "22222222-2222-4222-8222-222222222222";
const deviceProfile = "33333333-3333-4333-8333-333333333333";
const encode = (value: unknown) => Buffer.from(JSON.stringify(value)).toString("base64url");
const accessToken = `${encode({ alg: "HS256", typ: "JWT" })}.${encode({ exp: 4_102_444_800, sub: member })}.c2ln`;
const user = { id: member, email: "test@example.com", aud: "authenticated", app_metadata: {}, user_metadata: {} };
const supabaseSession = {
  user, access_token: accessToken, refresh_token: "rotated-test-refresh-token",
  expires_at: 4_102_444_800, expires_in: 3600, token_type: "bearer",
};
const savedSession = {
  url: project, key: "old-publishable-key", accessToken, refreshToken: "test-refresh-token", user: member, workspace,
};

let moduleId = 0;

async function newApp() {
  return import(`../src/teamConnection.ts?test=${++moduleId}`) as Promise<typeof import("../src/teamConnection.ts")>;
}

function device(t: TestContext, saved?: typeof savedSession) {
  let stored = saved ? JSON.stringify(saved) : null;
  const state = { onLine: true, member: true, rejectsPassword: false, refusesWorkspace: false };
  const requests: { url: URL; init?: RequestInit }[] = [];
  const globals = {
    localStorage: { getItem: (key: string) => key === "seo-team-session-profile" ? deviceProfile : null },
    navigator: state,
    window: {
      __TAURI_INTERNALS__: {
        async invoke(command: string, args: { profile: string; value: string | null }) {
          assert.equal(args.profile, deviceProfile);
          if (command === "read_team_session") return stored;
          assert.equal(command, "write_team_session");
          stored = args.value;
          return true;
        },
      },
    },
    async fetch(input: RequestInfo | URL, init?: RequestInit) {
      const url = new URL(input instanceof Request ? input.url : String(input));
      requests.push({ url, init });
      assert.equal(url.origin, project);
      if (url.pathname === "/auth/v1/token") {
        if (state.rejectsPassword) return Response.json({ code: "invalid_credentials", msg: "Invalid login credentials" }, {
          status: 400, headers: { "x-supabase-api-version": "2024-01-01" },
        });
        return Response.json(supabaseSession);
      }
      if (url.pathname === "/auth/v1/user") return Response.json(user);
      if (url.pathname === "/auth/v1/logout") return new Response(null, { status: 204 });
      assert.equal(url.pathname, "/rest/v1/workspaces");
      if (state.refusesWorkspace) return Response.json({ message: "Access denied" }, { status: 403 });
      return Response.json(state.member ? [{ id: workspace }] : []);
    },
  };
  for (const [key, value] of Object.entries(globals)) {
    const descriptor = Object.getOwnPropertyDescriptor(globalThis, key);
    Object.defineProperty(globalThis, key, { configurable: true, value });
    t.after(() => {
      if (descriptor) Object.defineProperty(globalThis, key, descriptor);
      else Reflect.deleteProperty(globalThis, key);
    });
  }
  return { state, requests, stored: () => stored && JSON.parse(stored) as typeof savedSession };
}

test("empty or other-project storage does not unlock the app or contact Supabase", async (t) => {
  const fixture = device(t, { ...savedSession, url: "https://another-project.example" });
  const app = await newApp();
  await app.restoreTeamSession();
  assert.equal(app.teamUser.value, "");
  assert.equal(app.teamWorkspace.value, "");
  assert.equal(app.teamProject.value, "");
  assert.equal(fixture.requests.length, 0);
  assert.ok(fixture.stored());
});

test("sign-in uses only the credentials and restores the protected session after reopening", async (t) => {
  const fixture = device(t);
  const app = await newApp();
  await app.restoreTeamSession();
  await app.connectTeam(" test@example.com ", "test-only-password");
  await nextTick();
  assert.equal(app.teamUser.value, member);
  assert.equal(app.teamWorkspace.value, workspace);
  const passwordLogins = () => fixture.requests.filter(({ url }) => url.searchParams.get("grant_type") === "password");
  assert.equal(passwordLogins().length, 1);
  assert.deepEqual(JSON.parse(String(passwordLogins()[0].init?.body)), {
    email: user.email, password: "test-only-password", gotrue_meta_security: {},
  });
  assert.equal(fixture.stored()?.refreshToken, supabaseSession.refresh_token);
  assert.equal(fixture.stored()?.workspace, workspace);
  await app.teamClient.value?.auth.stopAutoRefresh();

  const reopened = await newApp();
  await reopened.restoreTeamSession();
  assert.equal(reopened.teamUser.value, member);
  assert.equal(reopened.teamWorkspace.value, workspace);
  assert.equal(passwordLogins().length, 1);
  await reopened.disconnectTeam();
  assert.equal(fixture.stored(), null);
  assert.equal(reopened.teamProject.value, "");
  await reopened.restoreTeamSession();
  assert.equal(reopened.teamUser.value, "");
});

test("offline start restores only the remembered workspace, then reconnects without a password", async (t) => {
  const fixture = device(t, savedSession);
  fixture.state.onLine = false;
  const app = await newApp();
  await app.restoreTeamSession();
  assert.equal(app.teamUser.value, member);
  assert.equal(app.teamWorkspace.value, workspace);
  assert.equal(app.teamReachable.value, false);
  assert.equal(app.teamSession.value, null);
  assert.equal(fixture.requests.length, 0);

  fixture.state.onLine = true;
  await app.restoreTeamSession();
  t.after(() => app.teamClient.value?.auth.stopAutoRefresh());
  assert.equal(app.teamSession.value?.user.id, member);
  assert.equal(app.teamReachable.value, true);
  assert.ok(fixture.stored()?.key.startsWith("sb_publishable_"));
  assert.equal(fixture.requests.some(({ url }) => url.searchParams.get("grant_type") === "password"), false);
});

test("an authenticated account without workspace access stays locked and can retry", async (t) => {
  const fixture = device(t);
  fixture.state.member = false;
  const app = await newApp();
  await app.connectTeam(user.email, "test-only-password");
  t.after(() => app.teamClient.value?.auth.stopAutoRefresh());
  assert.equal(app.teamUser.value, member);
  assert.equal(app.teamWorkspace.value, "");
  assert.match(app.teamWorkspaceError.value, /üyeliği yok/);

  fixture.state.member = true;
  await app.resolveTeamWorkspace();
  assert.equal(app.teamWorkspace.value, workspace);
  assert.equal(app.teamWorkspaceError.value, "");
});

test("workspace access refusal cannot be converted into offline access by a connection drop", async (t) => {
  const fixture = device(t, savedSession);
  fixture.state.refusesWorkspace = true;
  const app = await newApp();
  await app.restoreTeamSession();
  t.after(() => app.teamClient.value?.auth.stopAutoRefresh());
  assert.equal(app.teamAccessIssue.value, true);
  app.markTeamDisconnected();
  assert.equal(app.teamReachable.value, false);
  assert.equal(app.teamAccessIssue.value, true);
});

test("incorrect credentials give an actionable error and do not create a saved session", async (t) => {
  const fixture = device(t);
  fixture.state.rejectsPassword = true;
  const app = await newApp();
  await assert.rejects(app.connectTeam(user.email, "wrong-test-password"), /E-posta adresi veya parola hatalı/);
  assert.equal(app.teamUser.value, "");
  assert.equal(fixture.stored(), null);
});

import { invoke } from "@tauri-apps/api/core";
import { shallowRef } from "vue";
import { refreshTeamCatalog } from "./teamCatalog";
import { teamAccessIssue, teamClient, teamProject, teamReachable, teamSession, teamUser, teamWorkspace } from "./teamConnection";
import { teamContacts } from "./teamContacts";
import { teamGscMode, teamOutcomeInputs } from "./teamGsc";
import { loadGscReport } from "./teamGscReport";
import { storeKinds } from "./ideasoftInventory";
import { listTeamProductSeo } from "./teamProductSeo";
import { listTeamStorePages } from "./teamStorePages";
import type { Contact, ItemRef, OpportunityReport, StorePageDetail } from "./types";

type Kind = ItemRef["kind"];
type Action = "done" | "dismiss" | "snooze" | "restore";
type Decision = {
  id: string;
  kind: Kind;
  reference: string;
  revision: number;
  action: Action;
  until_day: string | null;
  occurred_at: string;
  analyzed_at: string | null;
};
type ContactForQueue = Pick<Contact, "revision" | "name" | "company" | "last_contact_at" | "next_step_at" | "next_step_note" | "archived"> & { id: string };
type TeamProduct = { sku: string; name: string; meta_done: boolean; details_done: boolean; pushed_at: string | null };
type StoreEvent = NonNullable<Awaited<ReturnType<typeof teamOutcomeInputs>>>["events"][number];
export type TeamTodayInputs = {
  focus_scope?: string;
  report: OpportunityReport | null;
  products: TeamProduct[];
  events: StoreEvent[];
  pages: Pick<StorePageDetail, "kind" | "slug" | "remote_id" | "status">[];
  contacts: ContactForQueue[];
  decisions: Decision[];
};
type DecisionRequest = {
  target_workspace: string;
  operation_id: string;
  target_kind: Kind;
  target_reference: string;
  expected_revision: number | null;
  decision: Action;
  until_day: string | null;
  analysis_marker: string | null;
  acted_at: string;
  record_store_change: boolean;
  contact_revision: number | null;
};
type Pending = { id: string; contact: Decision; payload: DecisionRequest; conflict: string | null };
type Conflict = { id: string; kind: Kind; reference: string; reason: string };

export const teamTodayMode = teamGscMode;
export const teamTodayScope = () => teamWorkspace.value && teamUser.value && teamProject.value && !teamAccessIssue.value
  ? JSON.stringify([teamProject.value, teamUser.value, teamWorkspace.value, "today"]) : undefined;
export const todayLocalDay = (date = new Date()) =>
  [date.getFullYear(), String(date.getMonth() + 1).padStart(2, "0"), String(date.getDate()).padStart(2, "0")].join("-");
const cache = <T>(scope: string, request: object) => invoke<T>("team_cache", { scope, request });
const pending = (scope: string) => cache<Pending[]>(scope, { action: "pending" });
const keyOf = (kind: Kind, reference: string) => JSON.stringify([kind, reference]);
const transport = (status: number) => status === 0 || status === 429 || status >= 500;
const editable = (role?: string) => role === "owner" || role === "editor";
export const teamTodaySync = shallowRef<{ scope?: string; pending: number; conflicts: Conflict[]; canEdit: boolean }>({
  pending: 0, conflicts: [], canEdit: false,
});
const operations = new Map<string, Promise<unknown>>();
const flushes = new Map<string, Promise<boolean>>();

function current(scope: string) {
  if (scope !== teamTodayScope()) throw new Error("Çalışma alanı değişti.");
}

function serialize<T>(scope: string, run: () => Promise<T>) {
  const task = (operations.get(scope) ?? Promise.resolve()).catch(() => {}).then(run);
  operations.set(scope, task);
  void task.finally(() => { if (operations.get(scope) === task) operations.delete(scope); }).catch(() => {});
  return task;
}

function context(scope: string) {
  current(scope);
  const client = teamSession.value && (teamReachable.value ?? navigator.onLine) ? teamClient.value : null;
  return { client, workspace: teamWorkspace.value, user: teamUser.value };
}

async function readDecisions(scope: string, refresh = false) {
  return serialize(scope, async () => {
    const { client, workspace, user } = context(scope);
    let canEdit = (await cache<boolean | null>(scope, { action: "read", key: "can-edit" })) ?? false;
    if (refresh && client) {
      const member = await client.from("workspace_members").select("role").eq("workspace_id", workspace)
        .eq("user_id", user).maybeSingle().retry(false);
      current(scope);
      if (!member.error && member.data) {
        canEdit = editable(member.data.role);
        await cache(scope, { action: "write", key: "can-edit", data: canEdit });
        const rows: Decision[] = [];
        for (let offset = 0; ; offset += 200) {
          const page = await client.from("team_today_decisions")
            .select("kind,reference,revision,action,until_day,occurred_at,analyzed_at")
            .eq("workspace_id", workspace).order("kind").order("reference").range(offset, offset + 199).retry(false);
          current(scope);
          if (page.error) {
            if (transport(page.status)) break;
            throw new Error("Ortak iş kuyruğu Supabase’den okunamadı.");
          }
          rows.push(...page.data.map((row) => ({ ...row, id: keyOf(row.kind, row.reference) } as Decision)));
          if (page.data.length < 200) {
            await cache(scope, { action: "snapshot", contacts: rows });
            break;
          }
        }
      } else if (!transport(member.status)) {
        await cache(scope, { action: "write", key: "can-edit", data: false });
        throw new Error("Bu çalışma alanının iş kuyruğuna erişilemiyor.");
      }
    }
    return decisionOverlay(scope, canEdit);
  });
}

async function decisionOverlay(scope: string, canEdit?: boolean) {
  const [confirmed, waiting] = await Promise.all([
    cache<Decision[]>(scope, { action: "contacts" }),
    pending(scope),
  ]);
  current(scope);
  const latest = new Map(confirmed.map((row) => [row.id, row]));
  const conflicts = new Map<string, Conflict>();
  for (const op of waiting) {
    const row = op.contact;
    if (op.conflict) {
      conflicts.set(row.id, { id: row.id, kind: row.kind, reference: row.reference, reason: op.conflict });
    } else if (!conflicts.has(row.id)) {
      const server = latest.get(row.id);
      if (!server || server.revision < row.revision) latest.set(row.id, row);
    }
  }
  teamTodaySync.value = {
    scope, canEdit: canEdit ?? (teamTodaySync.value.scope === scope && teamTodaySync.value.canEdit),
    pending: waiting.filter((op) => !conflicts.has(op.contact.id)).length,
    conflicts: [...conflicts.values()],
  };
  return [...latest.values()];
}

export async function loadTeamTodayInputs(scope: string): Promise<TeamTodayInputs> {
  current(scope);
  const [report, catalog, seo, outcomes, contacts, pages, decisions] = await Promise.all([
    loadGscReport(), refreshTeamCatalog(), listTeamProductSeo(), teamOutcomeInputs(),
    teamContacts.list("", false, "", ""), Promise.all(storeKinds.map(listTeamStorePages)), readDecisions(scope, true),
  ]);
  current(scope);
  return {
    focus_scope: scope,
    report,
    products: (catalog?.products ?? []).map(({ sku, name }) => {
      const draft = seo.get(sku)?.draft;
      return {
        sku, name, meta_done: draft?.meta_status === "done", details_done: draft?.details_status === "done",
        pushed_at: typeof draft?.ideasoft_pushed_at === "string" ? draft.ideasoft_pushed_at : null,
      };
    }),
    events: outcomes?.events ?? [],
    pages: pages.flat().map(({ kind, slug, remote_id, status }) => ({ kind, slug, remote_id, status })),
    contacts: contacts.flatMap(({ id, revision, name, company, last_contact_at, next_step_at, next_step_note, archived }) =>
      typeof id === "string" ? [{ id, revision, name, company, last_contact_at, next_step_at, next_step_note, archived }] : []),
    decisions,
  };
}

export async function queueTodayDecision(
  scope: string, inputs: TeamTodayInputs, kind: Kind, reference: string, action: Action, untilDay: string | null = null,
  recordStoreChange = false,
) {
  return serialize(scope, async () => {
    const { workspace } = context(scope);
    if (!await cache<boolean>(scope, { action: "read", key: "can-edit" })) {
      throw new Error("İş kuyruğu düzenleme yetkisi bu cihazda doğrulanamadı.");
    }
    current(scope);
    const id = keyOf(kind, reference);
    const previous = inputs.decisions.find((row) => row.id === id);
    const actual = (await decisionOverlay(scope)).find((row) => row.id === id);
    if (teamTodaySync.value.conflicts.some((conflict) => conflict.id === id)) {
      throw new Error("Önce bu işin eşitleme çakışmasını çözün.");
    }
    if (actual?.revision !== previous?.revision) throw new Error("İş durumu değişti. Güncel kuyruktan yeniden deneyin.");
    if (actual?.action === action && actual.until_day === untilDay && action !== "done") return;
    if (action === "done" && actual?.action === "done" &&
        actual.analyzed_at === (inputs.report?.analyzed_at ?? "") &&
        todayLocalDay(new Date(/Z$|[+-]\d\d:\d\d$/.test(actual.occurred_at) ? actual.occurred_at : actual.occurred_at + "Z")) === todayLocalDay()) return;
    const contactRevision = kind === "contact" && action === "done"
      ? inputs.contacts.find((contact) => contact.id === reference)?.revision : null;
    if (kind === "contact" && action === "done" && contactRevision == null) {
      throw new Error("Kişinin ortak sürümünü görmek için Bugün ekranını yenileyin.");
    }
    const operationId = crypto.randomUUID();
    const actedAt = new Date().toISOString();
    const analysisMarker = action === "done" ? inputs.report?.analyzed_at ?? "" : null;
    const payload: DecisionRequest = {
      target_workspace: workspace, operation_id: operationId, target_kind: kind, target_reference: reference,
      expected_revision: actual?.revision ?? null, decision: action, until_day: untilDay,
      analysis_marker: analysisMarker, acted_at: actedAt, contact_revision: contactRevision ?? null,
      record_store_change: recordStoreChange,
    };
    const projected: Decision = {
      id, kind, reference, revision: (actual?.revision ?? 0) + 1, action, until_day: untilDay,
      analyzed_at: analysisMarker, occurred_at: actedAt.slice(0, -1),
    };
    await cache(scope, { action: "queue", id: operationId, contact: projected, payload });
    await decisionOverlay(scope);
  });
}

export function refreshTodayDecisions(scope: string, fromServer = false) {
  return readDecisions(scope, fromServer);
}

export function flushTeamToday(): Promise<boolean> {
  const scope = teamTodayScope();
  if (!scope) return Promise.resolve(false);
  const running = flushes.get(scope);
  if (running) return running;
  const { client } = context(scope);
  if (!client) return Promise.resolve(false);
  const task = serialize(scope, async () => {
    let changed = false;
    const blocked = new Set<string>();
    for (const op of await pending(scope)) {
      if (scope !== teamTodayScope()) return changed;
      if (op.conflict) blocked.add(op.contact.id);
      if (blocked.has(op.contact.id)) continue;
      const result = await client.rpc("team_sync_today_decision", op.payload).retry(false);
      if (result.error) {
        if (transport(result.status) || result.error.code === "PGRST202" || result.status === 401) break;
        const reason = result.error.code === "PT409"
          ? "Bu iş veya kişi başka bir oturumda güncellendi."
          : "Supabase bu iş kararını kabul etmedi.";
        await cache(scope, { action: "conflict", id: op.id, reason });
        blocked.add(op.contact.id);
        changed = true;
      } else {
        const saved = { ...result.data, id: op.contact.id } as Decision;
        if (!saved.revision || saved.kind !== op.contact.kind || saved.reference !== op.contact.reference) break;
        const known = await cache<Decision | null>(scope, { action: "read", key: `contact:${saved.id}` });
        await cache(scope, { action: "acknowledge", id: op.id, contact: known && known.revision > saved.revision ? known : saved });
        changed = true;
      }
    }
    if (scope === teamTodayScope()) await decisionOverlay(scope);
    return changed;
  }).finally(() => { flushes.delete(scope); });
  flushes.set(scope, task);
  return task;
}

export async function acceptTodayConflict(scope: string, id: string) {
  await serialize(scope, async () => {
    current(scope);
    const rows = (await pending(scope)).filter((op) => op.contact.id === id);
    if (!rows.some((op) => op.conflict)) return;
    await cache(scope, { action: "discard", ids: rows.map((op) => op.id) });
    await decisionOverlay(scope);
  });
}

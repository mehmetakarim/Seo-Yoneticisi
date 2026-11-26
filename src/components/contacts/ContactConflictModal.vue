<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, shallowRef } from "vue";
import {
  acceptTeamContactConflict, inspectTeamContactConflict, type TeamContactConflict, type TeamContactConflictArchive,
} from "../../teamContacts";
import type { Contact } from "../../types";
import Icon from "../Icon.vue";
import ModalShell from "../ModalShell.vue";

const props = defineProps<{ id: string; name: string; mode: "review" | "history"; history: TeamContactConflictArchive[] }>();
const emit = defineEmits<{ close: []; accepted: [remote: Contact] }>();
const root = ref<HTMLElement | null>(null);
const review = shallowRef<TeamContactConflict | null>(null);
const selectedArchive = ref(0);
const loading = ref(false);
const applying = ref(false);
const error = ref("");
let request: AbortController | undefined;
let app: HTMLElement | null = null;
let previousInert = false;

const isHistory = computed(() => props.mode === "history");
const title = computed(() => isHistory.value ? "Saklanan düzenlemeler" : "Kişi eşitleme çakışması");
const archived = computed(() => props.history[selectedArchive.value] ?? props.history[0]);
const record = computed(() => isHistory.value ? archived.value : review.value);
const dialog = () => root.value?.querySelector<HTMLElement>('[role="dialog"]');

onMounted(() => {
  app = document.getElementById("app");
  if (app) {
    previousInert = app.inert;
    app.inert = true;
  }
  const element = dialog();
  element?.setAttribute("aria-modal", "true");
  element?.setAttribute("tabindex", "-1");
  element?.focus({ preventScroll: true });
  if (!isHistory.value) void refresh();
});
onBeforeUnmount(() => {
  request?.abort();
  if (app) app.inert = previousInert;
});

function close() {
  if (!applying.value) emit("close");
}

function onKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") {
    event.preventDefault();
    close();
    return;
  }
  if (event.key !== "Tab") return;
  const controls = [...(root.value?.querySelectorAll<HTMLElement>('button:not(:disabled), select:not(:disabled), [tabindex="0"]') ?? [])];
  const focused = controls.indexOf(document.activeElement as HTMLElement);
  if (focused >= 0 && (event.shiftKey ? focused > 0 : focused < controls.length - 1)) return;
  event.preventDefault();
  (event.shiftKey ? controls[controls.length - 1] : controls[0])?.focus();
}

async function refresh() {
  request?.abort();
  const controller = new AbortController();
  request = controller;
  error.value = "";
  review.value = null;
  loading.value = true;
  dialog()?.focus({ preventScroll: true });
  try {
    const result = await inspectTeamContactConflict(props.id, controller.signal);
    if (!controller.signal.aborted) review.value = result;
  } catch (cause) {
    if (!controller.signal.aborted) error.value = cause instanceof Error ? cause.message : "Kişi kayıtları görüntülenemedi.";
  } finally {
    if (!controller.signal.aborted) loading.value = false;
  }
}

async function accept() {
  const snapshot = review.value;
  if (!snapshot?.remote || loading.value || applying.value || error.value) return;
  request?.abort();
  const controller = new AbortController();
  request = controller;
  applying.value = true;
  dialog()?.focus({ preventScroll: true });
  try {
    const remote = await acceptTeamContactConflict(snapshot, controller.signal);
    if (!controller.signal.aborted) emit("accepted", remote);
  } catch (cause) {
    if (!controller.signal.aborted) error.value = cause instanceof Error ? cause.message : "Ortak kayıt doğrulanamadı.";
  } finally {
    if (!controller.signal.aborted) applying.value = false;
  }
}

const dayFormat = new Intl.DateTimeFormat("tr-TR", { dateStyle: "medium", timeZone: "UTC" });
const timeFormat = new Intl.DateTimeFormat("tr-TR", { dateStyle: "medium", timeStyle: "short" });
function date(value: string | null | undefined, withTime = false) {
  const parsed = new Date(value ?? "");
  return Number.isFinite(parsed.getTime()) ? (withTime ? timeFormat : dayFormat).format(parsed) : "";
}

const fieldValues: { label: string; value: (contact: Contact) => string }[] = [
  { label: "Ad soyad", value: (person) => person.name },
  { label: "Firma", value: (person) => person.company },
  { label: "E-posta", value: (person) => person.email },
  { label: "Telefon", value: (person) => person.phone },
  { label: "Kanal", value: (person) => person.channel },
  { label: "Durum", value: (person) => person.archived ? "Arşivde" : "Aktif" },
  { label: "İlgi etiketleri", value: (person) => [...person.tags].sort().join(", ") },
  { label: "Not", value: (person) => person.note },
  { label: "Son temas", value: (person) => date(person.last_contact_at, true) },
  { label: "Sonraki adım", value: (person) => date(person.next_step_at) },
  { label: "Sonraki adım notu", value: (person) => person.next_step_note },
];

function productSkus(products: TeamContactConflict["products"]) {
  return products.map(({ sku }) => sku).sort((a, b) => a.localeCompare(b, "tr")).join("\n");
}

const rows = computed(() => {
  const data = record.value;
  if (!data) return [];
  const remote = data.remote;
  const values = fieldValues.map(({ label, value }) => ({ label, local: value(data.local), shared: remote ? value(remote) : null }));
  if (!isHistory.value && review.value) {
    values.push({ label: "Ürün bağlantıları", local: productSkus(review.value.localProducts), shared: remote ? productSkus(review.value.products) : null });
  }
  return values;
});

const eventTypes: Record<string, string> = { call: "Telefon", email: "E-posta", meeting: "Görüşme", note: "Not", followup_done: "Dönüş yapıldı" };
type PendingOperation = TeamContactConflict["operations"][number];
function describeOperation({ payload, contact }: PendingOperation) {
  if ("action" in payload) {
    switch (payload.action) {
      case "tags": return { title: "İlgi etiketleri güncellendi", lines: [payload.fields.tags.join(", ") || "Tüm etiketler kaldırıldı."], at: date(payload.occurred_at, true) };
      case "archive": return { title: payload.fields.archived ? "Kişi arşivlendi" : "Kişi arşivden çıkarıldı", lines: [], at: date(payload.occurred_at, true) };
      case "link":
      case "unlink": return { title: payload.action === "link" ? "Ürün bağlandı" : "Ürün bağlantısı kaldırıldı", lines: [payload.fields.sku], at: date(payload.occurred_at, true) };
    }
  }
  if ("occurred_at" in payload) {
    const fields = payload.fields;
    const nextStep = [date(fields.next_step_at), fields.next_step_note].filter(Boolean).join(" · ");
    return {
      title: `${eventTypes[fields.kind] ?? "Temas"} kaydı`,
      lines: [fields.note, nextStep && `Sonraki adım · ${nextStep}`].filter(Boolean),
      at: date(payload.occurred_at, true),
    };
  }
  return { title: "Kişi bilgileri düzenlendi", lines: [contact.note].filter(Boolean), at: "" };
}

const operations = computed(() => record.value?.operations.map((entry) => ({ id: entry.id, ...describeOperation(entry) })) ?? []);
const remoteMessage = computed(() => {
  const data = review.value;
  if (!data || data.remoteState === "ready") return "";
  if (data.remoteState === "offline") return "Ortak kaydı görmek için Supabase bağlantısı gerekiyor.";
  if (data.remoteState === "missing") return "Ortak kişi kaydı bulunamadı. Bu cihazdaki düzenlemeler saklanıyor.";
  return data.remoteAccessBlocked ? "Ortak kişi kaydını açmak için çalışma alanı erişimini doğrulayın."
    : "Ortak kişi kaydı şu anda okunamıyor. Karşılaştırmayı yenileyin.";
});
</script>

<template>
  <Teleport to="body">
    <div ref="root" @keydown.stop="onKeydown">
      <ModalShell open scroll :label="title" :title="title" :sub="name" :icon="isHistory ? 'clock' : 'alert'"
        :width="740" :closable="!applying" @close="close">
        <p v-if="loading" class="review-state" role="status">
          <Icon name="loader" :size="16" class="spin" aria-hidden="true" /> Kişi kayıtları okunuyor…
        </p>
        <p v-if="error" class="review-state failure" role="alert">{{ error }}</p>
        <p v-if="remoteMessage && !error" class="review-state" role="status">{{ remoteMessage }}</p>

        <label v-if="isHistory && history.length > 1" class="history-select">
          Saklama tarihi
          <select v-model.number="selectedArchive" class="fx">
            <option v-for="(entry, index) in history" :key="index" :value="index">{{ date(entry.saved_at, true) }}</option>
          </select>
        </label>
        <p v-else-if="isHistory && archived" class="review-state">Saklama tarihi · {{ date(archived.saved_at, true) }}</p>

        <template v-if="record">
          <div class="comparison-wrap">
            <table class="comparison" aria-label="Kişi sürümleri karşılaştırması">
              <thead>
                <tr>
                  <th scope="col">Alan</th>
                  <th scope="col">Bu cihaz <small v-if="record.local.revision != null">Sürüm {{ record.local.revision }}</small></th>
                  <th v-if="record.remote" scope="col">
                    Ortak sürüm <small v-if="record.remote.revision != null">Sürüm {{ record.remote.revision }}</small>
                  </th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="row in rows" :key="row.label" :class="{ changed: row.shared !== null && row.shared !== row.local }">
                  <th scope="row">{{ row.label }}</th>
                  <td>{{ row.local || "—" }}</td>
                  <td v-if="row.shared !== null">{{ row.shared || "—" }}</td>
                </tr>
              </tbody>
            </table>
          </div>

          <section v-if="operations.length" class="operation-list" aria-label="Cihazdaki işlemler">
            <h3>{{ isHistory ? "Saklanan işlemler" : "Eşitleme kuyruğundaki işlemler" }}</h3>
            <ol>
              <li v-for="item in operations" :key="item.id">
                <div class="operation-title"><strong>{{ item.title }}</strong><span v-if="item.at">{{ item.at }}</span></div>
                <p v-for="(line, index) in item.lines" :key="index">{{ line }}</p>
              </li>
            </ol>
          </section>
          <p v-if="review?.remote && !error" class="review-state">
            Ortak sürümü seçtiğinizde bu cihazdaki işlemler eşitleme kuyruğundan çıkar ve Saklanan düzenlemeler bölümüne alınır.
          </p>
        </template>
        <p v-else-if="isHistory" class="review-state">Bu kişi için saklanan düzenleme yok.</p>
        <template #footer>
          <div class="review-actions">
            <button type="button" class="secondary-action" :disabled="applying" @click="close">Kapat</button>
            <button v-if="!isHistory && !loading && (!review?.remote || error)" type="button" class="primary-action"
              :disabled="applying" @click="refresh">Karşılaştırmayı yenile</button>
            <button v-if="review?.remote && !error" type="button" class="primary-action" :disabled="applying"
              @click="accept">{{ applying ? "Ortak sürüm doğrulanıyor…" : "Ortak sürümü kullan" }}</button>
          </div>
        </template>
      </ModalShell>
    </div>
  </Teleport>
</template>

<style scoped>
.review-state {
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 0;
  color: var(--c-mid);
  font-size: 13px;
  line-height: 1.5;
}
.failure { color: var(--badge-eksik-c); }
.comparison-wrap { overflow: hidden; border: 1px solid var(--c-border); border-radius: 9px; }
.comparison { width: 100%; table-layout: fixed; border-collapse: collapse; font-size: 13px; }
.comparison :is(th, td) { padding: 8px 10px; text-align: start; vertical-align: top; overflow-wrap: anywhere; }
.comparison thead { background: var(--c-hover); }
.comparison thead th:first-child { width: clamp(82px, 26%, 150px); }
.comparison small { display: block; color: var(--c-mid); font-size: 11px; font-weight: 500; }
.comparison tbody :is(th, td) { border-top: 1px solid var(--c-border); }
.comparison tbody th { color: var(--c-mid); font-size: 12px; font-weight: 550; }
.comparison tbody td { white-space: pre-wrap; }
.comparison .changed { background: var(--accent-tint); }
.operation-list { border: 1px solid var(--c-border); border-radius: 9px; font-size: 13px; }
.operation-list h3 { margin: 0; padding: 9px 10px; background: var(--c-hover); font-size: 13px; border-radius: 8px 8px 0 0; }
.operation-list ol { margin: 0; padding: 0; list-style: none; }
.operation-list li { padding: 8px 10px; border-top: 1px solid var(--c-border); overflow-wrap: anywhere; }
.operation-title { display: flex; align-items: baseline; justify-content: space-between; flex-wrap: wrap; gap: 4px 10px; }
.operation-title span { color: var(--c-mid); font-size: 12px; }
.operation-list p { margin: 4px 0 0; color: var(--c-mid); white-space: pre-wrap; }
.history-select { display: flex; align-items: center; flex-wrap: wrap; gap: 6px 10px; color: var(--c-mid); font-size: 13px; }
.history-select select { max-width: 100%; padding: 6px 8px; border: 1px solid var(--c-border); border-radius: 8px; background: var(--c-input); color: var(--c-text); }
.review-actions { display: flex; flex-wrap: wrap; gap: 8px; }
@media (max-width: 520px) {
  .comparison :is(th, td) { padding: 6px; }
}
</style>

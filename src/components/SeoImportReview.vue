<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { teamClient, teamReachable, teamSession, teamWorkspace } from "../teamConnection";
import Icon from "./Icon.vue";

type Source = "local" | "backup";
type Draft = Record<string, unknown>;
type Selection = { sku: string; name: string; active: boolean; current: { draft: Draft; revision: number } | null; sources: Record<Source, Draft | null> };
const props = defineProps<{ initialBatch: string | null; workspaceName: string }>();
const emit = defineEmits<{ applying: [boolean]; records: [boolean]; choose: [] }>();
const batches = ref<{ id: string; created_at: string }[]>([]);
const listLoaded = ref(false);
const retryList = ref(0);
const batch = ref("");
const skus = ref<string[]>([]);
const sourceBySku = ref<Record<string, number>>({});
const sourceFilter = ref<"all" | "both" | "local" | "backup">("all");
const sku = ref("");
const search = ref("");
const choice = ref<Source | null>(null);
const detail = ref<Selection | null>(null);
const loading = ref(false);
const saving = ref(false);
const error = ref("");
const savedRevision = ref<number | null>(null);
const differencesOnly = ref(true);
let retry: { key: string; operation: string } | undefined;

const labels: Record<Source, string> = { local: "Aktaran cihazın kopyası", backup: "Seçilen yedek" };
const names: Record<string, string> = {
  target_keyword: "Hedef kelime", draft_title: "Sayfa başlığı", draft_descriptions: "Meta açıklama",
  draft_details: "Ürün açıklaması", draft_keywords: "Meta anahtar kelimeleri", draft_search_keywords: "Site içi arama kelimeleri",
  tech_specs_json: "Teknik tablo", tech_source_text: "Teknik tablo kaynak metni",
  meta_history_json: "Meta geçmişi", details_history_json: "Açıklama geçmişi", tech_history_json: "Teknik tablo geçmişi",
  meta_status: "Meta durumu", details_status: "Açıklama durumu", tech_status: "Teknik tablo durumu",
};
const fields = computed(() => {
  const record = detail.value;
  if (!record) return [];
  const sources = [record.current?.draft, record.sources.local, record.sources.backup].filter(Boolean);
  return [...new Set(sources.flatMap((draft) => Object.keys(draft ?? {})))].filter((field) =>
    !differencesOnly.value || sources.length < 2
    || new Set(sources.map((draft) => JSON.stringify(draft?.[field]))).size > 1,
  );
});
const matchesCurrent = computed(() => {
  const record = detail.value;
  if (!record?.current || !choice.value) return false;
  return JSON.stringify(record.current.draft) === JSON.stringify(record.sources[choice.value]);
});
const visibleSkus = computed(() => {
  const term = search.value.trim().toLocaleLowerCase("tr");
  const mask = { all: 0, local: 1, backup: 2, both: 3 }[sourceFilter.value];
  return skus.value.filter((item) => (!mask || sourceBySku.value[item] === mask)
    && item.toLocaleLowerCase("tr").includes(term));
});
const selectedIndex = computed(() => visibleSkus.value.indexOf(sku.value));
const sourceCounts = computed(() => {
  const counts = { local: 0, backup: 0, both: 0 };
  for (const mask of Object.values(sourceBySku.value)) {
    if (mask === 1) counts.local++;
    if (mask === 2) counts.backup++;
    if (mask === 3) counts.both++;
  }
  return counts;
});
watch(visibleSkus, (visible) => {
  if (sku.value && !visible.includes(sku.value) && !saving.value) sku.value = "";
});

watch(() => batches.value.length > 0, (present) => emit("records", present), { immediate: true });
watch([teamClient, teamWorkspace, () => teamSession.value?.user.id, teamReachable, retryList], async ([client, workspace, user, reachable], previous, cleanup) => {
  let active = true;
  cleanup(() => { active = false; });
  if (!previous || client !== previous[0] || workspace !== previous[1] || user !== previous[2]) {
    batches.value = [];
    batch.value = "";
    listLoaded.value = false;
  }
  error.value = "";
  loading.value = false;
  if (!client || !workspace || !user || reachable === false) return;
  loading.value = true;
  const result = await client.from("team_seo_imports").select("id,created_at").eq("workspace_id", workspace)
    .order("created_at", { ascending: false }).limit(100);
  if (!active) return;
  loading.value = false;
  if (result.error) {
    error.value = "Hazırlanan SEO aktarımları okunamadı.";
    return;
  }
  batches.value = result.data;
  listLoaded.value = true;
  const selection = batch.value || props.initialBatch;
  batch.value = result.data.find((item) => item.id === selection)?.id ?? result.data[0]?.id ?? "";
}, { immediate: true });

watch([batch, teamReachable], async ([id, reachable], [previousId], cleanup) => {
  let active = true;
  cleanup(() => { active = false; });
  if (id !== previousId) {
    skus.value = [];
    sourceBySku.value = {};
    sourceFilter.value = "all";
    sku.value = "";
    search.value = "";
  }
  const client = teamClient.value;
  if (!client || !id || !teamWorkspace.value || reachable === false) return;
  loading.value = true;
  error.value = "";
  const found = new Map<string, number>();
  const workspace = teamWorkspace.value;
  for (let offset = 0; ; offset += 200) {
    const page = await client.from("team_seo_import_rows").select("sku,source").eq("workspace_id", workspace)
      .eq("import_id", id).order("sku").order("source").range(offset, offset + 199);
    if (!active) return;
    if (page.error) {
      loading.value = false;
      error.value = "Hazırlanan ürün kodları okunamadı.";
      return;
    }
    for (const row of page.data) {
      found.set(row.sku, (found.get(row.sku) ?? 0) | (row.source === "local" ? 1 : 2));
    }
    if (page.data.length < 200) break;
  }
  loading.value = false;
  sourceBySku.value = Object.fromEntries(found);
  skus.value = [...found.keys()];
});

let request = 0;
async function readProduct() {
  const sequence = ++request;
  detail.value = null;
  choice.value = null;
  savedRevision.value = null;
  const client = teamClient.value;
  const workspace = teamWorkspace.value;
  if (!client || !batch.value || !sku.value || !workspace) return;
  const selected = sku.value;
  loading.value = true;
  error.value = "";
  const [catalog, current, staged] = await Promise.all([
    client.from("team_catalog").select("name,active").eq("workspace_id", workspace).eq("sku", selected).maybeSingle(),
    client.from("team_product_seo").select("draft,revision").eq("workspace_id", workspace).eq("sku", selected).maybeSingle(),
    client.from("team_seo_import_rows").select("source,draft").eq("workspace_id", workspace)
      .eq("import_id", batch.value).eq("sku", selected),
  ]);
  if (sequence !== request || client !== teamClient.value || workspace !== teamWorkspace.value) return;
  loading.value = false;
  if (catalog.error || current.error || staged.error) {
    error.value = "Ürün ve taslaklar karşılaştırılamadı.";
    return;
  }
  detail.value = {
    sku: selected, name: catalog.data?.name ?? selected, active: catalog.data?.active === true,
    current: current.data,
    sources: {
      local: staged.data.find((row) => row.source === "local")?.draft ?? null,
      backup: staged.data.find((row) => row.source === "backup")?.draft ?? null,
    },
  };
}
watch(sku, () => { void readProduct(); });

async function applyDraft() {
  const client = teamClient.value;
  const workspace = teamWorkspace.value;
  const user = teamSession.value?.user.id;
  const selected = detail.value;
  const source = choice.value;
  const importId = batch.value;
  if (saving.value || teamReachable.value === false || matchesCurrent.value || !client || !workspace
    || !user || !importId || !selected?.active || !source || !selected.sources[source]) return;
  const expected = selected.current?.revision ?? null;
  const key = JSON.stringify([workspace, user, importId, selected.sku, source, expected]);
  if (retry?.key !== key) retry = { key, operation: crypto.randomUUID() };
  saving.value = true;
  emit("applying", true);
  error.value = "";
  const matches = () => client === teamClient.value && workspace === teamWorkspace.value && user === teamSession.value?.user.id;
  try {
    const result = await client.rpc("team_apply_seo_import", {
      target_workspace: workspace, operation_id: retry.operation, target_import: importId,
      target_sku: selected.sku, source_choice: source, expected_revision: expected,
    });
    if (!matches()) return;
    if (result.error?.code === "PT409") {
      await readProduct();
      error.value = "Ortak taslak değişti. Güncel sürümü inceleyip seçimi yeniden yapın.";
      return;
    }
    if (result.error || typeof result.data !== "number") {
      error.value = "Uygulama doğrulanamadı. Aynı seçimi yeniden deneyebilirsiniz.";
      return;
    }
    detail.value = { ...selected, current: { draft: selected.sources[source]!, revision: result.data } };
    savedRevision.value = result.data;
    choice.value = null;
  } catch {
    if (matches()) error.value = "Uygulama doğrulanamadı. Aynı seçimi yeniden deneyebilirsiniz.";
  } finally {
    saving.value = false;
    emit("applying", false);
  }
}

function show(value: unknown) {
  return value == null ? "—" : typeof value === "string" ? value : JSON.stringify(value, null, 2);
}
</script>

<template>
  <div v-if="batches.length" class="review-controls">
    <label>Hazırlanan aktarım
      <select v-model="batch" class="fx" :disabled="saving">
        <option v-for="item in batches" :key="item.id" :value="item.id">
          {{ new Date(item.created_at).toLocaleString("tr-TR") }} · {{ item.id.slice(0, 8) }}
        </option>
      </select>
    </label>
    <label v-if="batch">SKU filtrele
      <input v-model="search" class="fx" placeholder="Ürün kodu" :disabled="saving" />
    </label>
    <label v-if="batch">Ürün
      <select v-model="sku" class="fx" :disabled="saving || !visibleSkus.length">
        <option value="">{{ search && !visibleSkus.length ? "Eşleşen SKU yok" : "Ürün seçin" }}</option>
        <option v-for="code in visibleSkus" :key="code" :value="code">{{ code }}</option>
      </select>
    </label>
  </div>
  <div v-if="skus.length" class="review-filters" aria-label="SEO kayıtlarını kaynağa göre filtrele">
    <button v-for="filter in [
      { key: 'all', label: 'Tümü', count: skus.length },
      { key: 'both', label: 'İki kaynakta', count: sourceCounts.both },
      { key: 'local', label: 'Yalnız aktaran cihaz', count: sourceCounts.local },
      { key: 'backup', label: 'Yalnız yedek', count: sourceCounts.backup },
    ] as const" :key="filter.key" class="secondary-action" :aria-pressed="sourceFilter === filter.key"
      type="button" :disabled="saving" @click="sourceFilter = filter.key">
      {{ filter.label }} <span>{{ filter.count }}</span>
    </button>
  </div>
  <p v-if="loading" role="status" class="review-note">SEO kayıtları okunuyor…</p>
  <p v-if="error" role="alert" class="review-note warning">{{ error }}</p>
  <button v-if="error && !listLoaded && teamReachable !== false" class="secondary-action review-retry"
    type="button" @click="retryList++">Yeniden dene</button>
  <div v-if="listLoaded && !batches.length && !loading && !error" class="review-empty">
    <span class="review-empty-icon"><Icon name="database" :size="19" aria-hidden="true" /></span>
    <div>
      <strong>Hazırlanan aktarım yok</strong>
      <p>Diğer cihazın SQLite yedeğini seçip karşılaştırın. Supabase’te hazırladığınız kayıtlar burada görünür.</p>
    </div>
    <button type="button" class="primary-action" @click="emit('choose')">
      <Icon name="upload" :size="15" aria-hidden="true" /> SQLite yedeği seç
    </button>
  </div>
  <template v-if="detail">
    <div class="review-product">
      <strong>{{ detail.name }}</strong>
      <span>{{ workspaceName }} · {{ detail.current ? `Ortak taslak v${detail.current.revision}` : "Ortak taslak henüz yok" }}</span>
    </div>
    <p v-if="!detail.active" class="review-note warning">Ürün etkin ortak katalogda bulunamadı. Kaynak kayıtları inceleme alanında duruyor.</p>
    <div class="review-tools">
      <label><input v-model="differencesOnly" type="checkbox" /> Yalnız farklı alanları göster</label>
      <button v-if="selectedIndex >= 0 && selectedIndex < visibleSkus.length - 1" type="button"
        class="secondary-action" :disabled="saving" @click="sku = visibleSkus[selectedIndex + 1]">Sonraki SKU</button>
    </div>
    <p v-if="!fields.length" class="review-note">Karşılaştırılan kayıtlarda alan farkı yok.</p>
    <div v-else class="review-table om-scroll">
      <table aria-label="SEO taslakları karşılaştırması">
        <thead><tr>
          <th scope="col">SEO alanı</th>
          <th scope="col">Güncel ortak taslak</th>
          <th scope="col">{{ labels.local }}</th>
          <th scope="col">{{ labels.backup }}</th>
        </tr></thead>
        <tbody>
          <tr v-for="field in fields" :key="field">
            <th scope="row">{{ names[field] ?? field.replace(/_/g, " ") }}</th>
            <td><pre>{{ show(detail.current?.draft[field]) }}</pre></td>
            <td><pre>{{ show(detail.sources.local?.[field]) }}</pre></td>
            <td><pre>{{ show(detail.sources.backup?.[field]) }}</pre></td>
          </tr>
        </tbody>
      </table>
    </div>
    <p v-if="savedRevision !== null" role="status" class="review-note">Ortak taslak v{{ savedRevision }} kaydedildi.</p>
    <div v-if="detail.active && savedRevision === null" class="review-action">
      <label>Uygulanacak kaynak
        <select v-model="choice" class="fx" :disabled="saving">
          <option :value="null">Kaynak seçin</option>
          <option v-if="detail.sources.local" value="local">{{ labels.local }}</option>
          <option v-if="detail.sources.backup" value="backup">{{ labels.backup }}</option>
        </select>
      </label>
      <button type="button" class="primary-action" :class="{ busy: saving }"
        :disabled="!choice || saving || matchesCurrent || teamReachable === false" @click="applyDraft">
        {{ saving ? "Uygulanıyor…" : "Ortak taslağa uygula" }}
      </button>
    </div>
    <p v-if="matchesCurrent" class="review-note">Seçili kaynak güncel ortak taslakla aynı.</p>
    <p v-else-if="teamReachable === false" class="review-note warning">Uygulamak için Supabase bağlantısı gerekli.</p>
  </template>
</template>

<style scoped>
.review-controls { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 10px; }
.review-controls label, .review-action label { display: grid; gap: 5px; font-size: 12px; color: var(--c-mid); min-width: 0; }
.review-controls .fx, .review-action .fx {
  width: 100%;
  min-width: 0;
  height: 38px;
  padding: 0 10px;
  border: 1px solid var(--c-border);
  border-radius: 9px;
  background: var(--c-input);
  color: var(--c-text);
  font: inherit;
  font-size: 13px;
  outline: none;
}
.review-controls .fx:disabled, .review-action .fx:disabled { opacity: 0.55; }
.review-empty {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr);
  gap: 12px;
  align-items: start;
  padding: 14px;
  border: 1px solid var(--c-border);
  border-radius: 10px;
  background: var(--c-panel);
  font-size: 13px;
}
.review-empty p { margin: 4px 0 0; color: var(--c-mid); line-height: 1.5; }
.review-empty-icon { display: grid; place-items: center; width: 34px; height: 34px; border-radius: 8px; background: var(--accent-tint); color: var(--accent); }
.review-empty button { grid-column: 2; justify-self: start; }
.review-retry { align-self: flex-start; }
.review-product { display: grid; gap: 4px; font-size: 14px; }
.review-product span { color: var(--c-soft); font-size: 12px; }
.review-table { overflow: auto; border: 1px solid var(--c-border); border-radius: 9px; }
.review-table table { width: 100%; min-width: 650px; table-layout: fixed; border-collapse: collapse; text-align: left; font-size: 12px; }
.review-table thead { background: var(--c-panel); }
.review-table th, .review-table td { padding: 9px 10px; border-bottom: 1px solid var(--c-border-soft); vertical-align: top; }
.review-table th:first-child { width: 18%; color: var(--c-mid); }
.review-table pre { margin: 0; max-height: 110px; overflow: auto; white-space: pre-wrap; overflow-wrap: anywhere; }
.review-note { margin: 0; font-size: 13px; }
.warning { color: var(--warn-text); }
.review-action { display: flex; align-items: end; gap: 10px; flex-wrap: wrap; }
.review-filters, .review-tools { display: flex; align-items: center; gap: 6px; flex-wrap: wrap; }
.review-filters button, .review-tools button { height: 32px; }
.review-filters button[aria-pressed="true"] { color: var(--accent); border-color: var(--accent); background: var(--accent-tint); }
.review-filters span { font-variant-numeric: tabular-nums; }
.review-tools { justify-content: space-between; font-size: 12px; color: var(--c-mid); }
.review-tools label { display: inline-flex; align-items: center; gap: 5px; }
.review-tools input { accent-color: var(--accent); }
@media (max-width: 650px) { .review-controls { grid-template-columns: 1fr; } }
</style>

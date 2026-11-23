<script setup lang="ts">
import { onBeforeUnmount, onMounted, reactive, ref } from "vue";
import { api } from "../../api";
import { TeamStorePageWriteUnconfirmed, writeTeamIdeasoftPage } from "../../teamIdeasoft";
import { flushTeamStorePages, recordTeamStorePage, storePageWriteTarget, teamStorePageScope, teamStorePageWriteMode } from "../../teamStorePages";
import type { NewStorePage, StorePageDetail } from "../../types";
import Icon from "../Icon.vue";
import ModalShell from "../ModalShell.vue";

const props = defineProps<{ kind: string; title: string }>();
const emit = defineEmits<{ close: []; created: [page: StorePageDetail] }>();
const draft = reactive<NewStorePage>({
  name: "", slug: "", pageTitle: "", metaDescription: "", excerpt: "", content: "", publish: false,
});
const busy = ref(false);
const error = ref("");
const needsInventoryCheck = ref(false);
const nameInput = ref<HTMLInputElement | null>(null);
const scope = teamStorePageScope();
const shared = teamStorePageWriteMode();
let app: HTMLElement | null = null;
let mounted = true;

onMounted(() => {
  app = document.getElementById("app");
  if (app) app.inert = true;
  nameInput.value?.focus();
});
onBeforeUnmount(() => {
  mounted = false;
  if (app) app.inert = false;
});

async function submit() {
  if (busy.value || needsInventoryCheck.value) return;
  busy.value = true;
  error.value = "";
  try {
    const kind = props.kind;
    const submitted = { ...draft };
    const current = () => teamStorePageScope() === scope && teamStorePageWriteMode() === shared;
    if (!current()) throw new Error("Çalışma alanı değişti. Yeni kayıt penceresini yeniden açın.");
    const target = await storePageWriteTarget();
    if (!mounted) return;
    if (!current() || (shared === "team") !== Boolean(target)) throw new Error("Ortak mağaza bağlantısını kontrol edip yeni kayıt penceresini yeniden açın.");
    let page: StorePageDetail;
    if (target) {
      const result = await writeTeamIdeasoftPage(target, { operation: "page_create", kind, draft: submitted });
      page = {
        kind, remote_id: result.remoteId, revision: 1, team_scope: target.key, ...result.facts,
        draft_page_title: "", draft_meta_description: "", draft_target_keyword: "", draft_showcase: "",
        draft_model: "", draft_at: "", pushed_at: result.facts.pushed_at ?? "", queries: [], impressions: 0, clicks: 0,
      };
      if (!result.recorded) {
        const receipt = result.receipt;
        if (!receipt) throw new TeamStorePageWriteUnconfirmed(`IdeaSoft kaydı oluşturdu (#${page.remote_id}). Ortak listeyi yenilemek için mağaza envanterini çekin.`);
        page = await recordTeamStorePage(target, page, result.facts, receipt).catch(() => {
          throw new TeamStorePageWriteUnconfirmed(`IdeaSoft kaydı oluşturdu (#${page.remote_id}). Ortak kayıt yazılamadı. Mağaza envanterini yenileyin.`);
        });
        void flushTeamStorePages().catch(console.warn);
      }
    } else {
      page = await api.createStorePage(kind, submitted);
    }
    if (!mounted) return;
    if (!current()) throw new TeamStorePageWriteUnconfirmed("Kayıt oluşturuldu. Çalışma alanı değiştiği için kaydı görmek üzere mağaza envanterini yenileyin.");
    emit("created", page);
  } catch (cause) {
    needsInventoryCheck.value = cause instanceof TeamStorePageWriteUnconfirmed;
    error.value = String(cause);
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <Teleport to="body">
    <div @keydown.stop @keydown.esc.stop="!busy && emit('close')">
      <ModalShell open scroll icon="plus" :title="title" :label="title" :width="560" :closable="!busy" @close="emit('close')">
        <form id="catalog-create-form" class="catalog-form" @submit.prevent="submit">
          <fieldset :disabled="busy">
            <label>
              {{ kind === "blog" ? "Yazı başlığı" : kind === "category" ? "Kategori adı" : "Marka adı" }}
              <input ref="nameInput" v-model.trim="draft.name" class="fx" maxlength="255" required />
            </label>
            <label>
              URL adı (isteğe bağlı)
              <input v-model.trim="draft.slug" class="fx" maxlength="255" pattern="[a-z0-9\-]+" placeholder="ornek-sayfa" />
            </label>
            <template v-if="kind === 'blog'">
              <label>
                Kısa özet
                <textarea v-model.trim="draft.excerpt" class="fx" rows="2" required></textarea>
              </label>
              <label>
                Yazı metni
                <textarea v-model.trim="draft.content" class="fx" rows="5" required></textarea>
              </label>
            </template>
            <label>
              SEO başlığı (isteğe bağlı)
              <input v-model.trim="draft.pageTitle" class="fx" maxlength="255" />
            </label>
            <label>
              Meta açıklama (isteğe bağlı)
              <textarea v-model.trim="draft.metaDescription" class="fx" maxlength="160" rows="2"></textarea>
            </label>
            <label class="publish">
              <input v-model="draft.publish" type="checkbox" />
              Oluşturduğumda mağazada yayınla
            </label>
          </fieldset>
          <p v-if="error" class="create-error" role="alert">{{ error }}</p>
        </form>
        <template #footer>
          <button type="submit" form="catalog-create-form" class="primary-action" :disabled="busy || needsInventoryCheck">
            <Icon :name="busy ? 'loader' : 'plus'" :size="15" :class="{ spin: busy }" />
            {{ busy ? "Oluşturuluyor…" : draft.publish ? "Oluştur ve yayınla" : "Pasif kayıt oluştur" }}
          </button>
        </template>
      </ModalShell>
    </div>
  </Teleport>
</template>

<style scoped>
.catalog-form fieldset {
  display: grid;
  gap: 12px;
  border: 0;
  padding: 0;
  margin: 0;
  min-width: 0;
}
.catalog-form label {
  display: grid;
  gap: 5px;
  color: var(--c-mid);
  font-size: 13px;
  font-weight: 560;
}
.fx {
  width: 100%;
  padding: 9px 12px;
  border: 1px solid var(--c-border);
  border-radius: 8px;
  background: var(--c-card);
  color: var(--c-text);
  font-size: 13px;
}
textarea.fx {
  resize: vertical;
}
.catalog-form .publish {
  display: flex;
  align-items: center;
  gap: 8px;
}
.publish input { accent-color: var(--accent); }
.create-error {
  color: var(--red);
  overflow-wrap: anywhere;
}
</style>

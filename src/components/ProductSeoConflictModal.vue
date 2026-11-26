<script setup lang="ts">
import {
  acceptTeamProductSeoConflict, inspectTeamProductSeoConflict,
  type TeamProductSeoConflict, type TeamProductSeoConflictArchive, type TeamSeoRecord,
} from "../teamProductSeo";
import SeoConflictModal from "./SeoConflictModal.vue";

const props = defineProps<{
  sku: string;
  name: string;
  mode: "review" | "history";
  history: TeamProductSeoConflictArchive[];
  captureEdits: () => Promise<Record<string, unknown>>;
}>();
const emit = defineEmits<{ close: []; accepted: [remote: TeamSeoRecord] }>();

const fields = ([
  ["target_keyword", "Hedef kelime"],
  ["draft_title", "Sayfa başlığı"],
  ["draft_descriptions", "Meta açıklama"],
  ["draft_keywords", "Meta anahtar kelimeleri"],
  ["draft_search_keywords", "Site içi arama kelimeleri"],
  ["draft_details", "Ürün açıklaması", "html"],
  ["tech_source_text", "Teknik kaynak metni"],
  ["tech_specs_json", "Teknik tablo", "table"],
  ["meta_status", "Meta durumu", "status"],
  ["details_status", "Açıklama durumu", "status"],
  ["tech_status", "Teknik tablo durumu", "status"],
] as const).map(([key, label, format]) => ({ key, label, format, value: (version: TeamSeoRecord) => version.draft[key] }));

async function load(signal: AbortSignal) {
  const edits = await props.captureEdits();
  return inspectTeamProductSeoConflict(props.sku, edits, signal);
}

async function accept(review: TeamProductSeoConflict, signal: AbortSignal) {
  return await acceptTeamProductSeoConflict(review, signal) ? review.remote : null;
}
</script>

<template>
  <SeoConflictModal :name="name" :mode="mode" :history="history" :fields="fields" :load="load" :accept="accept"
    @close="emit('close')" @accepted="emit('accepted', $event)" />
</template>

<script setup lang="ts">
/**
 * Düşüşte olanlar — önceki döneme göre trafik veya sıra kaybeden sayfalar.
 *
 * Neden ayrı bir araç: düşüşteki bir sayfa, hiç yükselmemiş olandan daha aciltir. Orada bir
 * şey BOZULMUŞ; müdahale edilmezse kayıp büyür. Ölçüm (2026-07-29): 49 sayfa, 907 tıklama
 * kaybı — ilk 10'un 6'sı 3D yazıcı, yani kayıp rastgele değil kategori kümeleniyor.
 */
import { computed, ref } from "vue";
import { useStore } from "../../store";
import ToolShell from "./ToolShell.vue";

const store = useStore();
const rows = computed(() => store.opportunity?.decay ?? []);
const days = computed(() => store.opportunity?.days ?? 90);
const lost = computed(() => Math.round(rows.value.reduce((a, d) => a + d.clicks_lost, 0)));

/** Uzun listede önce en çok kaybedeni göster. */
const limit = ref(30);
</script>

<template>
  <ToolShell
    :empty="!rows.length"
    empty-text="Gerileyen sayfa yok — önceki döneme göre kayıp yaşayan ürün bulunamadı."
  >
    <div class="head">
      <b>{{ rows.length }}</b> sayfa geriledi
      <span class="cost">{{ lost }} tıklama kaybı</span>
      <p class="note">
        Önceki {{ days }} güne göre gerileyen sayfalar. Konum değişmeden tıklama düştüyse sorun
        sıralamada değil, arama sonucundaki görünümde olabilir — başlık ve açıklamaya bakın.
      </p>
    </div>

    <div class="card">
      <table class="tbl">
        <thead>
          <tr>
            <th class="c-name">Ürün</th>
            <th class="c-num">Tıklama</th>
            <th class="c-num">Konum</th>
            <th class="c-num">Kayıp</th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="d in rows.slice(0, limit)"
            :key="d.sku"
            class="row"
            :title="`${d.name} — ürüne git`"
            @click="store.openProduct(d.sku)"
          >
            <td class="c-name">
              <div class="nm">{{ d.name }}</div>
              <div class="sku">{{ d.sku }}</div>
            </td>
            <td class="c-num">
              <span class="was">{{ Math.round(d.clicks_before) }}</span>
              <span class="arrow">→</span>{{ Math.round(d.clicks_now) }}
            </td>
            <td class="c-num">
              <span class="was">{{ d.position_before.toFixed(1) }}</span>
              <span class="arrow">→</span>{{ d.position_now.toFixed(1) }}
            </td>
            <td class="c-num miss">−{{ Math.round(d.clicks_lost) }}</td>
          </tr>
        </tbody>
      </table>
      <div v-if="rows.length > limit" class="more">
        <a class="link" @click="limit += 30">
          Sonraki 30'u göster ({{ rows.length - limit }} kaldı)
        </a>
      </div>
    </div>
  </ToolShell>
</template>

<style scoped>
.head {
  margin-bottom: 12px;
  font-size: 12.5px;
  color: var(--c-text);
}
.head b {
  font-weight: 660;
  font-variant-numeric: tabular-nums;
}
.cost {
  margin-left: 8px;
  padding: 2px 8px;
  border-radius: 999px;
  background: var(--warn-bg);
  color: var(--warn-text);
  font-size: 11px;
  font-weight: 640;
  font-variant-numeric: tabular-nums;
}
.note {
  margin: 6px 0 0;
  max-width: 640px;
  font-size: 11.5px;
  line-height: 1.5;
  color: var(--c-soft);
}
.row {
  cursor: pointer;
}
.row:hover {
  background: var(--c-hover);
}
.sku {
  font-size: 10.5px;
  color: var(--c-faint);
  margin-top: 2px;
}
/* Önce/sonra yan yana: kaybın nerede olduğu tek bakışta görünsün. */
.was {
  color: var(--c-faint);
  text-decoration: line-through;
}
.arrow {
  margin: 0 5px;
  color: var(--c-faint);
}
.more {
  padding: 10px 16px;
  border-top: 1px solid var(--c-border-soft);
  font-size: 12px;
}
</style>

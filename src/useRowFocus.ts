/**
 * Kuyruktan gelen satır odağı (Faz K) — araç ekranlarının ortak yardımcısı.
 *
 * ⚠️ **Neden ortak:** aynı üç adım (odak bu ekrana mı ait · hedef kırpmanın ötesinde mi ·
 * satırı seçili göster) beş araç ekranına ayrı ayrı yazılsaydı beş kopya olurdu. Bu oturumda
 * kopyalanan mantığın/geometrinin saptığı üç kez ölçüldü; dördüncüsü baştan engelleniyor.
 *
 * Kırpma sorunu gerçek: EOL ekranı 25, Düşüşte 30, Yükselmeye yakın 40 satır çiziyor —
 * kuyruk 300. satırı işaret ediyorsa o satır DOM'da hiç olmazdı.
 */
import { computed, watch, type Ref } from "vue";
import { useStore } from "./store";

export function useRowFocus(
  page: string,
  /** Ekranın TAM (kırpılmamış) satır kimlikleri — sırayla. */
  allIds: () => string[],
  /** Ekranın kırpma sınırı; hedef dışarıda kalıyorsa yükseltilir. */
  limit?: Ref<number>,
) {
  const store = useStore();

  /** Bu ekran için istenen odak; başka ekrana aitse boş. */
  const focusId = computed(() => (store.focus?.page === page ? store.focus.id : ""));

  watch(
    focusId,
    (id) => {
      if (!id || !limit) return;
      const i = allIds().indexOf(id);
      // Hedef kırpmanın ötesindeyse listeyi oraya kadar aç. `-1` (bulunamadı) dokunmuyor:
      // veri değişmiş olabilir, kullanıcıya boş bir "hepsini göster" listesi açmayalım.
      if (i >= 0 && i >= limit.value) limit.value = i + 1;
    },
    { immediate: true },
  );

  return focusId;
}

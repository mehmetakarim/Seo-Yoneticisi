<script setup lang="ts">
/**
 * Dil modelinden gelen metni güvenle işler: paragraf, madde listesi, **kalın**, `kod`.
 *
 * ⚠️ **`v-html` bilinçli olarak KULLANILMIYOR.** Bu metin doğrudan bir dil modelinden
 * geliyor; HTML olarak basmak, modelin (ya da modele veri sızdıran herhangi bir içeriğin)
 * uygulama içinde kod çalıştırmasına kapı açardı. Aynı gerekçe `UpdateModal`'da da yazılı
 * (orada metin release JSON'undan geliyordu) — burada yüzey daha da net.
 *
 * Bunun yerine metin GERÇEK Vue düğümlerine çevriliyor ve `{{ }}` ile kaçışlanıyor.
 * Bir bağımlılık (marked vb.) eklenmedi: uygulamanın Vue/Pinia/Tauri dışında çalışma
 * zamanı bağımlılığı yok, bu çizgi korunuyor.
 *
 * Desteklenen alt küme bilinçli olarak dar: modelden istenen biçim de bu kadarı
 * (sistem talimatında yazılı). Tablo/başlık/bağlantı desteklenmiyor.
 */
import { computed } from "vue";

const props = defineProps<{ text: string }>();

/** Satır içi parça: düz metin, kalın veya kod. */
interface Piece {
  kind: "text" | "bold" | "code";
  text: string;
}
interface Block {
  kind: "p" | "li";
  pieces: Piece[];
}

/** `**kalın**` ve `` `kod` `` işaretlerini parçalara ayırır. */
function inline(s: string): Piece[] {
  const out: Piece[] = [];
  // Tek geçiş: hangi işaret önce geliyorsa onu al. İç içe geçme desteklenmiyor —
  // model için istenen biçim bu kadar, fazlası sessizce düz metin kalır.
  const re = /\*\*(.+?)\*\*|`([^`]+)`/g;
  let last = 0;
  let m: RegExpExecArray | null;
  while ((m = re.exec(s))) {
    if (m.index > last) out.push({ kind: "text", text: s.slice(last, m.index) });
    if (m[1] !== undefined) out.push({ kind: "bold", text: m[1] });
    else out.push({ kind: "code", text: m[2] });
    last = m.index + m[0].length;
  }
  if (last < s.length) out.push({ kind: "text", text: s.slice(last) });
  return out.length ? out : [{ kind: "text", text: s }];
}

const blocks = computed<Block[]>(() => {
  const out: Block[] = [];
  for (const raw of (props.text ?? "").split(/\r?\n/)) {
    const line = raw.trim();
    if (!line) continue;
    const bullet = /^[-*•]\s+(.*)$/.exec(line);
    if (bullet) out.push({ kind: "li", pieces: inline(bullet[1]) });
    else out.push({ kind: "p", pieces: inline(line) });
  }
  return out;
});
</script>

<template>
  <div class="md">
    <template v-for="(b, i) in blocks" :key="i">
      <p v-if="b.kind === 'p'">
        <template v-for="(p, j) in b.pieces" :key="j">
          <b v-if="p.kind === 'bold'">{{ p.text }}</b>
          <code v-else-if="p.kind === 'code'">{{ p.text }}</code>
          <template v-else>{{ p.text }}</template>
        </template>
      </p>
      <div v-else class="li">
        <span class="dot"></span>
        <span>
          <template v-for="(p, j) in b.pieces" :key="j">
            <b v-if="p.kind === 'bold'">{{ p.text }}</b>
            <code v-else-if="p.kind === 'code'">{{ p.text }}</code>
            <template v-else>{{ p.text }}</template>
          </template>
        </span>
      </div>
    </template>
  </div>
</template>

<style scoped>
.md {
  font-size: 12.5px;
  line-height: 1.62;
}
.md p {
  margin: 0 0 8px;
}
.md p:last-child,
.li:last-child {
  margin-bottom: 0;
}
.li {
  display: flex;
  gap: 8px;
  margin: 0 0 5px;
}
/* Madde işareti bir düğüm — metin içeriğinden gelmiyor, dolayısıyla modelin
   ürettiği metnin görünümü etkilemesi mümkün değil. */
.dot {
  flex: none;
  width: 4px;
  height: 4px;
  margin-top: 8px;
  border-radius: 50%;
  background: var(--c-soft);
}
.md b {
  font-weight: 640;
}
.md code {
  font-family: ui-monospace, "SF Mono", Menlo, monospace;
  font-size: 11.5px;
  padding: 1px 5px;
  border-radius: 5px;
  background: var(--c-chip);
  color: var(--c-text);
}
</style>

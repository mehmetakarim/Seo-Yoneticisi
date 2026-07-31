#!/usr/bin/env python3
"""Görsel doğrulama harness'i: GERÇEK uygulamayı GERÇEK verilerle tarayıcıda çalıştırır.

    npm run build && python3 scripts/harness.py
    npx vite preview --port 4173      # sonra http://localhost:4173/harness.html

**Neden bu yaklaşım:** daha önce her doğrulamada elle statik HTML işaretleme yazılıyordu.
İki sorunu vardı: (1) işaretleme elle yazıldığı için gerçek bileşenden sapabiliyordu —
yani harness "geçer" dediğinde uygulama yine bozuk olabilirdi; (2) Vue SFC stilleri scoped
olduğu için her parçaya `data-v-<hash>` niteliğini elle eklemek gerekiyordu ve hash her
derlemede değişiyordu.

Burada bunun yerine `dist/index.html` alınıp başına bir Tauri IPC taklidi ekleniyor.
Uygulamanın kendi bundle'ı yükleniyor: gerçek bileşenler, gerçek store, gerçek stiller.
Tek sahte olan şey backend cevapları.

Veri kaynağı yerel veritabanıdır — uydurma sayı yok. Yalnızca önbellekte bulunmayan
alanlar (ör. eski bir önbellekte `decay` yoksa) sentetikle tamamlanır ve bu açıkça
işaretlenir.

⚠️ `npm run build` `dist/`i siler → harness'i her zaman derlemeden SONRA üretin.
⚠️ Üretilen `dist/harness.html` bir yapı çıktısıdır, `.gitignore` kapsamındadır.
"""

import json
import os
import pathlib
import sqlite3
import sys

DB = pathlib.Path.home() / (
    "Library/Application Support/com.kurumsalit.seo-yoneticisi/seo-yoneticisi.db"
)
DIST = pathlib.Path("dist")


def setting(conn, key):
    row = conn.execute("SELECT value FROM settings WHERE key = ?1", (key,)).fetchone()
    return row[0] if row else None


def load_report(conn):
    raw = setting(conn, "opportunity_json")
    if not raw:
        sys.exit("opportunity_json önbelleği yok — uygulamada bir kez analiz çalıştırın.")
    rep = json.loads(raw)
    # Eski önbelleklerde sonradan eklenen bölümler bulunmaz; harness onları da göstermeli.
    if not rep.get("decay"):
        rep["decay"] = [
            {
                "sku": "SENTETIK.1",
                "name": "[sentetik] Anycubic Photon P1 14K Reçine 3D Yazıcı",
                "url": "https://example.com/urun/sentetik-1",
                "clicks_now": 12.0, "clicks_before": 58.0, "clicks_lost": 46.0,
                "impressions_now": 1840.0, "impressions_before": 3110.0,
                "position_now": 14.2, "position_before": 6.8,
            }
        ]
        print("not: önbellekte `decay` yok → bölüm sentetik satırla gösteriliyor")
    return rep


def load_live_products(conn):
    """Canonical hedefi araması için SATIŞTAKİ ürünler (feed).

    ⚠️ IdeaSoft'un tam kataloğu DEĞİL: canonical yalnızca satıştaki bir ürüne verilebilir,
    yoksa ölü sayfa başka bir ölü sayfayı işaret eder (saha hatası, 2026-07-30).
    """
    rows = conn.execute(
        "SELECT sku, name, url FROM products WHERE url IS NOT NULL AND url <> '' LIMIT 400"
    ).fetchall()
    out = []
    for sku, name, url in rows:
        slug = url.rstrip("/").rsplit("/", 1)[-1].lower()
        if slug:
            out.append({"slug": slug, "id": 0, "name": name, "status": 1,
                        "stock": 0.0, "canonical": sku})
    return out


def load_product(conn):
    """Ürün ekranı ve gönderim modali için tek gerçek ürün."""
    row = conn.execute(
        "SELECT sku, name, brand FROM products ORDER BY sku LIMIT 1"
    ).fetchone()
    if not row:
        return None, None
    sku, name, brand = row
    listing = {
        "sku": sku, "name": name, "brand": brand, "img_url": None,
        "meta_badge": "hatali", "details_badge": "eksik", "overall": "bekliyor",
        "meta_done": False, "details_done": False, "tech_done": False, "image_count": 4,
    }
    detail = {k: None for k in (
        "brand main_category category quantity url img_url title descriptions keywords "
        "search_keywords details target_keyword draft_title draft_descriptions draft_keywords "
        "draft_search_keywords draft_details image_check tech_source_text tech_specs "
        "ideasoft_pushed_at ideasoft_seo_rule meta_model details_model tech_model"
    ).split()}
    detail.update({
        "sku": sku, "name": name, "brand": brand,
        "meta_status": "pending", "details_status": "pending",
        "badge": "hatali", "details_badge": "eksik", "overall": "bekliyor",
        "gallery": [], "image_count": 0, "image_badge": "eksik",
        "tech_status": "pending", "tech_badge": "eksik", "tech_history": [],
        "meta_history": [], "details_history": [],
    })
    return listing, detail


def main():
    if not DB.exists():
        sys.exit(f"veritabanı bulunamadı: {DB}")
    index = DIST / "index.html"
    if not index.exists():
        sys.exit("dist/index.html yok — önce `npm run build` çalıştırın.")

    conn = sqlite3.connect(f"file:{DB}?mode=ro", uri=True)
    report = load_report(conn)
    listing, detail = load_product(conn)
    live = load_live_products(conn)
    conn.close()

    handlers = {
        "get_opportunity_cache": report,
        "get_settings": {"theme": "light"},
        "get_last_sync": None,
        "list_products": [listing] if listing else [],
        "get_product": detail,
        "app_version": "harness",
    }
    # Canonical hedefi araması: gerçek feed ürünleri üzerinde, terimle süzülerek.
    live_json = json.dumps(live, ensure_ascii=False)

    # `?empty=1` → analiz hiç çalıştırılmamış gibi davranır. Boş durumları elle test etmek
    # mümkün değil: sayfalar açılırken önbelleği yeniden yüklüyor, store'u dışarıdan
    # sıfırlamak yetmiyor. Bu senaryo önemli — v0.5.9'da boş ekran bir saha hatasıydı.
    # Asistan akışı taklidi: gerçek Gemini çağrısı yapmadan arayüzü doğrulamak için.
    # Önce "düşünüyor" olayları, sonra parça parça cevap — canlı ölçümdeki sıranın aynısı
    # (Gemma önce muhakemesini yayınlıyor, cevap sonda geliyor).
    fake_answer = (
        "Bu ekrandaki veriye göre en çok tıklama kaçıran üç sayfa şunlar:\n"
        "- **Ergotron WorkFit-T 33-397-062** — 474 gösterim, 2 tıklama, konum 4.1. "
        "Konum iyi ama `CTR` çok düşük; sorun sıralamada değil başlıkta.\n"
        "- **Lenovo ThinkPad E16 G3** — 850 gösterim, 33 tıklama.\n"
        "- **Logitech M280** — 1207 gösterim, 3 tıklama.\n\n"
        "Önce ilkine bakın: konumu ilk sayfada olduğu için başlık ve açıklama düzeltmesi "
        "doğrudan tıklamaya dönüşür. Bunu **Ürünler** ekranından üretebilirsiniz."
    )

    stub = (
        "<script>\n"
        "// Tauri IPC taklidi — yalnızca harness için. Uygulama bundle'ı gerçek.\n"
        "window.__TAURI_INTERNALS__ = {\n"
        "  invoke: (cmd, args) => {\n"
        f"    const H = {json.dumps(handlers, ensure_ascii=False)};\n"
        "    if (new URLSearchParams(location.search).has('empty')\n"
        "        && cmd === 'get_opportunity_cache') return Promise.resolve(null);\n"
        # `?setup=1` → taze kurulum benzetimi. Sihirbaz kullanıcının GERÇEK veritabanına
        # dokunmadan uçtan uca denenebiliyor; yazan komutlar yutuluyor.
        "    if (new URLSearchParams(location.search).has('setup')) {\n"
        "      if (cmd === 'needs_setup') return Promise.resolve(true);\n"
        "      if (cmd === 'get_settings') return Promise.resolve({\n"
        "        feed_url: '', gemini_api_key: '', capsolver_api_key: '', seo_country: 'tr',\n"
        "        gsc_site_url: '', gsc_client_email: '', ideasoft_domain: '',\n"
        "        ideasoft_token: '', ideasoft_active: false, theme: 'light', last_backup_at: null });\n"
        "      if (cmd === 'list_products') return Promise.resolve([]);\n"
        "      if (cmd === 'save_settings' || cmd === 'mark_setup_done'\n"
        "          || cmd === 'set_gsc_service_account') return Promise.resolve('');\n"
        "      if (cmd === 'test_feed_url') return /^https?:\\/\\/.+\\..+/.test(args.url || '')\n"
        "        ? Promise.resolve(142) : Promise.reject('Geçerli bir URL girin (http/https).');\n"
        "      if (cmd === 'test_gemini_key') return (args.key || '').startsWith('AIza')\n"
        "        ? Promise.resolve('Anahtar geçerli') : Promise.reject('Gemini API anahtarı geçersiz.');\n"
        "      if (cmd === 'test_ideasoft') return Promise.reject('IdeaSoft token geçersiz.');\n"
        "      if (cmd === 'sync_feed') return Promise.resolve({ run_at: '2026-07-31T12:00',\n"
        "        active: 142, added: 142, updated: 0, deleted: 0, duplicate_skipped: 3 });\n"
        "    }\n"
        "    if (cmd === 'assistant_ask') {\n"
        f"      const ANS = {json.dumps(fake_answer, ensure_ascii=False)};\n"
        "      const ch = args && args.onEvent;\n"
        "      return new Promise((resolve) => {\n"
        "        let i = 0;\n"
        "        const think = setInterval(() => {\n"
        "          if (ch && ch.onmessage) ch.onmessage({ kind: 'thinking' });\n"
        "          if (++i >= 6) {\n"
        "            clearInterval(think);\n"
        "            const parts = ANS.match(/[\\s\\S]{1,28}/g) || [];\n"
        "            let j = 0;\n"
        "            const feed = setInterval(() => {\n"
        "              if (ch && ch.onmessage) ch.onmessage({ kind: 'chunk', text: parts[j] });\n"
        "              if (++j >= parts.length) { clearInterval(feed); resolve('gemma-4-31b-it'); }\n"
        "            }, 30);\n"
        "          }\n"
        "        }, 140);\n"
        "      });\n"
        "    }\n"
        # Sohbet geçmişi: bellekte tutulan minik bir taklit tablo. Kaydet → listele → aç →
        # sil akışının tamamı gerçek Gemini/SQLite'a dokunmadan doğrulanabiliyor.
        "    if (cmd === 'search_live_products') {\n"
        f"      const LIVE = {live_json};\n"
        "      const q = (args.term || '').trim().toLowerCase();\n"
        "      if (q.length < 3) return Promise.resolve([]);\n"
        "      return Promise.resolve(LIVE.filter(p =>\n"
        "        p.name.toLowerCase().includes(q) || p.canonical.toLowerCase().includes(q)\n"
        "      ).slice(0, 25));\n"
        "    }\n"
        "    if (cmd.endsWith('chat_session') || cmd.endsWith('chat_sessions')) {\n"
        "      const S = (window.__CHATS__ = window.__CHATS__ || { seq: 0, rows: [] });\n"
        "      const now = new Date().toISOString().slice(0, 16);\n"
        "      if (cmd === 'list_chat_sessions')\n"
        "        return Promise.resolve(S.rows.map(r => ({ id: r.id, title: r.title,\n"
        "          tool_page: r.tool_page, messages: r.messages.length, model: r.model,\n"
        "          updated_at: r.updated_at })).sort((a,b) => b.id - a.id));\n"
        "      if (cmd === 'get_chat_session') {\n"
        "        const r = S.rows.find(x => x.id === args.id);\n"
        "        return r ? Promise.resolve(r.messages) : Promise.reject('Bu sohbet artık yok.');\n"
        "      }\n"
        "      if (cmd === 'save_chat_session') {\n"
        "        let r = S.rows.find(x => x.id === args.id);\n"
        "        if (!r) {\n"
        "          const first = (args.messages.find(m => m.role === 'user') || {}).text || '';\n"
        "          r = { id: ++S.seq, title: first.slice(0, 60), tool_page: args.toolPage };\n"
        "          S.rows.push(r);\n"
        "        }\n"
        "        r.messages = args.messages; r.model = args.model; r.updated_at = now;\n"
        "        return Promise.resolve(r.id);\n"
        "      }\n"
        "      if (cmd === 'delete_chat_session') {\n"
        "        S.rows = S.rows.filter(x => x.id !== args.id); return Promise.resolve(null);\n"
        "      }\n"
        "      if (cmd === 'delete_all_chat_sessions') { S.rows = []; return Promise.resolve(null); }\n"
        "    }\n"
        "    return Promise.resolve(cmd in H ? H[cmd] : null);\n"
        "  },\n"
        "  transformCallback: (cb) => { const id = Math.random(); window[id] = cb; return id; },\n"
        "};\n"
        "</script>\n    "
    )

    html = index.read_text()
    out = DIST / "harness.html"
    out.write_text(html.replace('<script type="module"', stub + '<script type="module"', 1))
    counts = {k: len(v) for k, v in report.items() if isinstance(v, list)}
    print(f"{out} hazır · {counts}")


if __name__ == "__main__":
    os.chdir(pathlib.Path(__file__).resolve().parent.parent)
    main()

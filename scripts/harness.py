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
    conn.close()

    handlers = {
        "get_opportunity_cache": report,
        "get_settings": {"theme": "light"},
        "get_last_sync": None,
        "list_products": [listing] if listing else [],
        "get_product": detail,
        "app_version": "harness",
    }

    stub = (
        "<script>\n"
        "// Tauri IPC taklidi — yalnızca harness için. Uygulama bundle'ı gerçek.\n"
        "window.__TAURI_INTERNALS__ = {\n"
        "  invoke: (cmd) => {\n"
        f"    const H = {json.dumps(handlers, ensure_ascii=False)};\n"
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

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

import datetime
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
        "feed_changed": None,
        # Faz D sağlık skoru: bu ürün yerelde eksik ve mağazaya gönderilmemiş.
        "health": 60,
        "health_missing": [
            {"label": "teknik tablo", "points": 20},
            {"label": "mağazaya gönderim", "points": 15},
        ],
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
        "feed_changed": None,
    })
    return listing, detail


def load_gallery(conn):
    """Karşılaştırma stub'ı için GERÇEK görsel adresleri — küçük resimler gerçekten yüklensin."""
    rows = conn.execute(
        "SELECT img_url, picture2, picture3, picture4 FROM products "
        "WHERE img_url IS NOT NULL AND picture3 IS NOT NULL LIMIT 1"
    ).fetchone()
    return [u for u in (rows or []) if u]


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
    gallery = load_gallery(conn)
    conn.close()

    # JSON-LD kartı için gösterim örneği. ⚠️ Gerçek çıktıyı `seo_core::jsonld` üretiyor;
    # buradaki yalnızca kartın yerleşimini/kaydırmasını denemek için temsilî bir metin.
    jsonld_stub = json.dumps(
        {
            "@context": "https://schema.org",
            "@type": "Product",
            "name": (listing or {}).get("name", "Örnek Ürün"),
            "sku": (listing or {}).get("sku", "SKU-1"),
            "brand": {"@type": "Brand", "name": (listing or {}).get("brand") or "Marka"},
            "url": "https://ornek.com/urun/ornek",
            "image": ["https://ornek.com/1.jpg", "https://ornek.com/2.jpg"],
            "additionalProperty": [
                {"@type": "PropertyValue", "name": f"Özellik {i}", "value": f"Değer {i}"}
                for i in range(1, 13)
            ],
        },
        ensure_ascii=False,
        indent=2,
    )
    jsonld_stub = f'<script type="application/ld+json">\n{jsonld_stub}\n</script>'

    # Ölçüm omurgası (Faz Ö) — temsilî veri. Gerçek değerler `metrics_cmd`'den geliyor;
    # buradaki yalnızca rozet/şerit/zaman çizelgesi yerleşimini denemek için.
    ilk_sku = (listing or {}).get("sku", "SKU-1")
    outcome_summary = {
        "snapshots": 12, "oldest_window": "2025-08-22", "measured_events": 41,
        "improved": 9, "flat": 21, "worse": 4, "measuring": 5, "insufficient": 2,
        "net_delta_clicks": 137.0,
    }
    outcome_badges = [
        {"sku": ilk_sku, "label": "İyileşti", "tone": "uygun",
         "tip": "2026-06-14 gönderimi · 18 → 34 (+16 tıklama) · 2026-05-10 → 2026-06-07 ile 2026-07-07 → 2026-08-04 karşılaştırıldı"},
    ]
    product_timeline = {
        "has_store_event": True,
        "items": [
            {"at": "2026-06-14T11:20:00", "kind": "ideasoft_push", "label": "IdeaSoft'a gönderildi",
             "outcome_label": "İyileşti", "outcome_tone": "uygun",
             "outcome_tip": "2026-06-14 gönderimi · 18 → 34 (+16 tıklama)"},
            {"at": "2026-06-14T11:02:00", "kind": "meta_done", "label": "Meta tamamlandı",
             "outcome_label": None, "outcome_tone": None, "outcome_tip": None},
            {"at": "2026-06-13T16:40:00", "kind": "details_done", "label": "Açıklama tamamlandı",
             "outcome_label": None, "outcome_tone": None, "outcome_tip": None},
            {"at": "2026-06-13T15:10:00", "kind": "tech_done", "label": "Teknik tablo tamamlandı",
             "outcome_label": None, "outcome_tone": None, "outcome_tip": None},
        ],
    }

    handlers = {
        "get_eol_decisions": [],
        "get_jsonld": jsonld_stub,
        "get_outcome_summary": outcome_summary,
        "get_outcome_badges": outcome_badges,
        "get_product_timeline": product_timeline,
        "get_opportunity_cache": report,
        "get_settings": {"theme": "light"},
        "get_last_sync": None,
        "list_products": [listing] if listing else [],
        "get_product": detail,
        "app_version": "harness",
    }
    # CRM (Faz C). ⚠️ `?musteri=0` → hiç kişi yok (boş ekranın dürüstlüğü test edilebilsin).
    # Tarihler ÇALIŞMA ANINA göre üretiliyor: sabit tarih yazılsaydı "4 gün gecikti" birkaç
    # gün sonra "40 gün gecikti" derdi ve ekran ölçüsü kayardı.
    _bugun = datetime.date.today()
    _g = lambda n: (_bugun + datetime.timedelta(days=n)).isoformat()
    CONTACTS = [
        {"id": 1, "name": "Ahmet Yılmaz", "company": "Kurumsal BT", "email": "ahmet@kurumsalbt.com",
         "phone": "0532 000 00 00", "channel": "mail", "note": "Sunucu yenileme projesi.",
         "last_contact_at": _g(-11) + "T10:12:00", "next_step_at": _g(-4),
         "next_step_note": "fiyat teklifi verilecek", "archived": False, "event_count": 3,
         "tags": ["sunucu", "yenileme"]},
        {"id": 2, "name": "Zeynep Kaya", "company": "Anadolu Yapı", "email": "zeynep@anadoluyapi.com",
         "phone": "0555 111 11 11", "channel": "fuar", "note": "",
         "last_contact_at": _g(-2) + "T15:40:00", "next_step_at": _g(0),
         "next_step_note": "numune sonucu sorulacak", "archived": False, "event_count": 1,
         "tags": ["3D yazıcı"]},
        {"id": 3, "name": "Mert Demir", "company": "Demir Elektrik", "email": "", "phone": "0312 222 22 22",
         "channel": "telefon", "note": "", "last_contact_at": _g(-40) + "T09:00:00",
         "next_step_at": _g(12), "next_step_note": "bütçe sonrası tekrar", "archived": False,
         "event_count": 2, "tags": []},
        {"id": 4, "name": "Elif Şahin", "company": "", "email": "elif@ornek.com", "phone": "",
         "channel": "instagram", "note": "Tek seferlik alım.", "last_contact_at": None,
         "next_step_at": None, "next_step_note": "", "archived": True, "event_count": 0, "tags": []},
    ]
    CONTACT_EVENTS = {
        1: [{"id": 3, "at": _g(-11) + "T10:12:00", "kind": "call", "note": "Fiyat aralığı soruldu."},
            {"id": 2, "at": _g(-19) + "T14:02:00", "kind": "email", "note": "Katalog gönderildi."},
            {"id": 1, "at": _g(-26) + "T11:30:00", "kind": "meeting", "note": "Fuarda tanışıldı."}],
        2: [{"id": 4, "at": _g(-2) + "T15:40:00", "kind": "email", "note": "Numune kargoya verildi."}],
        3: [{"id": 6, "at": _g(-40) + "T09:00:00", "kind": "note", "note": "Bütçe yılbaşında."},
            {"id": 5, "at": _g(-70) + "T09:00:00", "kind": "call", "note": "İlk görüşme."}],
    }

    # Kişi → ürün bağı ve CSV önizlemesi (Faz C2).
    _urun0 = live[0] if live else {"slug": "ornek-urun", "name": "Örnek Ürün"}
    CONTACT_PRODUCTS = {
        1: [{"sku": _urun0["slug"], "name": _urun0["name"], "contact_id": 1, "at": _g(-11)}],
    }
    CSV_PREVIEW = {
        # ⚠️ Gerçek tuzağı taklit ediyor: Türkçe Excel `;` ile ayırıyor ve sütun adları
        # uygulamanınkilerle birebir değil ("Yetkili", "Ünvan").
        "headers": ["Yetkili", "Ünvan", "E-Posta", "Cep", "Adres"],
        "rows": [
            ["Ahmet Yılmaz", "Kurumsal BT", "ahmet@kurumsalbt.com", "0532 000 00 00", "Ankara"],
            ["Zeynep Kaya", "Anadolu Yapı", "zeynep@anadoluyapi.com", "0555 111 11 11", "İzmir"],
            ["Mert Demir", "Demir Elektrik", "", "0312 222 22 22", "Bursa"],
            ["Elif Şahin", "", "elif@ornek.com", "", "İstanbul"],
            ["Can Öz", "Öz Bilişim", "can@ozbilisim.com", "0216 333 33 33", "İstanbul"],
        ],
        "total_rows": 42,
        "delimiter": ";",
        "mapping": [0, 1, 2, 3, None, None],
        "fields": [["name", "Ad soyad"], ["company", "Firma"], ["email", "E-posta"],
                   ["phone", "Telefon"], ["channel", "Kanal"], ["note", "Not"]],
    }

    # Teklif (Faz T). Satırlar GERÇEK katalog fiyatlarından: 600→749 (marj %19,9) ve
    # negatif marjlı bir satır — uyarının ekranda gerçekten göründüğü doğrulanabilsin.
    def _kalem(i, sku, ad, adet, birim, kdv, maliyet):
        net = round(adet * birim, 2)
        marj = None
        if maliyet is not None:
            tutar = round(net - maliyet * adet, 2)
            pct = round(tutar / net * 100, 1) if net else 0
            durum = "negative" if pct < 0 else ("low" if pct < 10 else "ok")
            marj = {"amount": tutar, "pct": pct, "state": durum}
        return {"id": i, "sku": sku, "name": ad, "qty": adet, "unit_price": birim,
                "tax_rate": kdv, "cost": maliyet, "net": net, "margin": marj}

    _kalemler = [
        _kalem(1, "PC.LEN.13B9001MTR", "Lenovo ThinkCentre Neo 50q G5", 2, 949.0, 20, 860.0),
        _kalem(2, "MNL.GNX.4008050066", "Bambu Lab P2S Combo 3D Yazıcı", 1, 749.0, 20, 600.0),
        _kalem(3, None, "Kurulum ve montaj", 1, 150.0, 20, None),
        # 🔴 Zararına satır: katalogda 7 tane var.
        _kalem(4, "KAB.ZAR.1", "Zararına satılan kablo", 5, 10.0, 10, 20.67),
    ]
    _ara = round(sum(k["net"] for k in _kalemler), 2)
    _kdv20 = round(sum(k["net"] for k in _kalemler if k["tax_rate"] == 20) * 0.20, 2)
    _kdv10 = round(sum(k["net"] for k in _kalemler if k["tax_rate"] == 10) * 0.10, 2)
    _maliyetli = [k for k in _kalemler if k["cost"] is not None]
    _msat = round(sum(k["net"] for k in _maliyetli), 2)
    _mmal = round(sum(k["cost"] * k["qty"] for k in _maliyetli), 2)
    _mpct = round((_msat - _mmal) / _msat * 100, 1)
    QUOTES = [
        {"id": 1, "no": "T-2026-001", "contact_id": 1, "contact_name": "Ahmet Yılmaz",
         "status": "draft", "status_label": "Taslak", "currency": "USD", "fx_rate": None,
         "fx_date": None, "valid_until": _g(12), "note": "", "close_reason": "",
         "created_at": _g(-1), "sent_at": None, "items": _kalemler, "subtotal": _ara,
         "taxes": [
             {"rate": 10.0, "base": round(sum(k["net"] for k in _kalemler if k["tax_rate"] == 10), 2), "amount": _kdv10},
             {"rate": 20.0, "base": round(sum(k["net"] for k in _kalemler if k["tax_rate"] == 20), 2), "amount": _kdv20},
         ],
         "tax_total": round(_kdv10 + _kdv20, 2),
         "grand_total": round(_ara + _kdv10 + _kdv20, 2),
         "margin": {"amount": round(_msat - _mmal, 2), "pct": _mpct,
                    "state": "negative" if _mpct < 0 else ("low" if _mpct < 10 else "ok")},
         "version_count": 0},
        {"id": 2, "no": "T-2026-002", "contact_id": 2, "contact_name": "Zeynep Kaya",
         "status": "sent", "status_label": "Gönderildi", "currency": "TRY", "fx_rate": 47.5911,
         "fx_date": _g(-3), "valid_until": _g(-1), "note": "", "close_reason": "",
         "created_at": _g(-6), "sent_at": _g(-3), "items": [], "subtotal": 35645.73,
         "taxes": [{"rate": 20.0, "base": 35645.73, "amount": 7129.15}],
         "tax_total": 7129.15, "grand_total": 42774.88,
         "margin": {"amount": 7093.5, "pct": 19.9, "state": "ok"}, "version_count": 1},
        {"id": 3, "no": "T-2026-003", "contact_id": 3, "contact_name": "Mert Demir",
         "status": "lost", "status_label": "Kaybedildi", "currency": "USD", "fx_rate": None,
         "fx_date": None, "valid_until": _g(-20), "note": "", "close_reason": "fiyat",
         "created_at": _g(-30), "sent_at": _g(-28), "items": [], "subtotal": 1200.0,
         "taxes": [{"rate": 20.0, "base": 1200.0, "amount": 240.0}], "tax_total": 240.0,
         "grand_total": 1440.0, "margin": None, "version_count": 0},
    ]

    # Teklif belgesi.
    #
    # 🔧 **Elle yazılmadı**: gerçek üreticiden alındı —
    # `cargo test -p seo-core belge_ornegi_yaz -- --ignored --nocapture`
    # Elle yazılmış bir örnek zamanla gerçek çıktıdan sapardı (bu projede üç kez ölçüldü).
    # Belgenin DOĞRULUĞUNU `quote_html`in Rust testleri tutuyor; buradaki kopya yalnızca
    # modalın düzeni, kopyalama ve yazdırma düğmeleri için.
    QUOTE_DOC = {"html": "<table cellpadding=\"0\" cellspacing=\"0\" border=\"0\" width=\"560\" style=\"width:560px;border-collapse:collapse;font-family:-apple-system,BlinkMacSystemFont,Segoe UI,Roboto,Helvetica,Arial,sans-serif;color:#1d1d1f;font-size:12.5px;line-height:1.55;\"><tr><td style=\"padding:0;\"><table cellpadding=\"0\" cellspacing=\"0\" border=\"0\" width=\"100%\" style=\"width:100%;border-collapse:collapse;\"><tr><td style=\"vertical-align:top;padding:0;\"><div style=\"font-size:15px;font-weight:650;letter-spacing:-0.01em;\">Kurumsal BT</div><div style=\"color:#86868b;font-size:11.5px;margin-top:1px;\">Teklif T-2026-001</div></td><td style=\"vertical-align:top;padding:0;text-align:right;color:#86868b;font-size:11.5px;\"><div>10.08.2026</div><div>Geçerlilik 25.08.2026</div></td></tr></table><div style=\"margin-top:18px;\"><span style=\"color:#86868b;font-size:11.5px;\">Sayın</span><div style=\"font-weight:600;\">Ahmet Yılmaz · Anadolu Yapı</div></div><table cellpadding=\"0\" cellspacing=\"0\" border=\"0\" width=\"100%\" style=\"width:100%;border-collapse:collapse;margin-top:16px;\"><thead><tr><th align=\"left\" style=\"text-align:left;padding:0 0 6px;border-bottom:1px solid #d2d2d7;font-size:10px;font-weight:600;letter-spacing:0.04em;text-transform:uppercase;color:#86868b;\">Kalem</th><th align=\"right\" style=\"text-align:right;padding:0 0 6px;border-bottom:1px solid #d2d2d7;font-size:10px;font-weight:600;letter-spacing:0.04em;text-transform:uppercase;color:#86868b;padding-left:10px;\">Adet</th><th align=\"right\" style=\"text-align:right;padding:0 0 6px;border-bottom:1px solid #d2d2d7;font-size:10px;font-weight:600;letter-spacing:0.04em;text-transform:uppercase;color:#86868b;padding-left:10px;\">Birim</th><th align=\"right\" style=\"text-align:right;padding:0 0 6px;border-bottom:1px solid #d2d2d7;font-size:10px;font-weight:600;letter-spacing:0.04em;text-transform:uppercase;color:#86868b;padding-left:10px;\">KDV</th><th align=\"right\" style=\"text-align:right;padding:0 0 6px;border-bottom:1px solid #d2d2d7;font-size:10px;font-weight:600;letter-spacing:0.04em;text-transform:uppercase;color:#86868b;padding-left:10px;\">Tutar</th></tr></thead><tbody><tr><td style=\"border-bottom:1px solid #f0f0f2;padding:8px 0;\">Bambu Lab P2S Combo</td><td align=\"right\" style=\"text-align:right;border-bottom:1px solid #f0f0f2;font-variant-numeric:tabular-nums;padding:8px 0 8px 10px;color:#86868b;\">1</td><td align=\"right\" style=\"text-align:right;border-bottom:1px solid #f0f0f2;font-variant-numeric:tabular-nums;padding:8px 0 8px 10px;color:#86868b;\">749,00</td><td align=\"right\" style=\"text-align:right;border-bottom:1px solid #f0f0f2;font-variant-numeric:tabular-nums;padding:8px 0 8px 10px;color:#86868b;\">%20</td><td align=\"right\" style=\"text-align:right;border-bottom:1px solid #f0f0f2;font-variant-numeric:tabular-nums;padding:8px 0 8px 10px;font-weight:600;\">749,00</td></tr><tr><td style=\"border-bottom:1px solid #f0f0f2;padding:8px 0;\">Lenovo ThinkCentre Neo 50q G5</td><td align=\"right\" style=\"text-align:right;border-bottom:1px solid #f0f0f2;font-variant-numeric:tabular-nums;padding:8px 0 8px 10px;color:#86868b;\">2</td><td align=\"right\" style=\"text-align:right;border-bottom:1px solid #f0f0f2;font-variant-numeric:tabular-nums;padding:8px 0 8px 10px;color:#86868b;\">949,00</td><td align=\"right\" style=\"text-align:right;border-bottom:1px solid #f0f0f2;font-variant-numeric:tabular-nums;padding:8px 0 8px 10px;color:#86868b;\">%20</td><td align=\"right\" style=\"text-align:right;border-bottom:1px solid #f0f0f2;font-variant-numeric:tabular-nums;padding:8px 0 8px 10px;font-weight:600;\">1.898,00</td></tr></tbody></table><table cellpadding=\"0\" cellspacing=\"0\" border=\"0\" width=\"100%\" style=\"width:100%;border-collapse:collapse;margin-top:14px;\"><tr><td width=\"100%\" style=\"width:100%;padding:0;\">&nbsp;</td><td style=\"padding:0;vertical-align:top;white-space:nowrap;\"><table cellpadding=\"0\" cellspacing=\"0\" border=\"0\" style=\"border-collapse:collapse;font-size:12px;\"><tr><td style=\"padding:3px 18px 3px 0;white-space:nowrap;color:#86868b;\">Ara toplam</td><td align=\"right\" style=\"text-align:right;font-variant-numeric:tabular-nums;padding:3px 0;white-space:nowrap;\">2.647,00 USD</td></tr><tr><td style=\"padding:3px 18px 3px 0;white-space:nowrap;color:#86868b;\">KDV %20 · 2.647,00</td><td align=\"right\" style=\"text-align:right;font-variant-numeric:tabular-nums;padding:3px 0;white-space:nowrap;\">529,40</td></tr><tr><td style=\"padding:3px 18px 3px 0;white-space:nowrap;border-top:1px solid #d2d2d7;padding-top:8px;\">Genel toplam</td><td align=\"right\" style=\"text-align:right;font-variant-numeric:tabular-nums;padding:3px 0;white-space:nowrap;border-top:1px solid #d2d2d7;padding-top:8px;font-weight:660;font-size:14px;\">3.176,40 USD</td></tr></table></td></tr></table><div style=\"margin-top:10px;white-space:pre-wrap;font-size:11.5px;color:#86868b;\">Fiyatlarımız 15 gün geçerlidir. Teslim: stoktan 2 iş günü.</div></td></tr></table>", "text": "Kurumsal BT\nTeklif T-2026-001 · 10.08.2026\nGeçerlilik: 25.08.2026\nSayın Ahmet Yılmaz · Anadolu Yapı\n\nBambu Lab P2S Combo — 1 x 749,00 (KDV %20) = 749,00 USD\nLenovo ThinkCentre Neo 50q G5 — 2 x 949,00 (KDV %20) = 1.898,00 USD\n\nAra toplam: 2.647,00 USD\nKDV %20 (2.647,00 üzerinden): 529,40\nGenel toplam: 3.176,40 USD\n\nFiyatlarımız 15 gün geçerlidir. Teslim: stoktan 2 iş günü.\n"}

    # Canonical hedefi araması: gerçek feed ürünleri üzerinde, terimle süzülerek.
    live_json = json.dumps(live, ensure_ascii=False)

    # `?empty=1` → analiz hiç çalıştırılmamış gibi davranır. Boş durumları elle test etmek
    # mümkün değil: sayfalar açılırken önbelleği yeniden yüklüyor, store'u dışarıdan
    # sıfırlamak yetmiyor. Bu senaryo önemli — v0.5.9'da boş ekran bir saha hatasıydı.
    # Asistan akışı taklidi: gerçek Gemini çağrısı yapmadan arayüzü doğrulamak için.
    # Önce "düşünüyor" olayları, sonra parça parça cevap — canlı ölçümdeki sıranın aynısı
    # (Gemma önce muhakemesini yayınlıyor, cevap sonda geliyor).
    _dec0, _dec1 = (report.get("decay") or [{}, {}])[:2]
    _opp0 = (report.get("opportunities") or [{}])[0]
    # EOL kırpmasının (25 satır) ÖTESİNDEN bir satır — derin link testinin anlamlı olması için.
    _eol_ornek = (report.get("eol") or [{}])[40] if len(report.get("eol") or []) > 40 else (report.get("eol") or [{}])[0]

    # Bugün kuyruğu için örnek maddeler (Faz K).
    TODAY_Q = {
        "analyzed_at": "2026-08-07T21:27:10",
        "hidden": 0,
        "done_count": 0,
        # Uçuş süzgeci (2026-08-09): yapıldı ama sonucu 28 gün sonra görünecek işler.
        "in_flight": 81,
        "review_ready_at": "2026-09-15",
        "bucket_counts": [
            {"bucket": "urgent", "label": "Acil", "candidates": 10},
            {"bucket": "leverage", "label": "Yüksek kaldıraç", "candidates": 49},
            {"bucket": "leak", "label": "Kaçak trafik", "candidates": 2115},
            {"bucket": "review", "label": "Sonuç kontrolü", "candidates": 0},
            {"bucket": "upkeep", "label": "Bakım", "candidates": 52},
            {"bucket": "contact", "label": "Müşteri", "candidates": 2},
        ],
        "items": [
            # ⚠️ Kaçak maddesi GERÇEK rapordan ve bilerek **40. satırdan** alınıyor: EOL ekranı
            # ilk 25 satırı çiziyor, yani bu madde odak mekanizmasının kırpmayı yükseltme
            # yolunu gerçekten sınıyor. Uydurma bir URL kullanılsaydı derin link hiç
            # eşleşmez ve test yanlış yere "çalışıyor" derdi.
            # Müşteri maddeleri (Faz C): biri gecikmiş, biri bugünkü. Skor 40 + gecikme.
            {"reference": {"kind": "contact", "ref": "1"},
             "bucket": "contact", "title": "Ahmet Yılmaz · Kurumsal BT",
             "reason": "4 gündür bekliyor — fiyat teklifi verilecek",
             "clicks": 4, "score": 44, "page": "contacts", "focus_id": "1",
             "minutes": 5, "minutes_measured": False, "also": [], "done": False},
            {"reference": {"kind": "contact", "ref": "2"},
             "bucket": "contact", "title": "Zeynep Kaya · Anadolu Yapı",
             "reason": "bugün dönülecek — numune sonucu sorulacak",
             "clicks": 0, "score": 40, "page": "contacts", "focus_id": "2",
             "minutes": 5, "minutes_measured": False, "also": [], "done": False},
            {"reference": {"kind": "page", "ref": _eol_ornek["slug"]},
             "bucket": "leak", "title": _eol_ornek["slug"],
             "reason": f"{round(_eol_ornek['clicks'])} tıklama satın alınamayan bir sayfaya "
                       f"gidiyor (konum {_eol_ornek['position']:.1f})",
             "clicks": _eol_ornek["clicks"], "score": round(_eol_ornek["clicks"] * 0.6, 1),
             "page": "eol", "focus_id": _eol_ornek["url"],
             "minutes": 1, "minutes_measured": False, "also": [], "done": False},
            # ⚠️ Bakım maddeleri de GERÇEK rapordan: uydurma SKU'larla derin link hiçbir satıra
            # denk gelmiyordu ve doğrulama yanlışlıkla "ekran doğru" deyip geçiyordu.
            {"reference": {"kind": "product", "ref": _dec0["sku"]},
             "bucket": "upkeep", "title": _dec0["name"],
             "reason": f"tıklama {round(_dec0['clicks_before'])}→{round(_dec0['clicks_now'])}, "
                       f"konum {_dec0['position_before']:.1f}→{_dec0['position_now']:.1f}",
             "clicks": _dec0["clicks_lost"], "score": round(_dec0["clicks_lost"] * 0.8, 1),
             # Bu madde ÖLÇÜLMÜŞ süreyle: ekranda "≈" olmadan ve biraz belirgin yazmalı.
             "page": "decay", "focus_id": _dec0["sku"], "minutes": 4, "minutes_measured": True, "also": [], "done": False},
            {"reference": {"kind": "product", "ref": _dec1["sku"]},
             "bucket": "upkeep", "title": _dec1["name"],
             "reason": f"tıklama {round(_dec1['clicks_before'])}→{round(_dec1['clicks_now'])}, "
                       f"konum {_dec1['position_before']:.1f}→{_dec1['position_now']:.1f}",
             "clicks": _dec1["clicks_lost"], "score": round(_dec1["clicks_lost"] * 0.8, 1),
             "page": "decay", "focus_id": _dec1["sku"], "minutes": 2,
             # Birden çok kovada görünen ürünün "ayrıca" satırları (ölçüm: 12 ürün 2+ kovada).
             "also": ["mağazaya gönderildikten sonra feed değişti (açıklama) — canlıdaki metin bayat",
                      "konum 9.3, 1074 gösterim ama 11 tıklama — 19 tıklama kaçıyor"], "done": False},
            {"reference": {"kind": "product", "ref": "NB.LEN.21SX007CTX"},
             "bucket": "urgent", "title": "Lenovo ThinkPad E14 G7 21SX007CTX U7-255H 16G 512G",
             "reason": "mağazaya gönderildikten sonra feed değişti (açıklama) — canlıdaki metin bayat",
             "clicks": 17, "score": 57, "page": "products", "focus_id": "NB.LEN.21SX007CTX",
             "minutes": 2,
             "also": ["konum 6.8, 891 gösterim ama 17 tıklama — 19 tıklama kaçıyor"], "done": False},
            {"reference": {"kind": "product", "ref": "ADP.ARB.R9M79A"},
             "bucket": "urgent", "title": "Aruba R9M79A Instant On 12V/18W RW Güç Adaptörü",
             "reason": "mağazaya gönderildikten sonra feed değişti (açıklama) — canlıdaki metin bayat",
             "clicks": 0, "score": 40, "page": "products", "focus_id": "ADP.ARB.R9M79A",
             "minutes": 2, "minutes_measured": False, "also": [], "done": False},
            {"reference": {"kind": "product", "ref": _opp0["sku"]},
             "bucket": "leverage", "title": _opp0["name"],
             "reason": f"konum {_opp0['position']:.1f}, {round(_opp0['impressions'])} gösterim ama "
                       f"{round(_opp0['clicks'])} tıklama — {round(_opp0['missed_clicks'])} tıklama kaçıyor",
             "clicks": _opp0["missed_clicks"], "score": round(_opp0["missed_clicks"], 1),
             "page": "opportunities", "focus_id": _opp0["sku"], "minutes": 2, "minutes_measured": False, "also": [], "done": False},
        ],
    }
    # ⚠️ Arka uç kuyruğu skora göre sıralı döndürüyor; stub elle yazıldığı için burada
    # sıralanıyor. Sırasız bir önizleme ekranı olduğundan farklı gösterirdi.
    TODAY_Q["items"].sort(key=lambda i: -i["score"])

    fake_answer = (
        "Bu ekrandaki veriye göre en çok tıklama kaçıran üç sayfa şunlar:\n"
        "- **Ergotron WorkFit-T 33-397-062** — 474 gösterim, 2 tıklama, konum 4.1. "
        "Konum iyi ama `CTR` çok düşük; sorun sıralamada değil başlıkta.\n"
        "- **Lenovo ThinkPad E16 G3** — 850 gösterim, 33 tıklama.\n"
        "- **Logitech M280** — 1207 gösterim, 3 tıklama.\n\n"
        "Önce ilkine bakın: konumu ilk sayfada olduğu için başlık ve açıklama düzeltmesi "
        "doğrudan tıklamaya dönüşür. Bunu **Ürünler** ekranından üretebilirsiniz."
    )

    # `?nav=eol` → uygulama doğrudan o ekranda açılır. Başsız ekran görüntüsü almak için
    # gerekli (tıklama yok): tasarım promptlarına mevcut ekranların görüntüsü ekleniyor.
    nav_stub = (
        "<script>\n"
        "(() => {\n"
        "  const p = new URLSearchParams(location.search).get('nav');\n"
        "  if (!p) return;\n"
        "  // Store hazır olduğunda sayfayı değiştir; Pinia mağazası pencerede değil, bu yüzden\n"
        "  // kenar çubuğundaki düğmeye tıklanıyor — gerçek gezinme yolunun aynısı.\n"
        "  const tik = () => {\n"
        "    const b = document.querySelector(`[data-nav=\"${p}\"]`);\n"
        "    if (b) { b.click(); return true; }\n"
        "    return false;\n"
        "  };\n"
        "  // `&tema=koyu` → koyu temada görüntü al (tasarım promptları iki temayı da ister).\n"
        "  const koyu = new URLSearchParams(location.search).get('tema') === 'koyu';\n"
        "  const temaSec = () => {\n"
        "    if (!koyu) return true;\n"
        "    const b = [...document.querySelectorAll('button')].find(x => x.textContent.trim() === 'Koyu');\n"
        "    if (b) { b.click(); return true; }\n"
        "    return false;\n"
        "  };\n"
        "  const t = setInterval(() => { if (tik() && temaSec()) clearInterval(t); }, 120);\n"
        "  setTimeout(() => clearInterval(t), 6000);\n"
        "})();\n"
        "</script>\n"
    )

    stub = (
        "<script>\n"
        "// Tauri IPC taklidi — yalnızca harness için. Uygulama bundle'ı gerçek.\n"
        "window.__TAURI_INTERNALS__ = {\n"
        "  invoke: (cmd, args) => {\n"
        # Tauri eklentileri de aynı köprüden geçiyor ("plugin:dialog|open"). Dosya seçiciyi
        # taklit ediyoruz ki CSV içe aktarma akışı harness'ta baştan sona görülebilsin.
        "    if (cmd === 'plugin:dialog|open') return Promise.resolve('/tmp/musteriler.csv');\n"
        "    if (cmd === 'plugin:dialog|save') return Promise.resolve('/tmp/cikti.csv');\n"
        # ⚠️ `</` kaçışı ZORUNLU: JSON-LD örneği gövdesinde `</script>` geçiyor ve HTML
        # ayrıştırıcısı bunu gördüğü anda DIŞ script'i kapatıyor → sayfa sessizce boş açılıyor.
        # (Bu tuzağa bir kez düşüldü; `seo_core::jsonld::render_script` aynı kaçışı yapıyor.)
        f"    const H = {json.dumps(handlers, ensure_ascii=False).replace('</', '<\\/')};\n"
        # Bugün kuyruğu hem `get_today_queue` hem odak seansı tarafından okunuyor; tek yerde.
        f"    const TQ = {json.dumps(TODAY_Q, ensure_ascii=False)};\n"
        f"    const CT = {json.dumps(CONTACTS, ensure_ascii=False)};\n"
        f"    const CE = {json.dumps(CONTACT_EVENTS, ensure_ascii=False)};\n"
        f"    const CP = {json.dumps(CONTACT_PRODUCTS, ensure_ascii=False)};\n"
        f"    const CSVP = {json.dumps(CSV_PREVIEW, ensure_ascii=False)};\n"
        f"    const QT = {json.dumps(QUOTES, ensure_ascii=False)};\n"
        f"    const QDOC = {json.dumps(QUOTE_DOC, ensure_ascii=False)};\n"
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
        # `?changed=1` → feed değişikliği bayrağı benzetimi (K2). Gerçek veride bayrak ancak
        # tedarikçi bir ürünü değiştirdiğinde çıkıyor; uyarıyı beklemeden görmek için.
        # `?changed=1` karşılaştırma verisi; `?nosnap=1` ile "onay kaydı yok" hâli denenir.
        f"    const DIFF = {json.dumps({'has_snapshot': True, 'changed_fields': ['ad', 'açıklama', 'görseller'], 'fields': [{'field': 'ad', 'old': 'HPE Aruba R8W31A Instant On 802.3af 15.4W POE Midspan Injector', 'new': 'HPE Aruba R8W31A Instant On 802.3af 15.4W PoE Midspan Injector (Yeni Nesil)'}, {'field': 'açıklama', 'old': 'Bu ürün küçük ofisler için tasarlanmış tek portlu bir PoE enjektörüdür. 802.3af standardını destekler ve 15.4W güç sağlar.', 'new': 'HPE Aruba Instant On R8W31A, küçük ve orta ölçekli işletmeler için tasarlanmış tek portlu Power over Ethernet enjektörüdür. IEEE 802.3af standardına tam uyumludur, bağlı cihaza 15.4W güç sağlar ve Gigabit hızını korur. Kurulum için ek yapılandırma gerektirmez.'}], 'images_old': gallery[:3], 'images_new': gallery[1:] or gallery}, ensure_ascii=False)};\n"
        "    if (new URLSearchParams(location.search).has('changed')) {\n"
        "      const NOTE = 'ad, açıklama';\n"
        "      if (cmd === 'list_products') return Promise.resolve(H.list_products.map(\n"
        "        r => ({ ...r, feed_changed: NOTE, meta_done: true, details_done: true,\n"
        "                overall: 'tamamlandi' })));\n"
        "      if (cmd === 'get_product') return Promise.resolve({ ...H.get_product,\n"
        "        feed_changed: NOTE, meta_status: 'done', details_status: 'done' });\n"
        "      if (cmd === 'mark_feed_reviewed') return Promise.resolve(null);\n"
        "      if (cmd === 'get_feed_diff') return Promise.resolve(\n"
        "        new URLSearchParams(location.search).has('nosnap')\n"
        "          ? { has_snapshot: false, changed_fields: ['görseller'], fields: [],\n"
        "              images_old: [], images_new: DIFF.images_new }\n"
        "          : DIFF);\n"
        "    }\n"
        # `?nosonuc=1` → ölçüm geçmişi hiç yokken Genel Bakış'ın hâli (tohumlama düğmesi).
        "    if (new URLSearchParams(location.search).has('nosonuc')) {\n"
        "      if (cmd === 'get_outcome_summary') return Promise.resolve(\n"
        "        { snapshots: 0, oldest_window: '', measured_events: 0, improved: 0, flat: 0,\n"
        "          worse: 0, measuring: 0, insufficient: 0, net_delta_clicks: 0 });\n"
        "      if (cmd === 'get_outcome_badges') return Promise.resolve([]);\n"
        "      if (cmd === 'seed_metric_history') return Promise.resolve(\n"
        "        { snapshots_added: 12, rows_written: 35062, events_backfilled: 72, skipped_existing: 0 });\n"
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
        # Halef → canonical akışı (Satışta olmayanlar). Gerçek akış modeli ve IdeaSoft'u
        # çağırıyor; burada taklit ediliyor ki modal zinciri ağsız denenebilsin.
        # ⚠️ `?halefyok=1` → model uygun halef bulamamış hâli (hedef SEÇME modali yolu).
        # Bugün kuyruğu (Faz K). Maddeler gerçek `kuyruk_real` çıktısının şeklinde:
        # dört kova, birleştirilmiş "ayrıca" satırları, tıklamasız bir acil madde.
        # ⚠️ `?boskuyruk=1` → "bugün için seçilecek iş yok" hâli.
        "    if (cmd === 'get_today_queue') {\n"
        "      const Q = TQ;\n"
        "      if (new URLSearchParams(location.search).has('boskuyruk'))\n"
        "        return Promise.resolve({ ...Q, items: [], hidden: 2 });\n"
        "      const gizli = (window.__GIZLI__ = window.__GIZLI__ || []);\n"
        "      const yapildi = (window.__YAPILDI__ = window.__YAPILDI__ || []);\n"
        "      if (new URLSearchParams(location.search).has('hepsiyapildi'))\n"
        "        return Promise.resolve({ ...Q, hidden: 1, done_count: Q.items.length,\n"
        "          items: Q.items.map(i => ({ ...i, done: true })) });\n"
        "      const kalan = Q.items.filter(i => !gizli.includes(i.reference.ref))\n"
        "        .map(i => ({ ...i, done: yapildi.includes(i.reference.ref) }));\n"
        "      return Promise.resolve({ ...Q, hidden: gizli.length,\n"
        "        done_count: kalan.filter(i => i.done).length, items: kalan });\n"
        "    }\n"
        "    if (cmd === 'list_contacts') {\n"
        "      const bos = new URLSearchParams(location.search).get('musteri') === '0';\n"
        "      if (bos) return Promise.resolve([]);\n"
        "      const t = (args.search || '').toLowerCase();\n"
        "      return Promise.resolve(CT.filter(c => (args.includeArchived || !c.archived)\n"
        "        && (!t || (c.name + c.company + c.email + c.phone).toLowerCase().includes(t))));\n"
        "    }\n"
        "    if (cmd === 'get_contact')\n"
        "      return Promise.resolve(CT.find(c => c.id === args.id) || CT[0]);\n"
        "    if (cmd === 'get_contact_events')\n"
        "      return Promise.resolve(CE[args.contactId] || []);\n"
        "    if (cmd === 'save_contact') return Promise.resolve(args.id || 9);\n"
        "    if (cmd === 'get_quote_defaults') return Promise.resolve(\n"
        "      { tax_rate: 20, valid_days: 15, seller: 'Kurumsal BT',\n"
        "        footer: 'Fiyatlarımız 15 gün geçerlidir. Teslim: stoktan 2 iş günü.' });\n"
        "    if (cmd === 'set_quote_defaults') return Promise.resolve(null);\n"
        # Kazanma/kaybetme özeti — kayıp nedenleri raporlanabilsin (fazın bitiş şartı).
        "    if (cmd === 'quote_summary') return Promise.resolve({\n"
        "      open_count: 1, won_count: 4, lost_count: 3,\n"
        "      won_totals: [['USD', 18450.0]],\n"
        "      lost_reasons: [['fiyat', 2], ['termin', 1]] });\n"
        "    if (cmd === 'quotes_of_contact') return Promise.resolve(QT.slice(0, 2));\n"
        # ⚠️ Belge stub'ı GERÇEK üretici tarafından üretildi (aşağıda), elle yazılmadı:
        # maliyet sızıntısı testi ancak gerçek çıktı üzerinde anlamlı.
        "    if (cmd === 'render_quote') return Promise.resolve(QDOC);\n"
        "    if (cmd === 'list_quotes') return Promise.resolve(\n"
        "      args.status ? QT.filter(q => q.status === args.status) : QT);\n"
        "    if (cmd === 'get_quote') return Promise.resolve(QT.find(q => q.id === args.id) || QT[0]);\n"
        "    if (cmd === 'create_quote') return Promise.resolve(1);\n"
        "    if (cmd === 'save_quote' || cmd === 'delete_quote' || cmd === 'update_quote_item'\n"
        "        || cmd === 'delete_quote_item' || cmd === 'add_quote_item_manual'\n"
        "        || cmd === 'set_quote_status') return Promise.resolve(null);\n"
        # ⚠️ USD teklifte EUR ürün kur olmadan eklenemiyor — hata metni ekranda görünmeli.
        "    if (cmd === 'add_quote_item_from_catalog') return Promise.reject(\n"
        "      'TP-Link Switch için USD cinsinden fiyat hesaplanamadı. Ürünün para birimi EUR — "
        "USD teklifte kur girmeniz gerekiyor.');\n"
        "    if (cmd === 'snapshot_quote') return Promise.resolve(2);\n"
        "    if (cmd === 'list_contact_tags')\n"
        "      return Promise.resolve([...new Set(CT.flatMap(c => c.tags))]);\n"
        "    if (cmd === 'get_contact_products')\n"
        "      return Promise.resolve(CP[args.contactId] || []);\n"
        "    if (cmd === 'contacts_of_product')\n"
        "      return Promise.resolve([{ sku: args.sku, name: 'Ahmet Yılmaz · Kurumsal BT',\n"
        "        contact_id: 1, at: '' }]);\n"
        "    if (cmd === 'set_contact_tags' || cmd === 'link_contact_product'\n"
        "        || cmd === 'unlink_contact_product' || cmd === 'set_silence_days')\n"
        "      return Promise.resolve(null);\n"
        # Sessizlik eşiği: `?sessizlik=1` → yeterli veri birikmiş, öneri gösteriliyor.
        "    if (cmd === 'get_silence_state') return Promise.resolve(\n"
        "      new URLSearchParams(location.search).has('sessizlik')\n"
        "        ? { days: 0, suggestion: 25, sample_contacts: 7 }\n"
        "        : { days: 0, suggestion: null, sample_contacts: 2 });\n"
        "    if (cmd === 'preview_contact_csv') return Promise.resolve(CSVP);\n"
        "    if (cmd === 'import_contacts_csv') return Promise.resolve(\n"
        "      { added: 38, updated: 4, skipped: 0, skip_reason: '' });\n"
        "    if (cmd === 'archive_contact' || cmd === 'add_contact_event')\n"
        "      return Promise.resolve(null);\n"
        "    if (cmd === 'dismiss_queue_item') {\n"
        "      (window.__GIZLI__ = window.__GIZLI__ || []).push(args.reference);\n"
        "      return Promise.resolve(null);\n"
        "    }\n"
        # ⚠️ "Yapıldı" maddeyi SİLMİYOR, done=true yapıyor — gün bitebilsin diye.
        # Odak seansı (Faz S). ⚠️ `?seans=1` seans SÜRÜYOR hâli · `?seansozet=1` özet modali.
        # Sayaç gerçek: başlangıç damgası "şimdi - 3 dk" veriliyor ki çubuk canlı görünsün.
        # ⚠️ `?hepsiyapildi=1` → kullanıcının yaşadığı durum: günün 10 işi de bitmiş.
        # Düğme pasif olmalı, "Seans bitti · 0 iş" modali ÇIKMAMALI.
        "    if (cmd === 'has_lockable_item') {\n"
        "      if (new URLSearchParams(location.search).has('hepsiyapildi'))\n"
        "        return Promise.resolve(false);\n"
        "      const g = window.__GIZLI__ || [], y = window.__YAPILDI__ || [];\n"
        "      return Promise.resolve(TQ.items.some(i => !g.includes(i.reference.ref)\n"
        "        && !y.includes(i.reference.ref)));\n"
        "    }\n"
        "    if (cmd === 'start_focus_session'\n"
        "        && new URLSearchParams(location.search).has('hepsiyapildi'))\n"
        "      return Promise.resolve({ session_id: null, started_at: '', planned_minutes: 25,\n"
        "        break_minutes: 5, locked: null, done_count: 0, skipped_count: 0 });\n"
        "    if (cmd.startsWith('start_focus') || cmd === 'get_focus_state'\n"
        "        || cmd === 'resolve_focus_item') {\n"
        "      const S = (window.__SEANS__ = window.__SEANS__ || { i: 0, done: 0, skipped: 0 });\n"
        "      const acik = new URLSearchParams(location.search).has('seans')\n"
        "        || cmd === 'start_focus_session' || S.basladi;\n"
        "      if (cmd === 'start_focus_session') S.basladi = true;\n"
        "      if (cmd === 'resolve_focus_item') {\n"
        "        if (args.outcome === 'done') S.done++; else S.skipped++;\n"
        "        S.i++;\n"
        "      }\n"
        "      if (!acik) return Promise.resolve({ session_id: null, started_at: '',\n"
        "        planned_minutes: 25, break_minutes: 5, locked: null, done_count: 0, skipped_count: 0 });\n"
        "      const gizliS = window.__GIZLI__ || [];\n"
        "      const yapildiS = window.__YAPILDI__ || [];\n"
        "      const kalan = TQ.items.filter(i => !gizliS.includes(i.reference.ref)\n"
        "        && !yapildiS.includes(i.reference.ref));\n"
        "      const it = kalan[S.i];\n"
        # ⚠️ YEREL saat damgası: üretimdeki `now_str()` de yerel yazıyor ve JS ofsetsiz
        # damgayı yerel okuyor. `toISOString()` (UTC) kullanılınca sayaç saat farkı kadar
        # saptı — harness'te 158 dakika gösterdi (ölçüldü).
        "      const d = new Date(Date.now() - 3 * 60000);\n"
        "      const iki = n => String(n).padStart(2, '0');\n"
        "      const bas = `${d.getFullYear()}-${iki(d.getMonth()+1)}-${iki(d.getDate())}`\n"
        "        + `T${iki(d.getHours())}:${iki(d.getMinutes())}:${iki(d.getSeconds())}`;\n"
        # ⚠️ Kilitlenecek iş kalmadıysa arka uç SEANSI KAPATIYOR (session_id null döner).
        # Stub bunu taklit etmezse çubuk boş kilitle asılı kalır — ilk sürümde öyle oldu.
        "      if (!it) return Promise.resolve({ session_id: null, started_at: '',\n"
        "        planned_minutes: 25, break_minutes: 5, locked: null,\n"
        "        done_count: S.done, skipped_count: S.skipped });\n"
        "      return Promise.resolve({ session_id: 1, started_at: bas, planned_minutes: 25,\n"
        "        break_minutes: 5, done_count: S.done, skipped_count: S.skipped,\n"
        "        locked: { kind: it.reference.kind, reference: it.reference.ref,\n"
        "          bucket: it.bucket, title: it.title, reason: it.reason, page: it.page,\n"
        "          focus_id: it.focus_id, started_at: bas } });\n"
        "    }\n"
        "    if (cmd === 'end_focus_session') {\n"
        "      const S = (window.__SEANS__ = window.__SEANS__ || { done: 0, skipped: 0 });\n"
        "      window.__SEANS__ = { i: 0, done: 0, skipped: 0 };\n"
        "      return Promise.resolve({ done_count: S.done || 3, skipped_count: S.skipped || 1,\n"
        "        minutes: 12.4, ended_reason: args.reason || 'stopped',\n"
        "        buckets: [['upkeep', 2], ['urgent', 1]] });\n"
        "    }\n"
        "    if (cmd === 'get_focus_calibration') return Promise.resolve([\n"
        "      { bucket: 'upkeep', samples: 7, minutes: 4 },\n"
        "      { bucket: 'urgent', samples: 3, minutes: null }]);\n"
        "    if (cmd === 'complete_queue_item') {\n"
        "      (window.__YAPILDI__ = window.__YAPILDI__ || []).push(args.reference);\n"
        "      return Promise.resolve(null);\n"
        "    }\n"
        "    if (cmd === 'restore_queue_item') {\n"
        "      window.__YAPILDI__ = (window.__YAPILDI__ || []).filter(r => r !== args.reference);\n"
        "      window.__GIZLI__ = (window.__GIZLI__ || []).filter(r => r !== args.reference);\n"
        "      return Promise.resolve(null);\n"
        "    }\n"
        "    if (cmd === 'restore_queue_items') { window.__GIZLI__ = []; return Promise.resolve(null); }\n"
        # EOL karar deposu (Faz D). ⚠️ `?kararlar=1` → bazı satırlar karar verilmiş.
        "    if (cmd === 'get_eol_decisions') {\n"
        "      const K = (window.__KARAR__ = window.__KARAR__ || (\n"
        "        new URLSearchParams(location.search).has('kararlar')\n"
        "          ? [{ slug: (H.get_opportunity_cache.eol[0]||{}).slug, url: '', action: 'redirect_301',\n"
        "               target_slug: 'lenovo-thinkpad-t14-g6', target_sku: 'NB.LEN.T14G6',\n"
        "               source: 'ai', decided_at: '2026-08-08T12:00:00', exported_at: null },\n"
        "             { slug: (H.get_opportunity_cache.eol[1]||{}).slug, url: '', action: 'keep',\n"
        "               target_slug: null, target_sku: null, source: 'manual',\n"
        "               decided_at: '2026-08-08T12:01:00', exported_at: null }]\n"
        "          : []));\n"
        "      return Promise.resolve(K);\n"
        "    }\n"
        "    if (cmd === 'save_eol_decision') {\n"
        "      const K = (window.__KARAR__ = window.__KARAR__ || []);\n"
        "      K.push({ slug: args.slug, url: args.url, action: args.action,\n"
        "        target_slug: args.targetSlug, target_sku: args.targetSku, source: args.source,\n"
        "        decided_at: new Date().toISOString().slice(0,19), exported_at: null });\n"
        "      return Promise.resolve(null);\n"
        "    }\n"
        "    if (cmd === 'delete_eol_decision') {\n"
        "      window.__KARAR__ = (window.__KARAR__ || []).filter(k => k.slug !== args.slug);\n"
        "      return Promise.resolve(null);\n"
        "    }\n"
        "    if (cmd === 'export_redirect_csv')\n"
        "      return Promise.resolve({ decided_rows: (window.__KARAR__||[]).length,\n"
        "        undecided_rows: 673, bytes: 84210, path: args.path });\n"
        "    if (cmd === 'suggest_eol_successor') {\n"
        "      const yok = new URLSearchParams(location.search).has('halefyok');\n"
        "      return new Promise(r => setTimeout(() => r(yok\n"
        "        ? { sku: '', name: '', url: '', reason: 'Katalogda yeterince yakın bir ürün yok.' }\n"
        "        : { sku: 'NB.LEN.21QC00CKTX', name: 'Lenovo ThinkPad T14 G6 21QC00CKTX U7-255U 32GB 1TB DOS 14\\'\\'',\n"
        "            url: 'https://ornek.example/lenovo-thinkpad-t14-g6-21qc00cktx',\n"
        "            reason: 'Aynı seri, bir üst nesil.' }), 900));\n"
        "    }\n"
        "    if (cmd === 'preview_canonical') {\n"
        "      return Promise.resolve({ eolSlug: args.eolSlug, product_id: 4211,\n"
        "        product_name: 'Lenovo ThinkPad T14 G6 21QC00CKTX U7-255U 32GB 1TB DOS 14\\'\\'',\n"
        "        target_slug: args.targetSlug, current_canonical: '', new_canonical: args.targetSlug });\n"
        "    }\n"
        "    if (cmd === 'apply_canonical') return Promise.resolve(null);\n"
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
    # nav_stub bundle'dan SONRA çalışmalı (kenar çubuğu çizilmiş olmalı) → gövde sonuna.
    html = html.replace('<script type="module"', stub + '<script type="module"', 1)
    out.write_text(html.replace("</body>", nav_stub + "</body>", 1))
    counts = {k: len(v) for k, v in report.items() if isinstance(v, list)}
    print(f"{out} hazır · {counts}")
    tdz_uyar()


def tdz_uyar():
    """🔴 Aynı hata ÜÇ KEZ yapıldı: `watch(..., { immediate: true })` kurulum sırasında hemen
    çalışıp, kendisinden SONRA tanımlanmış bir `ref`e yazıyor → geçici ölüm bölgesi (TDZ).
    Üçünde de **ekran doğru çiziliyordu**, hatayı yalnızca konsol gösterdi:

      · `ContactCard`  → Müşteriler ekranı bomboş açıldı (kullanıcı bildirdi)
      · `QuoteEditor`  → arama alanı sıfırlanmıyordu
      · `QuoteEditor`  → geri alma yedeği

    ⚠️ Bu kontrolün İLK İKİ sürümü hatayı yakalayamadı ve bunu ancak **hatayı bilerek geri
    koyup deneyince** gördüm: (1) `ref<Jenerik>()` biçimini kaçırıyordu, (2) `immediate: true`
    ifadesini kendi doküman YORUMUNDA buluyordu. Sınanmamış bir koruma, koruma değil.

    Uyarı üretir, derlemeyi durdurmaz: sezgisel bir kontrol, yanlış alarm verebilir.
    """
    import re

    def yorumsuz(m: str) -> str:
        """Blok ve satır yorumlarını boşlukla değiştirir — konumlar korunur."""
        m = re.sub(r"/\*.*?\*/", lambda x: " " * len(x.group()), m, flags=re.S)
        return re.sub(r"//[^\n]*", lambda x: " " * len(x.group()), m)

    bulgular = []
    for f in sorted(pathlib.Path("src").rglob("*.vue")):
        metin = yorumsuz(f.read_text(encoding="utf-8"))
        for im in re.finditer(r"immediate:\s*true", metin):
            # İzleyicinin gövdesi: kendisinden önceki en yakın `watch(` ile `immediate` arası.
            w = metin.rfind("watch(", 0, im.start())
            if w < 0:
                continue
            govde = metin[w : im.start()]
            for yazilan in set(re.findall(r"\b(\w+)\.value\s*=", govde)):
                # Bu değişken izleyiciden SONRA mı tanımlanmış?
                if re.search(
                    rf"^const {re.escape(yazilan)} = (?:ref|reactive)[^(]*\(", metin[w:], re.M
                ):
                    bulgular.append(f"{f}: `{yazilan}`")

    if bulgular:
        print("\n⚠️  TDZ riski — `immediate` izleyici, kendisinden SONRA tanımlanan ref'e yazıyor:")
        for b in sorted(set(bulgular)):
            print(f"   · {b}")
        print("   → ref tanımlarını `watch`tan ÖNCEYE alın (brain.md 0b4).")


if __name__ == "__main__":
    os.chdir(pathlib.Path(__file__).resolve().parent.parent)
    main()

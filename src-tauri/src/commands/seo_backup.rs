use super::AppState;
use rusqlite::{Connection, OpenFlags};
use serde::Serialize;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use tauri::State;

const FIELDS: &[(&str, &str)] = &[
    ("target_keyword", "Hedef kelime"),
    ("draft_title", "Sayfa başlığı"),
    ("draft_descriptions", "Meta açıklama"),
    ("draft_keywords", "Meta anahtar kelimeleri"),
    ("draft_search_keywords", "Site içi arama kelimeleri"),
    ("draft_details", "Ürün açıklaması"),
    ("meta_status", "Meta durumu"),
    ("details_status", "Açıklama durumu"),
    ("tech_source_text", "Teknik tablo kaynak metni"),
    ("tech_specs_json", "Teknik tablo"),
    ("tech_status", "Teknik tablo durumu"),
    ("meta_history_json", "Meta geçmişi"),
    ("details_history_json", "Açıklama geçmişi"),
    ("tech_history_json", "Teknik tablo geçmişi"),
    ("research_json", "SEO araştırması"),
    ("image_check_json", "Görsel kontrolü"),
    ("image_check_fp", "Görsel kontrol damgası"),
    ("ideasoft_product_id", "IdeaSoft ürün kimliği"),
    ("ideasoft_pushed_at", "IdeaSoft gönderim tarihi"),
    ("ideasoft_seo_rule", "IdeaSoft SEO puanı"),
    ("meta_model", "Meta üretim modeli"),
    ("details_model", "Açıklama üretim modeli"),
    ("tech_model", "Teknik tablo üretim modeli"),
    ("reviewed_fp", "Ürün onay damgası"),
    ("reviewed_facts_json", "Onaylanan ürün bilgileri"),
];

#[derive(Serialize)]
struct Difference {
    field: &'static str,
    local: Value,
    backup: Value,
}

#[derive(Serialize)]
struct Conflict {
    sku: String,
    differences: Vec<Difference>,
}

#[derive(Serialize)]
pub struct SeoBackupPreview {
    local_only: usize,
    backup_only: usize,
    identical: usize,
    conflicts: Vec<Conflict>,
}

type Drafts = BTreeMap<String, Map<String, Value>>;

#[derive(Serialize)]
struct ImportRow {
    sku: String,
    draft: Map<String, Value>,
}

#[derive(Serialize)]
pub struct SeoBackupComparison {
    #[serde(flatten)]
    preview: SeoBackupPreview,
    local_rows: Vec<ImportRow>,
    backup_rows: Vec<ImportRow>,
}

#[tauri::command]
pub fn compare_seo_backup(
    state: State<'_, AppState>,
    path: String,
) -> Result<SeoBackupComparison, String> {
    let mut backup = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| format!("SQLite yedeği açılamadı: {e}"))?;
    let tx = backup
        .transaction()
        .map_err(|e| format!("SQLite yedeği okunamadı: {e}"))?;
    let other = draft_rows(&tx)?;
    let local = draft_rows(&state.conn.lock().unwrap())?;
    let preview = compare(&local, &other);
    let import_rows = |drafts: Drafts| {
        drafts
            .into_iter()
            .map(|(sku, draft)| ImportRow { sku, draft })
            .collect()
    };
    Ok(SeoBackupComparison {
        preview,
        local_rows: import_rows(local),
        backup_rows: import_rows(other),
    })
}

fn draft_rows(conn: &Connection) -> Result<Drafts, String> {
    let mut schema = conn
        .prepare("PRAGMA table_info(seo_status)")
        .map_err(|e| e.to_string())?;
    let columns = schema
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|e| e.to_string())?
        .collect::<Result<BTreeSet<_>, _>>()
        .map_err(|e| e.to_string())?;
    if !columns.contains("sku") {
        return Err("Bu dosyada SKU içeren bir SEO çalışma tablosu bulunamadı.".into());
    }

    let projection: Vec<&str> = FIELDS
        .iter()
        .map(|(column, _)| {
            if columns.contains(*column) {
                *column
            } else {
                "NULL"
            }
        })
        .collect();
    let mut query = conn
        .prepare(&format!(
            "SELECT sku, {} FROM seo_status",
            projection.join(", ")
        ))
        .map_err(|e| e.to_string())?;
    let rows = query
        .query_map([], |row| {
            let sku: String = row.get(0)?;
            let mut draft = Map::new();
            for (index, (column, _)) in FIELDS.iter().enumerate() {
                use rusqlite::types::ValueRef;
                let value = match row.get_ref(index + 1)? {
                    ValueRef::Null => continue,
                    ValueRef::Integer(n) => Value::from(n),
                    ValueRef::Real(n) => Value::from(n),
                    ValueRef::Text(_) => {
                        let text: String = row.get(index + 1)?;
                        if text.trim().is_empty()
                            || (column.ends_with("_status") && text == "pending")
                        {
                            continue;
                        }
                        if column.ends_with("_json") {
                            serde_json::from_str(&text).unwrap_or(Value::String(text))
                        } else {
                            Value::String(text)
                        }
                    }
                    ValueRef::Blob(_) => return Err(rusqlite::Error::InvalidQuery),
                };
                if !value.is_null()
                    && value.as_array().is_none_or(|items| !items.is_empty())
                    && value.as_object().is_none_or(|items| !items.is_empty())
                {
                    draft.insert((*column).into(), value);
                }
            }
            Ok((sku, draft))
        })
        .map_err(|e| e.to_string())?;
    let mut drafts = Drafts::new();
    for row in rows {
        let (sku, draft) = row.map_err(|e| format!("SEO kaydı okunamadı: {e}"))?;
        if !draft.is_empty() {
            if sku.trim().is_empty() || drafts.insert(sku, draft).is_some() {
                return Err("Yedekte boş veya yinelenen ürün kodu var.".into());
            }
        }
    }
    Ok(drafts)
}

fn compare(local: &Drafts, backup: &Drafts) -> SeoBackupPreview {
    let mut result = SeoBackupPreview {
        local_only: 0,
        backup_only: 0,
        identical: 0,
        conflicts: vec![],
    };
    for (sku, left) in local {
        match backup.get(sku) {
            None => result.local_only += 1,
            Some(right) if left == right => result.identical += 1,
            Some(right) => {
                let differences = FIELDS
                    .iter()
                    .filter_map(|(column, label)| {
                        let local = left.get(*column);
                        let backup = right.get(*column);
                        (local != backup).then(|| Difference {
                            field: label,
                            local: local.cloned().unwrap_or(Value::Null),
                            backup: backup.cloned().unwrap_or(Value::Null),
                        })
                    })
                    .collect();
                result.conflicts.push(Conflict {
                    sku: sku.clone(),
                    differences,
                });
            }
        }
    }
    result.backup_only = backup
        .keys()
        .filter(|sku| !local.contains_key(*sku))
        .count();
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_drafts_from_current_and_older_schemas_without_overwriting() {
        let current = Connection::open_in_memory().unwrap();
        current.execute_batch(r#"
            CREATE TABLE seo_status (sku TEXT PRIMARY KEY, draft_title TEXT, meta_status TEXT, meta_history_json TEXT, updated_at TEXT);
            INSERT INTO seo_status VALUES
                ('A','Current','done','[{"title":"Previous"}]','2026-11-18'),
                ('B','Same','pending',NULL,'2026-11-17'),
                ('C','Current only','pending',NULL,NULL),
                ('D',NULL,'pending','[]',NULL);
        "#).unwrap();
        let backup = Connection::open_in_memory().unwrap();
        backup.execute_batch("
            CREATE TABLE seo_status (sku TEXT PRIMARY KEY, draft_title TEXT, updated_at TEXT);
            INSERT INTO seo_status VALUES ('A','Other','2026-11-17'), ('B','Same','2026-10-12'), ('E','Backup only',NULL);
        ").unwrap();

        let local = draft_rows(&current).unwrap();
        let other = draft_rows(&backup).unwrap();
        let result = compare(&local, &other);
        assert_eq!(
            (result.local_only, result.backup_only, result.identical),
            (1, 1, 1)
        );
        assert_eq!(result.conflicts.len(), 1);
        assert_eq!(result.conflicts[0].sku, "A");
        assert_eq!(result.conflicts[0].differences.len(), 3);
        assert_eq!(result.conflicts[0].differences[0].local, "Current");
        assert_eq!(result.conflicts[0].differences[0].backup, "Other");
        assert_eq!(draft_rows(&current).unwrap(), local);
        assert_eq!(draft_rows(&backup).unwrap(), other);
    }
}

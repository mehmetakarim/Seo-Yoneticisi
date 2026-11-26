use super::AppState;
use chrono::DateTime;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use tauri::State;

#[derive(Deserialize)]
pub struct QuoteConflictRetry {
    id: String,
    contact: Value,
    payload: Value,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case", tag = "action")]
pub enum CacheRequest {
    Read {
        key: String,
    },
    Write {
        key: String,
        data: Value,
    },
    Contacts,
    Snapshot {
        contacts: Vec<Value>,
    },
    Queue {
        id: String,
        contact: Value,
        payload: Value,
    },
    Pending,
    Acknowledge {
        id: String,
        contact: Value,
    },
    AcknowledgeContactEvent {
        id: String,
        contact: Value,
        event: Value,
    },
    MergeContactEvents {
        contact_id: String,
        events: Vec<Value>,
    },
    CacheContactProducts {
        contact_id: String,
        revision: u64,
        products: Vec<Value>,
    },
    AcknowledgeContactProducts {
        id: String,
        contact: Value,
        products: Vec<Value>,
    },
    Conflict {
        id: String,
        reason: String,
    },
    Discard {
        ids: Vec<String>,
    },
    ResolveContactConflict {
        contact_id: String,
        expected_ids: Vec<String>,
        remote: Value,
        products: Vec<Value>,
    },
    ResolveProductSeoConflict {
        sku: String,
        expected_ids: Vec<String>,
        remote: Value,
        unsaved: Value,
    },
    ResolveStorePageConflict {
        kind: String,
        remote_id: u64,
        expected_ids: Vec<String>,
        remote: Value,
        unsaved: Value,
    },
    QuoteConflictHistory {
        quote_id: String,
    },
    ResolveQuoteConflict {
        quote_id: String,
        expected_ids: Vec<String>,
        remote: Value,
    },
    ReapplyQuoteConflict {
        quote_id: String,
        expected_ids: Vec<String>,
        remote: Value,
        retry: QuoteConflictRetry,
    },
}

#[tauri::command]
pub fn team_cache(
    state: State<'_, AppState>,
    scope: String,
    request: CacheRequest,
) -> Result<Value, String> {
    if scope.trim().is_empty() || scope.len() > 512 {
        return Err("Ekip önbelleği için geçerli bir çalışma alanı gerekli.".into());
    }
    let mut conn = Connection::open(state.db_path.with_file_name("team-cache.sqlite3"))
        .map_err(|e| e.to_string())?;
    conn.busy_timeout(std::time::Duration::from_secs(3))
        .map_err(|e| e.to_string())?;
    run(&mut conn, &scope, request).map_err(|e| format!("Ekip önbelleği: {e}"))
}

fn run(conn: &mut Connection, scope: &str, request: CacheRequest) -> rusqlite::Result<Value> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS cache (
           scope TEXT NOT NULL, key TEXT NOT NULL, data TEXT NOT NULL, PRIMARY KEY(scope, key)
         );
         CREATE TABLE IF NOT EXISTS outbox (
           seq INTEGER PRIMARY KEY AUTOINCREMENT, scope TEXT NOT NULL, id TEXT NOT NULL,
           contact TEXT NOT NULL, payload TEXT NOT NULL, conflict TEXT, UNIQUE(scope, id)
         );
         CREATE INDEX IF NOT EXISTS outbox_scope ON outbox(scope, seq);",
    )?;
    match request {
        CacheRequest::Read { key } => read_cache(conn, scope, &key),
        CacheRequest::Write { key, data } => {
            upsert(conn, scope, &key, &data)?;
            Ok(Value::Null)
        }
        CacheRequest::Contacts => {
            let mut stmt =
                conn.prepare("SELECT data FROM cache WHERE scope = ?1 AND key LIKE 'contact:%'")?;
            let contacts = stmt
                .query_map([scope], |row| {
                    serde_json::from_str::<Value>(&row.get::<_, String>(0)?).map_err(json_error)
                })?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Value::Array(contacts))
        }
        CacheRequest::Snapshot { contacts } => {
            let tx = conn.transaction()?;
            tx.execute(
                "DELETE FROM cache WHERE scope = ?1 AND key LIKE 'contact:%'",
                [scope],
            )?;
            for contact in contacts {
                let key = format!("contact:{}", contact["id"].as_str().unwrap_or_default());
                upsert(&tx, scope, &key, &contact)?;
            }
            tx.commit()?;
            Ok(Value::Null)
        }
        CacheRequest::Queue {
            id,
            contact,
            payload,
        } => {
            conn.execute(
                "INSERT INTO outbox (scope, id, contact, payload) VALUES (?1, ?2, ?3, ?4)",
                params![scope, id, contact.to_string(), payload.to_string()],
            )?;
            Ok(Value::Null)
        }
        CacheRequest::Pending => {
            let mut stmt = conn.prepare(
                "SELECT id, contact, payload, conflict FROM outbox WHERE scope = ?1 ORDER BY seq",
            )?;
            let rows = stmt.query_map([scope], |row| {
                let contact = serde_json::from_str::<Value>(&row.get::<_, String>(1)?).map_err(json_error)?;
                let payload = serde_json::from_str::<Value>(&row.get::<_, String>(2)?).map_err(json_error)?;
                Ok(json!({ "id": row.get::<_, String>(0)?, "contact": contact, "payload": payload, "conflict": row.get::<_, Option<String>>(3)? }))
            })?.collect::<Result<Vec<_>, _>>()?;
            Ok(Value::Array(rows))
        }
        CacheRequest::Acknowledge { id, contact } => {
            let key = format!("contact:{}", contact["id"].as_str().unwrap_or_default());
            let tx = conn.transaction()?;
            upsert(&tx, scope, &key, &contact)?;
            tx.execute(
                "DELETE FROM outbox WHERE scope = ?1 AND id = ?2",
                params![scope, id],
            )?;
            tx.commit()?;
            Ok(Value::Null)
        }
        CacheRequest::AcknowledgeContactEvent { id, contact, event } => {
            acknowledge_contact_event(conn, scope, &id, &contact, &event).map(Value::Bool)
        }
        CacheRequest::MergeContactEvents { contact_id, events } => {
            let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
            let merged = merge_contact_events(&tx, scope, &contact_id, &events)?;
            tx.commit()?;
            Ok(merged)
        }
        CacheRequest::CacheContactProducts {
            contact_id,
            revision,
            products,
        } => {
            let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
            let snapshot = cache_contact_products(&tx, scope, &contact_id, revision, &products)?;
            tx.commit()?;
            Ok(snapshot)
        }
        CacheRequest::AcknowledgeContactProducts {
            id,
            contact,
            products,
        } => acknowledge_contact_products(conn, scope, &id, &contact, &products)
            .map(Value::Bool),
        CacheRequest::Conflict { id, reason } => {
            conn.execute(
                "UPDATE outbox SET conflict = ?3 WHERE scope = ?1 AND id = ?2",
                params![scope, id, reason],
            )?;
            Ok(Value::Null)
        }
        CacheRequest::Discard { ids } => {
            let tx = conn.transaction()?;
            for id in ids {
                tx.execute(
                    "DELETE FROM outbox WHERE scope = ?1 AND id = ?2",
                    params![scope, id],
                )?;
            }
            tx.commit()?;
            Ok(Value::Null)
        }
        CacheRequest::ResolveContactConflict {
            contact_id,
            expected_ids,
            remote,
            products,
        } => resolve_contact_conflict(conn, scope, &contact_id, &expected_ids, &remote, &products)
            .map(Value::Bool),
        CacheRequest::ResolveProductSeoConflict {
            sku,
            expected_ids,
            remote,
            unsaved,
        } => resolve_product_seo_conflict(conn, scope, &sku, &expected_ids, &remote, &unsaved)
            .map(Value::Bool),
        CacheRequest::ResolveStorePageConflict {
            kind,
            remote_id,
            expected_ids,
            remote,
            unsaved,
        } => resolve_store_page_conflict(
            conn,
            scope,
            &kind,
            remote_id,
            &expected_ids,
            &remote,
            &unsaved,
        )
        .map(Value::Bool),
        CacheRequest::QuoteConflictHistory { quote_id } => {
            quote_conflict_history(conn, scope, &quote_id)
        }
        CacheRequest::ResolveQuoteConflict {
            quote_id,
            expected_ids,
            remote,
        } => resolve_quote_conflict(conn, scope, &quote_id, &expected_ids, &remote, None)
            .map(Value::Bool),
        CacheRequest::ReapplyQuoteConflict {
            quote_id,
            expected_ids,
            remote,
            retry,
        } => resolve_quote_conflict(conn, scope, &quote_id, &expected_ids, &remote, Some(&retry))
            .map(Value::Bool),
    }
}

fn acknowledge_contact_event(
    conn: &mut Connection,
    scope: &str,
    operation_id: &str,
    contact: &Value,
    event: &Value,
) -> rusqlite::Result<bool> {
    let Some(contact_id) = contact["id"].as_str().filter(|id| !id.is_empty()) else {
        return Ok(false);
    };
    if operation_id.is_empty()
        || event["id"].as_str() != Some(operation_id)
        || event["contact_id"].as_str() != Some(contact_id)
    {
        return Ok(false);
    }

    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let row = tx.query_row(
        "SELECT contact, payload FROM outbox WHERE scope = ?1 AND id = ?2 AND conflict IS NULL",
        params![scope, operation_id],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
    ).optional()?;
    let Some((queued_contact, payload)) = row else {
        return Ok(false);
    };
    let queued_contact: Value = serde_json::from_str(&queued_contact).map_err(json_error)?;
    let payload: Value = serde_json::from_str(&payload).map_err(json_error)?;
    let occurred_at = payload["occurred_at"].as_str().and_then(|at| DateTime::parse_from_rfc3339(at).ok());
    let event_at = event["at"].as_str().and_then(|at| DateTime::parse_from_rfc3339(at).ok());
    let queued_revision = queued_contact["revision"].as_u64();
    let revision = contact["revision"].as_u64();
    if queued_contact["id"].as_str() != Some(contact_id)
        || payload["target_contact"].as_str() != Some(contact_id)
        || payload["operation_id"].as_str() != Some(operation_id)
        || payload["fields"]["kind"].as_str() != event["kind"].as_str()
        || occurred_at.is_none()
        || occurred_at != event_at
        || queued_revision.is_none()
        || revision < queued_revision
    {
        return Ok(false);
    }

    merge_contact_events(&tx, scope, contact_id, std::slice::from_ref(event))?;
    let key = format!("contact:{contact_id}");
    if read_cache(&tx, scope, &key)?["revision"].as_u64() <= revision {
        upsert(&tx, scope, &key, contact)?;
    }
    tx.execute("DELETE FROM outbox WHERE scope = ?1 AND id = ?2", params![scope, operation_id])?;
    tx.commit()?;
    Ok(true)
}

fn merge_contact_events(
    conn: &Connection,
    scope: &str,
    contact_id: &str,
    incoming: &[Value],
) -> rusqlite::Result<Value> {
    if contact_id.is_empty() || incoming.iter().any(|event| event["contact_id"].as_str() != Some(contact_id)) {
        return Err(rusqlite::Error::InvalidQuery);
    }
    let key = format!("events:{contact_id}");
    let cached = read_cache(conn, scope, &key)?;
    let cached_events = match &cached {
        Value::Null => &[][..],
        Value::Array(events) => events.as_slice(),
        _ => return Err(rusqlite::Error::InvalidQuery),
    };
    // Team activity rows are append-only. Merge remote reads with locally acknowledged rows,
    // including when a request started before an acknowledgement in another window.
    let mut by_id = BTreeMap::new();
    for event in cached_events.iter().chain(incoming) {
        let (Some(id), Some(at), Some(kind), Some(note)) = (
            event["id"].as_str().filter(|id| !id.is_empty()),
            event["at"].as_str(),
            event["kind"].as_str(),
            event["note"].as_str(),
        ) else {
            return Err(rusqlite::Error::InvalidQuery);
        };
        let time = DateTime::parse_from_rfc3339(at).map_err(|_| rusqlite::Error::InvalidQuery)?;
        by_id.insert(id, (time, json!({ "id": id, "at": at, "kind": kind, "note": note })));
    }
    let mut entries: Vec<_> = by_id.into_iter().collect();
    entries.sort_by(|(left_id, (left_time, _)), (right_id, (right_time, _))| {
        right_time.cmp(left_time).then_with(|| right_id.cmp(left_id))
    });
    let merged = Value::Array(entries.into_iter().map(|(_, (_, event))| event).collect());
    upsert(conn, scope, &key, &merged)?;
    Ok(merged)
}

fn cache_contact_products(
    conn: &Connection,
    scope: &str,
    contact_id: &str,
    revision: u64,
    products: &[Value],
) -> rusqlite::Result<Value> {
    if contact_id.is_empty() || revision == 0 {
        return Err(rusqlite::Error::InvalidQuery);
    }
    let key = format!("products:{contact_id}");
    let cached = read_cache(conn, scope, &key)?;
    if cached["revision"]
        .as_u64()
        .is_some_and(|previous| previous > revision)
    {
        return Ok(cached);
    }

    let mut skus = BTreeSet::new();
    for product in products {
        let sku = product["sku"].as_str().filter(|sku| !sku.trim().is_empty());
        let linked_at = product["at"]
            .as_str()
            .and_then(|at| DateTime::parse_from_rfc3339(at).ok());
        if product["contact_id"].as_str() != Some(contact_id)
            || !sku.is_some_and(|sku| skus.insert(sku))
            || linked_at.is_none()
        {
            return Err(rusqlite::Error::InvalidQuery);
        }
    }

    // A removed link must stay removed when an older window's read finishes later.
    let snapshot = json!({ "revision": revision, "products": products });
    upsert(conn, scope, &key, &snapshot)?;
    Ok(snapshot)
}

fn acknowledge_contact_products(
    conn: &mut Connection,
    scope: &str,
    operation_id: &str,
    contact: &Value,
    products: &[Value],
) -> rusqlite::Result<bool> {
    let Some(contact_id) = contact["id"].as_str().filter(|id| !id.is_empty()) else {
        return Ok(false);
    };
    let Some(revision) = contact["revision"].as_u64().filter(|revision| *revision > 0) else {
        return Ok(false);
    };
    if operation_id.is_empty() {
        return Ok(false);
    }

    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let row = tx.query_row(
        "SELECT contact, payload FROM outbox WHERE scope = ?1 AND id = ?2 AND conflict IS NULL",
        params![scope, operation_id],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
    ).optional()?;
    let Some((queued_contact, payload)) = row else {
        return Ok(false);
    };
    let queued_contact: Value = serde_json::from_str(&queued_contact).map_err(json_error)?;
    let payload: Value = serde_json::from_str(&payload).map_err(json_error)?;
    let queued_revision = queued_contact["revision"].as_u64();
    let expected_revision = payload["expected_revision"]
        .as_u64()
        .filter(|revision| *revision > 0);
    let action = payload["action"].as_str();
    if queued_contact["id"].as_str() != Some(contact_id)
        || payload["target_contact"].as_str() != Some(contact_id)
        || payload["operation_id"].as_str() != Some(operation_id)
        || !matches!(action, Some("link" | "unlink"))
        || expected_revision.and_then(|revision| revision.checked_add(1)) != queued_revision
        || !queued_revision.is_some_and(|queued| revision >= queued)
    {
        return Ok(false);
    }
    let Some(sku) = payload["fields"]["sku"]
        .as_str()
        .map(str::trim)
        .filter(|sku| !sku.is_empty())
    else {
        return Ok(false);
    };
    if Some(revision) == queued_revision {
        let linked = products
            .iter()
            .any(|product| product["sku"].as_str() == Some(sku));
        if linked != (action == Some("link")) {
            return Ok(false);
        }
    }

    cache_contact_products(&tx, scope, contact_id, revision, products)?;
    let key = format!("contact:{contact_id}");
    if read_cache(&tx, scope, &key)?["revision"].as_u64() <= Some(revision) {
        upsert(&tx, scope, &key, contact)?;
    }
    tx.execute(
        "DELETE FROM outbox WHERE scope = ?1 AND id = ?2",
        params![scope, operation_id],
    )?;
    tx.commit()?;
    Ok(true)
}

fn record_outbox_operations(
    conn: &Connection,
    scope: &str,
    record_id: &str,
    drafts_only: bool,
) -> rusqlite::Result<Vec<Value>> {
    let mut stmt = conn.prepare(
        "SELECT id, contact, payload, conflict FROM outbox
         WHERE scope = ?1 AND json_extract(contact, '$.id') = ?2
           AND (?3 = 0 OR (json_type(payload, '$.facts') IS NULL AND json_type(payload, '$.receipt') IS NULL))
         ORDER BY seq",
    )?;
    let rows = stmt.query_map(params![scope, record_id, drafts_only], |row| {
        let contact: Value = serde_json::from_str(&row.get::<_, String>(1)?).map_err(json_error)?;
        let payload: Value = serde_json::from_str(&row.get::<_, String>(2)?).map_err(json_error)?;
        Ok(json!({ "id": row.get::<_, String>(0)?, "contact": contact,
            "payload": payload, "conflict": row.get::<_, Option<String>>(3)? }))
    })?;
    rows.collect()
}

fn reviewed_conflict_operations(
    conn: &Connection,
    scope: &str,
    record_id: &str,
    expected_ids: &[String],
) -> rusqlite::Result<Option<Vec<Value>>> {
    let operations = record_outbox_operations(conn, scope, record_id, false)?;
    let ids = operations.iter().map(|op| op["id"].as_str().unwrap_or_default());
    Ok((ids.eq(expected_ids.iter().map(String::as_str))
        && operations.iter().any(|op| op["conflict"].as_str().is_some()))
    .then_some(operations))
}

fn resolve_contact_conflict(
    conn: &mut Connection,
    scope: &str,
    contact_id: &str,
    expected_ids: &[String],
    remote: &Value,
    products: &[Value],
) -> rusqlite::Result<bool> {
    let Some(revision) = remote["revision"].as_u64().filter(|revision| *revision > 0) else {
        return Ok(false);
    };
    if expected_ids.is_empty() || contact_id.is_empty() || remote["id"].as_str() != Some(contact_id) {
        return Ok(false);
    }

    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let Some(operations) = reviewed_conflict_operations(&tx, scope, contact_id, expected_ids)? else {
        return Ok(false);
    };
    let contact_key = format!("contact:{contact_id}");
    if read_cache(&tx, scope, &contact_key)?["revision"].as_u64() > Some(revision) {
        return Ok(false);
    }
    let product_snapshot = cache_contact_products(&tx, scope, contact_id, revision, products)?;
    if product_snapshot["revision"].as_u64() != Some(revision) {
        return Ok(false);
    }

    let recovery_key = format!("contact-conflict:{contact_id}");
    let mut recovery = match read_cache(&tx, scope, &recovery_key)? {
        Value::Null => Vec::new(),
        Value::Array(records) => records,
        _ => return Err(rusqlite::Error::InvalidQuery),
    };
    let local = operations.last().ok_or(rusqlite::Error::InvalidQuery)?["contact"].clone();
    let saved_at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    recovery.insert(0, json!({
        "saved_at": saved_at, "local": local, "remote": remote,
        "products": products, "operations": operations,
    }));
    upsert(&tx, scope, &recovery_key, &Value::Array(recovery))?;
    upsert(&tx, scope, &contact_key, remote)?;
    let removed = tx.execute(
        "DELETE FROM outbox WHERE scope = ?1 AND json_extract(contact, '$.id') = ?2",
        params![scope, contact_id],
    )?;
    if removed != expected_ids.len() {
        return Ok(false);
    }
    tx.commit()?;
    Ok(true)
}

fn resolve_product_seo_conflict(
    conn: &mut Connection,
    scope: &str,
    sku: &str,
    expected_ids: &[String],
    remote: &Value,
    unsaved: &Value,
) -> rusqlite::Result<bool> {
    let Ok([_, _, workspace, kind]) = serde_json::from_str::<[String; 4]>(scope) else {
        return Ok(false);
    };
    let Some(revision) = remote["revision"].as_u64().filter(|revision| *revision > 0) else {
        return Ok(false);
    };
    let Some(unsaved) = unsaved.as_object() else {
        return Ok(false);
    };
    if kind != "product-seo"
        || sku.trim().is_empty()
        || expected_ids.is_empty()
        || remote["id"].as_str() != Some(sku)
        || remote["sku"].as_str() != Some(sku)
        || !remote["draft"].is_object()
        || unsaved.iter().any(|(field, value)| {
            !matches!(
                field.as_str(),
                "draft_title"
                    | "draft_descriptions"
                    | "draft_search_keywords"
                    | "target_keyword"
                    | "tech_source_text"
            ) || !value.is_string()
        })
    {
        return Ok(false);
    }

    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let Some(operations) = reviewed_conflict_operations(&tx, scope, sku, expected_ids)? else {
        return Ok(false);
    };
    if operations.iter().any(|operation| {
        operation["contact"]["sku"].as_str() != Some(sku)
            || operation["payload"]["target_sku"].as_str() != Some(sku)
            || operation["payload"]["target_workspace"].as_str() != Some(workspace.as_str())
            || operation["payload"]["operation_id"] != operation["id"]
    }) {
        return Ok(false);
    }

    let record_key = format!("contact:{sku}");
    if read_cache(&tx, scope, &record_key)?["revision"].as_u64() > Some(revision) {
        return Ok(false);
    }
    let mut local = operations.last().ok_or(rusqlite::Error::InvalidQuery)?["contact"].clone();
    let Some(draft) = local["draft"].as_object_mut() else {
        return Ok(false);
    };
    draft.extend(unsaved.clone());

    let recovery_key = format!("product-seo-conflict:{sku}");
    let mut recovery = match read_cache(&tx, scope, &recovery_key)? {
        Value::Null => Vec::new(),
        Value::Array(records) => records,
        _ => return Err(rusqlite::Error::InvalidQuery),
    };
    let saved_at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    recovery.insert(
        0,
        json!({
            "saved_at": saved_at, "local": local, "remote": remote, "operations": operations,
        }),
    );
    upsert(&tx, scope, &recovery_key, &Value::Array(recovery))?;
    upsert(&tx, scope, &record_key, remote)?;
    let removed = tx.execute(
        "DELETE FROM outbox WHERE scope = ?1 AND json_extract(contact, '$.id') = ?2",
        params![scope, sku],
    )?;
    if removed != expected_ids.len() {
        return Ok(false);
    }
    tx.commit()?;
    Ok(true)
}

fn resolve_store_page_conflict(
    conn: &mut Connection,
    scope: &str,
    kind: &str,
    remote_id: u64,
    expected_ids: &[String],
    remote: &Value,
    unsaved: &Value,
) -> rusqlite::Result<bool> {
    let Ok([project, user, workspace, namespace]) = serde_json::from_str::<[String; 4]>(scope)
    else {
        return Ok(false);
    };
    let Some(revision) = remote["revision"].as_u64().filter(|revision| *revision > 0) else {
        return Ok(false);
    };
    let Some(unsaved) = unsaved.as_object() else {
        return Ok(false);
    };
    let record_id = format!("{kind}:{remote_id}");
    let matches_record = |record: &Value| {
        record["kind"].as_str() == Some(kind)
            && record["remote_id"].as_u64() == Some(remote_id)
            && record["id"].as_str() == Some(record_id.as_str())
            && record["team_scope"].as_str() == Some(scope)
    };
    if namespace != "store-pages"
        || project.is_empty()
        || user.is_empty()
        || workspace.is_empty()
        || !matches!(kind, "category" | "brand" | "blog")
        || remote_id == 0
        || !matches_record(remote)
        || unsaved.iter().any(|(field, value)| {
            !matches!(
                field.as_str(),
                "draft_page_title"
                    | "draft_meta_description"
                    | "draft_target_keyword"
                    | "draft_showcase"
            ) || !value.is_string()
        })
    {
        return Ok(false);
    }

    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let operations = record_outbox_operations(&tx, scope, &record_id, true)?;
    let current_ids = operations
        .iter()
        .filter_map(|operation| operation["id"].as_str());
    if !current_ids.eq(expected_ids.iter().map(String::as_str))
        || (operations.is_empty() && unsaved.is_empty())
        || (!operations.is_empty()
            && !operations
                .iter()
                .any(|operation| operation["conflict"].is_string()))
        || operations.iter().any(|operation| {
            let payload = &operation["payload"];
            !matches_record(&operation["contact"])
                || payload["target_workspace"].as_str() != Some(workspace.as_str())
                || payload["target_kind"].as_str() != Some(kind)
                || payload["target_id"].as_u64() != Some(remote_id)
                || payload["operation_id"] != operation["id"]
                || !payload["changes"].is_object()
        })
    {
        return Ok(false);
    }

    let mut pages = match read_cache(&tx, scope, kind)? {
        Value::Null => Vec::new(),
        Value::Array(rows) => rows,
        _ => return Err(rusqlite::Error::InvalidQuery),
    };
    if let Some(page) = pages
        .iter_mut()
        .find(|page| page["id"].as_str() == Some(record_id.as_str()))
    {
        if page["revision"].as_u64() > Some(revision) {
            return Ok(false);
        }
        *page = remote.clone();
    } else {
        pages.push(remote.clone());
    }

    let mut local = operations
        .last()
        .map(|operation| &operation["contact"])
        .unwrap_or(remote)
        .clone();
    local
        .as_object_mut()
        .ok_or(rusqlite::Error::InvalidQuery)?
        .extend(unsaved.clone());
    let recovery_key = format!("store-page-conflict:{record_id}");
    let mut recovery = match read_cache(&tx, scope, &recovery_key)? {
        Value::Null => Vec::new(),
        Value::Array(records) => records,
        _ => return Err(rusqlite::Error::InvalidQuery),
    };
    let saved_at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    recovery.insert(
        0,
        json!({ "saved_at": saved_at, "local": local, "remote": remote, "operations": operations }),
    );
    upsert(&tx, scope, &recovery_key, &Value::Array(recovery))?;
    upsert(&tx, scope, kind, &Value::Array(pages))?;
    for operation_id in expected_ids {
        let removed = tx.execute(
            "DELETE FROM outbox WHERE scope = ?1 AND id = ?2",
            params![scope, operation_id],
        )?;
        if removed != 1 {
            return Ok(false);
        }
    }
    tx.commit()?;
    Ok(true)
}

fn quote_conflict_history(conn: &Connection, scope: &str, quote_id: &str) -> rusqlite::Result<Value> {
    let Ok([project, user, workspace, namespace]) = serde_json::from_str::<[String; 4]>(scope) else {
        return Ok(json!([]));
    };
    if namespace != "quotes"
        || project.is_empty()
        || user.is_empty()
        || workspace.is_empty()
        || quote_id.trim().is_empty()
        || quote_id.contains(':')
    {
        return Ok(json!([]));
    }

    let prefix = format!("quote-conflict:{quote_id}:");
    let end = format!("quote-conflict:{quote_id};");
    let mut stmt = conn.prepare(
        "SELECT key, data FROM cache WHERE scope = ?1 AND key >= ?2 AND key < ?3 ORDER BY rowid DESC",
    )?;
    let mut rows = stmt.query(params![scope, prefix, end])?;
    let mut history = Vec::new();
    while let Some(row) = rows.next()? {
        let key: String = row.get(0)?;
        let Some(operation_id) = key.strip_prefix(&prefix).filter(|id| !id.is_empty()) else {
            continue;
        };
        let operations = serde_json::from_str::<Vec<Value>>(&row.get::<_, String>(1)?);
        let Some(operation) = operations.as_ref().ok().and_then(|ops| ops.last()) else {
            continue;
        };
        let local = &operation["contact"];
        if operation["id"].as_str() != Some(operation_id) || local["id"].as_str() != Some(quote_id) {
            continue;
        }

        let remote = read_cache(
            conn,
            scope,
            &format!("quote-conflict-remote:{quote_id}:{operation_id}"),
        )?;
        let saved_at = read_cache(conn, scope, &format!("quote-conflict-at:{quote_id}:{operation_id}"))?;
        history.push(json!({
            "id": operation_id,
            "saved_at": saved_at.as_str().filter(|value| DateTime::parse_from_rfc3339(value).is_ok()),
            "local": local,
            "remote": (remote["id"].as_str() == Some(quote_id)).then_some(remote),
        }));
    }
    Ok(Value::Array(history))
}

fn resolve_quote_conflict(
    conn: &mut Connection,
    scope: &str,
    quote_id: &str,
    expected_ids: &[String],
    remote: &Value,
    retry: Option<&QuoteConflictRetry>,
) -> rusqlite::Result<bool> {
    let revision = remote["revision"].as_u64().unwrap_or(0);
    if expected_ids.is_empty()
        || remote["id"].as_str() != Some(quote_id)
        || revision == 0
    {
        return Ok(false);
    }
    if retry.is_some_and(|retry| {
        remote["status"] != "draft"
            || retry.id.is_empty()
            || expected_ids.contains(&retry.id)
            || retry.contact["id"].as_str() != Some(quote_id)
            || retry.contact["status"] != "draft"
            || retry.contact["revision"].as_u64().and_then(|value| value.checked_sub(1)) != Some(revision)
            || retry.payload["target_quote"].as_str() != Some(quote_id)
            || retry.payload["expected_revision"].as_u64() != Some(revision)
            || retry.payload["operation_id"].as_str() != Some(retry.id.as_str())
    }) {
        return Ok(false);
    }
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let Some(operations) = reviewed_conflict_operations(&tx, scope, quote_id, expected_ids)? else {
        return Ok(false);
    };
    // Preserve the inspected local operations separately from the sync queue.
    let operation_id = expected_ids.last().ok_or(rusqlite::Error::InvalidQuery)?;
    upsert(
        &tx,
        scope,
        &format!("quote-conflict:{quote_id}:{operation_id}"),
        &Value::Array(operations),
    )?;
    upsert(
        &tx,
        scope,
        &format!("quote-conflict-remote:{quote_id}:{operation_id}"),
        remote,
    )?;
    let saved_at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    upsert(&tx, scope, &format!("quote-conflict-at:{quote_id}:{operation_id}"), &json!(saved_at))?;
    upsert(&tx, scope, &format!("contact:{quote_id}"), remote)?;
    tx.execute(
        "DELETE FROM outbox WHERE scope = ?1 AND json_extract(contact, '$.id') = ?2",
        params![scope, quote_id],
    )?;
    if let Some(retry) = retry {
        tx.execute(
            "INSERT INTO outbox (scope, id, contact, payload) VALUES (?1, ?2, ?3, ?4)",
            params![scope, retry.id, retry.contact.to_string(), retry.payload.to_string()],
        )?;
    }
    tx.commit()?;
    Ok(true)
}

fn read_cache(conn: &Connection, scope: &str, key: &str) -> rusqlite::Result<Value> {
    let value = conn.query_row(
        "SELECT data FROM cache WHERE scope = ?1 AND key = ?2",
        params![scope, key],
        |row| row.get::<_, String>(0),
    ).optional()?;
    Ok(value
        .map(|text| serde_json::from_str(&text))
        .transpose()
        .map_err(json_error)?
        .unwrap_or(Value::Null))
}

fn upsert(conn: &Connection, scope: &str, key: &str, data: &Value) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO cache (scope, key, data) VALUES (?1, ?2, ?3)
         ON CONFLICT(scope, key) DO UPDATE SET data = excluded.data",
        params![scope, key, data.to_string()],
    )?;
    Ok(())
}

fn json_error(error: serde_json::Error) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
}

#[cfg(test)]
#[test]
fn outbox_is_scoped_and_acknowledged_with_cache_update() {
    let mut conn = Connection::open_in_memory().unwrap();
    let contact = json!({ "id": "contact", "name": "Test", "revision": 1 });
    run(
        &mut conn,
        "workspace:a",
        CacheRequest::Queue {
            id: "op".into(),
            contact: contact.clone(),
            payload: json!({"expected_revision": null}),
        },
    )
    .unwrap();
    assert_eq!(
        run(&mut conn, "workspace:b", CacheRequest::Pending).unwrap(),
        json!([])
    );
    assert_eq!(
        run(&mut conn, "workspace:a", CacheRequest::Pending).unwrap()[0]["id"],
        "op"
    );
    run(
        &mut conn,
        "workspace:a",
        CacheRequest::Acknowledge {
            id: "op".into(),
            contact: contact.clone(),
        },
    )
    .unwrap();
    assert_eq!(
        run(&mut conn, "workspace:a", CacheRequest::Pending).unwrap(),
        json!([])
    );
    assert_eq!(
        run(
            &mut conn,
            "workspace:a",
            CacheRequest::Read {
                key: "contact:contact".into()
            }
        )
        .unwrap(),
        contact
    );
}

#[cfg(test)]
#[test]
fn contact_event_acknowledgement_checks_identity_and_is_atomic() {
    let mut conn = Connection::open_in_memory().unwrap();
    let scope = "workspace:a";
    let contact = json!({ "id": "target", "revision": 2 });
    let event = json!({
        "id": "event-1", "contact_id": "target", "at": "2026-11-25T09:00:00+00:00",
        "kind": "call", "note": "Call note",
    });
    run(
        &mut conn,
        scope,
        CacheRequest::Queue {
            id: "event-1".into(),
            contact: contact.clone(),
            payload: json!({
                "operation_id": "event-1", "target_contact": "target", "expected_revision": 1,
                "occurred_at": "2026-11-25T09:00:00.000Z", "fields": { "kind": "call" },
            }),
        },
    )
    .unwrap();
    let acknowledge = |conn: &mut Connection, scope: &str, event: Value| {
        run(
            conn,
            scope,
            CacheRequest::AcknowledgeContactEvent {
                id: "event-1".into(),
                contact: contact.clone(),
                event,
            },
        )
    };

    assert_eq!(acknowledge(&mut conn, "workspace:b", event.clone()).unwrap(), json!(false));
    let mut mismatched = event.clone();
    mismatched["contact_id"] = json!("other-person");
    assert_eq!(acknowledge(&mut conn, scope, mismatched).unwrap(), json!(false));
    let mut mismatched = event.clone();
    mismatched["id"] = json!("different-event");
    assert_eq!(acknowledge(&mut conn, scope, mismatched).unwrap(), json!(false));

    conn.execute_batch(
        "CREATE TRIGGER block_ack BEFORE DELETE ON outbox
         BEGIN SELECT RAISE(ABORT, 'simulated dequeue failure'); END;",
    )
    .unwrap();
    assert!(acknowledge(&mut conn, scope, event.clone()).is_err());
    assert_eq!(read_cache(&conn, scope, "contact:target").unwrap(), Value::Null);
    assert_eq!(read_cache(&conn, scope, "events:target").unwrap(), Value::Null);
    assert_eq!(run(&mut conn, scope, CacheRequest::Pending).unwrap()[0]["id"], "event-1");
    conn.execute_batch("DROP TRIGGER block_ack;").unwrap();

    assert_eq!(acknowledge(&mut conn, scope, event.clone()).unwrap(), json!(true));
    assert_eq!(acknowledge(&mut conn, scope, event).unwrap(), json!(false));
    assert_eq!(read_cache(&conn, scope, "contact:target").unwrap(), contact);
    assert_eq!(read_cache(&conn, scope, "events:target").unwrap(), json!([{
        "id": "event-1", "at": "2026-11-25T09:00:00+00:00", "kind": "call", "note": "Call note",
    }]));
    assert_eq!(run(&mut conn, scope, CacheRequest::Pending).unwrap(), json!([]));
}

#[cfg(test)]
#[test]
fn contact_event_timeline_merges_stale_reads_without_duplicate_rows() {
    let mut conn = Connection::open_in_memory().unwrap();
    let scope = "workspace:a";
    let merge_events = |conn: &mut Connection, events: Vec<Value>| {
        run(conn, scope, CacheRequest::MergeContactEvents { contact_id: "target".into(), events })
    };
    let older = json!({
        "id": "older", "contact_id": "target", "at": "2026-11-25T11:00:00+03:00",
        "kind": "email", "note": "Earlier event",
    });
    let newer = json!({
        "id": "newer", "contact_id": "target", "at": "2026-11-25T09:00:00Z",
        "kind": "call", "note": "Later event",
    });
    merge_events(&mut conn, vec![newer.clone()]).unwrap();
    let merged = merge_events(&mut conn, vec![older.clone(), older]).unwrap();
    assert_eq!(merged.as_array().unwrap().len(), 2);
    assert_eq!(merged[0]["id"], "newer");
    assert_eq!(merged[1]["id"], "older");
    assert_eq!(merge_events(&mut conn, Vec::new()).unwrap(), merged);
    let mut unrelated = newer;
    unrelated["contact_id"] = json!("other-person");
    assert!(merge_events(&mut conn, vec![unrelated]).is_err());
    assert_eq!(read_cache(&conn, scope, "events:target").unwrap(), merged);
    assert_eq!(read_cache(&conn, "workspace:b", "events:target").unwrap(), Value::Null);
}

#[cfg(test)]
#[test]
fn contact_product_snapshots_preserve_removals_against_stale_reads() {
    let mut conn = Connection::open_in_memory().unwrap();
    let scope = "workspace:a";
    let contact_id = "target";
    let key = "products:target";
    let product = json!({ "sku": "ABC", "contact_id": contact_id, "at": "2026-11-25T09:00:00Z" });
    let store = |conn: &mut Connection, revision, products| {
        run(
            conn,
            scope,
            CacheRequest::CacheContactProducts {
                contact_id: contact_id.into(),
                revision,
                products,
            },
        )
    };

    let legacy = json!([product]);
    run(
        &mut conn,
        scope,
        CacheRequest::Write { key: key.into(), data: legacy.clone() },
    )
    .unwrap();
    assert_eq!(read_cache(&conn, scope, key).unwrap(), legacy);
    assert_eq!(
        store(&mut conn, 2, vec![product.clone()]).unwrap(),
        json!({ "revision": 2, "products": [product] }),
    );
    let removed = json!({ "revision": 3, "products": [] });
    assert_eq!(store(&mut conn, 3, vec![]).unwrap(), removed);
    assert_eq!(store(&mut conn, 2, vec![product.clone()]).unwrap(), removed);
    let mut foreign_product = product;
    foreign_product["contact_id"] = json!("another-contact");
    assert!(store(&mut conn, 4, vec![foreign_product]).is_err());
    assert_eq!(read_cache(&conn, scope, key).unwrap(), removed);
    assert_eq!(read_cache(&conn, "workspace:b", key).unwrap(), Value::Null);
}

#[cfg(test)]
#[test]
fn contact_product_acknowledgement_is_scoped_atomic_and_revision_checked() {
    let mut conn = Connection::open_in_memory().unwrap();
    let scope = "workspace:a";
    let product = json!({ "sku": "ABC", "contact_id": "target", "at": "2026-11-25T09:00:00Z" });
    let queued_contact = json!({ "id": "target", "revision": 2 });
    let other_op = CacheRequest::Queue {
        id: "unrelated".into(),
        contact: queued_contact.clone(),
        payload: json!({ "target_contact": "target", "action": "tags" }),
    };
    run(&mut conn, scope, other_op).unwrap();
    run(
        &mut conn,
        scope,
        CacheRequest::Queue {
            id: "link-1".into(),
            contact: queued_contact.clone(),
            payload: json!({
                "operation_id": "link-1", "target_contact": "target", "expected_revision": 1,
                "action": "link", "fields": { "sku": "ABC" },
            }),
        },
    )
    .unwrap();

    let ack = |conn: &mut Connection, scope: &str, id: &str, contact, products| {
        run(
            conn,
            scope,
            CacheRequest::AcknowledgeContactProducts { id: id.into(), contact, products },
        )
    };
    for (owner, id, contact, rows) in [
        ("workspace:b", "link-1", queued_contact.clone(), vec![product.clone()]),
        (scope, "unrelated", queued_contact.clone(), vec![product.clone()]),
        (scope, "link-1", json!({ "id": "another-contact", "revision": 2 }), vec![product.clone()]),
        (scope, "link-1", json!({ "id": "target", "revision": 1 }), vec![product.clone()]),
        (scope, "link-1", queued_contact.clone(), vec![]),
    ] {
        assert_eq!(ack(&mut conn, owner, id, contact, rows).unwrap(), json!(false));
    }

    conn.execute_batch(
        "CREATE TRIGGER block_product_dequeue BEFORE DELETE ON outbox
         BEGIN SELECT RAISE(ABORT, 'simulated dequeue failure'); END;",
    )
    .unwrap();
    assert!(ack(&mut conn, scope, "link-1", queued_contact.clone(), vec![product.clone()]).is_err());
    assert_eq!(read_cache(&conn, scope, "contact:target").unwrap(), Value::Null);
    assert_eq!(read_cache(&conn, scope, "products:target").unwrap(), Value::Null);
    assert_eq!(run(&mut conn, scope, CacheRequest::Pending).unwrap()[1]["id"], "link-1");
    conn.execute_batch("DROP TRIGGER block_product_dequeue;").unwrap();

    assert_eq!(ack(&mut conn, scope, "link-1", queued_contact.clone(), vec![product.clone()]).unwrap(), json!(true));
    assert_eq!(ack(&mut conn, scope, "link-1", queued_contact.clone(), vec![product.clone()]).unwrap(), json!(false));
    assert_eq!(read_cache(&conn, scope, "contact:target").unwrap(), queued_contact);
    assert_eq!(read_cache(&conn, scope, "products:target").unwrap(), json!({ "revision": 2, "products": [product] }));
    let pending = run(&mut conn, scope, CacheRequest::Pending).unwrap();
    assert_eq!(pending.as_array().unwrap().len(), 1);
    assert_eq!(pending[0]["id"], "unrelated");

    let newer_contact = json!({ "id": "target", "revision": 4, "name": "Current name" });
    run(
        &mut conn,
        scope,
        CacheRequest::Write { key: "contact:target".into(), data: newer_contact.clone() },
    )
    .unwrap();
    let newer_snapshot = run(
        &mut conn,
        scope,
        CacheRequest::CacheContactProducts {
            contact_id: "target".into(), revision: 4, products: vec![product.clone()],
        },
    )
    .unwrap();
    let queued_unlink = json!({ "id": "target", "revision": 3 });
    run(
        &mut conn,
        scope,
        CacheRequest::Queue {
            id: "unlink-1".into(),
            contact: queued_unlink.clone(),
            payload: json!({
                "operation_id": "unlink-1", "target_contact": "target", "expected_revision": 2,
                "action": "unlink", "fields": { "sku": "ABC" },
            }),
        },
    )
    .unwrap();
    assert_eq!(ack(&mut conn, scope, "unlink-1", queued_unlink.clone(), vec![product]).unwrap(), json!(false));
    assert_eq!(ack(&mut conn, scope, "unlink-1", queued_unlink, vec![]).unwrap(), json!(true));
    assert_eq!(read_cache(&conn, scope, "contact:target").unwrap(), newer_contact);
    assert_eq!(read_cache(&conn, scope, "products:target").unwrap(), newer_snapshot);
}

#[cfg(test)]
#[test]
fn contact_conflict_resolution_archives_only_the_exact_reviewed_queue() {
    let mut conn = Connection::open_in_memory().unwrap();
    let scope = "workspace:a";
    let remote = json!({ "id": "target", "revision": 8, "archived": false });
    let product = json!({ "sku": "SHARED", "contact_id": "target", "at": "2026-11-25T09:00:00Z" });
    for (owner, id, contact, payload) in [
        (scope, "call", json!({ "id": "target", "revision": 5 }), json!({ "fields": { "note": "Preserve this call" } })),
        (scope, "other", json!({ "id": "another-contact", "revision": 2 }), json!({})),
        (scope, "link", json!({ "id": "target", "revision": 6 }), json!({ "action": "link", "fields": { "sku": "LOCAL" } })),
        (scope, "archive", json!({ "id": "target", "revision": 7, "archived": true }), json!({ "action": "archive" })),
        ("workspace:b", "call", json!({ "id": "target", "revision": 5 }), json!({})),
    ] {
        run(&mut conn, owner, CacheRequest::Queue { id: id.into(), contact, payload }).unwrap();
    }
    let resolve = |conn: &mut Connection, owner, ids: &[&str], remote: Value| {
        run(conn, owner, CacheRequest::ResolveContactConflict {
            contact_id: "target".into(),
            expected_ids: ids.iter().map(|id| (*id).into()).collect(),
            remote,
            products: vec![product.clone()],
        })
    };
    let exact_ids = ["call", "link", "archive"];
    assert_eq!(resolve(&mut conn, scope, &exact_ids, remote.clone()).unwrap(), json!(false));
    run(&mut conn, scope, CacheRequest::Conflict { id: "call".into(), reason: "Revision conflict".into() }).unwrap();
    for ids in [&[][..], &["call", "link"][..], &["archive", "link", "call"][..], &["call", "other", "link", "archive"][..]] {
        assert_eq!(resolve(&mut conn, scope, ids, remote.clone()).unwrap(), json!(false));
    }
    assert_eq!(resolve(&mut conn, "workspace:b", &exact_ids, remote.clone()).unwrap(), json!(false));
    assert_eq!(resolve(&mut conn, scope, &exact_ids, json!({ "id": "other", "revision": 8 })).unwrap(), json!(false));
    assert_eq!(resolve(&mut conn, scope, &exact_ids, remote.clone()).unwrap(), json!(true));
    assert_eq!(resolve(&mut conn, scope, &exact_ids, remote.clone()).unwrap(), json!(false));

    let archive = read_cache(&conn, scope, "contact-conflict:target").unwrap();
    assert_eq!(archive.as_array().unwrap().len(), 1);
    assert!(DateTime::parse_from_rfc3339(archive[0]["saved_at"].as_str().unwrap()).is_ok());
    assert_eq!(archive[0]["local"], json!({ "id": "target", "revision": 7, "archived": true }));
    assert_eq!(archive[0]["remote"], remote);
    assert_eq!(archive[0]["products"], json!([product]));
    assert_eq!(archive[0]["operations"][0]["payload"]["fields"]["note"], "Preserve this call");
    assert_eq!(archive[0]["operations"][1]["payload"]["fields"]["sku"], "LOCAL");
    assert_eq!(archive[0]["operations"][2]["payload"]["action"], "archive");
    assert_eq!(read_cache(&conn, "workspace:b", "contact-conflict:target").unwrap(), Value::Null);
    assert_eq!(run(&mut conn, scope, CacheRequest::Pending).unwrap()[0]["id"], "other");
    assert_eq!(run(&mut conn, "workspace:b", CacheRequest::Pending).unwrap()[0]["id"], "call");
    assert_eq!(read_cache(&conn, scope, "contact:target").unwrap(), remote);
    assert_eq!(read_cache(&conn, scope, "products:target").unwrap(), json!({ "revision": 8, "products": [product] }));
}

#[cfg(test)]
#[test]
fn contact_conflict_preserves_prior_history_and_refuses_stale_or_partial_mutations() {
    let mut conn = Connection::open_in_memory().unwrap();
    let scope = "workspace:a";
    let queue_conflict = |conn: &mut Connection, id: &str, revision| {
        run(conn, scope, CacheRequest::Queue {
            id: id.into(), contact: json!({ "id": "target", "revision": revision }), payload: json!({}),
        }).unwrap();
        run(conn, scope, CacheRequest::Conflict { id: id.into(), reason: "Revision conflict".into() }).unwrap();
    };
    let resolve = |conn: &mut Connection, id: &str, revision, products| {
        run(conn, scope, CacheRequest::ResolveContactConflict {
            contact_id: "target".into(), expected_ids: vec![id.into()],
            remote: json!({ "id": "target", "revision": revision }), products,
        })
    };
    queue_conflict(&mut conn, "first", 4);
    run(&mut conn, scope, CacheRequest::Write {
        key: "contact:target".into(), data: json!({ "id": "target", "revision": 5 }),
    }).unwrap();
    assert_eq!(resolve(&mut conn, "first", 4, vec![]).unwrap(), json!(false));
    assert_eq!(read_cache(&conn, scope, "products:target").unwrap(), Value::Null);
    run(&mut conn, scope, CacheRequest::CacheContactProducts {
        contact_id: "target".into(), revision: 6, products: vec![],
    }).unwrap();
    assert_eq!(resolve(&mut conn, "first", 5, vec![]).unwrap(), json!(false));
    assert_eq!(read_cache(&conn, scope, "contact-conflict:target").unwrap(), Value::Null);
    assert_eq!(run(&mut conn, scope, CacheRequest::Pending).unwrap()[0]["id"], "first");
    assert_eq!(resolve(&mut conn, "first", 6, vec![]).unwrap(), json!(true));
    let first_archive = read_cache(&conn, scope, "contact-conflict:target").unwrap();

    queue_conflict(&mut conn, "second", 7);
    let foreign_product = json!({ "sku": "FOREIGN", "contact_id": "other", "at": "2026-11-25T09:00:00Z" });
    assert!(resolve(&mut conn, "second", 8, vec![foreign_product]).is_err());
    conn.execute_batch(
        "CREATE TRIGGER block_contact_conflict_dequeue BEFORE DELETE ON outbox
         BEGIN SELECT RAISE(ABORT, 'simulated dequeue failure'); END;",
    ).unwrap();
    assert!(resolve(&mut conn, "second", 8, vec![]).is_err());
    assert_eq!(read_cache(&conn, scope, "contact:target").unwrap()["revision"], 6);
    assert_eq!(read_cache(&conn, scope, "products:target").unwrap()["revision"], 6);
    assert_eq!(read_cache(&conn, scope, "contact-conflict:target").unwrap(), first_archive);
    assert_eq!(run(&mut conn, scope, CacheRequest::Pending).unwrap()[0]["id"], "second");
    conn.execute_batch("DROP TRIGGER block_contact_conflict_dequeue;").unwrap();

    assert_eq!(resolve(&mut conn, "second", 8, vec![]).unwrap(), json!(true));
    let archive = read_cache(&conn, scope, "contact-conflict:target").unwrap();
    assert_eq!(archive.as_array().unwrap().len(), 2);
    assert_eq!(archive[0]["operations"][0]["id"], "second");
    assert_eq!(archive[1], first_archive[0]);
    run(&mut conn, scope, CacheRequest::Snapshot { contacts: vec![] }).unwrap();
    assert_eq!(read_cache(&conn, scope, "contact-conflict:target").unwrap(), archive);
}

#[cfg(test)]
const PRODUCT_SEO_SCOPE: &str = r#"["project","user","workspace","product-seo"]"#;

#[cfg(test)]
const STORE_PAGE_SCOPE: &str = r#"["project","user","workspace","store-pages"]"#;

#[cfg(test)]
fn store_page_record(kind: &str, id: u64, revision: u64, title: &str, scope: &str) -> Value {
    json!({
        "id": format!("{kind}:{id}"), "kind": kind, "remote_id": id, "revision": revision,
        "team_scope": scope, "draft_page_title": title,
    })
}

#[cfg(test)]
fn queue_store_page_test(
    conn: &mut Connection,
    id: &str,
    page: &Value,
    payload: Option<Value>,
    conflict: bool,
) {
    let scope = page["team_scope"].as_str().unwrap();
    let payload = payload.unwrap_or_else(|| json!({
        "target_workspace": "workspace", "target_kind": page["kind"], "target_id": page["remote_id"],
        "operation_id": id, "expected_revision": page["revision"].as_u64().unwrap() - 1,
        "changes": { "draft_page_title": page["draft_page_title"] },
    }));
    run(
        conn,
        scope,
        CacheRequest::Queue {
            id: id.into(),
            contact: page.clone(),
            payload,
        },
    )
    .unwrap();
    if conflict {
        run(
            conn,
            scope,
            CacheRequest::Conflict {
                id: id.into(),
                reason: "Conflict".into(),
            },
        )
        .unwrap();
    }
}

#[cfg(test)]
fn review_store_page_test(
    conn: &mut Connection,
    scope: &str,
    kind: &str,
    remote_id: u64,
    ids: &[&str],
    remote: &Value,
    unsaved: Value,
) -> Value {
    let request = serde_json::from_value(json!({
        "action": "resolve_store_page_conflict", "kind": kind, "remote_id": remote_id,
        "expected_ids": ids, "remote": remote, "unsaved": unsaved,
    }))
    .unwrap();
    run(conn, scope, request).unwrap()
}

#[cfg(test)]
#[test]
fn store_page_conflict_keeps_facts_and_receipts_separate_from_the_ordered_draft_archive() {
    let mut conn = Connection::open_in_memory().unwrap();
    let scope = STORE_PAGE_SCOPE;
    let other_scope = r#"["project","user","other-workspace","store-pages"]"#;
    let first = store_page_record("category", 3, 2, "First title", scope);
    let second = store_page_record("category", 3, 3, "Last queued title", scope);
    let remote = store_page_record("category", 3, 4, "Shared title", scope);
    let sibling = store_page_record("category", 4, 11, "Sibling", scope);
    let brand = store_page_record("brand", 3, 11, "Brand", scope);
    let other = store_page_record("category", 3, 6, "Other workspace", other_scope);
    queue_store_page_test(&mut conn, "draft-1", &first, None, true);
    queue_store_page_test(
        &mut conn,
        "receipt",
        &first,
        Some(json!({ "receipt": "receipt token" })),
        true,
    );
    queue_store_page_test(
        &mut conn,
        "patch-facts",
        &first,
        Some(json!({ "facts": { "status": 1 } })),
        true,
    );
    queue_store_page_test(&mut conn, "draft-2", &second, None, false);
    queue_store_page_test(&mut conn, "brand", &brand, None, true);
    queue_store_page_test(&mut conn, "other-workspace", &other, None, true);
    upsert(&conn, scope, "category", &json!([first, sibling])).unwrap();
    upsert(&conn, scope, "brand", &json!([brand])).unwrap();
    let inventory = json!({ "id": "inventory-1", "rows": [] });
    upsert(&conn, scope, "inventory", &inventory).unwrap();

    let unsaved = json!({"draft_showcase": "Unsaved showcase"});
    for ids in [
        &["draft-2", "draft-1"][..],
        &["draft-1"][..],
        &["draft-1", "receipt", "draft-2"][..],
    ] {
        assert_eq!(
            review_store_page_test(
                &mut conn,
                scope,
                "category",
                3,
                ids,
                &remote,
                unsaved.clone()
            ),
            false
        );
    }
    assert_eq!(
        read_cache(&conn, scope, "store-page-conflict:category:3").unwrap(),
        Value::Null
    );
    assert_eq!(
        review_store_page_test(
            &mut conn,
            scope,
            "category",
            3,
            &["draft-1", "draft-2"],
            &remote,
            unsaved
        ),
        true
    );

    let archive = read_cache(&conn, scope, "store-page-conflict:category:3").unwrap();
    let archive = &archive[0];
    assert!(DateTime::parse_from_rfc3339(archive["saved_at"].as_str().unwrap()).is_ok());
    assert_eq!(archive["local"]["draft_page_title"], "Last queued title");
    assert_eq!(archive["local"]["draft_showcase"], "Unsaved showcase");
    assert_eq!(archive["remote"], remote);
    let ids: Vec<_> = archive["operations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|op| &op["id"])
        .collect();
    assert_eq!(ids, vec!["draft-1", "draft-2"]);
    assert_eq!(
        read_cache(&conn, scope, "category").unwrap(),
        json!([remote, sibling])
    );
    assert_eq!(read_cache(&conn, scope, "brand").unwrap(), json!([brand]));
    assert_eq!(read_cache(&conn, scope, "inventory").unwrap(), inventory);
    let pending = run(&mut conn, scope, CacheRequest::Pending).unwrap();
    let ids: Vec<_> = pending
        .as_array()
        .unwrap()
        .iter()
        .map(|op| &op["id"])
        .collect();
    assert_eq!(ids, vec!["receipt", "patch-facts", "brand"]);
    assert_eq!(pending[0]["conflict"], "Conflict");
    assert_eq!(
        run(&mut conn, other_scope, CacheRequest::Pending).unwrap()[0]["id"],
        "other-workspace"
    );
}

#[cfg(test)]
#[test]
fn store_page_conflict_rejects_stale_scopes_and_newer_rows_then_archives_an_unsaved_only_draft() {
    let mut conn = Connection::open_in_memory().unwrap();
    let scope = STORE_PAGE_SCOPE;
    let remote = store_page_record("brand", 3, 2, "Shared title", scope);
    let newer = store_page_record("brand", 3, 3, "Newer cached title", scope);
    let unsaved = json!({ "draft_page_title": "Unsaved title", "draft_target_keyword": "" });
    run(
        &mut conn,
        scope,
        CacheRequest::Write {
            key: "brand".into(),
            data: json!([newer]),
        },
    )
    .unwrap();
    assert_eq!(
        review_store_page_test(&mut conn, scope, "brand", 3, &[], &remote, unsaved.clone()),
        false
    );
    assert_eq!(
        review_store_page_test(&mut conn, scope, "blog", 3, &[], &newer, unsaved.clone()),
        false
    );
    assert_eq!(
        review_store_page_test(
            &mut conn,
            scope,
            "brand",
            3,
            &[],
            &newer,
            json!({ "name": "Changed" })
        ),
        false
    );
    let wrong_scope = r#"["project","user","workspace","quotes"]"#;
    let wrong = store_page_record("brand", 3, 3, "Wrong workspace", wrong_scope);
    assert_eq!(
        review_store_page_test(
            &mut conn,
            wrong_scope,
            "brand",
            3,
            &[],
            &wrong,
            unsaved.clone()
        ),
        false
    );
    let receipt = json!({ "facts": { "status": 1 }, "receipt": "token" });
    queue_store_page_test(&mut conn, "receipt", &newer, Some(receipt), true);
    assert_eq!(
        review_store_page_test(&mut conn, scope, "brand", 3, &[], &newer, unsaved),
        true
    );
    let archive = read_cache(&conn, scope, "store-page-conflict:brand:3").unwrap();
    assert_eq!(archive[0]["local"]["draft_page_title"], "Unsaved title");
    assert_eq!(archive[0]["local"]["draft_target_keyword"], "");
    assert_eq!(archive[0]["operations"], json!([]));
    assert_eq!(archive[0]["remote"], newer);
    assert_eq!(
        run(&mut conn, scope, CacheRequest::Pending).unwrap()[0]["id"],
        "receipt"
    );
}

#[cfg(test)]
#[test]
fn store_page_conflict_rolls_back_if_a_reviewed_draft_cannot_be_removed() {
    let mut conn = Connection::open_in_memory().unwrap();
    let scope = STORE_PAGE_SCOPE;
    let first = store_page_record("blog", 3, 2, "Queued title", scope);
    let remote = store_page_record("blog", 3, 4, "Shared title", scope);
    queue_store_page_test(&mut conn, "draft", &first, None, true);
    conn.execute_batch(
        "CREATE TRIGGER block_store_page_removal BEFORE DELETE ON outbox
        WHEN OLD.id = 'draft' BEGIN SELECT RAISE(IGNORE); END;",
    )
    .unwrap();
    assert_eq!(
        review_store_page_test(&mut conn, scope, "blog", 3, &["draft"], &remote, json!({})),
        false
    );
    assert_eq!(read_cache(&conn, scope, "blog").unwrap(), Value::Null);
    assert_eq!(
        read_cache(&conn, scope, "store-page-conflict:blog:3").unwrap(),
        Value::Null
    );
    assert_eq!(
        run(&mut conn, scope, CacheRequest::Pending).unwrap()[0]["id"],
        "draft"
    );
}

#[cfg(test)]
fn product_seo_record(sku: &str, revision: u64, title: &str) -> Value {
    json!({ "id": sku, "sku": sku, "revision": revision, "draft": { "draft_title": title } })
}

#[cfg(test)]
fn queue_product_seo(conn: &mut Connection, scope: &str, workspace: &str, id: &str, record: Value) {
    let payload = json!({
        "target_workspace": workspace,
        "operation_id": id,
        "target_sku": record["sku"],
        "expected_revision": record["revision"].as_u64().unwrap() - 1,
        "changes": record["draft"],
    });
    run(
        conn,
        scope,
        CacheRequest::Queue {
            id: id.into(),
            contact: record,
            payload,
        },
    )
    .unwrap();
}

#[cfg(test)]
fn review_product_seo(
    conn: &mut Connection,
    scope: &str,
    ids: &[&str],
    remote: &Value,
    unsaved: Value,
) -> rusqlite::Result<Value> {
    let request = serde_json::from_value(json!({
        "action": "resolve_product_seo_conflict",
        "sku": "TARGET",
        "expected_ids": ids,
        "remote": remote,
        "unsaved": unsaved,
    }))
    .unwrap();
    run(conn, scope, request)
}

#[cfg(test)]
#[test]
fn product_seo_conflict_archives_the_exact_sku_queue_and_unsaved_fields() {
    let mut conn = Connection::open_in_memory().unwrap();
    let scope = PRODUCT_SEO_SCOPE;
    let other_scope = r#"["project","other-user","other-workspace","product-seo"]"#;
    let wrong_kind = r#"["project","user","workspace","quotes"]"#;
    let queued = product_seo_record("TARGET", 6, "Last queued title");
    let remote = product_seo_record("TARGET", 7, "Shared title");
    queue_product_seo(
        &mut conn,
        scope,
        "workspace",
        "first",
        product_seo_record("TARGET", 5, "Queued title"),
    );
    queue_product_seo(
        &mut conn,
        scope,
        "workspace",
        "other-sku",
        product_seo_record("OTHER", 5, "Unrelated"),
    );
    queue_product_seo(&mut conn, scope, "workspace", "second", queued.clone());
    queue_product_seo(
        &mut conn,
        other_scope,
        "other-workspace",
        "first",
        product_seo_record("TARGET", 5, "Other user's draft"),
    );
    queue_product_seo(
        &mut conn,
        wrong_kind,
        "workspace",
        "third",
        product_seo_record("TARGET", 5, "Other cache kind"),
    );

    let ids = ["first", "second"];
    assert_eq!(
        review_product_seo(&mut conn, scope, &ids, &remote, json!({})).unwrap(),
        json!(false)
    );
    for (owner, id) in [(scope, "first"), (wrong_kind, "third")] {
        run(
            &mut conn,
            owner,
            CacheRequest::Conflict {
                id: id.into(),
                reason: "Revision conflict".into(),
            },
        )
        .unwrap();
    }
    for rejected in [
        &[][..],
        &["first"][..],
        &["second", "first"][..],
        &["first", "other-sku", "second"][..],
    ] {
        assert_eq!(
            review_product_seo(&mut conn, scope, rejected, &remote, json!({})).unwrap(),
            json!(false)
        );
    }
    assert_eq!(
        review_product_seo(&mut conn, wrong_kind, &["third"], &remote, json!({})).unwrap(),
        json!(false)
    );
    for rejected in [
        json!({"draft_title": 1}),
        json!({"sku": "OTHER"}),
        Value::Null,
    ] {
        assert_eq!(
            review_product_seo(&mut conn, scope, &ids, &remote, rejected).unwrap(),
            json!(false)
        );
    }
    let mut foreign = remote.clone();
    foreign["sku"] = json!("OTHER");
    assert_eq!(
        review_product_seo(&mut conn, scope, &ids, &foreign, json!({})).unwrap(),
        json!(false)
    );

    let unsaved = json!({"draft_title": "Still typing", "tech_source_text": "Local specs"});
    assert_eq!(
        review_product_seo(&mut conn, scope, &ids, &remote, unsaved.clone()).unwrap(),
        json!(true)
    );
    assert_eq!(
        review_product_seo(&mut conn, scope, &ids, &remote, unsaved.clone()).unwrap(),
        json!(false)
    );
    let history = read_cache(&conn, scope, "product-seo-conflict:TARGET").unwrap();
    assert_eq!(history.as_array().unwrap().len(), 1);
    assert!(DateTime::parse_from_rfc3339(history[0]["saved_at"].as_str().unwrap()).is_ok());
    assert_eq!(history[0]["local"]["revision"], queued["revision"]);
    assert_eq!(history[0]["local"]["draft"], unsaved);
    assert_eq!(
        history[0]["operations"][0]["contact"]["draft"]["draft_title"],
        "Queued title"
    );
    assert_eq!(history[0]["operations"][1]["contact"], queued);
    assert_eq!(history[0]["remote"], remote);
    assert_eq!(read_cache(&conn, scope, "contact:TARGET").unwrap(), remote);
    assert_eq!(
        read_cache(&conn, other_scope, "product-seo-conflict:TARGET").unwrap(),
        Value::Null
    );
    let remaining = run(&mut conn, scope, CacheRequest::Pending).unwrap();
    assert_eq!(remaining.as_array().unwrap().len(), 1);
    assert_eq!(remaining[0]["id"], "other-sku");
    assert_eq!(
        run(&mut conn, other_scope, CacheRequest::Pending).unwrap()[0]["id"],
        "first"
    );
}

#[cfg(test)]
#[test]
fn product_seo_conflict_preserves_queue_and_history_on_stale_or_failed_update() {
    let mut conn = Connection::open_in_memory().unwrap();
    let scope = PRODUCT_SEO_SCOPE;
    let queue = |conn: &mut Connection, id: &str, revision| {
        queue_product_seo(
            conn,
            scope,
            "workspace",
            id,
            product_seo_record("TARGET", revision, id),
        );
        run(
            conn,
            scope,
            CacheRequest::Conflict {
                id: id.into(),
                reason: "Revision conflict".into(),
            },
        )
        .unwrap();
    };

    queue(&mut conn, "first", 4);
    let current = product_seo_record("TARGET", 6, "Shared title");
    run(
        &mut conn,
        scope,
        CacheRequest::Write {
            key: "contact:TARGET".into(),
            data: current.clone(),
        },
    )
    .unwrap();
    let stale = product_seo_record("TARGET", 5, "Older title");
    assert_eq!(
        review_product_seo(&mut conn, scope, &["first"], &stale, json!({})).unwrap(),
        json!(false)
    );
    assert_eq!(
        read_cache(&conn, scope, "product-seo-conflict:TARGET").unwrap(),
        Value::Null
    );
    assert_eq!(
        run(&mut conn, scope, CacheRequest::Pending).unwrap()[0]["id"],
        "first"
    );
    assert_eq!(
        review_product_seo(&mut conn, scope, &["first"], &current, json!({})).unwrap(),
        json!(true)
    );
    let first_history = read_cache(&conn, scope, "product-seo-conflict:TARGET").unwrap();

    queue(&mut conn, "second", 7);
    let newer = product_seo_record("TARGET", 8, "New shared title");
    conn.execute_batch(
        "CREATE TRIGGER block_product_seo_conflict_dequeue BEFORE DELETE ON outbox
         BEGIN SELECT RAISE(ABORT, 'simulated dequeue failure'); END;",
    )
    .unwrap();
    assert!(review_product_seo(&mut conn, scope, &["second"], &newer, json!({})).is_err());
    assert_eq!(read_cache(&conn, scope, "contact:TARGET").unwrap(), current);
    assert_eq!(
        read_cache(&conn, scope, "product-seo-conflict:TARGET").unwrap(),
        first_history
    );
    assert_eq!(
        run(&mut conn, scope, CacheRequest::Pending).unwrap()[0]["id"],
        "second"
    );
    conn.execute_batch("DROP TRIGGER block_product_seo_conflict_dequeue;")
        .unwrap();

    assert_eq!(
        review_product_seo(&mut conn, scope, &["second"], &newer, json!({})).unwrap(),
        json!(true)
    );
    let history = read_cache(&conn, scope, "product-seo-conflict:TARGET").unwrap();
    assert_eq!(history.as_array().unwrap().len(), 2);
    assert_eq!(history[0]["operations"][0]["id"], "second");
    assert_eq!(history[1], first_history[0]);
    run(
        &mut conn,
        scope,
        CacheRequest::Snapshot { contacts: vec![] },
    )
    .unwrap();
    assert_eq!(
        read_cache(&conn, scope, "product-seo-conflict:TARGET").unwrap(),
        history
    );
}

#[cfg(test)]
#[test]
fn quote_conflict_resolution_requires_the_exact_queue_and_preserves_other_work() {
    let mut conn = Connection::open_in_memory().unwrap();
    let scope = "workspace:a";
    for (queue_scope, id, quote_id, revision) in [
        (scope, "first", "target", 2),
        (scope, "other", "other-quote", 2),
        (scope, "last", "target", 3),
        ("workspace:b", "separate", "target", 2),
    ] {
        run(
            &mut conn,
            queue_scope,
            CacheRequest::Queue {
                id: id.into(),
                contact: json!({ "id": quote_id, "revision": revision }),
                payload: json!({"expected_revision": revision - 1}),
            },
        )
        .unwrap();
    }
    let remote = json!({ "id": "target", "revision": 4 });
    let resolve = |conn: &mut Connection, ids: &[&str]| {
        run(
            conn,
            scope,
            CacheRequest::ResolveQuoteConflict {
                quote_id: "target".into(),
                expected_ids: ids.iter().map(|id| (*id).into()).collect(),
                remote: remote.clone(),
            },
        )
        .unwrap()
    };
    assert_eq!(resolve(&mut conn, &["first", "last"]), json!(false));
    run(
        &mut conn,
        scope,
        CacheRequest::Conflict {
            id: "first".into(),
            reason: "revision changed".into(),
        },
    )
    .unwrap();
    assert_eq!(resolve(&mut conn, &["first"]), json!(false));
    assert_eq!(
        resolve(&mut conn, &["first", "other", "last"]),
        json!(false)
    );
    assert_eq!(resolve(&mut conn, &["first", "last"]), json!(true));
    let remaining = run(&mut conn, scope, CacheRequest::Pending).unwrap();
    assert_eq!(remaining.as_array().unwrap().len(), 1);
    assert_eq!(remaining[0]["id"], "other");
    assert_eq!(
        run(&mut conn, "workspace:b", CacheRequest::Pending).unwrap()[0]["id"],
        "separate"
    );
    let read = |conn: &mut Connection, key: &str| {
        run(conn, scope, CacheRequest::Read { key: key.into() }).unwrap()
    };
    assert_eq!(read(&mut conn, "contact:target"), remote);
    assert_eq!(
        read(&mut conn, "quote-conflict:target:last")[1]["contact"]["revision"],
        3
    );
    assert_eq!(read(&mut conn, "quote-conflict-remote:target:last"), remote);
    let saved_at = read(&mut conn, "quote-conflict-at:target:last");
    assert!(DateTime::parse_from_rfc3339(saved_at.as_str().unwrap()).is_ok());
}

#[cfg(test)]
#[test]
fn quote_conflict_history_reads_legacy_archives_in_order_and_keeps_records_scoped() {
    let mut conn = Connection::open_in_memory().unwrap();
    let scope = r#"["project","user","workspace","quotes"]"#;
    let other_workspace = r#"["project","user","other-workspace","quotes"]"#;
    let other_user = r#"["project","other-user","workspace","quotes"]"#;
    let old_date = "2026-11-18T12:00:00Z";
    let record = |id: &str, quote_id: &str, note: &str| {
        json!({ "id": id, "contact": { "id": quote_id, "note": note, "updated_at": old_date } })
    };
    for (area, key, data) in [
        (
            scope,
            "quote-conflict:target:older",
            json!([record("earlier-op", "target", "earlier draft"), record("older", "target", "legacy local")]),
        ),
        (scope, "quote-conflict:target:reapplied", json!([record("reapplied", "target", "prior local")])),
        (scope, "quote-conflict-remote:target:reapplied", json!({ "id": "target", "note": "prior shared" })),
        (scope, "quote-conflict:target:new", json!([record("new", "target", "latest local")])),
        (scope, "quote-conflict-at:target:new", json!("2026-11-25T10:00:00Z")),
        (scope, "quote-conflict-remote:target:new", json!({ "id": "target", "note": "latest shared" })),
        (scope, "quote-conflict:target-extra:other", json!([record("other", "target-extra", "other quote")])),
        (scope, "quote-conflict:target:malformed", json!([record("malformed", "other-quote", "wrong record")])),
        (other_workspace, "quote-conflict:target:other-workspace", json!([record("other-workspace", "target", "workspace draft")])),
        (other_user, "quote-conflict:target:new", json!([record("new", "target", "user draft")])),
    ] {
        run(&mut conn, area, CacheRequest::Write { key: key.into(), data }).unwrap();
    }

    let request = serde_json::from_value(json!({"action": "quote_conflict_history", "quote_id": "target"})).unwrap();
    let archives = run(&mut conn, scope, request).unwrap();
    assert_eq!(archives, json!([
        { "id": "new", "saved_at": "2026-11-25T10:00:00Z", "local": record("new", "target", "latest local")["contact"],
            "remote": { "id": "target", "note": "latest shared" } },
        { "id": "reapplied", "saved_at": null, "local": record("reapplied", "target", "prior local")["contact"],
            "remote": { "id": "target", "note": "prior shared" } },
        { "id": "older", "saved_at": null, "local": record("older", "target", "legacy local")["contact"], "remote": null },
    ]));
    assert_eq!(quote_conflict_history(&conn, scope, "missing").unwrap(), json!([]));
    assert_eq!(quote_conflict_history(&conn, scope, "target:new").unwrap(), json!([]));
    assert_eq!(quote_conflict_history(&conn, r#"["project","user","workspace","store-pages"]"#, "target").unwrap(), json!([]));
}

#[cfg(test)]
#[test]
fn quote_conflict_retry_archives_both_drafts_and_replaces_only_the_reviewed_operations() {
    let mut conn = Connection::open_in_memory().unwrap();
    let scope = "workspace:a";
    for (id, quote_id) in [("stale", "target"), ("other", "unrelated")] {
        run(
            &mut conn,
            scope,
            CacheRequest::Queue {
                id: id.into(),
                contact: json!({ "id": quote_id, "note": "local" }),
                payload: json!({}),
            },
        )
        .unwrap();
    }
    run(
        &mut conn,
        scope,
        CacheRequest::Conflict { id: "stale".into(), reason: "revision changed".into() },
    )
    .unwrap();

    let remote = json!({ "id": "target", "status": "draft", "revision": 4, "note": "remote" });
    let ids = ["stale".to_string()];
    let retry = |id: &str, revision: u64| QuoteConflictRetry {
        id: id.into(),
        contact: json!({ "id": "target", "status": "draft", "revision": revision + 1, "note": "merged" }),
        payload: json!({ "target_quote": "target", "expected_revision": revision, "operation_id": id }),
    };
    let resolve = |conn: &mut Connection, retry: QuoteConflictRetry| {
        run(
            conn,
            scope,
            CacheRequest::ReapplyQuoteConflict {
                quote_id: "target".into(),
                expected_ids: ids.to_vec(),
                remote: remote.clone(),
                retry,
            },
        )
    };
    let read = |conn: &mut Connection, key: &str| {
        run(conn, scope, CacheRequest::Read { key: key.into() }).unwrap()
    };
    assert_eq!(resolve(&mut conn, retry("fresh", 3)).unwrap(), json!(false));
    assert!(resolve(&mut conn, retry("other", 4)).is_err());
    assert_eq!(read(&mut conn, "quote-conflict:target:stale"), Value::Null);
    assert_eq!(read(&mut conn, "quote-conflict-remote:target:stale"), Value::Null);
    assert_eq!(read(&mut conn, "quote-conflict-at:target:stale"), Value::Null);
    assert_eq!(read(&mut conn, "contact:target"), Value::Null);
    assert_eq!(run(&mut conn, scope, CacheRequest::Pending).unwrap()[0]["id"], "stale");

    assert_eq!(resolve(&mut conn, retry("fresh", 4)).unwrap(), json!(true));
    assert_eq!(resolve(&mut conn, retry("different", 4)).unwrap(), json!(false));
    let queue = run(&mut conn, scope, CacheRequest::Pending).unwrap();
    assert_eq!(queue.as_array().unwrap().len(), 2);
    assert_eq!(queue[0]["id"], "other");
    assert_eq!(queue[1]["id"], "fresh");
    assert_eq!(queue[1]["contact"]["note"], "merged");
    assert_eq!(queue[1]["conflict"], Value::Null);
    assert_eq!(read(&mut conn, "quote-conflict:target:stale")[0]["contact"]["note"], "local");
    assert_eq!(read(&mut conn, "quote-conflict-remote:target:stale"), remote);
    let saved_at = read(&mut conn, "quote-conflict-at:target:stale");
    assert!(DateTime::parse_from_rfc3339(saved_at.as_str().unwrap()).is_ok());
    assert_eq!(read(&mut conn, "contact:target"), remote);
}

#[cfg(test)]
#[test]
fn quote_conflict_cache_actions_have_separate_protocols() {
    let mut request = json!({
        "action": "resolve_quote_conflict", "quote_id": "target", "expected_ids": ["stale"],
        "remote": {"id": "target", "status": "draft", "revision": 4},
    });
    assert!(matches!(serde_json::from_value(request.clone()), Ok(CacheRequest::ResolveQuoteConflict { .. })));

    request["action"] = json!("reapply_quote_conflict");
    assert!(serde_json::from_value::<CacheRequest>(request.clone()).is_err());
    request["retry"] = json!({ "id": "fresh", "contact": {}, "payload": {} });
    assert!(matches!(serde_json::from_value(request), Ok(CacheRequest::ReapplyQuoteConflict { .. })));
}

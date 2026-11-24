use super::AppState;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Deserialize;
use serde_json::{json, Value};
use tauri::State;

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
    Conflict {
        id: String,
        reason: String,
    },
    Discard {
        ids: Vec<String>,
    },
    ResolveQuoteConflict {
        quote_id: String,
        expected_ids: Vec<String>,
        remote: Value,
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
        CacheRequest::Read { key } => {
            let value = conn
                .query_row(
                    "SELECT data FROM cache WHERE scope = ?1 AND key = ?2",
                    params![scope, key],
                    |row| row.get::<_, String>(0),
                )
                .optional()?;
            Ok(value
                .map(|text| serde_json::from_str(&text))
                .transpose()
                .map_err(json_error)?
                .unwrap_or(Value::Null))
        }
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
        CacheRequest::ResolveQuoteConflict {
            quote_id,
            expected_ids,
            remote,
        } => {
            resolve_quote_conflict(conn, scope, &quote_id, &expected_ids, &remote).map(Value::Bool)
        }
    }
}

fn resolve_quote_conflict(
    conn: &mut Connection,
    scope: &str,
    quote_id: &str,
    expected_ids: &[String],
    remote: &Value,
) -> rusqlite::Result<bool> {
    if expected_ids.is_empty()
        || remote["id"].as_str() != Some(quote_id)
        || remote["revision"].as_u64().unwrap_or(0) == 0
    {
        return Ok(false);
    }
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let operations = {
        let mut stmt = tx.prepare(
            "SELECT id, contact, payload, conflict FROM outbox
             WHERE scope = ?1 AND json_extract(contact, '$.id') = ?2 ORDER BY seq",
        )?;
        let rows = stmt.query_map(params![scope, quote_id], |row| {
            let contact: Value =
                serde_json::from_str(&row.get::<_, String>(1)?).map_err(json_error)?;
            let payload: Value =
                serde_json::from_str(&row.get::<_, String>(2)?).map_err(json_error)?;
            Ok(json!({ "id": row.get::<_, String>(0)?, "contact": contact,
                "payload": payload, "conflict": row.get::<_, Option<String>>(3)? }))
        })?;
        rows.collect::<Result<Vec<_>, _>>()?
    };
    let ids = operations
        .iter()
        .map(|op| op["id"].as_str().unwrap_or_default());
    if !ids.eq(expected_ids.iter().map(String::as_str))
        || !operations
            .iter()
            .any(|op| op["conflict"].as_str().is_some())
    {
        return Ok(false);
    }
    // Preserve the inspected local operations separately from the sync queue.
    let recovery = format!(
        "quote-conflict:{}:{}",
        quote_id,
        expected_ids.last().unwrap()
    );
    upsert(&tx, scope, &recovery, &Value::Array(operations))?;
    upsert(&tx, scope, &format!("contact:{quote_id}"), remote)?;
    tx.execute(
        "DELETE FROM outbox WHERE scope = ?1 AND json_extract(contact, '$.id') = ?2",
        params![scope, quote_id],
    )?;
    tx.commit()?;
    Ok(true)
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
}

//! Agent-reported context use, bound to the current session and model rather than inferred from chat text.
use crate::hub::Hub;
use crate::store::{BotConfig, BotRow, Store, now_ms};
use anyhow::Result;
use serde_json::{Value, json};

pub fn record(hub: &Hub, cfg: &BotConfig, session: &str, used: u64, size: u64) -> Result<()> {
    if size == 0 || used > size || size > 9_000_000_000_000_000 {
        return Ok(());
    }
    let value = json!({
        "session": session, "backend": cfg.backend, "model": cfg.model,
        "used": used, "size": size, "reportedAt": now_ms()
    });
    hub.store.kv_set(&format!("context.usage.{}", cfg.id), &value.to_string())?;
    hub.emit_bot(&cfg.id);
    Ok(())
}

pub fn snapshot(store: &Store, row: &BotRow) -> Option<Value> {
    if row.config.is_group() || row.deleted {
        return None;
    }
    let value: Value = serde_json::from_str(&store.kv_get(&format!("context.usage.{}", row.config.id))?).ok()?;
    let session = row.session_id.as_deref()?;
    if value["session"] != session
        || value["backend"] != row.config.backend
        || value["model"] != json!(row.config.model)
    {
        return None;
    }
    Some(json!({"used": value["used"], "size": value["size"], "reportedAt": value["reportedAt"]}))
}

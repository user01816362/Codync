//! Recorded delivery state, scoped to the caller's own chat and rooms it belongs to.
use crate::hub::Hub;
use crate::store::Lane;
use anyhow::{Result, anyhow, ensure};
use serde_json::{Value, json};

pub(super) fn tool() -> Value {
    json!({
        "name": "message_status",
        "description": "Inspect recorded delivery of your latest message or a known entry in your own chat or a group you belong to. A userSeen flag means a client acknowledged the chat on screen, not proof of comprehension. For bot requests use requestId from message_bot; queued is not started, sent means its turn started. Do not poll; replies arrive automatically.",
        "inputSchema": {"type": "object", "additionalProperties": false, "properties": {
            "chatId": {"type": "string", "description": "Optional group ID; defaults to your own chat."},
            "entryId": {"type": "string"}, "requestId": {"type": "string"}
        }},
        "annotations": {"readOnlyHint": true, "idempotentHint": true}
    })
}

pub(super) fn call(hub: &Hub, caller: &str, args: &Value) -> Result<Value> {
    let chat = args["chatId"].as_str().unwrap_or(caller);
    let row = hub.store.bot(chat)?.filter(|row| !row.deleted).ok_or_else(|| anyhow!("unknown chat"))?;
    ensure!(
        chat == caller || (row.config.is_group() && row.config.members.iter().any(|id| id == caller)),
        "chat is not accessible"
    );
    ensure!(args.get("entryId").is_none() || args.get("requestId").is_none(), "provide entryId or requestId, not both");
    let entry = if let Some(id) = args["entryId"].as_str() {
        hub.store.entry(id).filter(|entry| entry.bot_id == chat)
    } else if let Some(id) = args["requestId"].as_str() {
        ensure!(id.len() <= 200, "invalid requestId");
        hub.store.history(chat, i64::MAX, 200)?.into_iter().rev().find(|entry| entry.data["delegationId"] == id)
    } else {
        hub.store
            .messages_after(&Lane::main(chat), 0, 200)?
            .into_iter()
            .rev()
            .find(|entry| entry.kind == "agent" && (entry.data["author"] == caller || chat == caller))
    };
    let Some(entry) = entry else {
        return Ok(
            json!({"found": false, "note": "No matching entry in the recent history; absence is not a delivery failure."}),
        );
    };
    let mut member_prompts = Vec::new();
    if row.config.is_group() {
        for member in row.config.members.iter().take(50) {
            let key = crate::chat::group::receipt_key(chat, member);
            let receipt = hub.store.kv_read(&key)?.map(|raw| serde_json::from_str::<Value>(&raw)).transpose()?;
            member_prompts.push(json!({"botId": member, "lastPromptDispatch": receipt}));
        }
    }
    Ok(json!({
        "found": true, "entryId": entry.id, "seq": entry.seq, "chatId": chat,
        "memberPrompts": member_prompts,
        "memberPromptMeaning": "A cursor written to the agent's prompt transport; bounded context may omit or clip older messages. Not proof of reading or comprehension. Missing receipts are unknown.",
        "status": entry.data.get("status"),
        "requestId": entry.data.get("delegationId"),
        "userSeen": (entry.kind == "agent" && entry.data["final"] == true).then_some(row.read_rev >= entry.rev),
        "botStartedAt": entry.data["botMessage"].get("startedAt"),
        "userSeenMeaning": "A client acknowledged this chat after this entry revision; not proof a human read it.",
        "botStarted": match entry.data.get("botMessage").and_then(|_| entry.data["status"].as_str()) {
            Some("sent" | "completed") => Some(true),
            _ if entry.data["botMessage"]["startedAt"].is_i64() => Some(true),
            Some("queued") => Some(false),
            _ => None,
        },
        "note": "Failed or cancelled requests may have performed partial work. A mention alone outside a group never queues a bot; use message_bot."
    }))
}

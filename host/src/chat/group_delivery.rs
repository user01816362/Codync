//! Group communication without granting group-management rights or private history access.
use crate::hub::Hub;
use crate::store::{BotConfig, EntryKind, Lane};
use anyhow::{Context as _, Result, ensure};
use serde_json::{Value, json};
use std::sync::Arc;

pub const INSTRUCTIONS: &str = "Use list_chat_groups to discover groups and your membership. \
Use post_group_message to send a message to another group you belong to, including from a different group. \
In the group currently asking you, write your normal final reply instead of posting to yourself. \
read_group_messages reads only groups you currently belong to. list_group_tasks and read_group_task inspect recorded assignments/results in those groups without changing them. A successful post means delivered, not answered. \
Use these tools only for the user's task; do not forward private conversations or entire transcripts unasked. \
Never claim a group replied until you have read an actual response.";

pub fn tools() -> Vec<Value> {
    vec![
        json!({"name":"list_chat_groups", "description":"List group names, purposes and your membership. Does not read group conversations or grant management permissions.",
            "inputSchema":{"type":"object","properties":{},"additionalProperties":false},"annotations":{"readOnlyHint":true}}),
        json!({"name":"read_group_messages", "description":"Read the latest 24 messages of a group you currently belong to.",
            "inputSchema":{"type":"object","properties":{"groupId":{"type":"string"}},"required":["groupId"],"additionalProperties":false},"annotations":{"readOnlyHint":true}}),
        json!({"name":"list_group_tasks","description":"List up to 20 recorded tasks and latest outcomes in a group you currently belong to. Completion is an execution outcome, not independent verification.","inputSchema":{"type":"object","properties":{"groupId":{"type":"string"}},"required":["groupId"],"additionalProperties":false},"annotations":{"readOnlyHint":true}}),
        json!({"name":"read_group_task","description":"Read one recorded assignment and bounded history in a group you currently belong to. Does not grant task-management permissions.","inputSchema":{"type":"object","properties":{"groupId":{"type":"string"},"taskId":{"type":"string"}},"required":["groupId","taskId"],"additionalProperties":false},"annotations":{"readOnlyHint":true}}),
        json!({"name":"post_group_message", "description":"Deliver one attributed message to another group you belong to. Returns delivery only; does not wait for replies. Use a stable unique requestId when retrying the same message.",
            "inputSchema":{"type":"object","properties":{"groupId":{"type":"string"},"text":{"type":"string"},"requestId":{"type":"string"}},"required":["groupId","text","requestId"],"additionalProperties":false},"annotations":{"readOnlyHint":false,"idempotentHint":true}}),
    ]
}

pub fn handles(name: &str) -> bool {
    matches!(
        name,
        "list_chat_groups" | "read_group_messages" | "post_group_message" | "list_group_tasks" | "read_group_task"
    )
}

pub fn call(hub: &Arc<Hub>, bot: &str, name: &str, args: &Value) -> Result<Value> {
    let source = agent(hub, bot)?;
    if name == "list_chat_groups" {
        let groups: Vec<Value> = hub
            .store
            .bots()?
            .into_iter()
            .filter(|row| !row.deleted && row.config.is_group())
            .map(|row| {
                json!({"id":row.config.id,"name":row.config.name,"purpose":row.config.description,
                "member":row.config.members.contains(&source.id)})
            })
            .collect();
        return Ok(json!({"groups":groups}));
    }
    let target = group(hub, string(args, "groupId")?)?;
    ensure!(
        target.members.contains(&source.id),
        "you are not a member of this group; do not read or post to its conversation"
    );
    if name == "list_group_tasks" {
        let entries = hub.store.group_tasks(&target.id)?;
        let items: Vec<Value> = entries.iter().take(20).map(|entry| {
            let task = &entry.data["groupTask"];
            let last = task["history"].as_array().and_then(|history| history.last());
            json!({"id":entry.id,"title":task["title"],"owner":task["owner"],"status":task["status"],"latestOutcome":last.and_then(|event| event["detail"].as_str()).map(|detail| detail.chars().take(1500).collect::<String>())})
        }).collect();
        return Ok(json!({"items":items,"truncated":entries.len() > 20,"limit":20}));
    }
    if name == "read_group_task" {
        let entry = hub
            .store
            .entry(string(args, "taskId")?)
            .filter(|entry| entry.bot_id == target.id && entry.data["groupTask"].is_object())
            .context("unknown group task")?;
        return Ok(json!({"groupId":target.id,"task":entry.data["groupTask"]}));
    }
    if name == "read_group_messages" {
        let entries = hub.store.messages_after(&Lane::main(&target.id), 0, 24)?;
        let messages: Vec<Value> = entries.into_iter().map(|entry| {
            let author = if entry.kind == EntryKind::User.as_str() { "user" }
                else { entry.data["author"].as_str().unwrap_or(&target.id) };
            json!({"id":entry.id,"author":author,"name":if author == "user" { "You".to_owned() } else { hub.bot_name(author) },
                "text":entry.data["text"],"createdAt":entry.created_at})
        }).collect();
        return Ok(json!({"groupId":target.id,"messages":messages}));
    }
    let from = source_chat(hub, &source)?;
    ensure!(from != target.id, "this is the group currently asking you; write your final reply to the room instead");
    let text = string(args, "text")?.trim();
    ensure!(!text.is_empty() && text.len() <= 32768, "text must be nonempty and at most 32 KiB");
    let request = string(args, "requestId")?;
    ensure!(
        !request.is_empty()
            && request.len() <= 80
            && request.bytes().all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_')),
        "requestId must be 1–80 letters, digits, dashes or underscores"
    );
    let id = format!("group-post:{}:{request}", source.id);
    let display =
        if from == source.id { text.to_owned() } else { format!("From {}\n\n{text}", group(hub, &from)?.name) };
    let data =
        json!({"text":display,"final":true,"author":source.id,"groupPost":{"sourceChat":from,"sourceBot":source.id}});
    let (entry, added) =
        hub.add_entry_once(&id, &Lane::main(&target.id), EntryKind::Agent, hub.store.max_turn(&target.id), &data)?;
    ensure!(
        entry.bot_id == target.id && entry.data["text"] == data["text"] && entry.data["groupPost"] == data["groupPost"],
        "requestId already belongs to a different message"
    );
    if added {
        crate::chat::group::start(hub, &target, Lane::main(&target.id));
        hub.emit_bot(&target.id);
    }
    Ok(json!({"status":"delivered","entryId":entry.id,"groupId":target.id,"duplicate":!added}))
}

pub(super) fn string<'a>(args: &'a Value, key: &str) -> Result<&'a str> {
    args[key].as_str().with_context(|| format!("{key} is required"))
}

pub(super) fn agent(hub: &Hub, id: &str) -> Result<BotConfig> {
    let row = hub.store.bot(id)?.filter(|row| !row.deleted && !row.config.is_group()).context("unknown bot")?;
    Ok(row.config)
}

pub(super) fn group(hub: &Hub, id: &str) -> Result<BotConfig> {
    let row = hub.store.bot(id)?.filter(|row| !row.deleted && row.config.is_group()).context("unknown group")?;
    Ok(row.config)
}

pub(super) fn source_chat(hub: &Hub, source: &BotConfig) -> Result<String> {
    let chat = hub.runtime(&source.id).lane.map_or_else(|| source.id.clone(), |lane| lane.chat);
    if chat != source.id {
        ensure!(group(hub, &chat)?.members.contains(&source.id), "you no longer belong to the source group");
    }
    Ok(chat)
}

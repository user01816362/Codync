//! The user's messages in a chat: typed (`send`) or relayed from a voice call (`chat::calls`).

use crate::agent::bot::Cmd;
use crate::hub::Hub;
use crate::store::{BotConfig, Entry, EntryKind, Lane, now_ms};
use anyhow::{Result, anyhow, bail};
use serde_json::Value;
use std::sync::Arc;

/// Posts `data` (the message's text and what it carries) as the user's newest message in `cfg`'s chat and hands it
/// on: to the bot's agent, or to a group's members.
pub fn post(hub: &Arc<Hub>, cfg: &BotConfig, mut data: Value) -> Result<Entry> {
    // A group has no agent of its own to queue behind: its message is simply sent.
    data["status"] = (if cfg.is_group() { "sent" } else { "queued" }).into();
    let lane = Lane::main(&cfg.id);
    let turn = hub.store.max_turn(&cfg.id) + 1;
    let e = hub.add_entry(&lane, EntryKind::User, turn, &data).ok_or_else(|| anyhow!("couldn't save the message"))?;
    // Before the turn starts, so its reply lands unread.
    if let Err(error) = hub.mark_read(&cfg.id) {
        tracing::warn!(%error, bot = %cfg.id, "couldn't mark bot read after send");
    }
    if cfg.is_group() {
        // Each member reads it in its room prompt (`chat::group`).
        crate::chat::group::start(hub, cfg, lane);
    } else {
        // `/skill …` reaches the agent as "read this skill first"; the chat keeps what was typed.
        let prompt = crate::chat::prompt::of_entry(hub, cfg, &e.data);
        hub.send_cmd(&cfg.id, Cmd::Send { lane, entry_id: e.id.clone(), text: prompt })?;
    }
    Ok(e)
}

/// Edits the user's sent message `id` and hands the edited text to the bot as a new turn, so a message
/// sent by mistake is answered again. Attachments stay; the message keeps its place in the chat and
/// only its turn moves, which orders the agent's work and its reply.
pub fn edit(hub: &Arc<Hub>, cfg: &BotConfig, id: &str, text: &str) -> Result<Entry> {
    if cfg.is_group() {
        bail!("a group's messages can't be edited");
    }
    let text = text.trim();
    // Checked before anything changes, so a refused edit leaves the message as it was.
    let before = hub.store.entry(id).filter(|e| e.bot_id == cfg.id && e.kind == EntryKind::User.as_str());
    let before = before.ok_or_else(|| anyhow!("unknown message"))?;
    match before.data["status"].as_str() {
        None | Some("sent") => {}
        Some("queued" | "sending") => bail!("this message is still waiting to be sent"),
        Some(_) => bail!("this message wasn't sent"),
    }
    if text.is_empty() && before.data["attachments"].as_array().is_none_or(Vec::is_empty) {
        bail!("empty message");
    }
    // Sent again unchanged (a retried request): the message is already right, and it needs no second answer.
    if text == before.data["text"].as_str().unwrap_or_default() {
        return Ok(before);
    }
    let turn = hub.store.max_turn(&cfg.id) + 1;
    let e = hub.edit_entry_in_turn(id, turn, |e| {
        if e.bot_id != cfg.id || e.kind != EntryKind::User.as_str() {
            bail!("unknown message");
        }
        e.data["text"] = text.into();
        e.data["editedAt"] = now_ms().into();
        Ok(true)
    })?;
    // Before the turn starts, so its reply lands unread.
    if let Err(error) = hub.mark_read(&cfg.id) {
        tracing::warn!(%error, bot = %cfg.id, "couldn't mark bot read after an edit");
    }
    let prompt = crate::chat::prompt::of_entry(hub, cfg, &e.data);
    hub.send_cmd(&cfg.id, Cmd::Send { lane: Lane::main(&cfg.id), entry_id: e.id.clone(), text: prompt })?;
    Ok(e)
}

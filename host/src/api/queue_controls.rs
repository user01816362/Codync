//! Queue mutations are serialized with the actor's dispatch.
use super::str_arg;
use crate::{agent::bot::Cmd, hub::Hub};
use anyhow::{Context as _, Result, ensure};
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;

pub async fn control(hub: &Arc<Hub>, body: &Value) -> Result<Value> {
    let bot = str_arg(body, "botId")?;
    let row = hub.store.bot(bot)?.context("unknown bot")?;
    ensure!(!row.deleted && !row.config.is_group(), "groups do not have a user-message queue");
    let (done, result) = tokio::sync::oneshot::channel();
    hub.send_cmd(bot, Cmd::QueueControl {
        action: str_arg(body, "action")?.to_owned(),
        entry_id: body["entryId"].as_str().map(str::to_owned),
        done,
    })?;
    tokio::time::timeout(Duration::from_secs(10), result).await.context("queue control timed out")?
        .context("queue control interrupted")?
}

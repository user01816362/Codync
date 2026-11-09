//! HTTP API: `POST /api/<method>` commands + `GET /events` SSE for this computer's own
//! apps and helpers (loopback + `Authorization: Bearer <token>`), and `GET /channel`, the
//! direct end-to-end encrypted channel phones use (see `channel`).

mod bots;
pub mod devices;
mod events;
mod files;
mod folders;
mod host;
mod http;
mod marketplace;
mod network;
mod queue_controls;
mod storage_usage;
mod reach;
mod remote;
mod workspace_browser;
mod workspace_preview;

pub use events::{events_stream, term_events};
pub use http::{public_router, router};

use crate::api::devices::{Caller, Forbidden};
use crate::hub::Hub;
use crate::market;
use anyhow::{Result, anyhow};
use axum::http::StatusCode;
use serde_json::Value;
use std::ops::ControlFlow;
use std::sync::Arc;

/// `POST /api/<method>` with a method this host doesn't know (404, not 400).
#[derive(Debug)]
struct UnknownMethod;

impl std::fmt::Display for UnknownMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("unknown method")
    }
}

impl std::error::Error for UnknownMethod {}

/// HTTP status and message for a failed call; channels send the same pair in `err`.
pub fn error_status(e: &anyhow::Error) -> (StatusCode, String) {
    if e.is::<UnknownMethod>() {
        (StatusCode::NOT_FOUND, e.to_string())
    } else if e.is::<Forbidden>() {
        (StatusCode::FORBIDDEN, e.to_string())
    } else if e.is::<crate::remote::cloud::Conflict>() {
        (StatusCode::CONFLICT, e.to_string())
    } else {
        // `{:#}` keeps the context chain ("starting `npx …`: No such file or directory").
        (StatusCode::BAD_REQUEST, format!("{e:#}"))
    }
}

fn str_arg<'a>(b: &'a Value, k: &str) -> Result<&'a str> {
    b[k].as_str().ok_or_else(|| anyhow!("`{k}` is required"))
}

/// Runs one API method for `caller` (permissions per spec §6.6).
pub async fn dispatch(hub: &Arc<Hub>, caller: &Caller, method: &str, b: Value) -> Result<Value> {
    devices::permit(caller, method)?;
    let observed = crate::analytics::observe_call(hub, caller, method, &b);
    let result = route(hub, caller, method, b).await;
    let report = observed.and_then(|(event, properties)| match &result {
        Ok(answer) => crate::analytics::answered(event, properties, answer).map(|p| (event, p)),
        Err(error) => crate::analytics::failed(event, properties, error),
    });
    if let Some((event, properties)) = report {
        crate::analytics::capture(hub, event, properties);
    }
    result
}

async fn route(hub: &Arc<Hub>, caller: &Caller, method: &str, b: Value) -> Result<Value> {
    if method == "setIcon" {
        return crate::icons::set(hub, &b).await;
    }
    if method == "icons" {
        return Ok(serde_json::to_value(crate::icons::snapshot(&hub.store)?)?);
    }
    if method.starts_with("reach")
        && let ControlFlow::Break(value) = reach::call(hub, method, b.clone()).await?
    {
        return Ok(value);
    }
    if method.contains("onnector")
        || method.starts_with("composio")
        || method == "setComposioKey"
        || method.starts_with("credential")
        // Only signing in reads the vault; other computer tools work without a keyring.
        || (method == "computerCall" && b["name"] == "type_login")
        || method.starts_with("voice")
        || method == "setVoiceKey"
        || matches!(method, "agentAuth" | "agentAuthenticate" | "setAgentEnv")
    {
        market::vault::unlock(hub.clone()).await?;
    }
    if method == "storageOverview" || method == "compactStorage" {
        return storage_usage::overview(hub, method == "compactStorage", &b).await;
    }
    if method == "groupTasks" {
        return crate::chat::tasks::list(hub, str_arg(&b, "groupId")?);
    }
    if method == "createGroupTask" {
        return crate::chat::tasks::create(hub, &b);
    }
    if method == "updateGroupTask" {
        return crate::chat::tasks::update(hub, &b).await;
    }
    if method == "queueControl" {
        return queue_controls::control(hub, &b).await;
    }
    if method == "groupInbox" {
        return crate::chat::group_mail::inbox(hub, str_arg(&b, "groupId")?);
    }
    if method == "setGroupInboxPolicy" {
        return crate::chat::group_mail::set_policy(hub, &b);
    }
    if method == "decideGroupMail" {
        return crate::chat::group_mail::decide(hub, &b);
    }
    if method == "quoteArtifactSelection" {
        let bot = str_arg(&b, "botId")?;
        let artifact = crate::chat::artifacts::get(&hub.store, bot, str_arg(&b, "artifactId")?).ok_or_else(|| anyhow::anyhow!("unknown artifact"))?;
        let entry = artifact.entry_id.ok_or_else(|| anyhow::anyhow!("this artifact has no source message to quote"))?;
        let selected = serde_json::json!({"text":str_arg(&b, "text")?,"part":format!("artifact:{}",artifact.id)});
        let source = hub.store.entry(&entry).ok_or_else(|| anyhow::anyhow!("artifact source message is unavailable"))?;
        let quote = crate::chat::replies::selection(&hub.store, &source.bot_id, &entry, &selected)?;
        return Ok(serde_json::json!({"chatId":source.bot_id,"quote":quote}));
    }
    if method == "quoteFile" {
        let entry = hub.store.entry(str_arg(&b, "entryId")?).ok_or_else(|| anyhow::anyhow!("unknown message"))?;
        let selected =
            serde_json::json!({"text": str_arg(&b, "text")?, "part": format!("file:{}", str_arg(&b, "fileId")?)});
        let quote = crate::chat::replies::selection(&hub.store, &entry.bot_id, &entry.id, &selected)?;
        return Ok(serde_json::json!({"chatId": entry.bot_id, "quote": quote}));
    }
    if method == "formatTable" {
        return crate::chat::table_preview::format(str_arg(&b, "name")?, str_arg(&b, "text")?);
    }
    if method == "readFile" {
        return files::read(hub, &b).await;
    }
    if method == "listWorkspaceFiles" {
        return workspace_browser::list(hub, &b).await;
    }
    if method == "previewWorkspaceFile" {
        return workspace_preview::read(hub, &b).await;
    }
    // Each method group answers its own methods and hands the body back otherwise.
    let b = match host::call(hub, caller, method, b).await? {
        ControlFlow::Break(v) => return Ok(v),
        ControlFlow::Continue(b) => b,
    };
    let b = match marketplace::call(hub, caller, method, b).await? {
        ControlFlow::Break(v) => return Ok(v),
        ControlFlow::Continue(b) => b,
    };
    let b = match network::call(hub, method, b).await? {
        ControlFlow::Break(v) => return Ok(v),
        ControlFlow::Continue(b) => b,
    };
    let b = match bots::call(hub, caller, method, b).await? {
        ControlFlow::Break(v) => return Ok(v),
        ControlFlow::Continue(b) => b,
    };
    match remote::call(hub, caller, method, b).await? {
        ControlFlow::Break(v) => Ok(v),
        ControlFlow::Continue(_) => Err(UnknownMethod.into()),
    }
}

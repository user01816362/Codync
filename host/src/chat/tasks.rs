//! Durable group assignments. Each attempt is a normal, attributable user message to one owner.
use super::group_delivery::{agent, group, string};
use crate::{
    agent::bot::Cmd,
    hub::Hub,
    store::{Entry, EntryKind, Lane, now_ms},
};
use anyhow::{Context as _, Result, ensure};
use serde_json::{Value, json};
use std::sync::Arc;

fn task(hub: &Hub, id: &str) -> Result<Entry> {
    hub.store
        .entry(id)
        .filter(|entry| entry.kind == "notice" && entry.data["groupTask"].is_object())
        .context("unknown task")
}

fn save(hub: &Hub, entry: &Entry, mut data: Value, status: &str, detail: &str) -> Result<Entry> {
    data["groupTask"]["status"] = status.into();
    let mut history = data["groupTask"]["history"].as_array().cloned().unwrap_or_default();
    history.push(json!({"at":now_ms(),"status":status,"owner":data["groupTask"]["owner"],"detail":detail.chars().take(4000).collect::<String>()}));
    if history.len() > 100 {
        history.remove(0);
    }
    data["groupTask"]["history"] = history.into();
    data["text"] = format!("Task: {} — {status}", data["groupTask"]["title"].as_str().unwrap_or_default()).into();
    let saved = hub.set_entry_if_revision(entry, &data)?.context("task changed elsewhere; refresh and try again")?;
    hub.emit_bot(&entry.bot_id);
    Ok(saved)
}

pub fn list(hub: &Hub, group_id: &str) -> Result<Value> {
    group(hub, group_id)?;
    Ok(json!({"items":hub.store.group_tasks(group_id)?,"limit":100}))
}

pub fn create(hub: &Arc<Hub>, args: &Value) -> Result<Value> {
    let target = group(hub, string(args, "groupId")?)?;
    ensure!(
        hub.store
            .group_tasks(&target.id)?
            .iter()
            .filter(|entry| !matches!(
                entry.data["groupTask"]["status"].as_str(),
                Some("completed" | "failed" | "interrupted")
            ))
            .count()
            < 100,
        "this group already has 100 unfinished tasks"
    );
    let owner = string(args, "owner")?;
    ensure!(target.members.iter().any(|id| id == owner), "choose a current member");
    agent(hub, owner)?;
    let title = string(args, "title")?.trim();
    let text = string(args, "text")?.trim();
    ensure!(!title.is_empty() && title.chars().count() <= 160, "task title must contain 1–160 characters");
    ensure!(!text.is_empty() && text.len() <= 32768, "task instructions must contain 1–32768 bytes");
    let request = string(args, "requestId")?;
    ensure!(
        !request.is_empty() && request.len() <= 80 && request.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-'),
        "invalid requestId"
    );
    let id = format!("group-task:{}:{request}", target.id);
    let data = json!({"text":format!("Task: {title} — draft"),"groupTask":{"title":title,"text":text,"owner":owner,"status":"draft","attempt":0,"history":[{"at":now_ms(),"status":"draft","owner":owner,"detail":"Created by user"}]}});
    let (saved, _) = hub.add_entry_once(&id, &Lane::main(&target.id), EntryKind::Notice, 0, &data)?;
    ensure!(
        saved.data["groupTask"]["title"] == title
            && saved.data["groupTask"]["text"] == text
            && saved.data["groupTask"]["owner"] == owner,
        "requestId belongs to another task"
    );
    hub.emit_bot(&target.id);
    Ok(json!({"entry":saved}))
}

pub async fn update(hub: &Arc<Hub>, args: &Value) -> Result<Value> {
    let entry = task(hub, string(args, "taskId")?)?;
    let target = group(hub, &entry.bot_id)?;
    ensure!(args["revision"].as_i64() == Some(entry.rev), "task changed elsewhere; refresh and try again");
    let mut data = entry.data.clone();
    let status = string(&data["groupTask"], "status")?.to_owned();
    let action = string(args, "action")?;
    if action == "reassign" {
        ensure!(
            matches!(status.as_str(), "draft" | "paused" | "failed" | "interrupted"),
            "pause the task before changing its owner"
        );
        let owner = string(args, "owner")?;
        ensure!(target.members.iter().any(|id| id == owner), "choose a current member");
        agent(hub, owner)?;
        data["groupTask"]["owner"] = owner.into();
        return Ok(json!({"entry":save(hub, &entry, data, &status, "Owner changed by user")?}));
    }
    if action == "pause" {
        ensure!(matches!(status.as_str(), "queued" | "running"), "only a queued or running task can be paused");
        let saved = save(hub, &entry, data, "pausing", "Pause requested; waiting for the actor to stop")?;
        let owner = string(&saved.data["groupTask"], "owner")?;
        let run = string(&saved.data["groupTask"], "runEntry")?;
        let (done, result) = tokio::sync::oneshot::channel();
        hub.send_cmd(owner, Cmd::PauseTask { task_id: saved.id.clone(), entry_id: run.to_owned(), done })?;
        tokio::time::timeout(std::time::Duration::from_secs(10), result)
            .await
            .context("pause confirmation timed out; refresh task status")?
            .context("pause interrupted")??;
        return Ok(json!({"entry":task(hub, &entry.id)?}));
    }
    ensure!(action == "start" || action == "resume", "unknown task action");
    ensure!(
        matches!(status.as_str(), "draft" | "paused" | "failed" | "interrupted"),
        "task already started or completed"
    );
    let owner = string(&data["groupTask"], "owner")?.to_owned();
    ensure!(target.members.contains(&owner), "task owner is no longer a group member");
    let cfg = agent(hub, &owner)?;
    let attempt = data["groupTask"]["attempt"].as_u64().unwrap_or_default() + 1;
    let run_id = format!("{}:attempt:{attempt}", entry.id);
    data["groupTask"]["attempt"] = attempt.into();
    data["groupTask"]["runEntry"] = run_id.clone().into();
    let saved = save(hub, &entry, data, "queued", "Attempt queued by user")?;
    let text = format!(
        "Group task for {}: {}\n\n{}\n\n{}",
        target.name,
        saved.data["groupTask"]["title"].as_str().unwrap_or_default(),
        saved.data["groupTask"]["text"].as_str().unwrap_or_default(),
        if attempt > 1 {
            "This resumes an earlier attempt. Inspect earlier work before repeating side effects."
        } else {
            "Report the result in your final reply."
        }
    );
    let message = json!({"text":text,"status":"queued","taskId":entry.id});
    hub.add_entry_once(&run_id, &Lane::main(&owner), EntryKind::User, hub.store.max_turn(&owner) + 1, &message)?;
    let prompt = super::prompt::of_entry(hub, &cfg, &message);
    if let Err(error) =
        hub.send_cmd(&owner, Cmd::Send { lane: Lane::main(&owner), entry_id: run_id.clone(), text: prompt })
    {
        finish(hub, &entry.id, &run_id, "failed", &format!("Could not dispatch: {error}"))?;
        return Err(error);
    }
    Ok(json!({"entry":task(hub, &entry.id)?}))
}

pub fn finish(hub: &Hub, id: &str, run: &str, status: &str, detail: &str) -> Result<()> {
    let entry = task(hub, id)?;
    if entry.data["groupTask"]["runEntry"] != run {
        return Ok(());
    }
    let current = entry.data["groupTask"]["status"].as_str().unwrap_or_default();
    if !matches!(current, "queued" | "running" | "pausing") {
        return Ok(());
    }
    let next = if current == "pausing" { "paused" } else { status };
    save(hub, &entry, entry.data.clone(), next, detail)?;
    Ok(())
}

pub fn restore(hub: &Hub) -> Result<()> {
    for row in hub.store.bots()?.into_iter().filter(|row| !row.deleted && row.config.is_group()) {
        for entry in hub.store.group_tasks(&row.config.id)? {
            let state = entry.data["groupTask"]["status"].as_str().unwrap_or_default();
            if matches!(state, "queued" | "running" | "pausing") {
                save(
                    hub,
                    &entry,
                    entry.data.clone(),
                    "interrupted",
                    "Host restarted; inspect earlier work before resuming",
                )?;
            }
        }
    }
    Ok(())
}

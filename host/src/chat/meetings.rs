//! Temporary rooms reuse group transcripts; decisions are explicit, revisioned host state.
use crate::hub::Hub;
use crate::store::{BotConfig, Entry, EntryKind, Lane};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::sync::Arc;

fn string<'a>(args: &'a Value, key: &str) -> Result<&'a str> {
    args[key].as_str().with_context(|| format!("{key} is required"))
}

fn bounded<'a>(args: &'a Value, key: &str, max: usize) -> Result<&'a str> {
    let text = string(args, key)?.trim();
    ensure!(!text.is_empty() && text.chars().count() <= max, "{key} must contain 1–{max} characters");
    Ok(text)
}

pub fn entry(hub: &Hub, id: &str) -> Option<Entry> {
    entry_for_store(&hub.store, id)
}

pub fn entry_for_store(store: &crate::store::Store, id: &str) -> Option<Entry> {
    store.entry(&format!("meeting:{id}")).filter(|e| e.data["meeting"].is_object())
}

fn room(hub: &Hub, id: &str) -> Result<(BotConfig, Entry)> {
    let group = hub.store.bot(id)?.filter(|r| !r.deleted && r.config.is_group()).context("unknown room")?.config;
    let state = entry(hub, id).context("this group is not a meeting room")?;
    Ok((group, state))
}

fn validate_delegates(hub: &Hub, delegates: &Value) -> Result<Vec<String>> {
    let delegates = delegates.as_array().context("delegates is required")?;
    ensure!((2..=5).contains(&delegates.len()), "choose two to five delegates");
    let mut ids = Vec::new();
    for delegate in delegates {
        let id = string(delegate, "botId")?;
        ensure!(!ids.iter().any(|i| i == id), "each agent can represent only one department per room");
        let bot = hub.store.bot(id)?.filter(|r| !r.deleted && !r.config.is_group()).context("delegate unavailable")?;
        if let Some(origin) = delegate["groupId"].as_str() {
            let department = hub
                .store
                .bot(origin)?
                .filter(|r| !r.deleted && r.config.is_group())
                .context("department unavailable")?;
            ensure!(entry(hub, origin).is_none(), "a meeting cannot represent a department");
            ensure!(department.config.members.contains(&bot.config.id), "delegate is not a current department member");
        }
        ids.push(id.to_owned());
    }
    Ok(ids)
}

/// A readable, bounded invitation preview, validated before the approval card opens.
pub fn approval_detail(hub: &Hub, args: &Value) -> Result<String> {
    let name = bounded(args, "name", 40)?;
    let agenda = bounded(args, "agenda", 4000)?;
    let members = validate_delegates(hub, &args["delegates"])?;
    let chair = string(args, "chair")?;
    ensure!(members.iter().any(|id| id == chair), "chair must be a delegate");
    let kind = string(args, "kind")?;
    ensure!(matches!(kind, "consultation" | "meeting" | "decision"), "invalid room kind");
    let delegates = args["delegates"]
        .as_array()
        .context("delegates is required")?
        .iter()
        .map(|d| {
            let agent = hub.bot_name(d["botId"].as_str().unwrap_or_default());
            let origin = d["groupId"].as_str().map(|id| hub.bot_name(id)).unwrap_or_else(|| "Personal".into());
            format!("{agent} · {origin}")
        })
        .collect::<Vec<_>>()
        .join("\n");
    Ok(format!(
        "{kind}: {name}\nAgenda: {agenda}\n\nDelegates:\n{delegates}\nChair: {}\n\nThis approves these invitations. Existing shared agent-session history and permissions still apply. Discussion is limited to ten turns and ten minutes. The room starts paused; the user controls Start and final approval.",
        hub.bot_name(chair)
    ))
}

/// User API, or a managing agent after a mandatory approval card containing this exact agenda/delegate list.
pub fn create(hub: &Arc<Hub>, args: &Value, creator: Option<&str>) -> Result<Value> {
    if let Some(creator) = creator {
        ensure!(
            hub.store.bot(creator)?.is_some_and(|r| !r.deleted && r.config.manage_groups),
            "group-management permission was revoked"
        );
    }
    let name = bounded(args, "name", 40)?;
    let agenda = bounded(args, "agenda", 4000)?;
    let kind = string(args, "kind")?;
    ensure!(matches!(kind, "consultation" | "meeting" | "decision"), "invalid room kind");
    let ids = validate_delegates(hub, &args["delegates"])?;
    let chair = string(args, "chair")?;
    ensure!(ids.iter().any(|id| id == chair), "chair must be a delegate");
    let mut delegates = args["delegates"].clone();
    for delegate in delegates.as_array_mut().context("delegates is required")? {
        delegate["name"] = hub.bot_name(string(delegate, "botId")?).into();
        delegate["department"] =
            delegate["groupId"].as_str().map(|id| hub.bot_name(id)).unwrap_or_else(|| "Personal".into()).into();
    }
    let cfg: BotConfig = serde_json::from_value(json!({"id":"", "kind":"group", "name":name,
        "description":agenda,"members":ids,"createdBy":creator,"section":"Meetings"}))?;
    let group = hub.create_meeting_bot(cfg)?;
    let id = string(&group, "id")?;
    let data = json!({"text":format!("{kind}: {name} — paused. Open Meeting settings to start."),
        "meeting":{"kind":kind,"agenda":agenda,"chair":chair,"delegates":delegates,
        "status":"paused","proposal":"","proposalRevision":0,"positions":{},"spent":0,
        "limit":10,"remainingMs":600_000,"startedAt":null,"deadline":null,"approved":false}});
    if let Err(error) = hub.add_entry_once(&format!("meeting:{id}"), &Lane::main(id), EntryKind::Notice, 0, &data) {
        let _ = hub.delete_bot(id);
        return Err(error);
    }
    hub.update_bot(&json!({"id":id}))?;
    state(hub, id)
}

pub fn state(hub: &Hub, id: &str) -> Result<Value> {
    let (group, entry) = room(hub, id)?;
    if entry.data["meeting"]["status"] == "running"
        && entry.data["meeting"]["deadline"].as_i64().unwrap_or(0) <= crate::store::now_ms()
    {
        let mut meeting = entry.data["meeting"].clone();
        meeting["status"] = "unresolved".into();
        meeting["remainingMs"] = 0.into();
        meeting["startedAt"] = Value::Null;
        let saved = save(hub, &entry, meeting)?;
        super::group::stop(hub, id);
        return Ok(saved);
    }
    Ok(json!({"groupId":group.id,"name":group.name,"rev":entry.rev,"meeting":entry.data["meeting"]}))
}

fn save(hub: &Hub, previous: &Entry, meeting: Value) -> Result<Value> {
    let mut data = previous.data.clone();
    data["text"] = format!(
        "{} room — {} · proposal {} · {}/{} turns",
        meeting["kind"].as_str().unwrap_or("Meeting"),
        meeting["status"].as_str().unwrap_or("paused"),
        meeting["proposalRevision"],
        meeting["spent"],
        meeting["limit"]
    )
    .into();
    data["meeting"] = meeting;
    hub.set_entry_if_revision(previous, &data)?.context("room changed on another device; refresh and try again")?;
    hub.emit_bot(&previous.bot_id);
    state(hub, &previous.bot_id)
}

fn current(hub: &Hub, group: &BotConfig, meeting: &Value, actor: &str) -> Result<()> {
    ensure!(group.members.iter().any(|id| id == actor), "you are no longer a room member");
    let invited = validate_delegates(hub, &meeting["delegates"])?;
    ensure!(invited.iter().all(|id| group.members.contains(id)), "a required delegate left the room");
    ensure!(
        meeting["delegates"].as_array().is_some_and(|ds| ds.iter().any(|d| d["botId"] == actor)),
        "not an invited delegate"
    );
    Ok(())
}

/// Agents can propose and record only their own position. Final approval/lifecycle remain user actions.
pub fn action(hub: &Arc<Hub>, args: &Value, actor: Option<&str>) -> Result<Value> {
    let id = string(args, "groupId")?;
    let _ = state(hub, id)?;
    let (group, previous) = room(hub, id)?;
    ensure!(args["rev"].as_i64() == Some(previous.rev), "room changed; read its current revision first");
    let mut meeting = previous.data["meeting"].clone();
    let command = string(args, "action")?;
    if let Some(actor) = actor {
        current(hub, &group, &meeting, actor)?;
        ensure!(matches!(command, "propose" | "position"), "only the user may control or approve the room");
    }
    let status = meeting["status"].as_str().unwrap_or("paused").to_owned();
    ensure!(status != "archived" || command == "reopen", "reopen the room before changing it");
    if command == "report" {
        ensure!(actor.is_none(), "only the user publishes department outcomes");
        ensure!(
            matches!(status.as_str(), "resolved" | "completed" | "unresolved" | "cancelled"),
            "finish or pause an unresolved discussion before reporting"
        );
        validate_delegates(hub, &meeting["delegates"])?;
        let positions = meeting["delegates"]
            .as_array()
            .context("invalid delegates")?
            .iter()
            .map(|d| {
                let position = &meeting["positions"][d["botId"].as_str().unwrap_or_default()];
                format!(
                    "{} · {}: {} — {}",
                    d["name"].as_str().unwrap_or("Delegate"),
                    d["department"].as_str().unwrap_or("Personal"),
                    position["position"].as_str().unwrap_or("not answered"),
                    position["reason"].as_str().unwrap_or_default()
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        let summary = format!(
            "Meeting outcome: {} — {status}\nAgenda: {}\nProposal revision {}: {}\nRecorded positions:\n{positions}\nFinal user approval: {}",
            group.name,
            meeting["agenda"].as_str().unwrap_or_default(),
            meeting["proposalRevision"],
            meeting["proposal"].as_str().unwrap_or_default(),
            meeting["approved"].as_bool().unwrap_or(false)
        );
        let mut destinations = Vec::new();
        for delegate in meeting["delegates"].as_array().context("invalid delegates")? {
            if let Some(destination) = delegate["groupId"].as_str() {
                if destinations.iter().any(|id| id == destination) {
                    continue;
                }
                let target = hub
                    .store
                    .bot(destination)?
                    .filter(|r| !r.deleted && r.config.is_group())
                    .context("department unavailable")?;
                let key = format!("meeting-report:{id}:{}:{destination}", meeting["proposalRevision"]);
                hub.add_entry_once(
                    &key,
                    &Lane::main(&target.config.id),
                    EntryKind::Agent,
                    hub.store.max_turn(destination),
                    &json!({"text":summary,"final":true,"author":meeting["chair"],"meetingOutcome":id}),
                )?;
                hub.emit_bot(destination);
                destinations.push(destination.to_owned());
            }
        }
        return Ok(json!({"deliveredTo":destinations,"followupScheduled":false}));
    }
    match command {
        "propose" => {
            ensure!(matches!(status.as_str(), "running" | "paused" | "awaiting_owner"), "room is closed");
            ensure!(
                actor.is_none() || meeting["chair"] == actor.unwrap_or_default(),
                "only the chair proposes a decision"
            );
            meeting["proposal"] = bounded(args, "text", 8000)?.into();
            meeting["proposalRevision"] = (meeting["proposalRevision"].as_u64().unwrap_or(0) + 1).into();
            meeting["positions"] = json!({});
            meeting["approved"] = false.into();
            if status == "awaiting_owner" {
                meeting["status"] = "paused".into();
            }
        }
        "position" => {
            ensure!(matches!(status.as_str(), "running" | "paused" | "awaiting_owner"), "room is closed");
            let actor = actor.context("positions must be recorded by the authenticated delegate")?;
            ensure!(meeting["proposalRevision"].as_u64().unwrap_or(0) > 0, "no proposal to review");
            ensure!(args["proposalRevision"] == meeting["proposalRevision"], "proposal changed; review it again");
            let position = string(args, "position")?;
            ensure!(matches!(position, "approve" | "object" | "abstain"), "invalid position");
            meeting["positions"][actor] = json!({"position":position,"reason":bounded(args,"text",2000)?,"proposalRevision":meeting["proposalRevision"]});
            let all = meeting["delegates"]
                .as_array()
                .context("invalid delegates")?
                .iter()
                .all(|d| d["botId"].as_str().is_some_and(|id| meeting["positions"][id]["position"] == "approve"));
            if all {
                meeting["status"] = "awaiting_owner".into();
            } else if status == "awaiting_owner" {
                meeting["status"] = "paused".into();
            }
        }
        "finish" => {
            ensure!(
                meeting["kind"] != "decision",
                "decision rooms require explicit delegate responses and final approval"
            );
            ensure!(
                matches!(status.as_str(), "running" | "paused" | "awaiting_owner" | "unresolved"),
                "room is closed"
            );
            meeting["proposal"] = bounded(args, "text", 8000)?.into();
            meeting["proposalRevision"] = (meeting["proposalRevision"].as_u64().unwrap_or(0) + 1).into();
            meeting["positions"] = json!({});
            meeting["approved"] = false.into();
            meeting["status"] = "completed".into();
        }
        "approve" => {
            validate_delegates(hub, &meeting["delegates"])?;
            ensure!(status == "awaiting_owner", "all delegates must explicitly approve the current proposal first");
            ensure!(
                meeting["delegates"].as_array().is_some_and(|ds| ds
                    .iter()
                    .all(|d| d["botId"].as_str().is_some_and(|id| group.members.iter().any(|member| member == id)))),
                "a required delegate left the room"
            );
            meeting["approved"] = true.into();
            meeting["status"] = "resolved".into();
        }
        "reopen" => {
            meeting["status"] = "paused".into();
            meeting["approved"] = false.into();
            meeting["positions"] = json!({});
            meeting["proposalRevision"] = (meeting["proposalRevision"].as_u64().unwrap_or(0) + 1).into();
        }
        "pause" | "archive" | "cancel" => {
            meeting["status"] = match command {
                "archive" => "archived",
                "cancel" => "cancelled",
                _ => "paused",
            }
            .into();
        }
        "resume" => {
            ensure!(matches!(status.as_str(), "paused" | "unresolved"), "room cannot be resumed in its current state");
            validate_delegates(hub, &meeting["delegates"])?;
            ensure!(
                meeting["spent"].as_u64().unwrap_or(10) < 10,
                "discussion budget exhausted; start a new scoped meeting"
            );
            let remaining = meeting["remainingMs"].as_i64().unwrap_or(0);
            ensure!(remaining > 0, "discussion time budget exhausted");
            meeting["status"] = "running".into();
            meeting["deadline"] = (crate::store::now_ms() + remaining).into();
            meeting["startedAt"] = crate::store::now_ms().into();
        }
        _ => anyhow::bail!("unknown meeting action"),
    }
    if meeting["status"] != "running"
        && let Some(started) = meeting["startedAt"].as_i64()
    {
        meeting["remainingMs"] =
            (meeting["remainingMs"].as_i64().unwrap_or(0) - (crate::store::now_ms() - started)).max(0).into();
        meeting["startedAt"] = Value::Null;
    }
    let saved = save(hub, &previous, meeting)?;
    if saved["meeting"]["status"] != "running" {
        super::group::stop(hub, id);
    }
    if command == "archive" || command == "reopen" {
        hub.update_bot(&json!({"id":id,"section":if command == "archive" {"Archived meetings"} else {"Meetings"}}))?;
    }
    if command == "resume" {
        let text = format!(
            "Meeting agenda: {}. Review the current proposal using meeting_status. The chair records a proposal; each delegate records their own position. Silence is not approval.",
            saved["meeting"]["agenda"].as_str().unwrap_or_default()
        );
        hub.add_entry(&Lane::main(id), EntryKind::User, hub.store.max_turn(id), &json!({"text":text}));
        super::group::start(hub, &group, Lane::main(id));
        let weak = Arc::downgrade(hub);
        let room_id = id.to_owned();
        let deadline = saved["meeting"]["deadline"].as_i64().unwrap_or(0);
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(
                (deadline - crate::store::now_ms()).max(0).unsigned_abs(),
            ))
            .await;
            if let Some(hub) = weak.upgrade()
                && let Some(current) = entry(&hub, &room_id)
                && current.data["meeting"]["status"] == "running"
                && current.data["meeting"]["deadline"] == deadline
            {
                let _ = state(&hub, &room_id);
            }
        });
    }
    Ok(saved)
}

/// Persist a room-wide turn reservation; retries and late rounds cannot reset the budget.
pub fn reserve(hub: &Hub, id: &str, actor: &str) -> Result<bool> {
    let Some(previous) = entry(hub, id) else { return Ok(true) };
    let mut meeting = previous.data["meeting"].clone();
    if meeting["status"] != "running" {
        return Ok(false);
    }
    let group = hub.store.bot(id)?.context("room unavailable")?.config;
    let valid = current(hub, &group, &meeting, actor).is_ok();
    let spent = meeting["spent"].as_u64().unwrap_or(10);
    if !valid || spent >= 10 || meeting["deadline"].as_i64().unwrap_or(0) <= crate::store::now_ms() {
        meeting["status"] = "unresolved".into();
        save(hub, &previous, meeting)?;
        return Ok(false);
    }
    meeting["spent"] = (spent + 1).into();
    save(hub, &previous, meeting)?;
    Ok(true)
}

pub fn prompt(hub: &Hub, id: &str, actor: &str) -> Option<String> {
    let entry = entry(hub, id)?;
    let meeting = &entry.data["meeting"];
    let delegate = meeting["delegates"].as_array()?.iter().find(|d| d["botId"] == actor)?;
    Some(format!(
        "Meeting room: {}. You represent {} as {}. Chair: {}. Use meeting_status(groupId) and meeting_action with the current revision to propose (chair only) or record your own approve/object/abstain position. Do not infer agreement from silence. Shared agent-session history still applies: do not disclose unrelated private conversation material. Room id: {id}",
        meeting["kind"], delegate["department"], delegate["name"], meeting["chair"]
    ))
}

pub fn tools() -> Vec<Value> {
    vec![
        json!({"name":"meeting_status","description":"Read a meeting you belong to, including proposal, recorded positions and revision.","inputSchema":{"type":"object","properties":{"groupId":{"type":"string"}},"required":["groupId"]},"annotations":{"readOnlyHint":true}}),
        json!({"name":"meeting_action","description":"Chair proposes text; each delegate records their own position on the exact proposal revision. User alone controls lifecycle and final approval.","inputSchema":{"type":"object","properties":{"groupId":{"type":"string"},"rev":{"type":"integer"},"action":{"enum":["propose","position"]},"text":{"type":"string"},"position":{"enum":["approve","object","abstain"]},"proposalRevision":{"type":"integer"}},"required":["groupId","rev","action","text"]}}),
    ]
}

pub fn call(hub: &Arc<Hub>, bot: &str, name: &str, args: &Value) -> Result<Value> {
    let id = string(args, "groupId")?;
    let (group, entry) = room(hub, id)?;
    current(hub, &group, &entry.data["meeting"], bot)?;
    if name == "meeting_status" { state(hub, id) } else { action(hub, args, Some(bot)) }
}

/// Revalidate a queued turn immediately before any model work starts.
pub fn active(hub: &Hub, id: &str, actor: &str) -> Result<()> {
    let Some(entry) = entry(hub, id) else { return Ok(()) };
    let group = hub.store.bot(id)?.context("meeting unavailable")?.config;
    current(hub, &group, &entry.data["meeting"], actor)?;
    ensure!(
        entry.data["meeting"]["status"] == "running"
            && entry.data["meeting"]["deadline"].as_i64().unwrap_or(0) > crate::store::now_ms(),
        "meeting is paused, closed or expired"
    );
    Ok(())
}

pub fn stopped(hub: &Hub, id: &str) {
    let Some(previous) = entry(hub, id) else { return };
    let mut meeting = previous.data["meeting"].clone();
    if meeting["status"] != "running" {
        return;
    }
    meeting["remainingMs"] = (meeting["deadline"].as_i64().unwrap_or(0) - crate::store::now_ms()).max(0).into();
    meeting["startedAt"] = Value::Null;
    meeting["status"] = "paused".into();
    let _ = save(hub, &previous, meeting);
}

pub fn revalidate(hub: &Hub) {
    let Ok(rows) = hub.store.bots() else { return };
    for row in rows.into_iter().filter(|r| !r.deleted && r.config.is_group()) {
        if let Some(entry) = entry(hub, &row.config.id)
            && entry.data["meeting"]["status"] == "running"
            && validate_delegates(hub, &entry.data["meeting"]["delegates"]).is_err()
        {
            super::group::stop(hub, &row.config.id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::identity::Identity;
    use crate::store::Store;

    fn fixture() -> Arc<Hub> {
        let store = Store::open(std::path::Path::new(":memory:")).expect("store");
        for value in [
            json!({"id":"a","name":"Alpha","backend":"claude","cwd":"/tmp"}),
            json!({"id":"b","name":"Beta","backend":"claude","cwd":"/tmp"}),
            json!({"id":"outside","name":"Outsider","backend":"claude","cwd":"/tmp"}),
            json!({"id":"dept","name":"Development","kind":"group","members":["a","b"]}),
            json!({"id":"room","name":"Launch","kind":"group","members":["a","b"]}),
        ] {
            let cfg: BotConfig = serde_json::from_value(value).expect("config");
            store.save_bot(&cfg).expect("save");
        }
        let identity = Identity {
            sign: ed25519_dalek::SigningKey::from_bytes(&[1; 32]),
            box_secret: x25519_dalek::StaticSecret::from([2; 32]),
        };
        let hub = Hub::new(store, "host".into(), identity, "token".into(), 0);
        hub.add_entry_once("meeting:room", &Lane::main("room"), EntryKind::Notice, 0, &json!({"text":"Decision room","meeting":{"kind":"decision","status":"paused","agenda":"Launch?","chair":"a","delegates":[{"botId":"a","groupId":"dept","name":"Alpha","department":"Development"},{"botId":"b","groupId":"dept","name":"Beta","department":"Development"}],"proposal":"","proposalRevision":0,"positions":{},"spent":0,"limit":10,"remainingMs":600_000,"startedAt":null}})).expect("room");
        hub
    }

    fn act(hub: &Arc<Hub>, who: Option<&str>, command: &str, extra: Value) -> Result<Value> {
        let mut args = json!({"groupId":"room","rev":state(hub,"room")?["rev"],"action":command});
        for (key, value) in extra.as_object().into_iter().flatten() {
            args[key] = value.clone();
        }
        action(hub, &args, who)
    }

    #[test]
    fn approval_requires_explicit_current_positions_and_the_user() {
        let hub = fixture();
        act(&hub, None, "propose", json!({"text":"Launch after review"})).expect("proposal");
        assert!(call(&hub, "outside", "meeting_status", &json!({"groupId":"room"})).is_err());
        act(&hub, Some("a"), "position", json!({"proposalRevision":1,"position":"approve","text":"Evidence checked"}))
            .expect("first");
        assert!(act(&hub, None, "approve", json!({})).is_err());
        act(&hub, Some("b"), "position", json!({"proposalRevision":1,"position":"abstain","text":"Missing evidence"}))
            .expect("abstain");
        assert!(act(&hub, None, "approve", json!({})).is_err());
        act(&hub, Some("b"), "position", json!({"proposalRevision":1,"position":"approve","text":"Evidence supplied"}))
            .expect("second");
        assert!(act(&hub, Some("a"), "approve", json!({})).is_err());
        act(&hub, None, "approve", json!({})).expect("owner approval");
        assert_eq!(state(&hub, "room").expect("state")["meeting"]["status"], "resolved");
        act(&hub, None, "report", json!({})).expect("report");
        act(&hub, None, "report", json!({})).expect("retry");
        assert_eq!(hub.store.messages_after(&Lane::main("dept"), 0, 24).expect("history").len(), 1);
    }

    #[test]
    fn new_proposals_invalidate_votes_and_stale_revisions() {
        let hub = fixture();
        let old = act(&hub, None, "propose", json!({"text":"First"})).expect("proposal");
        act(&hub, Some("a"), "position", json!({"proposalRevision":1,"position":"approve","text":"Reviewed"}))
            .expect("vote");
        act(&hub, None, "propose", json!({"text":"Changed"})).expect("revision");
        assert!(state(&hub, "room").expect("state")["meeting"]["positions"].as_object().expect("positions").is_empty());
        assert!(
            act(&hub, Some("b"), "position", json!({"proposalRevision":1,"position":"approve","text":"Old"})).is_err()
        );
        assert!(action(&hub, &json!({"groupId":"room","rev":old["rev"],"action":"archive"}), None).is_err());
    }

    #[test]
    fn budgets_survive_turns_and_meetings_are_not_permanent_group_duplicates() {
        let hub = fixture();
        let mut saved = entry(&hub, "room").expect("entry");
        saved.data["meeting"]["status"] = "running".into();
        saved.data["meeting"]["deadline"] = (crate::store::now_ms() + 600_000).into();
        saved.data["meeting"]["spent"] = 9.into();
        hub.set_entry_if_revision(&saved, &saved.data).expect("save");
        assert!(reserve(&hub, "room", "a").expect("last turn"));
        assert!(!reserve(&hub, "room", "b").expect("budget"));
        assert_eq!(state(&hub, "room").expect("state")["meeting"]["status"], "unresolved");
        assert_eq!(
            super::super::group::with_members(&hub.store, &["a".into(), "b".into()]).expect("lookup"),
            Some("dept".into())
        );
    }
}

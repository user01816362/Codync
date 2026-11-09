//! Department settings use the terminal's existing labeled-field sheets and host validation.
use super::{Fields, Input, Reply, Submit};
use crate::tui::app::{App, Editor, Overlay};
use serde_json::{Value, json};

fn input(name: &str, label: &str, value: &str, multiline: bool) -> Input {
    Input {
        name: name.into(),
        label: label.into(),
        secret: false,
        required: false,
        multiline,
        placeholder: String::new(),
        ed: Editor::with(value),
    }
}

impl App {
    pub(in crate::tui) fn open_department(&mut self, group: &str, kind: &str) {
        let method = match kind {
            "tasks" => "groupTasks",
            "inbox" => "groupInbox",
            _ => "storageOverview",
        };
        let request_id = uuid::Uuid::new_v4().to_string();
        self.overlays.push(Overlay::Fields(Box::new(Fields {
            title: "Department settings".into(),
            note: "Loading…".into(),
            choices: vec![],
            choice: 0,
            inputs: vec![],
            cursor: 0,
            error: None,
            saving: true,
            submit: Submit::Department {
                group: group.into(),
                kind: kind.into(),
                data: Value::Null,
                request_id: request_id.clone(),
            },
        })));
        self.sheet(
            method,
            json!({"groupId":group}),
            Reply::Department { group: group.into(), kind: kind.into(), request_id },
        );
    }

    pub(in crate::tui) fn department_loaded(&mut self, group: String, kind: String, request_id: String, data: Value) {
        if !matches!(self.overlays.last(), Some(Overlay::Fields(fields)) if matches!(&fields.submit, Submit::Department {request_id: current,..} if current == &request_id))
        {
            return;
        }
        self.overlays.pop();
        let rows = data["items"].as_array().cloned().unwrap_or_default();
        let default_owner = self
            .bots
            .get(&group)
            .and_then(|bot| bot.members.first())
            .and_then(|id| self.bots.get(id))
            .map_or(String::new(), |bot| bot.name.clone());
        let mut choices = vec![if kind == "tasks" {
            "New task".into()
        } else if kind == "inbox" {
            "Inbox settings".into()
        } else {
            "Inspect only".into()
        }];
        for item in &rows {
            let value = if kind == "tasks" { &item["data"]["groupTask"] } else { &item["mail"] };
            let title = value[if kind == "tasks" { "title" } else { "subject" }].as_str().unwrap_or_default();
            choices.push(format!("{title} · {}", value["status"].as_str().unwrap_or_default()));
        }
        let (title, note, inputs) = match kind.as_str() {
            "tasks" => (
                "Group tasks",
                "Create saves a draft. Select a task and enter start, pause, resume, reassign, or history (copies full history). Attempts run in the owner's chat. Pause waits for cancellation; resume does not undo side effects.",
                vec![
                    input("action", "Action", "start", false),
                    input("owner", "Owner name", &default_owner, false),
                    input("title", "New task title", "", false),
                    input("text", "New task instructions", "", true),
                ],
            ),
            "inbox" => {
                let handler = data["policy"]["handler"]
                    .as_str()
                    .and_then(|id| self.bots.get(id))
                    .map_or(default_owner, |bot| bot.name.clone());
                (
                    "Group inbox",
                    "Settings: enter manual or automatic and a handler. Select a request to approve/reject; current handler membership is checked by the host. Automatic approval applies only to new mail and does not grant history access.",
                    vec![
                        input(
                            "action",
                            "Action",
                            if data["policy"]["autoApprove"] == true { "automatic" } else { "manual" },
                            false,
                        ),
                        input("owner", "Handler name", &handler, false),
                    ],
                )
            }
            _ => {
                choices.push("Compact database (preserve messages/files)".into());
                (
                    "Host storage",
                    "Inspection does not change data. Selecting Compact database and saving confirms SQLite VACUUM. It can need twice the database size in temporary free space and briefly blocks database operations. Shared/workspace files are not deleted.",
                    vec![],
                )
            }
        };
        let mut fields = Fields {
            title: title.into(),
            note: note.into(),
            choices,
            choice: 0,
            inputs,
            cursor: 0,
            error: None,
            saving: false,
            submit: Submit::Department { group, kind, data, request_id },
        };
        department_choice(&mut fields);
        self.overlays.push(Overlay::Fields(Box::new(fields)));
    }

    pub(super) fn submit_department(&mut self, fields: &mut Fields, values: &serde_json::Map<String, Value>) {
        let Submit::Department { group, kind, data, request_id } = &fields.submit else { return };
        let action = values.get("action").and_then(Value::as_str).unwrap_or_default().to_ascii_lowercase();
        let owner = values.get("owner").and_then(Value::as_str).unwrap_or_default();
        let resolve_owner = || {
            let members = self.bots.get(group).map(|bot| &bot.members);
            let matches: Vec<String> = self
                .bots
                .values()
                .filter(|bot| {
                    members.is_some_and(|ids| ids.contains(&bot.id))
                        && (bot.name.eq_ignore_ascii_case(owner) || bot.id == owner)
                })
                .map(|bot| bot.id.clone())
                .collect();
            (matches.len() == 1).then(|| matches[0].clone())
        };
        let result = (|| -> Result<(&'static str, Value), String> {
            if kind == "storage" {
                return Ok((
                    if fields.choice == 1 { "compactStorage" } else { "storageOverview" },
                    json!({"confirm":fields.choice == 1}),
                ));
            }
            if kind == "inbox" && fields.choice == 0 {
                if action != "manual" && action != "automatic" {
                    return Err("Choose manual or automatic.".into());
                }
                return Ok((
                    "setGroupInboxPolicy",
                    json!({"groupId":group,"autoApprove":action == "automatic","handler":resolve_owner().ok_or("Choose a unique current member name or ID")?}),
                ));
            }
            if kind == "tasks" && fields.choice == 0 {
                return Ok((
                    "createGroupTask",
                    json!({"groupId":group,"title":values.get("title"),"text":values.get("text"),"owner":resolve_owner().ok_or("Choose a unique current member name or ID")?,"requestId":request_id}),
                ));
            }
            let row = data["items"]
                .as_array()
                .and_then(|items| items.get(fields.choice.saturating_sub(1)))
                .ok_or("Refresh this list before changing it")?;
            if kind == "tasks" {
                if !["start", "pause", "resume", "reassign"].contains(&action.as_str()) {
                    return Err("Choose start, pause, resume, reassign, or history.".into());
                }
                return Ok((
                    "updateGroupTask",
                    json!({"taskId":row["id"],"revision":row["rev"],"action":action,"owner":if action == "reassign" {resolve_owner()} else {None}}),
                ));
            }
            if action != "approve" && action != "reject" {
                return Err("Choose approve or reject for an inbox request.".into());
            }
            Ok(("decideGroupMail", json!({"mailId":row["id"],"approve":action == "approve","handler":resolve_owner()})))
        })();
        if kind == "tasks"
            && action == "history"
            && fields.choice > 0
            && let Some(events) = data["items"][fields.choice - 1]["data"]["groupTask"]["history"].as_array()
        {
            let text = events
                .iter()
                .map(|event| {
                    format!(
                        "{} · {} · {}\n{}",
                        event["at"],
                        event["status"].as_str().unwrap_or_default(),
                        event["owner"]
                            .as_str()
                            .and_then(|id| self.bots.get(id))
                            .map_or("Unavailable owner", |bot| bot.name.as_str()),
                        event["detail"].as_str().unwrap_or_default()
                    )
                })
                .collect::<Vec<_>>()
                .join("\n\n");
            crate::tui::app::copy(&text);
            fields.saving = false;
            self.flash("Task history copied");
            return;
        }
        match result {
            Ok((method, body)) => self.sheet(method, body, Reply::DepartmentSaved { request_id: request_id.clone() }),
            Err(error) => {
                fields.saving = false;
                fields.error = Some(error);
            }
        }
    }
}

pub(super) fn department_choice(fields: &mut Fields) {
    let Submit::Department { kind, data, .. } = &fields.submit else { return };
    if kind == "storage" {
        fields.note = format!(
            "Database: {} bytes · reusable: {} bytes · {} entries. Shared: {} bytes; uploads: {} bytes; artifacts: {} bytes. Partial flags: {}, {}, {}. Compact preserves live data, needs temporary free space up to twice the database size, and blocks database operations while running.",
            data["databaseBytes"],
            data["reusableBytes"],
            data["entries"],
            data["sharedFiles"]["bytes"],
            data["uploads"]["bytes"],
            data["artifacts"]["bytes"],
            data["sharedFiles"]["truncated"],
            data["uploads"]["truncated"],
            data["artifacts"]["truncated"]
        );
    } else if fields.choice > 0 {
        let row = &data["items"][fields.choice - 1];
        let value = if kind == "tasks" { &row["data"]["groupTask"] } else { &row["mail"] };
        fields.note = format!(
            "{}\nStatus: {}. {}",
            value["text"].as_str().unwrap_or_default().chars().take(1200).collect::<String>(),
            value["status"].as_str().unwrap_or_default(),
            if kind == "tasks" {
                "Actions: start, pause, resume, reassign, history (copies full recorded history)."
            } else {
                "Actions: approve or reject. Choose the handler by unique current member name."
            }
        );
    }
}

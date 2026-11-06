use super::{
    GroupTurn, MEMBER_TIMEOUT, admit, alert, answers, ask, busy_elsewhere, cited, group, members,
    since_last_user_message,
};
use crate::LockExt;
use crate::agent::bot::{Cmd, NoticeStyle};
use crate::hub::Hub;
use crate::store::{BotConfig, EntryKind, Lane};
use anyhow::Result;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::oneshot;

/// Late replies share the room's admission budget and seen cursors, so they can hand off
/// without resetting the room, duplicating an already-claimed mention, or evading Stop.
pub(super) struct LateMentions {
    pub(super) epoch: u64,
    pub(super) rounds: usize,
    pub(super) asked: Arc<Mutex<HashMap<String, i64>>>,
    pub(super) budget: Arc<Mutex<usize>>,
}

pub(super) async fn answer_later(
    hub: Arc<Hub>,
    lane: Lane,
    member: BotConfig,
    answer: oneshot::Receiver<Result<Option<String>>>,
    state: LateMentions,
) {
    if let Ok(Ok(Some(text))) = answer.await {
        if let Err(error) = alert(&hub, &lane.chat, &member.id, &member.name, &text) {
            tracing::warn!(group = lane.chat, error = format!("{error:#}"), "late room reply not pushed");
        }
        if let Err(error) = continue_mentions(&hub, &lane, state).await {
            tracing::warn!(group = lane.chat, error = format!("{error:#}"), "late mention continuation failed");
        }
    }
}

async fn continue_mentions(hub: &Arc<Hub>, lane: &Lane, state: LateMentions) -> Result<()> {
    for _ in 0..state.rounds {
        if !hub.groups.current(lane, state.epoch) {
            return Ok(());
        }
        let group = group(hub, &lane.chat)?;
        let members = members(hub, &group);
        let mut answered = false;
        for member in &members {
            if !hub.groups.current(lane, state.epoch) {
                return Ok(());
            }
            let messages = since_last_user_message(hub, lane)?;
            let (prompt, shown) = ask(hub, &group, &members, member, lane)?;
            {
                let mut seen = state.asked.locked();
                if !cited(&members, &messages, &seen).iter().any(|m| m.id == member.id) {
                    continue;
                }
                if !admit(&state.budget) {
                    room_notice(hub, lane, "The group reply limit was reached. Send another message to continue.");
                    return Ok(());
                }
                seen.insert(member.id.clone(), shown);
            }
            let (reply, answer) = oneshot::channel();
            let turn = GroupTurn {
                lane: lane.clone(),
                prompt,
                shown,
                epoch: state.epoch,
                late: busy_elsewhere(hub, &member.id, lane),
                reply,
                answers: answers(&messages),
            };
            if let Err(error) = hub.send_cmd(&member.id, Cmd::Group(turn)) {
                room_notice(hub, lane, &format!("{} could not take the mention: {error}", member.name));
                continue;
            }
            match tokio::time::timeout(MEMBER_TIMEOUT, answer).await {
                Ok(Ok(Ok(Some(text)))) => {
                    answered = true;
                    alert(hub, &lane.chat, &member.id, &member.name, &text)?;
                }
                Err(_) => {
                    room_notice(hub, lane, &format!("{} did not answer the mention within ten minutes.", member.name));
                }
                _ => {}
            }
        }
        if !answered {
            break;
        }
    }
    Ok(())
}

pub(super) fn room_notice(hub: &Hub, lane: &Lane, text: &str) {
    hub.add_entry(
        lane,
        EntryKind::Notice,
        hub.store.max_turn(&lane.chat),
        &serde_json::json!({"text": text, "style": NoticeStyle::Info}),
    );
}

//! Conditional transcript mutations used by inbox approval decisions.
use super::entries::{ENTRY_COLS, row_entry};
use super::{Entry, Store, next_rev, now_ms};
use crate::LockExt;
use anyhow::Result;
use rusqlite::params;
use serde_json::Value;

impl Store {
    pub fn update_entry_if_revision(&self, id: &str, expected: i64, data: &Value) -> Result<Option<Entry>> {
        let c = self.db.locked();
        let rev = next_rev(&c)?;
        let changed = c.execute(
            "UPDATE entries SET rev = ?3, data = ?4, updated_at = ?5 WHERE id = ?1 AND rev = ?2",
            params![id, expected, rev, data.to_string(), now_ms()],
        )?;
        if changed == 0 {
            return Ok(None);
        }
        Ok(Some(c.query_row(&format!("SELECT {ENTRY_COLS} FROM entries WHERE id = ?1"), [id], row_entry)?))
    }

    pub fn group_inbox(&self, group: &str) -> Result<Vec<Entry>> {
        let c = self.db.locked();
        let mut query = c.prepare(&format!(
            "SELECT {ENTRY_COLS} FROM entries WHERE bot_id = ?1 AND kind = 'notice'
             AND json_extract(data, '$.groupMail.direction') = 'incoming'
             ORDER BY CASE json_extract(data, '$.groupMail.status') WHEN 'pending' THEN 0 ELSE 1 END, seq DESC LIMIT 100"
        ))?;
        Ok(query.query_map([group], row_entry)?.collect::<rusqlite::Result<_>>()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{EntryKind, Lane};
    use serde_json::json;

    #[test]
    fn only_one_client_can_decide_the_same_pending_request() {
        let store = Store::open(std::path::Path::new(":memory:")).expect("store");
        let pending = store
            .insert_entry(
                &Lane::main("group"),
                EntryKind::Notice,
                0,
                &json!({"groupMail":{"direction":"incoming","status":"pending"}}),
            )
            .expect("pending mail");
        let approved = json!({"groupMail":{"direction":"incoming","status":"approved","handler":"a"}});
        let rejected = json!({"groupMail":{"direction":"incoming","status":"rejected"}});
        assert!(store.update_entry_if_revision(&pending.id, pending.rev, &approved).expect("approve").is_some());
        assert!(
            store.update_entry_if_revision(&pending.id, pending.rev, &rejected).expect("stale rejection").is_none()
        );
        assert_eq!(store.entry(&pending.id).expect("saved mail").data, approved);
    }

    #[test]
    fn pending_mail_stays_out_of_group_prompts_and_other_inboxes() {
        let store = Store::open(std::path::Path::new(":memory:")).expect("store");
        let lane = Lane::main("group");
        let pending = store
            .insert_entry(
                &lane,
                EntryKind::Notice,
                0,
                &json!({"text":"private external request","groupMail":{"direction":"incoming","status":"pending"}}),
            )
            .expect("pending mail");
        store.insert_entry(&lane, EntryKind::Notice, 0, &json!({"text":"ordinary notice"})).expect("notice");
        assert!(store.messages_after(&lane, 0, 24).expect("group prompt messages").is_empty());
        let inbox = store.group_inbox("group").expect("inbox");
        assert_eq!(inbox.len(), 1);
        assert_eq!(inbox[0].id, pending.id);
        assert!(store.group_inbox("other").expect("other inbox").is_empty());
    }
}

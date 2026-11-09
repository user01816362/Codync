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
        if changed == 0 { return Ok(None); }
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

//! SQLite-owned storage accounting and explicit compaction; no user files are deleted.
use super::Store;
use crate::LockExt;
use anyhow::Result;
use serde_json::{Value, json};

impl Store {
    pub fn storage_usage(&self) -> Result<Value> {
        let c = self.db.locked();
        let pages: u64 = c.query_row("PRAGMA page_count", [], |row| row.get(0))?;
        let size: u64 = c.query_row("PRAGMA page_size", [], |row| row.get(0))?;
        let free: u64 = c.query_row("PRAGMA freelist_count", [], |row| row.get(0))?;
        let entries: u64 = c.query_row("SELECT COUNT(*) FROM entries", [], |row| row.get(0))?;
        Ok(json!({"databaseBytes":pages.saturating_mul(size),"reusableBytes":free.saturating_mul(size),"entries":entries}))
    }

    pub fn compact_storage(&self) -> Result<()> {
        self.db.locked().execute_batch("VACUUM")?;
        Ok(())
    }
}

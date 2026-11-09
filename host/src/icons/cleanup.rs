//! Remove local icon overrides after their conversation is deleted.
//! Any icon whose target isn't a live bot goes, including agent icons saved before agents lost them.
use super::{Icon, commit, list, live_bot};
use crate::{hub::Hub, store::Store};
use anyhow::Result;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

pub(crate) async fn run(hub: Arc<Hub>) {
    let mut timer = tokio::time::interval(Duration::from_mins(1));
    timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        timer.tick().await;
        if let Err(error) = sweep(&hub) {
            tracing::warn!(error = format!("{error:#}"), "custom icon cleanup will retry");
        }
    }
}

pub(super) fn sweep(hub: &Hub) -> Result<()> {
    let Ok(_guard) = hub.store.icon_lock.try_lock() else { return Ok(()) };
    let mut icons = list(&hub.store)?;
    for target in stale(&hub.store, &icons)?.into_iter().take(8) {
        icons.remove(&target);
        icons = commit(hub, icons)?.icons;
    }
    Ok(())
}

/// Targets that aren't a live bot: deleted bots, and agents (which no longer take custom icons).
pub(super) fn stale(store: &Store, icons: &BTreeMap<String, Icon>) -> Result<Vec<String>> {
    let mut stale = Vec::new();
    for target in icons.keys() {
        if !live_bot(store, target)? {
            stale.push(target.clone());
        }
    }
    Ok(stale)
}

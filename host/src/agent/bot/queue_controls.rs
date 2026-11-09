//! User-controlled queue pause and ordering, saved before actor dispatch changes.
use super::{Actor, Queued};
use crate::store::Lane;
use anyhow::{Context as _, Result, bail, ensure};
use serde_json::{Value, json};

impl Actor {
    pub(super) fn queue_paused(&self) -> bool {
        self.hub.store.kv_get(&format!("queue.paused.{}", self.cfg.id)).as_deref() == Some("true")
    }

    pub(super) fn save_managed_queue(&self) -> Result<()> {
        let paused = self.queue_paused();
        for (index, item) in self.queue.iter().enumerate() {
            if let Queued::User { entry_id, .. } = item {
                let mut entry = self.hub.store.entry(entry_id).context("queued message is unavailable")?;
                if entry.data["taskId"].is_string() {
                    continue;
                }
                entry.data["durableQueue"] = true.into();
                entry.data["queuePaused"] = paused.into();
                entry.data["queueOrder"] = index.into();
                self.hub.set_entry(entry_id, &entry.data).context("couldn't save queue order")?;
            }
        }
        Ok(())
    }

    pub(super) fn restore_managed_queue(&mut self) {
        match self.hub.store.managed_queue(&self.cfg.id) {
            Ok(entries) => {
                for entry in entries {
                    let text = crate::chat::prompt::of_entry(&self.hub, &self.cfg, &entry.data);
                    self.queue.push_back(Queued::User { lane: Lane::main(&self.cfg.id), entry_id: entry.id, text });
                }
            }
            Err(error) => tracing::error!(%error, "couldn't restore managed queue"),
        }
    }

    pub(super) fn control_queue(&mut self, action: &str, id: Option<&str>) -> Result<Value> {
        match action {
            "state" => {}
            "pause" | "resume" => {
                self.hub.store.kv_set(
                    &format!("queue.paused.{}", self.cfg.id),
                    if action == "pause" { "true" } else { "false" },
                )?;
                self.save_managed_queue()?;
            }
            "earlier" | "later" => {
                let id = id.context("entryId is required")?;
                ensure!(!self.steers_in_flight.contains_key(id), "message is already being dispatched");
                let positions: Vec<usize> = self
                    .queue
                    .iter()
                    .enumerate()
                    .filter_map(|(at, item)| matches!(item, Queued::User { .. }).then_some(at))
                    .collect();
                let at = positions
                    .iter()
                    .position(|at| matches!(&self.queue[*at], Queued::User { entry_id, .. } if entry_id == id))
                    .context("message is no longer waiting")?;
                let next = if action == "earlier" {
                    at.checked_sub(1)
                } else {
                    at.checked_add(1).filter(|next| *next < positions.len())
                }
                .context("message is already at the end")?;
                self.queue.swap(positions[at], positions[next]);
                if let Err(error) = self.save_managed_queue() {
                    self.queue.swap(positions[at], positions[next]);
                    return Err(error);
                }
            }
            _ => bail!("unknown queue action"),
        }
        let runtime = self.hub.runtime(&self.cfg.id);
        Ok(
            json!({"paused":self.queue_paused(),"reason":if self.queue_paused() { "Paused by you" } else if self.turn.is_some() { "Waiting for the current turn" } else { "Ready" },"activity":runtime.activity}),
        )
    }
}

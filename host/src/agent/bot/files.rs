//! File copies run outside the actor; publication still belongs to its original turn. Artifact cards post the same way.
use super::Actor;
use crate::chat::files::{self, Snapshot};
use crate::store::EntryKind;
use anyhow::{Result, anyhow, ensure};
use serde_json::{Value, json};
use std::path::PathBuf;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tokio::sync::{mpsc, oneshot};

type Reply = oneshot::Sender<Result<()>>;
pub(super) struct Completed {
    epoch: u64,
    text: Option<String>,
    cancelled: Arc<AtomicBool>,
    reply: Option<Reply>,
    image_entry: Option<String>,
    result: Result<Snapshot>,
}

pub(super) struct FileShares {
    epoch: u64,
    pub summary: Option<String>,
    pending: Vec<Arc<AtomicBool>>,
    images: Vec<Arc<AtomicBool>>,
    pub tx: mpsc::UnboundedSender<Completed>,
    pub rx: mpsc::UnboundedReceiver<Completed>,
}
impl Default for FileShares {
    fn default() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self { epoch: 0, summary: None, pending: Vec::new(), images: Vec::new(), tx, rx }
    }
}
impl FileShares {
    pub fn cancel(&mut self) {
        self.epoch = self.epoch.wrapping_add(1);
        for flag in self.pending.drain(..) {
            flag.store(true, Ordering::Release);
        }
        self.images.clear();
    }

    pub fn images_pending(&self) -> bool {
        !self.images.is_empty()
    }
}
impl Drop for FileShares {
    fn drop(&mut self) {
        self.cancel();
    }
}

impl Actor {
    pub(super) fn begin_file(
        &mut self,
        source: String,
        name: Option<String>,
        text: Option<String>,
        cancelled: Arc<AtomicBool>,
        reply: Reply,
    ) {
        if let Err(error) = self.can_send_file() {
            let _ = reply.send(Err(error));
            return;
        }
        self.files.pending.retain(|flag| !flag.load(Ordering::Acquire));
        if self.files.pending.len() >= 4 {
            let _ = reply.send(Err(anyhow!("too many pending file shares")));
            return;
        }
        self.files.pending.push(cancelled.clone());
        let epoch = self.files.epoch;
        let root = files::root(&self.cfg.id);
        let cwd = PathBuf::from(&self.cfg.cwd);
        let tx = self.files.tx.clone();
        tokio::spawn(async move {
            let flag = cancelled.clone();
            let result =
                tokio::task::spawn_blocking(move || files::snapshot(&root, &cwd, &source, name.as_deref(), &flag))
                    .await;
            let result = result.map_err(anyhow::Error::from).and_then(std::convert::identity);
            let _ = tx.send(Completed { epoch, text, cancelled, reply: Some(reply), image_entry: None, result });
        });
    }

    /// ACP sends image content directly, with no tool response that can wait for the disk copy.
    pub(super) fn begin_image(&mut self, content: &Value) {
        const MAX_ENCODED: usize = (20 * 1024 * 1024_usize).div_ceil(3) * 4;
        if self.can_send_file().is_err() {
            return;
        }
        let (Some(mime), Some(data)) = (content["mimeType"].as_str(), content["data"].as_str()) else { return };
        // Check before cloning the agent's payload onto a blocking worker.
        if data.len() > MAX_ENCODED || self.files.pending.len() >= 4 {
            tracing::warn!(bot = %self.cfg.id, "agent image exceeds the size or pending-share limit");
            return;
        }
        // Reserve the message's chronological position before later ACP chunks can arrive. The empty nonfinal
        // entry is hidden until the verified snapshot is ready, and stays hidden on cancellation or failure.
        self.close_seg();
        let Some(entry) = self.add(EntryKind::Agent, self.turn.unwrap_or(0), json!({"text": "", "final": false}))
        else {
            return;
        };
        let image_entry = entry.id;
        let cancelled = Arc::new(AtomicBool::new(false));
        self.files.pending.push(cancelled.clone());
        self.files.images.push(cancelled.clone());
        let epoch = self.files.epoch;
        let root = files::root(&self.cfg.id);
        let (mime, data) = (mime.to_owned(), data.to_owned());
        let tx = self.files.tx.clone();
        tokio::spawn(async move {
            let flag = cancelled.clone();
            let result = tokio::task::spawn_blocking(move || files::snapshot_image(&root, &mime, &data, &flag)).await;
            let result = result.map_err(anyhow::Error::from).and_then(std::convert::identity);
            let _ = tx.send(Completed { epoch, text: None, cancelled, reply: None, image_entry: Some(image_entry), result });
        });
    }

    fn can_send_file(&self) -> Result<i64> {
        ensure!(!self.stop_requested, "turn is stopping");
        ensure!(
            self.active_group.is_none() && self.active_request.is_none() && self.active_routine.is_none(),
            "send_file and artifact cards are only available in the bot's own chat"
        );
        self.turn.ok_or_else(|| anyhow!("no turn is running: between turns, tell the user with send_message"))
    }

    /// A published artifact's card (`chat::artifacts`, its entry's `data`): a message of the turn, like a shared file.
    pub(super) fn post_artifact(&mut self, data: Value) -> Result<String> {
        let turn = self.can_send_file()?;
        let text = data["text"].as_str().unwrap_or_default().to_owned();
        self.close_seg();
        let entry = self.add(EntryKind::Agent, turn, data).ok_or_else(|| anyhow!("couldn't save the artifact card"))?;
        self.files.summary = Some(text);
        self.set_activity("Working…");
        Ok(entry.id)
    }

    pub(super) fn finish_file(&mut self, completed: Completed) {
        self.files.pending.retain(|flag| !Arc::ptr_eq(flag, &completed.cancelled));
        self.files.images.retain(|flag| !Arc::ptr_eq(flag, &completed.cancelled));
        let result = (|| {
            ensure!(
                completed.epoch == self.files.epoch
                    && !completed.cancelled.load(Ordering::Acquire)
                    && completed.reply.as_ref().is_none_or(|reply| !reply.is_closed()),
                "file sharing cancelled"
            );
            let turn = self.can_send_file()?;
            ensure!(self.hub.store.bot(&self.cfg.id)?.is_some_and(|bot| !bot.deleted), "bot was deleted");
            let mut snapshot = completed.result?;
            let text = format!("{} ({} bytes)", snapshot.meta.name, snapshot.meta.size);
            if let Some(entry_id) = &completed.image_entry {
                let mut entry = self.hub.store.entry(entry_id).ok_or_else(|| anyhow!("image message was removed"))?;
                entry.data["text"] = text.clone().into();
                entry.data["final"] = true.into();
                entry.data["files"] = json!([snapshot.meta]);
                self.hub.set_entry(entry_id, &entry.data).ok_or_else(|| anyhow!("couldn't save the image message"))?;
            } else {
                ensure!(completed.text.is_none() || snapshot.meta.plays_in_chat,
                    "text goes only with a voice note: an mp3, m4a, wav or aac file up to 20 MiB");
                self.close_seg();
                let text = match completed.text {
                    Some(text) => self.post_voice_note(turn, text, &snapshot)?,
                    None => self.post_file(turn, &snapshot)?,
                };
                self.files.summary = Some(text);
            }
            snapshot.commit();
            self.files.summary = Some(text);
            self.set_activity("Working…");
            Ok(())
        })();
        if let Some(reply) = completed.reply {
            let _ = reply.send(result);
        } else if let Err(error) = result {
            tracing::warn!(bot = %self.cfg.id, error = %error, "couldn't publish agent image");
        }
    }
}

impl Actor {
    /// A file message: the card, with its name and size for a reader without one.
    fn post_file(&mut self, turn: i64, snapshot: &Snapshot) -> Result<String> {
        let text = format!("{} ({} bytes)", snapshot.meta.name, snapshot.meta.size);
        self.add(EntryKind::Agent, turn, json!({"text": text, "final": true, "files": [snapshot.meta]}))
            .ok_or_else(|| anyhow!("couldn't save the file message"))?;
        Ok(text)
    }

    /// A message carrying a voice note: one bubble, the player over the text, sent like `send_message`.
    fn post_voice_note(&mut self, turn: i64, text: String, snapshot: &Snapshot) -> Result<String> {
        let data = json!({"text": text, "final": true, "files": [snapshot.meta], "voiceNote": true});
        self.record_sent(turn, data, text.clone())?;
        Ok(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancelling_a_turn_invalidates_old_images_without_cancelling_new_ones() {
        let mut shares = FileShares::default();
        let old = Arc::new(AtomicBool::new(false));
        shares.pending.push(old.clone());
        shares.images.push(old.clone());
        let old_epoch = shares.epoch;
        assert!(shares.images_pending());
        shares.cancel();
        assert!(old.load(Ordering::Acquire));
        assert!(!shares.images_pending());
        assert_ne!(old_epoch, shares.epoch);

        let new = Arc::new(AtomicBool::new(false));
        shares.pending.push(new.clone());
        shares.images.push(new.clone());
        shares.pending.retain(|flag| !Arc::ptr_eq(flag, &old));
        shares.images.retain(|flag| !Arc::ptr_eq(flag, &old));
        assert!(shares.images_pending());
        assert!(!new.load(Ordering::Acquire));
    }

}

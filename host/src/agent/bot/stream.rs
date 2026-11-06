//! Explicit, user-facing streaming. ACP narration remains in the trace.
use super::Actor;
use crate::store::EntryKind;
use anyhow::{Result, anyhow, bail, ensure};
use serde_json::json;

const MAX_BYTES: usize = 32 * 1024;
const MAX_DELTAS: usize = 512;

pub(super) struct ActiveStream {
    id: String,
    text: String,
    deltas: usize,
}

impl Actor {
    /// A restarted host cannot continue an agent's old stream. Keep it in trace, not live chat.
    pub(super) fn recover_streams(&self) {
        match self.hub.store.unfinished_streams(&self.cfg.id) {
            Ok(entries) => {
                for mut entry in entries {
                    entry.data["streaming"] = false.into();
                    self.hub.set_entry(&entry.id, &entry.data);
                }
            }
            Err(error) => tracing::warn!(bot = %self.cfg.id, %error, "couldn't recover unfinished message streams"),
        }
    }

    /// Agent opts in explicitly; at most one stream belongs to the current private turn.
    pub(super) fn stream_message(&mut self, action: &str, text: &str) -> Result<()> {
        ensure!(!self.stop_requested, "turn is stopping");
        ensure!(self.turn.is_some(), "no turn is running");
        ensure!(
            self.active_group.is_none() && self.active_request.is_none() && self.active_routine.is_none(),
            "stream_message is available only in the bot's own chat"
        );
        match action {
            "start" => {
                ensure!(self.stream.is_none(), "a stream is already open");
                ensure!(!text.is_empty() && text.len() <= MAX_BYTES, "invalid stream text");
                self.close_seg();
                let entry = self
                    .add(
                        EntryKind::Agent,
                        self.turn.unwrap_or(0),
                        json!({"text": text, "final": false, "streaming": true}),
                    )
                    .ok_or_else(|| anyhow!("couldn't save the stream"))?;
                self.stream = Some(ActiveStream { id: entry.id, text: text.to_owned(), deltas: 1 });
                self.set_activity("Writing a reply…");
            }
            "append" => {
                let stream = self.stream.as_mut().ok_or_else(|| anyhow!("no stream is open"))?;
                ensure!(!text.is_empty() && stream.text.len() + text.len() <= MAX_BYTES, "invalid stream text");
                ensure!(stream.deltas < MAX_DELTAS, "too many stream updates");
                let mut entry = self.hub.store.entry(&stream.id).ok_or_else(|| anyhow!("stream was removed"))?;
                stream.text.push_str(text);
                stream.deltas += 1;
                entry.data["text"] = stream.text.clone().into();
                self.hub.set_entry(&stream.id, &entry.data);
            }
            "finish" => {
                ensure!(text.is_empty(), "finish does not accept text");
                ensure!(self.stream.is_some(), "no stream is open");
                self.settle_stream(true);
            }
            "cancel" => {
                ensure!(text.is_empty(), "cancel does not accept text");
                ensure!(self.stream.is_some(), "no stream is open");
                self.settle_stream(false);
            }
            _ => bail!("unknown stream action"),
        }
        Ok(())
    }

    /// A completed stream becomes a normal final message; cancelled drafts stay in the trace.
    pub(super) fn settle_stream(&mut self, finish: bool) {
        if let Some(stream) = self.stream.take() {
            if let Some(mut entry) = self.hub.store.entry(&stream.id) {
                entry.data["streaming"] = false.into();
                if finish {
                    entry.data["final"] = true.into();
                    self.sent.push(stream.text);
                    self.last_text = None;
                }
                self.hub.set_entry(&stream.id, &entry.data);
            }
        }
    }
}

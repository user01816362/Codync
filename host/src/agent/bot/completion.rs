//! Turn outcomes, sign-in recovery and durable completion records.
use super::{Actor, Done, RESUME_PROMPT, RoutineCompletion, inflight_key};
use crate::analytics::{self, TurnOutcome, TurnSource};
use crate::chat::memory;
use crate::hub::BotStatus;
use crate::market::agent_sign_in;
use crate::store::{Lane, now_ms};
use anyhow::{Result, anyhow};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

impl Actor {
    pub(super) fn start_failed(&mut self, e: &anyhow::Error) {
        for id in &self.turn_entries {
            if let Some(entry) = self.hub.store.entry(id)
                && let Some(task) = entry.data["taskId"].as_str()
                && let Err(error) =
                    crate::chat::tasks::finish(&self.hub, task, id, "failed", &format!("Could not start: {e:#}"))
            {
                tracing::error!(%error, "couldn't record task start failure");
            }
        }
        if self.turn.is_some() {
            self.report_turn(false, Some(e), "start_failed");
        }
        self.finish_routine(crate::routines::Status::Failed, Some(format!("Could not start routine: {e:#}")), None);
        self.complete_request(Err(anyhow!("couldn't start recipient: {e:#}")), false);
        self.complete_group(Err(anyhow!("couldn't start: {e:#}")));
        if !self.invite_sign_in(e) {
            // `{:#}` keeps the reason ("starting `npx …`: EACCES"), not just "agent process exited".
            self.failure_notice(e, &format!("Couldn't start the agent: {e:#}"), "start_failed");
        }
        // What the chain carried in (a recap, a takeover) was this turn's; the user's handoff waits for the next.
        self.relay.carried = None;
        self.turn = None;
        self.reply_hops = None;
        self.reply_collaboration = None;
        self.active_prompt = false;
        memory::turns::drop_turn(&self.cfg.id);
        if self.routine_completion.is_none() {
            self.set_inflight(false);
        }
        self.hub.set_runtime(&self.id(), |r| {
            r.status = BotStatus::Error;
            r.activity = "Couldn't start".into();
            r.started_at = None;
            r.lane = None;
            r.message = None;
            r.session = None;
        });
        self.hub.team.end_turn(&self.cfg.id);
    }

    /// When `e` asks for signing in: marks the harness signed out and puts a sign-in card in the
    /// chat, which sends this turn's messages again once the user has signed in. Whether it did.
    /// A process older than the turn may hold a sign-in replaced since: a new one tries its messages first.
    pub(super) fn invite_sign_in(&mut self, e: &anyhow::Error) -> bool {
        let backend = self.cfg.backend.clone();
        if crate::agent::backends::harness(&backend).is_none() || !crate::agent::acp::needs_sign_in(e) {
            return false;
        }
        // The process keeps the credentials it started with (Claude's revoked token): the next turn starts a new one.
        self.restart_after_turn = true;
        let held = self.held_for_sign_in().to_vec();
        if self.process_predates_turn() && !held.is_empty() {
            match agent_sign_in::send_again(&self.hub, &self.cfg.id, held.iter().map(String::as_str)) {
                Ok(()) => return true,
                Err(error) => tracing::warn!(bot = %self.cfg.id, error = format!("{error:#}"), "couldn't retry"),
            }
        }
        crate::agent::backends::set_signed_in(&backend, Some(false));
        // An account check can still say signed in (`claude auth status` with a revoked token): this refusal wins.
        crate::agent::backends::note_prompt_refused(&backend);
        self.sign_in_card(&backend, &held)
    }

    /// The agent's process was already running when this turn started.
    fn process_predates_turn(&self) -> bool {
        let (Some(conn), Some(started_at)) = (&self.conn, self.hub.runtime(&self.id()).started_at) else {
            return false;
        };
        let turn_age = Duration::from_millis(u64::try_from(now_ms() - started_at).unwrap_or(0));
        conn.started.elapsed() > turn_age
    }

    /// Agent `backend`'s sign-in card in the chat, holding `entries`. Whether it's there.
    pub(super) fn sign_in_card(&self, backend: &str, entries: &[String]) -> bool {
        let Some(h) = crate::agent::backends::harness(backend) else { return false };
        let invited = agent_sign_in::invite(&self.hub, &self.cfg.id, &self.lane, backend, h.name, entries);
        invited
            .inspect_err(
                |error| tracing::warn!(bot = %self.cfg.id, error = format!("{error:#}"), "couldn't ask to sign in"),
            )
            .is_ok()
    }

    pub(super) fn complete_request(&mut self, result: Result<String>, cancelled: bool) {
        if let Some(request) = self.active_request.take() {
            request.complete(result, cancelled);
        }
    }

    pub(super) fn complete_group(&mut self, result: Result<Option<String>>) {
        if let Some(turn) = self.active_group.take() {
            let _ = turn.reply.send(result);
        }
    }

    pub(super) fn finish_routine(
        &mut self,
        status: crate::routines::Status,
        detail: Option<String>,
        text: Option<String>,
    ) {
        if let Some(id) = self.active_routine.take() {
            // The deadline also covers a session setup that ran out the clock before any turn began.
            let timed_out = self.routine_timed_out || self.routine_deadline.is_some_and(|d| Instant::now() >= d);
            self.routine_completion = Some(RoutineCompletion { id, status, detail, text, timed_out });
            self.routine_deadline = None;
            self.retry_routine_completion();
        }
    }

    /// If SQLite is temporarily unavailable, keep the result and block this
    /// actor's next turn until the durable completion has been saved.
    pub(super) fn retry_routine_completion(&mut self) -> bool {
        let Some(completion) = &self.routine_completion else {
            return true;
        };
        match self.hub.routines.complete(
            &self.hub,
            &completion.id,
            completion.status,
            completion.detail.clone(),
            completion.text.clone(),
            completion.timed_out,
        ) {
            Ok(()) => {
                // A finished run's session is never resumed.
                let slot = crate::routines::session_slot(&completion.id);
                self.routine_completion = None;
                self.set_session(Some(&slot), None);
                true
            }
            Err(error) => {
                tracing::error!(
                    error = format!("{error:#}"),
                    run = completion.id,
                    "couldn't save routine result; will retry"
                );
                false
            }
        }
    }

    /// Grok Bot's upgrade resume: the previous host process stopped mid-turn, so
    /// the agent is told, in the same session, to finish without redoing steps.
    pub(super) async fn resume_interrupted(&mut self, lane: Lane, done_tx: &mpsc::UnboundedSender<Done>) {
        self.set_inflight(false);
        let slot = self.slot();
        if self.session_for(slot.as_deref()).is_none() {
            self.finish_routine(
                crate::routines::Status::Interrupted,
                Some("No saved agent session is available".into()),
                None,
            );
            return;
        }
        let prompt = if self.active_routine.is_some() {
            format!(
                "{RESUME_PROMPT} This is still a scheduled routine run. If its instruction permits silence and there is nothing to report, return exactly (pass)."
            )
        } else {
            RESUME_PROMPT.to_owned()
        };
        if let Err(e) = self.start_turn(lane, &[], &prompt, true, done_tx).await {
            self.start_failed(&e);
        }
    }

    /// Marks a turn in the bot's own chat as running across host restarts (see [`Self::resume_interrupted`]).
    pub(super) fn set_inflight(&self, on: bool) {
        let value = if on { now_ms().to_string() } else { String::new() };
        if let Err(error) = self.hub.store.kv_set(&inflight_key(&self.cfg.id), &value) {
            tracing::warn!(bot = %self.cfg.id, error = format!("{error:#}"), "couldn't record the running turn");
        }
    }
    /// `turn_finished`, while the turn's start time is still in the runtime.
    pub(super) fn report_turn(&self, stopped: bool, failure: Option<&anyhow::Error>, stop_reason: &str) {
        let outcome = if stopped {
            TurnOutcome::Stopped
        } else if failure.is_some() {
            TurnOutcome::Failed
        } else if stop_reason == "end_turn" {
            TurnOutcome::Done
        } else {
            TurnOutcome::Incomplete
        };
        let started = self.hub.runtime(&self.id()).started_at.unwrap_or_else(now_ms);
        let ms = now_ms() - started;
        analytics::turn_finished(&self.hub, &self.cfg.backend, outcome, stop_reason, failure, self.source(), ms);
    }

    /// What started the turn that is starting or running.
    pub(super) fn source(&self) -> TurnSource {
        if self.active_routine.is_some() {
            TurnSource::Routine
        } else if self.active_group.is_some() {
            TurnSource::Group
        } else if self.active_request.is_some() || self.reply_hops.is_some() {
            TurnSource::Bot
        } else if self.active_prompt {
            TurnSource::Host
        } else {
            TurnSource::User
        }
    }
}

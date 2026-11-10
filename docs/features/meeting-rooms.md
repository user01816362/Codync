# Temporary meeting rooms

Status: initial implementation, 2026-11-10. Build validation is tracked in GitHub Actions; real-app interaction remains unverified. The sections below distinguish the researched target from the first version. Defaults are product choices, not measured optimal values.

## Recommendation

Add a temporary room that reuses the group conversation UI and host scheduler while recording an explicit meeting identity, department representation, agenda and outcome. Use selected representatives rather than broadcasting every question to every member of every department. A direct department question should continue to use the existing inbox; a shared decision involving several departments can justify a room.

Offer three presets over one room model:

| Preset | Purpose | Completion |
| --- | --- | --- |
| Consultation | Gather specialist advice | Required advisers respond or the room records missing input |
| Meeting | Discuss a topic and assign follow-ups | Owner publishes a summary, decisions and tasks |
| Decision | Compare alternatives and record positions | Required delegates respond to the same proposal revision, followed by the configured final approval |

A model may recommend opening a room. It should not silently enroll agents, disclose a department transcript or authorize work. The host enforces the configured policy.

## Verified existing foundation

- [Group scheduling](../../host/src/chat/group.rs) gives groups their own transcript and runs members in their own sessions. Current limits are three rounds and ten replies per room turn, with a ten-minute member timeout. Subsequent rounds follow mentions; passing or reaching a limit ends a turn. None of those events proves agreement.
- Groups contain distinct agent bots; a group cannot itself be a member. Meetings should therefore enroll named delegates, not nest groups or pretend a department is an agent.
- [Department delivery](../../host/src/chat/group_delivery.rs) allows members to read and post to their groups. [Department mail](../../host/src/chat/group_mail.rs) lets nonmembers submit requests under the destination inbox policy without joining or receiving its history.
- [Agent context](../../host/src/chat/context.rs) explicitly describes history as shared across a bot's conversations. A room transcript alone would not isolate the participant's model context. Context scoping needs an explicit implementation before claiming private meeting boundaries.
- The collaboration display currently consumes chat/exchange events. Preserve those paths rather than create a parallel messaging engine. Room creation, invitations, speaking, proposal revisions and outcomes need corresponding durable events.

## User experience

Use the existing group layout, composer, profile cards, member rows, quote UI, files, RTL handling and collaboration display modes. Add a small Consultation, Meeting or Decision badge; keep the normal department name visible beside each delegate's display name, for example `Backend Engineer · Development`. A bot belonging to several groups represents only the department specified in this invitation.

The room header shows its title, current state and a compact progress summary such as `Reviewing proposal · 2 of 3 required responses`. Do not call that consensus while a required participant is unavailable or has not responded. Avoid a new floating control for every participant.

A single settings sheet contains agenda, participants, representation, chair, limits, approval policy and reporting destinations. The outcome is an expandable card with the chosen proposal, evidence links, objections, abstentions, missing responses, responsibilities and links to recorded tasks. Keep technical errors and interrupted work visible without labeling them as rejection.

Archive finished rooms under a Meetings section, retaining their transcript and outcome; temporary means limited lifecycle rather than automatic deletion. Allow search and explicit reopening. Reopening or changing the proposal starts a new decision revision.

## Workflow and stopping

1. The requesting agent proposes a title, question, participants, context packet and intended result.
2. The host validates the requesting agent's authority and each department's invitation policy. Invitations to other departments use a dedicated approval flow with their handler; existing mail auto-approval must not automatically become meeting-membership approval.
3. Accepted delegates receive the agenda and explicitly shared material. No automatic copying of full department histories, files or permissions.
4. Each delegate submits an independent initial position before seeing the other initial positions. A chair then requests focused review of conflicting evidence or options. This is a proposed measure against uncritical agreement, not a guarantee of accuracy.
5. The chair publishes a versioned proposal. Delegates record `approve`, `object` or `abstain`, with reasons and evidence. Only the authenticated delegate can record its response; text saying another bot agrees is not a vote.
6. The host applies the decision rule to the exact proposal revision and required delegate list. Silence, pass, timeout and offline status are never approval. One bot cannot count as several department votes by holding multiple memberships without an explicit representation rule.
7. Publish the result to authorized destination inboxes, including disagreement or missing input. Deduplicate publication by room, proposal revision and destination. Report delivery independently from receipt, reading, acceptance or execution.

Recommended initial limits: three to five delegates, up to three discussion rounds and ten substantive replies, plus explicit proposal/decision control events. An overall time limit should pause with a partial summary; the current member timeout is not an overall meeting budget. Numeric limits are proposed starting points and should be tuned using real usage.

Persist room-wide limits across scheduler turns, retries, late answers and reconnects. An agent must not reset the budget by posting another message or spawning a child room. Human Continue can explicitly add budget. Enforce any token/cost limit only where the provider supplies reliable usage; otherwise show the limitation and retain message/time caps.

States should distinguish proposed, awaiting invitations, running, paused, awaiting owner, resolved, unresolved, cancelled and archived. Resume preserves accepted invitations, proposal versions and recorded positions. Restart must not replay already completed tool effects. Stop cancels pending room work and prevents late replies from restarting a closed meeting.

## Minimal settings

| Setting | Recommended default |
| --- | --- |
| Agents may propose rooms | Enabled for authorized agents |
| Automatic creation | Off; user approves the participant/context proposal |
| Department invitations | Approval by the destination's designated authority |
| Decision rule | Explicit responses from all required departments; abstention recorded separately |
| Final decision approval | Room owner/user required |
| Tool execution during discussion | Existing per-agent permissions, with room-specific restrictions if needed; no permission expansion |
| Discussion budget | Bounded rounds, replies and overall duration |
| Reporting | Explicit source/destination groups; summary and outcome, not full transcript by default |
| Completion | Archive with transcript preserved |

Advanced policies can later allow trusted automatic invitations, alternative quorum rules and delegated final approval. They should be separate permissions, not a broad switch that enables all automation.

## Host design and integration requirements

Store room metadata, invitations, delegate representation, agenda, states, budget, proposal revisions, responses, approval and reporting receipts in the Rust host. Clients render shared models and send intents. Use stable IDs for routing and current display names for presentation, preserving origin-at-the-time alongside current membership checks.

Reuse the group transcript and scheduler primitives, but introduce an explicit room discriminator. Existing `with_members` deduplicates ordinary groups by member set; two meetings with identical delegates must not merge with each other or with a permanent department. Close/archive must not delete or rename the original departments.

Revalidate invitation authority and membership on each privileged operation. Revocation stops future access and pending work; it cannot erase context already shown to an agent. Shared agent sessions mean room-specific context isolation, queued work and memory write-back require deliberate design. Do not claim that hiding messages from a client removes them from model history.

Room tools should cover proposing, inspecting, inviting, submitting a proposal, recording a position, pausing, requesting final approval and closing. Do not let an agent self-grant membership, impersonate another delegate, increase its own budget or directly mark universal agreement. Concurrent responses and approvals use revision checks; changed proposals invalidate earlier approvals.

Existing collaboration Hide/Compact/Full preferences should affect presentation only. Required invitations, permission decisions, errors and the final outcome remain discoverable. Verify desktop, iOS, Android and terminal behavior with one event schema. Start with departments and delegates on one host, matching the current group/member lookup. Cross-host rooms require additional identity, authorization and delivery design and should not be implied by the first version. No new orchestration framework dependency is recommended for this feature.

## Validation before calling it complete

Verify delegate origin labels, duplicate display names, multiple department memberships, invitation rejection/revocation, simultaneous edits, proposal changes, objection/abstention, unavailable members, cancelled late answers, restart/resume, budget exhaustion, duplicate publication and provider failure. Validate collaboration modes, quoting, RTL text, accessibility and archived-room navigation. Agreement quality must be evaluated separately from successful message routing.

## Research sources and limits

- [Speaker selection and participant filtering](https://microsoft.github.io/autogen/stable/user-guide/agentchat-user-guide/selector-group-chat.html): demonstrates a bounded team conversation with controlled speaker candidates. This supports the architectural pattern, not the quality of a specific Codync implementation.
- [Termination controls](https://microsoft.github.io/autogen/stable/user-guide/agentchat-user-guide/tutorial/termination.html): documents message, time, usage, handoff and external stop conditions. Usage caps require reported provider usage.
- [Multi-agent context boundaries and workflow patterns](https://docs.langchain.com/oss/python/langchain/multi-agent): emphasizes selective context and notes that a single agent may suffice for many complex tasks. This supports using meetings selectively.
- [Study of agreement and sycophancy in multi-agent reasoning](https://aclanthology.org/2025.findings-acl.1141/): evaluates reliability and cost issues on benchmark reasoning tasks. It motivates recording evidence and disagreement; it does not establish that meetings reliably produce correct business decisions.

The initial version adds persisted meeting metadata in existing transcript records, mandatory approval for agent-requested creation, user-approved delegate enrollment, revisioned proposals and authenticated responses, a fixed ten-turn/ten-minute budget, pause/resume/cancel/archive, owner approval and idempotent outcome publication. It reuses group scheduling and chat rendering. Desktop creation is in the create menu; phone creation is in group settings. Terminal group settings uses Ctrl+R for creation/inspection/actions.

The richer independent-first-opinion phase, configurable quorum/budgets, automatic invitation policies, isolated agent sessions, cross-host meetings, task conversion and configurable report recipients remain outside this first version. Consultation and meeting presets can be completed by the owner with an explicit summary. Decision rooms require explicit approval from every delegate on the same proposal and final user approval. Previous proposal versions and their recorded positions are preserved (up to 32 versions). Nonmember bots do not discover private room agendas through department listing; direct group reads/posts and mail inspection also enforce room access. Source department membership is revalidated, and only the user may publish outcomes to represented departments. Delivery is recorded separately from comprehension; publication does not automatically wake destination groups.

No real-device interaction checks have run under the no-local-build restriction. CI results must be checked for the installed revision. Discussion counts include scheduled member turns, passes, failed requests and queued turns. A host restart pauses interrupted rooms without replaying tools or resetting the budget. A room awaiting final user approval uses the existing needs-input indicator rather than a separate popup system. The host enforces representation and decision revisions; this does not guarantee decision correctness.

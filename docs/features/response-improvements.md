# Conversation improvements

This page records the current implementation and its limits. Features reuse the Rust host,
existing conversation state, Library, file transfer, reply composer and shared UI controls.
No separate file index, folder store, account requirement, analytics, dock or floating widget
is introduced. Source changes ship as ordinary commits on main; versions follow upstream.

## Delivery status

| Area | Implemented | Practical limits |
| --- | --- | --- |
| Text previews | Markdown Read/Source, copy, wrap, retry, literal search with numbered matching lines; per-file preferences | UTF-8 text up to 1 MiB; first 100 matching lines shown; up to 100 preference identities per device |
| Conversation files | Group Library, filename search, dated rows, source labels, older-file paging, Show in chat | Search/counts cover loaded pages; source navigation loads at most 25 contiguous pages per attempt; call-only sources have an explicit error |
| Selected replies | Message, collaboration, shared-file, upload, workspace and artifact excerpts reuse the reply bar | Excerpts are bounded to 4,000 characters; authority and source identity come from host storage |
| File provenance | Shared-file hash and size checks; uploaded-file size and literal checks; unique passages get line ranges | Repeated text has no guessed line range; rendered text that differs from source asks for Source selection |
| Workspace excerpts | Host-confined read followed by a saved excerpt with path, owner, digest and line range | Later file edits do not replace the saved excerpt; no additional file access is granted |
| PDF and media | Bounded verified PDF/audio/video previews; original download remains available | 32 MiB; native codec support varies; phones refuse more than 500 PDF pages; terminal downloads binaries |
| Delimited tables | Shared host CSV/TSV formatter with readable tables and Source fallback | 64 KiB formatted input, 100 rows, 20 columns, 256 characters/cell; no XLSX parser |
| Workspace browsing | Read-only directory navigation within authorized roots on Unix and Windows | 2,000 entries examined, 500 shown; no symlink/reparse traversal; no binary workspace preview grant |
| Queue | Queued text editing, pause/resume, earlier/later ordering and reported wait reason | Actor-serialized; dispatched messages cannot be edited; explicit Send now overrides pause for ordinary messages |
| Queue recovery | Managed queued user messages survive restart; new messages in a paused queue are retained | Untouched queues keep the existing restart-cancellation behavior; Stop still cancels pending messages; task recovery is separate |
| Context | Compact reported usage beside the composer and detailed usage in bot details | Session/model-bound agent reports only; no guessed token counts; summary before/after shown only when a reported drop is observed within 60 seconds |
| Rendering | Offline code highlighting, display math, Mermaid expansion/fit/zoom; inline math with bold/emphasis/strike/code | Formula paragraphs are bounded; links/HTML retain native rendering and navigation; failed formula rendering retains original text |
| Streaming | Explicit stream_message delivery, reconnect cleanup and unchanged Markdown-block reuse | Generic v1 ACP chunks remain trace content; draft v2 fields are not interpreted without negotiation |
| Sources | Numbered report citations open a source sheet with title, URL, open and copy | No source fetch or verification is invented; ordinary links stay ordinary links |
| Organization | Existing collapsible roster sections, rename/remove/move, desktop bulk Organize chats | Flat conversation folders, not workspace isolation, permissions, nesting or department membership |
| Group identity | Existing image editor supports local group icons, upload/URL/tint/reset | Same bounded image normalizer and authenticated sync; accountless local storage; member-composite default |
| Group messaging | Members discover, read and post to other joined groups with source attribution | No nonmember transcript access; posts are deduplicated delivery, not guaranteed replies |
| Department inbox | Pending requests, explicit approve/reject, designated reply handler, automatic approval setting | Text-only, one routed reply; no membership grant; removed/unavailable handlers fall back to manual approval for new mail |
| Group tasks | Draft assignments, owner/start, cancellation-aware pause, resume/reassign, result/history | Real attempts run separately in the owner's chat; side effects are not rolled back; restart requires manual resumption |
| Storage | Database/reusable-page accounting, bounded managed-file inventory, explicit compaction | Not whole-disk usage; no automatic file deletion; SQLite compaction needs temporary free space |

Implementation does not establish device-level interaction correctness. See Validation below.

## File loading and source navigation

All previews use existing authenticated transfer or host-confined workspace reads. Shared
snapshots verify recorded size and SHA-256. Unsupported, oversized, binary, invalid UTF-8,
offline and integrity failures retain explicit errors and download/source alternatives.
HTML source is not executed by the text viewer. Retrying grants no new access.

Library pages examine 500 file-bearing messages using a host-owned sequence cursor. Older
pages extend the existing Library; artifact details appear once on the first page. File/upload
rows retain their source message IDs. Show in chat loads contiguous transcript pages, never
inserts one isolated old entry that would conceal a gap from normal history paging. A source
beyond the navigation bound requires another explicit attempt after loading earlier messages.

Workspace selection captures only a bounded excerpt and its provenance as a notice in the
requesting conversation. Shared/upload selection re-reads and validates the saved file.
Sending a reply revalidates its source identity. Line numbers are attached only for a unique
literal match. Neither selecting nor quoting proves that a document's claims are correct.

## Artifact selection and isolated rendering

Quote selected artifact text is an explicit app control. Desktop accepts a bounded response
from the exact owned frame and a one-use nonce; phones evaluate a bounded selection expression
in the owned viewer. The host resolves the artifact and its real source card. The resulting
excerpt is reviewed in the ordinary reply bar and never sent automatically. There is no file,
command or permission bridge. Artifact scripts can misrepresent their own document, so the
excerpt is described as user-provided content, not verified research or trusted instructions.

Code highlighting, math and diagrams reuse the packaged offline bundles. Rendering cannot
fetch remote resources. KaTeX runs in untrusted mode with bounded expansion; Mermaid uses
strict security and bounded text/edges. Formula paragraphs allow at most eight paragraphs per
Markdown view, 4,000 UTF-16 units per paragraph, 1,024 units per formula and eight nesting levels.
Code spans stay literal. Native font scaling is passed through on phones. Original source
remains visible if rendering fails; layout callbacks expose only a finite bounded height.

## Group settings and department delivery

Group editors use labeled Name and Group instructions fields and group-specific member,
inbox, task, Library, image and storage controls. A group has no independent model, agent
session or workspace; those remain properties of its members. At least one member is required.

Ordinary bots discover groups through chat tools, independently of Manage groups. Current members can inspect task summaries and individual recorded assignments through list_group_tasks/read_group_task; these tools do not grant task-management rights. Reading or
posting requires current membership. Posting does not block while the destination answers,
avoiding actor wait cycles when a bot belongs to both groups. Same-room replies stay normal
final replies. Request IDs deduplicate saved delivery but do not promise comprehension.

Nonmembers use send_group_mail. Pending mail is excluded from group prompts. Manual approval
chooses a current member; only that designated current member can send the single routed reply.
The sender receives that reply without destination history. Mail inspection follows current
source/destination membership, including revocation when a sender leaves its source department.
Concurrent decisions use revision checks; approved unpublished mail offers a retry.

Automatically approve incoming mail is off by default in Group inbox settings. It requires
a designated current handler. New requests are approved and dispatched through the same path;
existing pending requests stay pending. Missing/removed handlers leave new requests pending.
Disabling the setting does not undo deliveries. No bot can use mail content as authority to
change this policy or enroll itself. Mail attachments, multiple replies, forwarding chains
and a standalone address book are not part of this text-inbox feature.

A saved delivery is not a durable exactly-once execution queue: Stop, provider errors,
deleted groups and removed handlers can prevent a response. The inbox lists up to 100 requests,
pending first. Terminal group settings uses Ctrl+B for inbox approval/policy editing through the same host API.

## Group tasks and queue semantics

Create task saves a draft. Start queues a distinct user-authorized attempt in the chosen
current member's own chat. Attempts are not batched with ordinary messages or injected into
another running turn. Pause stays Pausing until the actor confirms cancellation. Only stopped
attempts can be reassigned or resumed. Resume asks the bot to inspect earlier work before
repeating actions; it is not rollback or a transaction around provider side effects.

Normal Stop and queue removal record failed attempts. Owner removal/group deletion prevents
queued task dispatch. A host restart marks queued/running tasks interrupted and requires the
user to resume. History stores the newest events up to 100 events/128 KiB per task and 4,000 characters per event detail.
The board lists up to 100 tasks, unfinished first, and refuses more than 100 unfinished
assignments. Detailed execution remains in the owner's chat; result history is in the source
group. Completed means the attempt ended normally, not that Codync independently verified the
user's real-world objective. Terminal group settings uses Ctrl+T for task creation and start/pause/resume/reassignment; the history action copies full recorded history.

Ordinary queue pause affects future user messages without cancelling the running turn.
Earlier/later reorders only waiting user slots. Messages already being steered are refused.
Pause or reorder opts messages into durable queue recovery. Queue state read failures keep
user messages paused. Stop cancels pending messages. Send now remains an explicit override for
ordinary messages; task attempts use task controls to preserve their separate cancellation.

Terminal shortcuts: `~` toggles queue pause, `(`/`)` move the next queued user message;
Library `n` loads older files, `g` locates a loaded source or requests more history;
text preview `/` starts a literal search, Enter finishes input, `n` finds the next matching
numbered line, and Read/Source or wrap clears the search. All clients persist bounded view preferences on device without file contents or search terms. Terminal keys are hashed with the host address and file identity.

## Storage semantics

Storage overview reports SQLite pages and scans only managed shared-file/upload/artifact
roots, at most 10,000 entries per category and depth eight, without following symbolic links.
Partial/unavailable results are labeled. Project files, logs, caches and journals are excluded.
Compact database uses [SQLite VACUUM](https://www.sqlite.org/lang_vacuum.html), preserving live
rows and files. Confirmation explains temporary free space up to twice the database size and
briefly serialized database access. Files are never labeled garbage from bounded Library
history, and no shared or workspace file is deleted. The terminal opens the same accounting/confirmation through O, or Ctrl+O in group settings.

## Protocol boundary

The current [ACP v2 migration guide](https://github.com/agentclientprotocol/agent-client-protocol/blob/main/docs/protocol/v2/migration.mdx)
describes message-ID upserts and foreground state updates, but explicitly labels v2 draft and
requires negotiated, feature-gated support alongside v1. This host's v1 connections must not
consume those fields as if v2 had been negotiated. Explicit stream_message is the supported
live-chat contract. Full v2 lifecycle negotiation is a separate protocol migration, not a
rendering switch or a guessed finality signal. Adapter-specific mappings require a documented
signal from the actual adapter in use.

## Validation

Local dependency installation, builds and app/simulator launches are prohibited in this
session. Local verification is source review, generated-template consistency and whitespace
checks. Desktop types/tests, isolated-browser rendering, iOS compilation and Rust formatting,
lints/unit/integration checks run in GitHub Build. Final packaging must finish before a build
is called distributable. Android Actions remain disabled by request; Android source changes
are reviewed but not build-verified here.

Real app keyboard/touch selection, focus, cancellation timing, screen readers, reduced motion,
large fonts/content and codec behavior remain unverified. CI does not substitute for those
checks. The code and documentation must retain that distinction.

Terminal group settings also exposes Ctrl+P for the existing custom-image editor. Terminals
retain a text avatar; graphical clients show the normalized image. All department mutations
remain host-validated. Terminal owner/handler fields resolve unique current display names
(or explicit IDs) and reject ambiguous names instead of guessing.

## Upstream synchronization

Version 2.43.1 includes the upstream late-group-answer scheduler, OpenCode startup/refusal
compatibility, connector-name validation, signed development release channels and version-bump
tooling. Group display-name mentions, quoted-line filtering, department tools and read receipts
remain in place. The local development Worker is isolated and includes no hosted-machine
provisioning. Account enrollment and hosted Codync Cloud are available only through explicit sign-in and
hosted-computer setup. Continue without an account remains the primary onboarding action.
Host relay enrollment is opt-in; custom/private relay configuration works independently.
The support invitation remains disabled. Telemetry is off on host, desktop and Android;
iPhone analytics remain removed. No private-relay implementation changes are required.

## Group profile and pending requests

Group editors use the shared profile cards, field surfaces and compact avatar controls on desktop, iOS and Android. Group activity links open tasks, inbox and library; member editing keeps at least one member. The desktop details panel uses the same card spacing and includes a tasks shortcut.

A compact Requests counter opens approvals from other chats without navigating away. Each request identifies its agent, chat and computer and uses the existing permission options and host response method. Requests never open automatically or advance under the pointer after a decision; the next request requires an explicit selection. Offline computers cannot receive approvals. Pending permissions are included in fresh and incremental snapshots even when older history was trimmed. Collaboration display preferences and execution routing are unchanged.

Validation for this change uses GitHub CI. Real app interaction and accessibility walkthroughs were not run because local builds and launches are disabled for this workspace. Android source is updated without triggering its workflow.

## Bidirectional message text

Desktop prose blocks and composer fields resolve direction from their content using HTML `dir="auto"`; multiline prose and drafts use paragraph-aware bidi handling. List indentation and quote borders use logical start edges. Inline code is isolated LTR and links isolate their own direction. Composer measurement uses the same direction rules as the visible field. Code previews remain LTR across the offline renderer copies.

iOS uses SwiftUI’s iOS 26 content-based writing direction for Markdown and composition, with list markers included in the native text paragraph. Android uses content-based text direction and the Unicode bidi engine for Markdown row layout. These rules apply to message content without changing app navigation direction or storing invisible formatting characters in messages. Native quote borders and nested-list indentation still need interaction checks in mixed-direction conversations.

Desktop emphasis now parses enclosing formatting before its nested code spans, so bold text containing inline code no longer leaves literal emphasis markers.

References: [W3C structural direction](https://www.w3.org/International/questions/qa-html-dir), [W3C inline isolation](https://www.w3.org/International/articles/inline-bidi-markup/), [SwiftUI content-based writing](https://developer.apple.com/documentation/swiftui/text/writingdirectionstrategy/contentbased), [Compose text direction](https://developer.android.com/reference/kotlin/androidx/compose/ui/text/style/TextDirection).

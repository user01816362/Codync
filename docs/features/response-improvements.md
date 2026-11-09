# Conversation improvements

Deliver improvements as feature batches, one commit per batch. Each completed batch is committed and pushed without waiting for intermediate Actions runs, as requested. Review the latest CI results after the implementation batches and repair failures before claiming validation. The final build completes packaging. Versions continue to follow upstream. No dock, floating widget, account requirement or analytics is added.

| Batch | Scope | State |
| --- | --- | --- |
| 1 | Existing text preview: Markdown Read/Source, copy, wrap, retry, clear errors; desktop, phones and terminal | Desktop, iOS and host checks passed; intermediate packaging cancelled by request |
| 2 | Extend the existing Library with Files filtering/counts, text previews, source labels and retry; retain downloads | Implemented; CI pending |
| 3 | Phone message selection/reply parity through existing metadata | Implemented; CI pending |
| 3b | Structured shared-file excerpts and collaboration selections with source context | Implemented for saved file snapshots/collaboration; live-file excerpts pending |
| 4 | Bounded local PDF/audio/video adapters using verified transfers; terminal download fallback | Implemented; CI/device checks pending |
| 5 | Shared host CSV/TSV formatting with Source fallback; XLSX remains a separate dependency decision | Implemented; CI pending |
| 6 | Read-only workspace directory navigation within host-authorized roots | Implemented; CI/platform checks pending |
| 7 | Queued-only text editing through the actor, preserving files/replies | Implemented; CI pending |
| 8 | Session-bound reported context usage in bot details; reuse existing compaction state | Implemented; CI pending |
| 9 | Diagram expansion, fit/zoom and scrolling with existing offline bundles | Implemented; CI pending |
| 9b | Bounded inline formulas in plain prose, preserving native styled/link/code paragraphs | Implemented; CI pending |
| 10 | Incremental Markdown presentation and adapter-specific streaming finality | Block reuse implemented; generic ACP finality unavailable in the current protocol |
| 11 | Inspect numbered report sources with title, URL, open and copy actions | Implemented; CI pending |

Persistent group task ownership/reassignment is a separate host-model proposal, not a setting on today's group rounds. Custom group icons, storage overview and artifact selection bridges are optional later improvements. Existing model settings, memory, skills, routines, sections, reactions, collaboration controls and artifact Library are reused rather than duplicated. A renderer replacement needs measured correctness, accessibility and performance benefits across affected clients before selection.

## Batch 1 behavior

Markdown opens in readable form. Source switches to selectable raw text. The preview toolbar copies the original text, toggles line wrapping for source, and retries through the same authorized load path. Reloading does not reset the current view controls. Workspace errors retain the host's explanation; snapshot failures explain UTF-8, size and integrity requirements without bypassing them. Desktop rejects oversized text snapshots before starting the download.

Phone preview presentation is shared between attachments and workspace files. Preview load state stays in its existing client loader. Terminal shortcuts: `v` toggles Markdown/source, `w` toggles source wrapping, `c` copies through the terminal's existing clipboard facility, `r` retries, and Esc closes. Clipboard support depends on the terminal. With wrapping disabled, terminal lines are clipped to the viewport; enable wrapping to read the full line.

The file remains read-only; HTML source is not executed. Retry grants no additional file access. Image viewing remains the existing image viewer. Find-in-file and remembering preferences after closing a preview are not delivered by batch 1.

## Validation

Local dependency installation, builds and app/simulator launches are prohibited in this session. Source review and whitespace checks run locally; desktop type checking, iOS compilation and host checks run in GitHub Build. Android Actions remain disabled by request. Android changes receive source review but are not build-verified here. Keyboard, screen-reader, touch-selection and real-device interaction checks remain pending and must not be represented as passing because CI is green.

## Batch 2 behavior

The existing Library already indexes published artifacts, bot-shared files and user uploads. Batch 2 extends that view instead of creating a parallel Files store or index. All/Files filtering and counts refer to items returned by the existing Library API, not an unlimited disk inventory. The host currently examines at most the newest 500 file-bearing messages. Existing file and image authorization is unchanged.

Supported text rows open the same preview as message cards. Shared-file downloads remain available alongside previews (desktop/Android) or in the row actions (iPhone). Images retain the existing image viewer. Unsupported binary files do not open in the text renderer. Retry reloads the Library through the same host API. File detail labels distinguish the bot's shared files from the user's uploads where that metadata is known.

Terminal: Enter opens artifacts in the browser or supported text in the preview; `F` toggles All/Files, `R` reloads, `f` downloads and `r` renames an artifact. The existing terminal image behavior remains download-only. The Library is still reached through bot details; group Library navigation and Show in chat are not delivered by this batch. Those need coordinated navigation work rather than a duplicate sidebar or inferred sender metadata.

## Batch 3 behavior

On iPhone and Android, selecting a passage in an ordinary user or agent message now opens the existing reply bar, preserving its source entry ID and selected text instead of inserting a formatted block into the draft. The bar names the author and distinguishes Quoted from from Replying to. Sending includes `replySelection` through the existing host API. The host derives the author from the stored message; selection remains a user-provided excerpt, not proof of source accuracy. The selected passage is bounded to 4,000 Unicode scalars/code points. Failed-send metadata retains the excerpt for retry.

Desktop already uses this structured message-selection contract. Terminal selection still uses its existing source/quote flow; this batch adds no terminal-native selection interception. File excerpts and collaboration-sidechat selections remain a separate batch. Phone collaboration selections now use the same structured reply bar: request/reply parts retain their stored exchange ID and author. The host validates the part against the saved exchange before dispatch.

On iPhone a selected reply uses normal authenticated delivery instead of the older text-only mailbox contract, which cannot preserve the excerpt. An offline failure stays retryable with its reply metadata; the passage is never silently downgraded to a whole-message reply by the client. Android's pending-send body already travels intact through its delivery path and now includes selection metadata.

## Queued editing

Queue rows on desktop and phones offer Edit queued message. The terminal uses `e` on a queued message and its existing multiline field sheet. The host serializes the edit with dispatch: a message already taken from the queue is refused, without creating a replacement send. A timed-out request is skipped if the actor has not begun handling it. Text is bounded to 64 KiB and cannot be empty. Attachments and reply metadata remain attached. Because this editor does not track pasted substrings, edited text is conservatively marked as pasted for memory attribution.

Group queue editing is not offered. An older host can reject the new method without losing the original message. Durable queue pause/reorder and richer host-reported waiting reasons remain separate work; Send now and Remove retain their existing behavior.

## PDF and media previews

File cards and Library rows open supported PDF, audio and video snapshots up to 32 MiB through the existing file preview. Supported extensions are PDF, MP3, M4A, WAV, OGG, MP4, M4V, MOV and WebM. Decoding/playback support depends on the platform's installed codecs. Failed or unsupported previews retain the existing download path. Live workspace binary paths are not newly authorized by this feature.

Shared files are checked against saved size and SHA-256; uploads use their bounded authenticated transfer. Desktop creates revocable local object URLs with fixed MIME types and uses built-in PDF/media viewing. Only blob frames/objects are added to its content policy; remote frame origins are not added. The chat window enables its bundled PDF plugin while retaining the renderer sandbox and context isolation. Phone PDF viewers use native rendering; iPhone and Android refuse more than 500 pages. Android renders one page at a time with a maximum 2048-pixel edge. Native player controls require user playback; no remote agent URL or generated HTML is sent to the player. iPhone playback uses a temporary file removed when closing the viewer. Desktop object URLs are revoked on closure/reload. Terminal keeps downloading binary files rather than attempting graphical playback.

Native PDF/media interactions and codec failures need device testing. Source/CI validation alone does not establish viewer accessibility, playback behavior, decoder memory usage or PDF annotation/link behavior on every OS.

## Delimited table previews

CSV and TSV text previews request the same host formatter on desktop and phones; the terminal uses that formatter directly. Inputs are limited to 64 KiB for formatted tables, 100 rows, 20 columns and 256 Unicode characters per cell. Original text previews retain their 1 MiB limit and remain accessible through Source. The host formats client-supplied text already loaded by the authorized preview; the formatting method grants no file access, stores nothing and reads no path.

Quoted fields, doubled quotes, embedded newlines and CRLF records are handled. Malformed quotes produce an explicit error and Source fallback. Cells escape Markdown punctuation and do not execute formulas, links, HTML or macros. Columns use numeric labels: the first record is preserved as data rather than guessed to be a header. Truncation is labeled and never changes the original file. Empty data shows No rows. Large, offline or older-host formatting failures leave Source available. A formatted view requires the connected host on phones/desktop; cached raw text remains readable offline.

XLSX parsing is not shipped by this batch. It requires a separately chosen workbook/decompression implementation, bounded ZIP handling and multiple-sheet semantics, not merely adding its extension to text previews.

## Expanded rendering

Graphical rich previews expose Expand preview using the existing sheet. The same bounded renderer and source copy action remain in use. Mermaid adds local Zoom in, Zoom out and Fit controls, a zoom percentage, scroll-to-pan and keyboard `+`, `-`, `0` on its focusable viewport. These controls run only inside the isolated preview and add no network or application bridge. Zoom is bounded from 0.1× to 8×. The canonical template and all three packaged copies carry identical control logic; vendored library bytes and versions are unchanged.

Terminal keeps the readable source rather than a graphical viewport. Existing fenced and standalone display math remain supported. Inline formula layout within native flowing text is not delivered by this batch and must not be implied by expanding a math preview.

## Source inspection

Existing numbered report citations and the Sources list now open the same source-inspection sheet on graphical clients. It preserves the source number, title and full selectable URL, with explicit Open source and Copy URL actions. No source is fetched in the background and no independent verification is claimed. The existing host report schema, source bounds and HTTP(S) link restrictions remain unchanged. Terminal report fallback already shows numbered titles/URLs and remains readable without a graphical sheet. Plain Markdown links remain ordinary links; no citation metadata is invented from them.

## Inline math paragraphs

Graphical clients render bounded formulas within plain prose paragraphs: explicit `\(...\)` and unambiguous `$...$` expressions. Currency-like pairs without mathematical content stay literal. Formulas use the existing offline math bundle in untrusted mode. Invalid expressions retain their original text, not an empty paragraph. Paragraphs containing other Markdown styling, links or code outside formulas keep the existing native renderer; this conservative path does not claim full mixed Markdown/math layout.

At most eight such paragraphs per Markdown view are embedded, each at most 4,000 UTF-16 units and each formula at most 1,024 units. Extra paragraphs remain readable source. An observer reports rendered height so paragraphs are not assigned a guessed fixed height. Desktop accepts only layout messages from its exact iframe; native clients expose only a finite 24–1,200 pixel height callback with no data, file, navigation or network actions. The native layout handler is removed when the view closes. Original source remains available through the message's selection/copy flow. Terminal keeps source. Real-device selection, accessibility and dynamic font-size behavior remain unverified.

## Workspace navigation

Bot details now offers Browse workspace on desktop and phones. The terminal uses `W` while a message is selected; inside the browser, `g` returns to the root, `H` toggles hidden entries and `c` copies the selected path. Listings use the same host-authorized workspace roots as text previews, with directory navigation, parent/root controls, hidden-file filtering and retry. At most 2,000 directory entries are examined and 500 displayed; truncated results are labeled. Listing order is directories first, then names. A listing is not a disk-wide search or an access grant.

Supported text files up to 1 MiB open the existing read-only preview. Links and special files cannot be opened through the browser. Unsupported binaries retain the explicit bot-sharing route. Groups have no workspace of their own. Windows enumeration uses the opened directory handle and rejects reparse targets; Unix listing retains the confined directory descriptor. Actual filesystem, touch and keyboard interactions remain unverified on devices.

## Collaboration excerpts

Phone collaboration request and reply selections now preserve the exchange ID and distinguish the request author from the reply author. They reuse the normal reply bar and existing host-validated `request`/`reply` selection contract, matching desktop. They no longer insert a formatted block into the draft. Shared-file excerpts remain separate from conversation excerpts and require validated file identity before they can claim an origin.

## Shared-file excerpts

Text snapshots explicitly shared in a saved chat message can be quoted through the existing reply composer. Desktop uses the selection context menu; phones offer Select text / Quote. The host resolves the saved message, checks that the file ID belongs to its files or attachments, and derives the author and filename from storage. Sending revalidates the same source. Excerpts remain bounded to 4,000 characters and are identified to the agent as user-selected content, not instructions from the file. Errors retain the original preview and source instead of inserting unattributed text into the draft.

The preview affordance covers shared snapshots and uploads opened from their saved message. Library uploads and live-workspace previews without that source entry do not invent one; their text remains selectable/copyable. This does not grant arbitrary file access or prove that a rendered selection is a verbatim file substring.

## Streaming presentation and finality

Desktop keeps unchanged Markdown blocks through its existing block comparison; iPhone now explicitly skips unchanged blocks with Equatable views, and Android retains parsed blocks with its existing source-keyed cache. Appending text updates the same explicit streaming entry and retains block positions, including an unfinished code fence. This is block reuse, not a claim that parsing is constant-time or that arbitrary Markdown can be finalized before later input arrives.

The current ACP protocol does not provide a reliable final-answer boundary for ordinary message chunks. Its message-ID proposal explicitly documents ambiguous boundaries; proposal fields must not be treated as a deployed finality guarantee. Consequently ordinary ACP chunks remain trace content, thought chunks remain separate, and the existing explicit `stream_message` contract is the supported live-chat delivery path. An adapter-specific final-answer mapping requires that adapter's documented signal before enabling it. See the [ACP message-ID proposal](https://github.com/agentclientprotocol/agent-client-protocol/blob/main/docs/rfds/message-id.mdx). Stop, reconnect and assistive-technology interactions still require device validation.

## Group editor and messaging audit

The group editor now uses labeled, boxed Name and Group instructions fields on desktop,
iPhone and Android, matching existing bot form controls. It retains the member-composite
avatar and group-specific membership/actions, and prevents removing the last member.
Groups still have no model, private workspace or agent permissions of their own: those
belong to their members. Custom group icons, explicit task owners and inbox delegation
are separate capabilities, not missing bot-profile controls to copy into a group.

Sidebar organization already exists as host-backed roster sections: right-click a bot,
choose Move to section, and choose an existing section or New section. Sections can be
renamed, removed or collapsed. These are flat conversation folders; they are not isolated
workspaces, permission boundaries, nested folders or department membership. The desktop + menu now exposes Organize chats for selecting several bots or groups and
moving them into an existing or newly named section. It reuses the same host mutations,
reports partial failure, and keeps pinned chats in their pinned band. No duplicate
folder store has been added. The batch list above does not claim those broader capabilities.

Ordinary bots now discover groups through the chat tools independently of Manage groups.
They can read and post only to groups they belong to, including posting from another group.
The host attributes the sender and source department, bounds messages, and deduplicates
successful retries by request ID. Posts start a receiving room turn without blocking the
sending bot, avoiding a wait cycle when it belongs to both groups. Delivery is not an
acknowledgement or a guaranteed reply. Same-room messages remain normal final replies.
Nonmember requests now use a durable group inbox. Desktop group details and all graphical
group editors expose Group inbox. The user approves or rejects each request and chooses a
current member as handler. Pending mail is a notice, excluded from group prompts; approval
publishes an attributed request and starts the room turn. Only the chosen current member may
send the single routed reply. The source conversation receives that reply without access to destination
history. Mail inspection follows current source/destination membership: removing a bot
from a department revokes its access to that department’s mail, including mail it originally
sent there. Private senders retain access to their own requests. Concurrent decisions use revision checks, and deterministic entry IDs deduplicate
retries. Approved requests whose publication failed offer Retry approved delivery. Mail is
external conversation data, not authority to change permissions. Automatic approval is an explicit Group inbox setting, off by default. It requires a valid current member as handler; new requests are approved and dispatched to that member. Existing pending requests stay pending. A missing or removed handler falls back to manual approval. No bot self-enrollment is added. The inbox shows up to 100 requests, pending first.

A delivered request is not a guarantee of execution or response: deleted groups, removed
handlers, Stop, provider failures and unavailable actors can prevent one. Approved delivery
survives as a saved chat entry; scheduling is not a durable exactly-once execution queue.
The current inbox does not support attachments, multiple replies, forwarding chains,
standalone department address books. Terminal users can see mail
notices but do not yet have an interactive inbox approval sheet.

## Remaining recommendation gaps

| Recommendation | What is still missing |
| --- | --- |
| Conversation files | Show in chat navigation, group Library navigation, a date/size-rich file inventory beyond the current bounded Library results |
| Text preview | Delivered: literal case-insensitive search with numbered matching lines (first 100 shown); Read/Source and wrap preferences retained for up to 100 file identities on each device |
| File selections | Library upload origins and file/artifact labels implemented. Saved-file selections require literal matching and integrity/size checks; unique passages receive verified line ranges. Live workspace quote origin remains in progress. |
| Queue | Delivered: actor-serialized pause/resume and earlier/later ordering; managed user messages survive restart, with host-reported queue reason. Device interaction still needs validation. |
| Context | Delivered: compact per-chat reported usage; compaction notices retain a reported before/after drop when received within 60 seconds. No token estimate is invented when the adapter reports none. |
| Inline math | Mixed styled Markdown/formula layout and accessibility/device interaction validation |
| Streaming | Documented adapter-specific finality; generic ACP chunks remain trace content |
| Optional task lifecycle | Implemented: group assignments, explicit owner/start, cancellation-aware pause, resume/reassign and bounded history. Restart marks active attempts interrupted rather than replaying side effects. CI and interaction checks pending. |
| Optional identity | Delivered: existing custom-icon editor exposed for groups; accountless local host storage; default remains the member composite |
| Optional storage | Implemented: bounded managed-file inventory, SQLite page/reusable-space accounting and explicit database compaction with a temporary-space warning. No automatic file deletion. |
| Optional artifact interaction | Implemented: explicit Quote selected artifact text, bounded selection, host-validated source card and ordinary reply-bar review. No automatic send or command bridge. |
| Validation | Actual app selection/focus, keyboard, screen-reader, reduced-motion and large-content checks |

These are not claimed complete by a green build. PDF/media decoding varies by platform;
CSV/TSV tables are bounded and do not imply XLSX support. Existing sidebar sections are
organization only. No dock, account catalog or floating shell is part of these batches.

## Preview search and group identity

Text previews now search the loaded UTF-8 snapshot locally, showing matching line numbers
and up to 100 matching lines without changing file contents. View controls persist on the
device for up to 100 identities; file contents and search terms are not persisted.
Custom group images reuse the existing bounded host image normalizer and icon snapshot.
Upload, URL, tint and reset are available in group settings. Icons are saved locally and
sent to paired clients through existing authenticated state; Codync sign-in is not needed.
An explicit cloudBackup API option retains the existing optional remote transport.

## Managed queue controls

Queue controls pause future user messages without cancelling the running turn. Earlier/later
changes only the ordering of waiting user messages, preserving other kinds of scheduled work.
Messages already being steered are refused. Pausing or reordering opts those waiting messages
into durable recovery; untouched queues retain the existing restart-cancellation behavior.
New messages sent to a paused queue are retained too. Stop still cancels pending user messages.
Explicit Send now remains an override. Terminal: `~` toggles pause, `(`/`)` move the next user
message earlier/later, and the paused marker is shown beside queued text.

## Group task lifecycle

Group settings exposes Group tasks on graphical clients. Create saves a draft; Start sends
a distinct user-authorized attempt to the chosen current member's own chat. Attempts are not
batched with other queued messages or injected into another turn. Pause requests cancellation
and stays Pausing until the running turn actually ends. Only stopped drafts/attempts can be
reassigned or resumed. Resume is a new attempt with an explicit instruction to inspect earlier
work before repeating side effects; it is not rollback or transactional execution. Normal Stop
and queue removal record failed attempts. The board retains up to 100 history events per task,
with each detail bounded to 4,000 characters. Active tasks are listed first, up to 100; new tasks
are refused when 100 unfinished assignments exist. Restart marks running/queued tasks interrupted;
resuming requires the user, so provider side effects are never replayed automatically. Task result
history appears in the source group; detailed execution remains in the owner's chat.

## Host storage

Host storage is available in conversation details/group settings on graphical clients.
It reports SQLite allocated/reusable pages and a bounded scan of shared-file, upload and
artifact roots (10,000 examined entries per category, depth eight; no followed symbolic
links). Partial/unavailable results are labeled; this is not whole-disk usage. Logs,
project files, caches and SQLite journals are excluded. Compact database uses SQLite VACUUM,
preserving live rows and files, with explicit confirmation and a warning that temporary
free space up to twice the database size may be required. It runs off the async runtime but
serializes database access. No file is classified as garbage from the Library's bounded
history, and no workspace or shared file is deleted. See SQLite's [VACUUM documentation](https://www.sqlite.org/lang_vacuum.html).

## Artifact and file excerpt provenance

Artifact viewers expose Quote selected artifact text. Reading selection happens only after
the app button is pressed; desktop checks the exact source frame and a one-use request
nonce, while phones evaluate the bounded selection expression in the owned viewer. The
host resolves the current artifact and its real source message. The excerpt opens the
ordinary reply bar for review and does not send automatically. An artifact cannot invoke
host commands, read files or grant access through this path. Artifact selections remain
user-provided excerpts rather than proof that the artifact's scripts represented data honestly.
Saved file quotes now re-read at most 1 MiB, verify shared-file hashes/upload size, and require
a literal match. Unique occurrences get start/end line numbers; repeated text has no guessed
line range. A rendered passage that is not literal asks the user to quote Source instead.

# Conversation improvements

Deliver improvements as feature batches, one commit per batch. Each completed batch is committed and pushed without waiting for intermediate Actions runs, as requested. Review the latest CI results after the implementation batches and repair failures before claiming validation. The final build completes packaging. Versions continue to follow upstream. No dock, floating widget, account requirement or analytics is added.

| Batch | Scope | State |
| --- | --- | --- |
| 1 | Existing text preview: Markdown Read/Source, copy, wrap, retry, clear errors; desktop, phones and terminal | Desktop, iOS and host checks passed; intermediate packaging cancelled by request |
| 2 | Extend the existing Library with Files filtering/counts, text previews, source labels and retry; retain downloads | Implemented; CI pending |
| 3 | Phone message selection/reply parity through existing metadata | Implemented; CI pending |
| 3b | Structured shared-file excerpts and collaboration selections with source context | Implemented for shared snapshots/collaboration; uploaded/live-file excerpts pending |
| 4 | Bounded local PDF/audio/video adapters using verified transfers; terminal download fallback | Implemented; CI/device checks pending |
| 5 | Shared host CSV/TSV formatting with Source fallback; XLSX remains a separate dependency decision | Implemented; CI pending |
| 6 | Read-only workspace directory navigation within host-authorized roots | Implemented; CI/platform checks pending |
| 7 | Queued-only text editing through the actor, preserving files/replies | Implemented; CI pending |
| 8 | Session-bound reported context usage in bot details; reuse existing compaction state | Implemented; CI pending |
| 9 | Diagram expansion, fit/zoom and scrolling with existing offline bundles | Implemented; CI pending |
| 9b | Bounded inline formulas in plain prose, preserving native styled/link/code paragraphs | Implemented; CI pending |
| 10 | Incremental Markdown presentation and adapter-specific streaming finality | Planned |
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

Bot details now offers Browse workspace on desktop and phones. The terminal uses `W` while a message is selected. Listings use the same host-authorized workspace roots as text previews, with directory navigation, parent/root controls, hidden-file filtering and retry. At most 2,000 directory entries are examined and 500 displayed; truncated results are labeled. Listing order is directories first, then names. A listing is not a disk-wide search or an access grant.

Supported text files up to 1 MiB open the existing read-only preview. Links and special files cannot be opened through the browser. Unsupported binaries retain the explicit bot-sharing route. Groups have no workspace of their own. Windows enumeration uses the opened directory handle and rejects reparse targets; Unix listing retains the confined directory descriptor. Actual filesystem, touch and keyboard interactions remain unverified on devices.

## Collaboration excerpts

Phone collaboration request and reply selections now preserve the exchange ID and distinguish the request author from the reply author. They reuse the normal reply bar and existing host-validated `request`/`reply` selection contract, matching desktop. They no longer insert a formatted block into the draft. Shared-file excerpts remain separate from conversation excerpts and require validated file identity before they can claim an origin.

## Shared-file excerpts

Text snapshots explicitly shared in a saved chat message can be quoted through the existing reply composer. Desktop uses the selection context menu; phones offer Select text / Quote. The host resolves the saved message, checks that the file ID belongs to its files or attachments, and derives the author and filename from storage. Sending revalidates the same source. Excerpts remain bounded to 4,000 characters and are identified to the agent as user-selected content, not instructions from the file. Errors retain the original preview and source instead of inserting unattributed text into the draft.

The current preview affordance is for shared-file snapshots with a saved source entry ID. Upload-only and live-workspace previews without that source entry do not invent one; their text remains selectable/copyable. This does not grant arbitrary file access or prove that a rendered selection is a verbatim file substring.

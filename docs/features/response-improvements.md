# Conversation improvements

Deliver improvements as feature batches, one commit per batch. Each completed batch is committed and pushed without waiting for intermediate Actions runs, as requested. Review the latest CI results after the implementation batches and repair failures before claiming validation. The final build completes packaging. Versions continue to follow upstream. No dock, floating widget, account requirement or analytics is added.

| Batch | Scope | State |
| --- | --- | --- |
| 1 | Existing text preview: Markdown Read/Source, copy, wrap, retry, clear errors; desktop, phones and terminal | Desktop, iOS and host checks passed; intermediate packaging cancelled by request |
| 2 | Extend the existing Library with Files filtering/counts, text previews, source labels and retry; retain downloads | Implemented; CI pending |
| 3 | Phone message selection/reply parity through existing metadata | Implemented; CI pending |
| 3b | Structured file excerpts and collaboration selections with source context | Planned |
| 4 | Bounded PDF/audio/video preview adapters using existing authenticated transfers | Planned |
| 5 | Read-only CSV/TSV tables; assess XLSX separately with decompression/cell limits | Planned |
| 6 | Read-only workspace directory navigation within host-authorized roots | Planned |
| 7 | Edit queued messages and waiting reasons; host-owned reorder/pause where supported | Planned |
| 8 | Session-bound reported context usage in bot details; reuse existing compaction state | Implemented; CI pending |
| 9 | Inline math and diagram expansion/zoom with existing rendering bundles | Planned |
| 10 | Incremental Markdown presentation and adapter-specific streaming finality | Planned |
| 11 | Citation/source navigation using existing reports and supplied source metadata | Planned |

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

Desktop already uses this structured message-selection contract. Terminal selection still uses its existing source/quote flow; this batch adds no terminal-native selection interception. File excerpts and collaboration-sidechat selections remain a separate batch. Existing phone sidechat selection continues its previous draft behavior until that batch supplies a validated source-part contract.

On iPhone a selected reply uses normal authenticated delivery instead of the older text-only mailbox contract, which cannot preserve the excerpt. An offline failure stays retryable with its reply metadata; the passage is never silently downgraded to a whole-message reply by the client. Android's pending-send body already travels intact through its delivery path and now includes selection metadata.

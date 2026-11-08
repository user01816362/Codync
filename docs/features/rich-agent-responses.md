# Rich agent responses

For the current upstream library and security assessment, see [rich response research](rich-response-research.md).

Codync's host owns conversations and sends durable entries to the desktop, phone and terminal clients. The clients render those entries using their own controls and the same semantics. `send_message` creates complete messages; agents can explicitly stream a user-facing message with `stream_message`. Ordinary ACP text chunks live in the trace. Do not treat a Markdown renderer as if it changes that delivery contract.

## Existing capabilities

Messages support Markdown and file cards. Agents can publish interactive HTML artifacts in a separate viewer, and the chat has real tool activity and bot collaboration views. An artifact may run scripts inside its isolated viewer; this is a different trust model from the chat's Markdown or future native inline components. See [chat messages](chat-messages.md), [artifacts](artifacts.md), and [bot collaboration](bot-collaboration.md).

## Direction

- Improve message readability incrementally on each client, retaining a useful terminal rendering. Never add network fetches or execute embedded HTML or scripts while rendering ordinary Markdown.
- Design a versioned, validated host-owned representation for optional structured content. Old clients must retain meaningful text, and unknown component types must degrade safely. Keep published responses in normal history so reconnect and offline use work.
- Start with read-only citations, tables and a small set of charts. The host should record source provenance supplied by an actual agent or tool; never invent links, research steps, or tool activity.
- Any interactive action must use explicit host capabilities and existing permissions. Treat all model and tool content as untrusted. Bound document size, depth, node count and update rates; avoid unbounded parsing, network loads and rendering work.
- Evaluate streaming separately from presentation. Define how partial text, cancellation, reconnect, errors and the completed `send_message` interact before enabling it. Avoid duplicate responses or exposing private reasoning.
- Preserve existing artifacts. Evaluate [MCP Apps](https://github.com/modelcontextprotocol/ext-apps) interoperability separately because sandboxed web apps have different capabilities and risks from a native component catalog.
- Evaluate [Streamdown](https://github.com/vercel/streamdown) for desktop streaming Markdown and [A2UI](https://github.com/a2ui-project/a2ui) as a protocol reference. Neither removes the need for native phone and terminal support. Compare current versions, licenses, accessibility, security and performance against Codync's implementation before adding dependencies.

## Initial increment (historical)

The existing fenced-code renderers understand nested longer fences, tildes and incomplete fences, retaining code text while stripping the opening indentation. Desktop, iPhone and Android expose a copy control and language label; the terminal continues to copy the whole selected message. That initial code-block change alone did not introduce streaming replies or structured content. Subsequent increments are described in the current coverage table below.

## Shared text previews

Small bot-shared and user-uploaded Markdown and source files open from their file cards in desktop side panels, phone sheets and the terminal. Bot-shared files use authenticated, integrity-checked snapshots; uploads use the authenticated upload path and size checks but have no recorded digest. The renderer does not run file content. Supported ACP v1 image chunks in a bot's own chat are published as immutable file snapshots and shown by the existing image viewer. The optional image URI is not fetched.

## Coverage audit against idea2

This is a status inventory, not a claim that the whole proposal has shipped.

| Requested capability | Current state | Remaining work |
|---|---|---|
| Markdown basics, tables, fenced code | Present in the graphical clients; terminal renders a subset. Code blocks have copy controls and a language label. | Highlighted source previews are implemented in the graphical clients. Markdown conformance and long-message performance still need cross-client interaction checks. |
| File and Markdown preview | Bot-shared and user-uploaded text files up to 1 MiB open in desktop/phone viewers and the terminal. Explicit local Markdown links can request a workspace text preview on Unix and Windows hosts; the terminal opens the first such link. | Workspace preview requires a user action and a control-scoped device; relative reads stay in the author's configured workspace; absolute links resolve to the most specific registered, nondeleted bot workspace, and symlinks are rejected. Windows final handle paths are checked against the canonical workspace; alternate data streams and device paths are rejected. Group-only workspaces are not supported. Plain path mentions are not links. Uploads lack a digest; Fenced source has an optional offline highlighted preview. |
| Generated images | An agent can publish an image with `send_file` or as an ACP v1 image chunk in its own chat; the host saves a snapshot and clients show supported images up to 20 MiB. | PNG, JPEG and WebP are fully decoded with 8192-pixel side and 128 MiB allocation limits. GIF logical screen dimensions are checked (16 million pixels); animated GIF frames are not fully decoded by the host. ACP frames are limited to 32 MiB before parsing and queued image data to 64 MiB; oversized input ends the connection with an explicit failure. Replayed image notifications are discarded during session/load because the durable snapshots already hold them. Non-image history notifications remain unbounded to avoid deadlocking session replay. Provider/model generation is not supplied by the renderer; group and delegated turns do not publish image chunks. |
| Live response streaming | An agent can opt in using `stream_message` to start, append, finish or cancel a user-facing reply. Updated clients show its provisional entry; older clients see the final message. ACP text chunks remain nonfinal trace entries. | Automatic streaming of generic ACP chunks is not implemented: they can be narration or duplicate a later message. Real-device reconnect, Stop and accessibility checks remain outstanding. |
| Math, Mermaid and syntax highlighting | Desktop, iOS and Android render fenced Markdown natively by default, with a compact header toggle between Code and Preview. Mermaid, math/LaTeX/TeX fences and standalone `$$…$$` paragraphs start in Preview; ordinary source starts in Code and offers highlighting in Preview. Terminal keeps the source. | Inline dollar math is not parsed. Renderers are isolated, have no app bridge or network access, and retain source on errors; device interaction checks remain pending. |
| Citations and research | Ordinary Markdown links and agent-authored structured reports with explicit source links render; tool traces show actual tool events. | Structured sections retain citation indexes and explicit source metadata alongside text. `research_progress` updates the actual working-details checklist; labels are agent-reported activity, not independent verification. Source provenance is not independently verified. |
| Structured cards, charts and forms | HTML artifacts support interactive documents in an isolated viewer. The host validates a versioned read-only report and renders sections as portable Markdown. Version 1 reports have native expandable sections, comparisons, timelines, data rows, bar charts and text-answer forms on desktop and phones; terminal and older clients get a readable fallback. | The catalog is bounded to 32 KiB, 24 sections and 30 items per section; forms have at most eight fields. Forms send ordinary user messages through the existing authorized outbox, grant no permission and execute no generated actions. No arbitrary native components or executable model code are accepted. |
| Tool and MCP activity | Existing expandable working details and full conversation show real activity; Sidechat shows bot exchanges. | Improve presentation only where the underlying events contain the information; do not infer private reasoning or fabricate progress. |
| Accessibility and performance | Existing components and platform conventions are reused. | Profile long responses and several active agents; test screen readers, keyboard access, reduced motion and small screens for each new component. |

Image publication reuses the host's immutable snapshots and `readFile` authorization; its pending writes settle or are cancelled before ending a turn. Immediate completion, slow-write cancellation, reconfiguration and device-level UI behavior still need end-to-end verification.

## Delivery and mentions

Group mentions accept a bot name, compact name, first name or exact `@bot-id`; the prompt gives exact IDs to disambiguate names. Fullwidth `＠` is normalized; email-like word interiors do not notify a bot. Outside a group, use `message_bot`: writing a mention in an ordinary reply does not queue another bot.

Busy group members keep their queued turn. A late reply can now continue its mentions within the remaining bounded handoffs, sharing seen cursors and admission budget with the room to prevent duplicate work. Unavailable recipients, timeouts and exhausted limits produce visible notices. A group turn admits at most ten member turns; Stop and a newer epoch suppress old continuations. No bot is required to produce words: `(pass)` is valid.

`message_status` exposes only the caller's own chat or rooms it belongs to. It accepts an entry ID or a request ID, or selects the latest own reply. User seen state is the existing client acknowledgement for an agent message revision, not proof of reading or comprehension; non-message entries return unknown. Bot request start timestamps survive failure/cancellation; queued, started, completed and failed remain distinct. Group status includes the last prompt-dispatch cursor and time for up to 50 members, recorded only after the prompt is written to the ACP transport. This does not imply comprehension or complete inclusion of older messages in a bounded prompt. Unknown/old receipts are never inferred. Recent request lookup is bounded to 200 entries.

## Offline renderer

The graphical clients ship identical pinned KaTeX 0.19.1, Mermaid 12.3.0 and highlight.js 11.12.0 source bundles. Refresh tooling, licenses and hashes are recorded in [renderer notices](rich-renderer-notices.md). No packages are installed by the refresh script. Content is passed as UTF-8 JSON encoded as Base64, never interpolated into executable source. A restrictive CSP blocks network, frames, forms and objects. Desktop additionally uses an opaque sandbox origin; native web views expose no app bridge or navigation. Mermaid configuration directives are rejected, strict security is enabled, and chart links are disabled. KaTeX uses untrusted mode, bounded expansion and MathML output. Browser previews accept at most 16,000 UTF-16 code units; original source remains selectable and copyable. Markdown previews use the existing native renderer, without loading a browser. Nested Markdown fences remain code to prevent recursive preview expansion. A small top-right Code/Preview control replaces the large bottom button. Desktop browser previews reuse the existing isolated artifact protocol: `srcdoc` inherits the app CSP and would block the renderer’s scripts. The app CSP is preserved. CI reproduces that parent policy with an intercepted standalone preview origin and asserts successful code, math and diagram rendering without external requests.

## Validation status

The Build workflow retains macOS desktop checks, the iOS package build and host checks. Windows retains its separate workflow. Android source remains in the repository, but its workflow is manually disabled at the maintainer’s request; do not describe Android as CI-validated by Build. An offline browser rendering check runs in Build; no new test files were added. Local builds, dependency installation and simulator/device checks were not run at the user's request. Compilation and CI results must be reported from the resulting commit; this document does not label unrun interaction, accessibility or performance checks as passed.


## Long task titles

The compact working indicator constrains the grid and every nested flex row to the available chat width. Its activity label ellipsizes while the timer and disclosure control remain visible; hovering the desktop activity label reveals its complete text. Expanded tool titles retain their full text and wrap long paths on desktop and phones. The terminal already wraps tool text. The desktop behavior follows [flex minimum sizing](https://developer.mozilla.org/en-US/docs/Web/CSS/min-width) and [overflow wrapping](https://developer.mozilla.org/en-US/docs/Web/CSS/overflow-wrap). Build checks compact and expanded layouts at 220, 480 and 900 pixels using a deliberately long unbroken path.


Renderer controls follow the existing app style and the block-level control pattern documented by [Streamdown](https://github.com/vercel/streamdown/blob/main/apps/website/content/docs/plugins/mermaid.mdx). This reuses Codync's native Markdown implementation and pinned offline libraries rather than adding a second React-only Markdown framework. ChatGPT and Claude production chat source is not claimed as an adopted dependency.


## Forwarded workspace links

A Control-authorized client may open an absolute text-file link that another bot forwarded from a registered bot workspace. The host selects the most specific matching registered workspace; relative links remain scoped to the author. There is no basename search, disk-wide access or new permission prompt. Deleted bots and groups supply no preview roots. Paths outside all registered workspaces remain blocked and should be shared explicitly with `send_file`. Existing traversal, symlink, regular-file, UTF-8 and 1 MiB limits still apply. Desktop and phone workspace previews display the host error so an unavailable file can be distinguished from a rendering failure. Additional folder grants are not implemented.

## Replying to selected text

Desktop **Quote selection** now uses the existing reply bar instead of inserting a Markdown
blockquote into the draft. It displays **Quoted from [author]** and the selected passage;
ordinary replies display **Replying to [author]**. The host resolves the original message ID
and author within the chat and stores the user-selected passage, bounded to 4,000 characters.
Collaboration quotes identify the request or reply. This is a user-selected excerpt, not
independently verified source provenance: rendered Markdown can differ from its raw text.
Retries retain that metadata, and the agent receives the message ID and selected passage.

The iPhone and Android selection sheets still insert a text quote into the draft; their
selected-text quotes do not yet use desktop's structured reply metadata. Normal replies are
supported. The terminal's quote action also produces draft text. See [chat messages](chat-messages.md).

## Release and validation boundaries

The source version follows upstream; release tags add a build ID to the Git tag so fixes at
the same app version can be discovered. Ad-hoc Mac builds check this repository and offer a DMG
download, but do not install automatically. See [updates](../../FORK.md#updates).
CI status belongs to a particular commit, not this inventory. A successful build does not
establish real-device interaction, accessibility, provider behavior or Windows runtime validation.

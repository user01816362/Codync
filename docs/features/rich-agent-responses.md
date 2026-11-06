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

## First increment

The existing fenced-code renderers understand nested longer fences, tildes and incomplete fences, retaining code text while stripping the opening indentation. Desktop, iPhone and Android expose a copy control and language label; the terminal continues to copy the whole selected message. This does not introduce streaming final messages, syntax highlighting, citations, charts, or generated UI.

## Shared text previews

Small bot-shared and user-uploaded Markdown and source files open from their file cards in desktop side panels, phone sheets and the terminal. Bot-shared files use authenticated, integrity-checked snapshots; uploads use the authenticated upload path and size checks but have no recorded digest. The renderer does not run file content. Supported ACP v1 image chunks in a bot's own chat are published as immutable file snapshots and shown by the existing image viewer. The optional image URI is not fetched.

## Coverage audit against idea2

This is a status inventory, not a claim that the whole proposal has shipped.

| Requested capability | Current state | Remaining work |
|---|---|---|
| Markdown basics, tables, fenced code | Present in the graphical clients; terminal renders a subset. Code blocks have copy controls and a language label. | Conformance, highlighting and long-message performance need measurement and cross-client fixtures. |
| File and Markdown preview | Bot-shared and user-uploaded text files up to 1 MiB open in desktop/phone viewers and the terminal. Explicit local Markdown links can request a workspace text preview on a Unix host; the terminal opens the first such link. | Workspace preview requires a user action and a control-scoped device; reads are confined to the author's configured bot workspace, and symlinks are rejected. Windows hosts and group-only workspaces are not supported. Plain path mentions are not links. Uploads lack a digest; syntax highlighting is not implemented. |
| Generated images | An agent can publish an image with `send_file` or as an ACP v1 image chunk in its own chat; the host saves a snapshot and clients show supported images up to 20 MiB. | Image signatures are checked but not fully decoded or dimension-limited by the host. ACP's existing input queue is unbounded before the image limit is applied. Provider/model generation is not supplied by the renderer; group and delegated turns do not publish image chunks. |
| Live response streaming | An agent can opt in using `stream_message` to start, append, finish or cancel a user-facing reply. Updated clients show its provisional entry; older clients see the final message. ACP text chunks remain nonfinal trace entries. | Automatic streaming of generic ACP chunks is not implemented: they can be narration or duplicate a later message. Real-device reconnect, Stop and accessibility checks remain outstanding. |
| Math, Mermaid and syntax highlighting | No dedicated cross-client rendering was found in the chat Markdown renderers. | Evaluate maintained implementations and native/terminal fallbacks; never run generated scripts as Markdown. |
| Citations and research | Ordinary Markdown links and agent-authored structured reports with explicit source links render; tool traces show actual tool events. | Source provenance is not independently verified, and research-specific progress is not implemented. Only display events actually provided by an agent or tool. |
| Structured cards, charts and forms | HTML artifacts support interactive documents in an isolated viewer. The host validates a versioned read-only report and renders sections as portable Markdown. No approved native inline registry is implemented. | Specify and validate a native component catalog, bound payload sizes and complexity, define text fallbacks, and implement native renderers. Forms/actions require explicit permission and host capability checks. |
| Tool and MCP activity | Existing expandable working details and full conversation show real activity; Sidechat shows bot exchanges. | Improve presentation only where the underlying events contain the information; do not infer private reasoning or fabricate progress. |
| Accessibility and performance | Existing components and platform conventions are reused. | Profile long responses and several active agents; test screen readers, keyboard access, reduced motion and small screens for each new component. |

Image publication reuses the host's immutable snapshots and `readFile` authorization; its pending writes settle or are cancelled before ending a turn. Immediate completion, slow-write cancellation, reconfiguration and device-level UI behavior still need end-to-end verification.

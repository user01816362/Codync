# Rich agent responses

Codync's host owns conversations and sends durable entries to the desktop, phone and terminal clients. The clients render those entries using their own controls and the same semantics. `send_message` currently creates complete messages; agent text chunks live in the trace. Do not treat a Markdown renderer as if it changes that delivery contract.

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

Small bot-shared Markdown and source files open from their file cards in desktop side panels and phone sheets. They use the existing authenticated, verified file snapshot; the renderer does not run file content. Images already use the shared image viewer. ACP message chunks currently extract text, so directly returned ACP image content is not yet published automatically; agents can explicitly send an image file with `send_file` today.

## Coverage audit against idea2

This is a status inventory, not a claim that the whole proposal has shipped.

| Requested capability | Current state | Remaining work |
|---|---|---|
| Markdown basics, tables, fenced code | Present in the graphical clients; terminal renders a subset. Code blocks have copy controls and a language label. | Conformance, highlighting and long-message performance need measurement and cross-client fixtures. |
| File and Markdown preview | Bot-shared text snapshots up to 1 MiB open in desktop/phone viewers and in the terminal; other files download. | Previewing user-uploaded non-image files, arbitrary workspace paths, and syntax highlighting are not implemented. A path mentioned in chat is not itself an authorized shared file. |
| Generated images | An agent can publish an image file with `send_file`; clients show supported images up to 20 MiB. | ACP image content in an `agent_message_chunk` is currently discarded by `content_text`. Preserve supported image blocks as verified snapshots, with size limits and correct turn/cancellation behavior, before claiming direct image support. Provider/model generation is not supplied by the renderer. |
| Live response streaming | ACP text chunks are saved as nonfinal trace entries; normal chat bubbles are final messages from `send_message` or the turn's fallback. | Design a separate streaming contract covering duplicate suppression, reconnection, Stop, offline history and older clients. |
| Math, Mermaid and syntax highlighting | No dedicated cross-client rendering was found in the chat Markdown renderers. | Evaluate maintained implementations and native/terminal fallbacks; never run generated scripts as Markdown. |
| Citations and research | Ordinary Markdown links render; tool traces show actual tool events. | Structured source records, provenance, research-specific progress and cited final reports are not implemented. Only display events actually provided by an agent or tool. |
| Structured cards, charts and forms | HTML artifacts support interactive documents in an isolated viewer. No approved native inline registry is implemented. | Specify and validate a versioned host-owned component catalog, bound payload sizes and complexity, define text fallbacks, and implement native renderers. Forms/actions require explicit permission and host capability checks. |
| Tool and MCP activity | Existing expandable working details and full conversation show real activity; Sidechat shows bot exchanges. | Improve presentation only where the underlying events contain the information; do not infer private reasoning or fabricate progress. |
| Accessibility and performance | Existing components and platform conventions are reused. | Profile long responses and several active agents; test screen readers, keyboard access, reduced motion and small screens for each new component. |

The next high-impact addition is direct image-block delivery, followed by a versioned read-only content catalog and source provenance. Image publication must reuse the host's immutable file snapshots and `readFile` authorization, reject malformed/oversized payloads, avoid storing base64 in entries, and settle or cancel pending writes safely before ending a turn. `FileShares` currently cancels pending writes at turn completion, so simply launching an asynchronous image copy from `on_update` would lose fast responses; its lifecycle must be addressed and tested first.

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

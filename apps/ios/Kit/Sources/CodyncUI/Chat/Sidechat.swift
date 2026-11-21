import CodyncKit
import SwiftUI

/// A compact entry point for the real bot exchanges already in this chat.
struct SidechatIndicator: View {
    let botId: String
    let open: () -> Void
    @AppStorage("collaborationActivityMode") private var mode = "compact"
    @Environment(BotStore.self) private var model

    private var exchanges: [BotExchange] { BotExchange.all(fetched: model.collaborationUpdates[botId] ?? [], live: model.chat(botId)) }

    var body: some View {
        let exchanges = exchanges
        let active = Set(exchanges.filter { $0.outcome == .pending }.map(\.peerId)).count
        if mode != "hidden" {
            Button(action: open) {
                HStack(spacing: 7) {
                    Image(systemName: "person.2")
                    Text(active > 0
                         ? L10n.tr("{count} bots collaborating").replacingOccurrences(of: "{count}", with: "\(active + 1)")
                         : L10n.tr("Collaboration activity"))
                    Image(systemName: "chevron.up").font(.caption2)
                }
                .font(.footnote.weight(.medium))
                .foregroundStyle(Palette.secondary)
                .padding(.horizontal, 12).padding(.vertical, 7)
                .background(Palette.surface, in: Capsule())
            }
            .buttonStyle(PressScale())
            .accessibilityHint("Opens bot collaboration activity")
        }
    }
}

/// A read-only timeline; fetching older exchanges never inserts them into the chat mirror.
struct SidechatView: View {
    let botId: String
    @Environment(BotStore.self) private var model
    @State private var fetched: [Entry] = []
    @State private var loadError: String?

    private var exchanges: [BotExchange] { BotExchange.all(fetched: fetched, live: model.chat(botId) + (model.collaborationUpdates[botId] ?? [])) }

    var body: some View {
        VStack(spacing: 0) {
            ModalHeader("Collaboration")
            ScrollView {
                LazyVStack(alignment: .leading, spacing: 20) {
                    if let loadError {
                        Text(loadError).font(.footnote).foregroundStyle(Palette.danger)
                    }
                    ForEach(BotCollaboration.group(exchanges)) { group in
                        VStack(alignment: .leading, spacing: 16) {
                            if let first = group.exchanges.first {
                                HStack {
                                    Text(L10n.tr("Started by {name}").replacingOccurrences(of: "{name}", with: model.authorName(first.source, fallback: first.source == first.peerId ? first.peerName : nil)))
                                    Spacer()
                                    Text(first.date, style: .date)
                                }
                                .font(.footnote.weight(.medium)).foregroundStyle(Palette.tertiary)
                            }
                            ForEach(group.exchanges) { exchange in SidechatExchange(exchange: exchange) }
                        }
                    }
                }
                .padding(.horizontal, 20).padding(.vertical, 16)
            }
            .defaultScrollAnchor(.bottom)
        }
        .background(Palette.background)
        .onChange(of: model.chatClearGenerations[botId] ?? 0) { _, _ in
            fetched = []
            loadError = nil
        }
        .task(id: botId) {
            let generation = model.chatClearGenerations[botId] ?? 0
            do {
                let entries = try await model.botExchanges(botId)
                guard !Task.isCancelled, generation == (model.chatClearGenerations[botId] ?? 0) else { return }
                fetched = entries
            } catch {
                guard !Task.isCancelled, generation == (model.chatClearGenerations[botId] ?? 0) else { return }
                loadError = L10n.tr("Couldn't load earlier collaboration activity.")
            }
        }
    }

}

struct SidechatExchange: View {
    let exchange: BotExchange
    @State private var expanded = true
    @State private var selection: QuoteSelectionTarget?
    @Environment(BotStore.self) private var model
    private func name(_ id: String, exchange: BotExchange) -> String {
        model.authorName(id, fallback: id == exchange.peerId ? exchange.peerName : nil)
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 7) {
            Button { withAnimation { expanded.toggle() } } label: {
            HStack(spacing: 6) {
                if let bot = model.bots[exchange.source] { CharacterAvatar(bot: bot, size: 20, animated: false) }
                Text(name(exchange.source, exchange: exchange)).fontWeight(.semibold)
                Image(systemName: "arrow.right").foregroundStyle(Palette.tertiary)
                if let bot = model.bots[exchange.target] { CharacterAvatar(bot: bot, size: 20, animated: false) }
                Text(name(exchange.target, exchange: exchange)).fontWeight(.semibold)
                Spacer(minLength: 4)
                Text(exchange.date, style: .time).foregroundStyle(Palette.tertiary)
                Image(systemName: expanded ? "chevron.down" : "chevron.right").font(.caption2)
            }
            .font(.footnote)
            }
            .buttonStyle(.plain)
            .accessibilityValue(expanded ? "Expanded" : "Collapsed")
            if expanded {
            Text(exchange.text).font(.subheadline).foregroundStyle(Palette.text).textSelection(.enabled)
                .contextActions { [MenuItem("Select text / Quote", icon: "arrowshape.turn.up.left") {
                    selection = QuoteSelectionTarget(text: exchange.text, author: name(exchange.source, exchange: exchange), part: "request")
                }] }
            if let reply = exchange.reply, !reply.isEmpty {
                Text(L10n.tr("Reply from {name}").replacingOccurrences(of: "{name}", with: name(exchange.target, exchange: exchange)))
                    .font(.footnote.weight(.medium)).foregroundStyle(Palette.secondary)
                Text(reply).font(.subheadline).foregroundStyle(Palette.secondary).textSelection(.enabled)
                    .contextActions { [MenuItem("Select text / Quote", icon: "arrowshape.turn.up.left") {
                        selection = QuoteSelectionTarget(text: reply, author: name(exchange.target, exchange: exchange), part: "reply")
                    }] }
            }
            }
            switch exchange.outcome {
            case .pending:
                Text(L10n.tr("Waiting for a reply…")).font(.footnote).foregroundStyle(Palette.secondary)
            case .failed(let detail):
                Text(detail.isEmpty ? L10n.tr("Didn't complete.") : detail).font(.footnote).foregroundStyle(Palette.danger)
            case .done:
                Text(L10n.tr("Completed")).font(.footnote).foregroundStyle(Palette.tertiary)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .codyncSheet(item: $selection) { target in
            QuoteSelectionSheet(text: target.text, author: target.author, structured: true) {
                model.selectReply(exchange, text: $0, part: target.part ?? "request")
            }
        }
    }
}

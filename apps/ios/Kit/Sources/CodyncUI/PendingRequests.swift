import CodyncKit
import SwiftUI

/// One compact entry point for approvals in other conversations, with no automatic expansion or bulk grant.
public struct PendingRequests: View {
    @Environment(AccountStore.self) private var accounts
    @State private var showing = false
    @State private var selected: String?

    public init() {}

    private struct Request: Identifiable {
        let store: BotStore
        let entry: Entry
        let id: String
        let author: String
        let chat: String
    }

    private var requests: [Request] {
        var result: [Request] = []
        for store in accounts.stores.values {
            for entries in store.entries.values {
                for entry in entries where entry.kind == "permission" && entry.data.status == "pending" && entry.data.removed != true {
                    if accounts.selection?.computerId == store.computer.id && accounts.selection?.botId == entry.botId { continue }
                    result.append(Request(store: store, entry: entry,
                        id: "\(store.computer.id)/\(entry.id)",
                        author: store.bots[entry.data.author ?? entry.botId]?.name ?? "Agent",
                        chat: store.bots[entry.botId]?.name ?? "Conversation"))
                }
            }
        }
        return result.sorted { a, b in a.entry.createdAt == b.entry.createdAt ? a.id < b.id : a.entry.createdAt < b.entry.createdAt }
    }

    public var body: some View {
        let items = requests
        HStack {
            if !items.isEmpty {
                Spacer(minLength: 0)
                Button {
                    selected = items.first?.id
                    Motion.animate { showing = true }
                } label: {
                    Label("\(L10n.tr("Requests")) · \(items.count)", systemImage: "hand.raised")
                        .font(.callout)
                }
                .buttonStyle(.secondary)
                .accessibilityLabel("\(L10n.tr("Review requests")) · \(items.count)")
                .padding(.horizontal, 16).padding(.vertical, 6)
            }
        }
        .codyncSheet(isPresented: $showing) {
            VStack(spacing: 0) {
                ModalHeader("Requests")
                ScrollView {
                    VStack(alignment: .leading, spacing: 16) {
                        if let current = items.first(where: { $0.id == selected }) {
                            ChoicePicker(selection: Binding(get: { selected ?? "" }, set: { selected = $0 }),
                                options: items.map { ($0.id, "\($0.author) · \($0.chat) · \($0.store.hostName)") }, fitsAvailableWidth: true)
                            Text(current.store.hostName).font(.caption).foregroundStyle(Palette.secondary)
                            PermissionCard(entry: current.entry, hostName: current.store.hostName,
                                answering: current.store.connection == .online ? current.store.answering[current.entry.id] : "offline") {
                                    current.store.respond(current.entry, option: $0)
                                }
                                .id(current.id)
                            if current.store.connection != .online {
                                Text(L10n.tr("Reconnect to this computer before responding.")).font(.footnote).foregroundStyle(Palette.warning)
                            }
                            if let error = current.store.lastError { Text(error).font(.footnote).foregroundStyle(Palette.danger) }
                            Button(L10n.tr("Open chat")) {
                                accounts.selection = BotReference(accountId: accounts.accountId, computerId: current.store.computer.id, botId: current.entry.botId)
                                showing = false
                            }.buttonStyle(.secondary)
                        } else {
                            Text(L10n.tr(items.isEmpty ? "No requests waiting." : "This request is no longer waiting."))
                            if !items.isEmpty {
                                Button(L10n.tr("Review next request")) { selected = items.first?.id }.buttonStyle(.primary)
                            }
                        }
                        Text(L10n.tr("Each approval applies to the selected request. Always allow uses the agent’s supplied permission scope."))
                            .font(.caption).foregroundStyle(Palette.secondary)
                    }.padding(16)
                }
            }
        }
    }
}

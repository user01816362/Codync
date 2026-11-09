import CodyncKit
import SwiftUI

struct GroupInbox: View {
    let groupId: String
    @Environment(BotStore.self) private var model
    @State private var items: [MailItem]?
    @State private var failure: String?
    @State private var attempt = 0
    @State private var busy: String?
    @State private var handlers: [String: String] = [:]
    private var members: [Bot] { (model.bots[groupId]?.members ?? []).compactMap { model.bots[$0] } }

    var body: some View {
        VStack(spacing: 0) {
            ModalHeader("Group inbox") {
                IconButton("Refresh inbox", systemImage: "arrow.clockwise") { attempt += 1 }.disabled(busy != nil)
            }
            ScrollView {
                VStack(alignment: .leading, spacing: 20) {
                    Text(L10n.tr("Approve an external request and choose who handles it. The sender receives only that handler’s reply, with no access to this conversation."))
                        .font(.caption).foregroundStyle(Palette.secondary)
                    if let failure { Text(failure).foregroundStyle(Palette.danger).textSelection(.enabled) }
                    if let items {
                        if items.isEmpty { Text(L10n.tr("No group mail.")).foregroundStyle(Palette.secondary) }
                        ForEach(items) { item in mailCard(item) }
                        if items.count == 100 { Text(L10n.tr("Showing up to 100 requests, pending first.")).font(.caption) }
                    } else if failure == nil { Spinner() }
                }.padding(20).frame(maxWidth: .infinity, alignment: .leading)
            }
        }
        .background(Palette.background)
        .task(id: "\(groupId):\(attempt)") {
            failure = nil
            guard let client = model.live else { failure = L10n.tr("Computer is offline"); return }
            do {
                let result: Inbox = try await client.call("groupInbox", ["groupId": groupId])
                if !Task.isCancelled { items = result.items }
            } catch { if !Task.isCancelled { failure = error.localizedDescription } }
        }
    }

    private func mailCard(_ item: MailItem) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(item.mail.subject).font(.subheadline.weight(.semibold))
            Text(L10n.tr("From") + " " + item.from).font(.caption).foregroundStyle(Palette.secondary)
            Text(item.mail.text).textSelection(.enabled)
            Text(L10n.tr(item.replied ? "Replied" : item.delivered ? "Delivered to handler" : item.mail.status == "approved" ? "Approved · delivery pending" : item.mail.status == "rejected" ? "Rejected" : "Awaiting your approval"))
                .font(.caption).foregroundStyle(Palette.secondary)
            if item.mail.status == "pending" {
                Picker("Handler", selection: Binding(get: { handlers[item.id] ?? members.first?.id ?? "" }, set: { handlers[item.id] = $0 })) {
                    ForEach(members) { Text($0.name).tag($0.id) }
                }.disabled(busy != nil)
                HStack {
                    Button("Approve delivery") { decide(item, approve: true) }.disabled(busy != nil || members.isEmpty)
                    Button("Reject", role: .destructive) { decide(item, approve: false) }.disabled(busy != nil)
                }.buttonStyle(SecondaryButtonStyle())
            } else if item.mail.status == "approved" && !item.delivered {
                Button("Retry approved delivery") { decide(item, approve: true) }
                    .buttonStyle(SecondaryButtonStyle()).disabled(busy != nil)
            }
        }.frame(maxWidth: .infinity, alignment: .leading)
    }

    private func decide(_ item: MailItem, approve: Bool) {
        guard busy == nil, let client = model.live else { return }
        busy = item.id
        failure = nil
        Task {
            defer { busy = nil }
            do {
                let _: Decision = try await client.call("decideGroupMail", Approval(mailId: item.id, approve: approve, handler: handlers[item.id] ?? members.first?.id))
                attempt += 1
            } catch { failure = error.localizedDescription }
        }
    }

    private struct Approval: Encodable { let mailId: String; let approve: Bool; let handler: String? }
    private struct Decision: Decodable { let status: String }
    private struct Inbox: Decodable { let items: [MailItem] }
    private struct MailItem: Decodable, Identifiable {
        let id: String
        let from: String
        let delivered: Bool
        let replied: Bool
        let mail: Mail
    }
    private struct Mail: Decodable { let subject: String; let text: String; let status: String }
}

import CodyncKit
import SwiftUI

struct GroupInbox: View {
    let groupId: String
    @Environment(BotStore.self) private var model
    @State private var policy = Policy(autoApprove: false, handler: "")
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
                    Toggle("Automatically approve incoming mail", isOn: Binding(get: { policy.autoApprove }, set: { enabled in
                        savePolicy(Policy(autoApprove: enabled, handler: members.contains(where: { $0.id == policy.handler }) ? policy.handler : members.first?.id ?? ""))
                    })).toggleStyle(.codync).disabled(items == nil || busy != nil || members.isEmpty)
                    Text("New requests go directly to the selected bot. Existing pending requests still need approval. If the handler leaves, incoming requests wait for approval.").font(.caption).foregroundStyle(Palette.secondary)
                    VStack(alignment: .leading, spacing: 8) {
                        Text("Automatic mail handler").font(.caption).foregroundStyle(Palette.secondary)
                        ChoicePicker(selection: Binding(get: { policy.handler }, set: { savePolicy(Policy(autoApprove: policy.autoApprove, handler: $0)) }), options: members.map { (id: $0.id, label: $0.name) })
                            .disabled(items == nil || busy != nil)
                    }
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
                if !Task.isCancelled { items = result.items; policy = result.policy ?? Policy(autoApprove: false, handler: "") }
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
                ChoicePicker(selection: Binding(get: { handler(for: item.id) }, set: { handlers[item.id] = $0 }), options: members.map { (id: $0.id, label: $0.name) })
                    .disabled(busy != nil)
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

    private func handler(for id: String) -> String {
        if let chosen = handlers[id], members.contains(where: { $0.id == chosen }) { return chosen }
        return members.first?.id ?? ""
    }

    private func decide(_ item: MailItem, approve: Bool) {
        guard busy == nil, let client = model.live else { return }
        busy = item.id
        failure = nil
        Task {
            defer { busy = nil }
            do {
                let _: Decision = try await client.call("decideGroupMail", Approval(mailId: item.id, approve: approve, handler: handler(for: item.id)))
                attempt += 1
            } catch { failure = error.localizedDescription }
        }
    }

    private func savePolicy(_ next: Policy) {
        guard busy == nil, let client = model.live else { return }
        busy = "policy"
        failure = nil
        Task {
            defer { busy = nil }
            do {
                let saved: Policy = try await client.call("setGroupInboxPolicy", PolicyUpdate(groupId: groupId, autoApprove: next.autoApprove, handler: next.handler))
                policy = saved
            } catch { failure = error.localizedDescription }
        }
    }

    private struct Policy: Codable { let autoApprove: Bool; let handler: String }
    private struct PolicyUpdate: Encodable { let groupId: String; let autoApprove: Bool; let handler: String }
    private struct Approval: Encodable { let mailId: String; let approve: Bool; let handler: String? }
    private struct Decision: Decodable { let status: String }
    private struct Inbox: Decodable { let items: [MailItem]; let policy: Policy? }
    private struct MailItem: Decodable, Identifiable {
        let id: String
        let from: String
        let delivered: Bool
        let replied: Bool
        let mail: Mail
    }
    private struct Mail: Decodable { let subject: String; let text: String; let status: String }
}

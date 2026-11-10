import CodyncKit
import SwiftUI

/// Meeting controls use host state and the same cards as group profiles.
struct MeetingRoom: View {
    @Environment(BotStore.self) private var model
    let groupId: String
    @State private var room: Snapshot?
    @State private var name = ""
    @State private var agenda = ""
    @State private var kind = "consultation"
    @State private var selected: [Delegate] = []
    @State private var chair = ""
    @State private var proposal = ""
    @State private var busy = false
    @State private var notice: String?
    @State private var failure: String?
    @State private var attempt = 0
    private var candidates: [Bot] {
        model.bots.values.filter { bot in !bot.isGroup && (!bot.hidden || selected.contains(where: { $0.botId == bot.id })) }.sorted { $0.name.localizedStandardCompare($1.name) == .orderedAscending }
    }
    private var group: Bot? { model.bots[groupId] }

    var body: some View {
        VStack(spacing: 0) {
            ModalHeader(room == nil ? "New meeting room" : "Meeting room")
            ScrollView {
                VStack(alignment: .leading, spacing: 24) {
                    Text("Participants keep their existing permissions and shared agent-session history. Share only material intended for this room.")
                        .font(.caption).foregroundStyle(Palette.secondary)
                    if let notice { Text(notice).font(.caption).foregroundStyle(Palette.secondary) }
                    if let failure { Text(failure).foregroundStyle(Palette.danger).textSelection(.enabled) }
                    if let room {
                        CardSection("\(room.meeting.kind) · \(room.meeting.status)") {
                            MarkdownText(room.meeting.agenda)
                            Text("\(room.meeting.spent)/\(room.meeting.limit) discussion turns used").foregroundStyle(Palette.secondary)
                        }
                        CardSection("Proposal · revision \(room.meeting.proposalRevision)") {
                            MarkdownText(room.meeting.proposal.isEmpty ? "No proposal recorded." : room.meeting.proposal)
                            ForEach(room.meeting.delegates, id: \.botId) { delegate in
                                VStack(alignment: .leading, spacing: 4) {
                                    Text("\(delegate.name ?? model.bots[delegate.botId]?.name ?? "Delegate") · \(delegate.department ?? "Personal")")
                                    Text(room.meeting.positions[delegate.botId].map { "\($0.position) · \($0.reason)" } ?? "Not answered")
                                        .foregroundStyle(Palette.secondary)
                                }
                            }
                        }
                        if ["paused", "running", "awaiting_owner"].contains(room.meeting.status) {
                            CardSection("Owner proposal") {
                                TextField("Proposal", text: $proposal, axis: .vertical).lineLimit(2...8).fieldBox()
                                Button("Publish proposal and reset responses") { act("propose") }.disabled(proposal.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                            }
                        }
                        if let history = room.meeting.history, !history.isEmpty {
                            CardSection("Previous proposals") {
                                DisclosureGroup("\(history.count) recorded versions") {
                                    ForEach(history, id: \.proposalRevision) { version in
                                        VStack(alignment: .leading, spacing: 8) {
                                            Text("Revision \(version.proposalRevision) · \(version.status)").font(.headline)
                                            MarkdownText(version.proposal)
                                            ForEach(room.meeting.delegates, id: \.botId) { delegate in
                                                Text("\(delegate.name ?? "Delegate"): \(version.positions[delegate.botId]?.position ?? "Not answered")").font(.caption)
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        CardSection("Room actions") {
                            if ["paused", "unresolved"].contains(room.meeting.status) {
                                Button("Start / Resume") { act("resume") }.disabled(room.meeting.spent >= room.meeting.limit)
                            }
                            if room.meeting.status == "running" { Button("Pause") { act("pause") } }
                            if room.meeting.kind != "decision", !["completed", "resolved", "archived", "cancelled"].contains(room.meeting.status) { Button("Complete with summary") { act("finish") }.disabled(proposal.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty) }
                            if room.meeting.status == "awaiting_owner" { Button("Approve final decision") { act("approve") } }
                            if ["resolved", "completed", "unresolved", "cancelled"].contains(room.meeting.status) { Button("Publish outcome to departments") { act("report") } }
                            Button(room.meeting.status == "archived" ? "Reopen" : "Archive") { act(room.meeting.status == "archived" ? "reopen" : "archive") }
                            if !["resolved", "completed", "archived", "cancelled"].contains(room.meeting.status) { Button("Cancel meeting", role: .destructive) { act("cancel") } }
                        }
                    } else if group?.meeting != true {
                        creation
                    } else if failure == nil { Spinner() }
                }.padding(14).disabled(busy || model.isOffline)
            }
        }
        .background(Palette.background)
        .task(id: "\(groupId):\(attempt):\(model.entries[room?.groupId ?? groupId]?.first(where: { $0.id == "meeting:\(room?.groupId ?? groupId)" })?.rev ?? 0)") {
            if group?.meeting == true || room != nil { await load() }
            else if name.isEmpty {
                name = String("\(group?.name ?? "Team") meeting".prefix(40))
                agenda = group?.description ?? ""
                selected = Array((group?.members ?? []).prefix(5)).map { Delegate(botId: $0, groupId: groupId) }
                chair = selected.first?.botId ?? ""
            }
        }
    }

    private var creation: some View {
        VStack(alignment: .leading, spacing: 24) {
            CardSection("Room") {
                Field("Name") { TextField("Name", text: $name).fieldBox() }
                Field("Agenda") { TextField("Agenda", text: $agenda, axis: .vertical).lineLimit(3...8).fieldBox() }
                ChoicePicker(selection: $kind, options: ["consultation", "meeting", "decision"].map { ($0, $0) })
            }
            CardSection("Delegates · \(selected.count)/5") {
                ForEach(candidates) { bot in
                    VStack(alignment: .leading, spacing: 8) {
                        Button {
                            if selected.contains(where: { $0.botId == bot.id }) { selected.removeAll { $0.botId == bot.id } }
                            else if selected.count < 5 { selected.append(Delegate(botId: bot.id)) }
                        } label: {
                            HStack { Text(bot.name); Spacer(); Image(systemName: selected.contains(where: { $0.botId == bot.id }) ? "checkmark.circle.fill" : "circle") }
                        }.buttonStyle(.plain)
                        if let index = selected.firstIndex(where: { $0.botId == bot.id }) {
                            ChoicePicker(selection: Binding(get: { selected.first(where: { $0.botId == bot.id })?.groupId ?? "" }, set: { value in
                                if let current = selected.firstIndex(where: { $0.botId == bot.id }) { selected[current].groupId = value.isEmpty ? nil : value }
                            }), options: [(id: "", label: "Personal")] + model.roster.filter { $0.isGroup && !$0.meeting && $0.members.contains(bot.id) }.map { ($0.id, $0.name) })
                            Text(selected[index].groupId.flatMap { model.bots[$0]?.name } ?? "Personal").font(.caption).foregroundStyle(Palette.secondary)
                        }
                    }
                }
            }
            CardSection("Chair") { ChoicePicker(selection: $chair, options: selected.map { ($0.botId, model.bots[$0.botId]?.name ?? "Delegate") }) }
            Text("Creating the room approves these invitations. Ten turns and ten minutes maximum; all delegates must approve the current proposal before final user approval. Starts paused.").font(.caption).foregroundStyle(Palette.secondary)
            Button("Create room and approve invitations") { create() }.buttonStyle(.primary)
                .disabled(selected.count < 2 || !selected.contains(where: { $0.botId == chair }) || name.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty || agenda.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
        }
    }

    private func load() async {
        guard let client = model.live else { return }
        do { let saved: Snapshot = try await client.call("meetingState", ["groupId": room?.groupId ?? groupId]); if !Task.isCancelled { room = saved } }
        catch { if !Task.isCancelled { failure = error.localizedDescription } }
    }
    private func act(_ action: String) {
        guard let client = model.live, let room, !busy else { return }
        busy = true
        failure = nil
        Task {
            defer { busy = false }
            do { let response: Accepted = try await client.call("meetingAction", Action(groupId: room.groupId, rev: room.rev, action: action, text: ["propose", "finish"].contains(action) ? proposal : nil)); if let destinations = response.deliveredTo { notice = "Outcome published to \(destinations.count) departments. No automatic follow-up was requested." }; attempt += 1 }
            catch { failure = error.localizedDescription; attempt += 1 }
        }
    }
    private func create() {
        guard let client = model.live, !busy else { return }
        busy = true
        Task {
            defer { busy = false }
            do { room = try await client.call("createMeeting", Creation(name: name, agenda: agenda, kind: kind, chair: chair, delegates: selected)) }
            catch { failure = error.localizedDescription }
        }
    }
    private struct Accepted: Decodable { let deliveredTo: [String]? }
    private struct Action: Encodable { let groupId: String; let rev: Int64; let action: String; let text: String? }
    private struct Creation: Encodable { let name: String; let agenda: String; let kind: String; let chair: String; let delegates: [Delegate] }
    private struct Delegate: Codable { let botId: String; var groupId: String? = nil; var name: String? = nil; var department: String? = nil }
    private struct Position: Decodable { let position: String; let reason: String }
    private struct Snapshot: Decodable { let groupId: String; let rev: Int64; let meeting: Info }
    private struct Version: Decodable { let proposalRevision: Int; let proposal: String; let status: String; let positions: [String: Position] }
    private struct Info: Decodable {
        let history: [Version]?
        let kind: String; let status: String; let agenda: String; let proposal: String; let proposalRevision: Int
        let delegates: [Delegate]; let positions: [String: Position]; let spent: Int; let limit: Int
    }
}

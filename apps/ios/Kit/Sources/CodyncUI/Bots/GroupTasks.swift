import CodyncKit
import SwiftUI

struct GroupTasks: View {
    let groupId: String
    @Environment(BotStore.self) private var model
    @State private var items: [TaskEntry] = []
    @State private var title = ""
    @State private var instructions = ""
    @State private var owner = ""
    @State private var requestId = UUID().uuidString
    @State private var attempt = 0
    @State private var busy = false
    @State private var loaded = false
    @State private var expanded: Set<String> = []
    @State private var failure: String?
    private var members: [Bot] { (model.bots[groupId]?.members ?? []).compactMap { model.bots[$0] } }
    var body: some View {
        VStack(spacing: 0) {
            ModalHeader("Group tasks") { IconButton("Refresh tasks", systemImage: "arrow.clockwise") { attempt += 1 } }
            ScrollView {
                VStack(alignment: .leading, spacing: 20) {
                    Text("Attempts run in the owner's chat. Pause waits for cancellation; resume asks the bot to inspect earlier work first.").font(.caption).foregroundStyle(Palette.secondary)
                    Field("Task title") { TextField("Task title", text: $title).fieldBox() }
                    Field("Instructions") { TextField("Instructions", text: $instructions, axis: .vertical).lineLimit(3...8).fieldBox() }
                    Field("Owner") { ChoicePicker(selection: $owner, options: members.map { (id: $0.id, label: $0.name) }) }
                    Button("Create task") { create() }.buttonStyle(SecondaryButtonStyle()).disabled(busy || title.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty || instructions.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty || !members.contains(where: { $0.id == owner }))
                    if let failure { Text(failure).foregroundStyle(Palette.danger).textSelection(.enabled) }
                    if !loaded && failure == nil { Spinner() }
                    if loaded && items.isEmpty { Text("No tasks yet.").foregroundStyle(Palette.secondary) }
                    ForEach(items) { entry in taskCard(entry) }
                }.padding(20).disabled(busy)
            }
        }.background(Palette.background)
        .onChange(of: title) { _, _ in requestId = UUID().uuidString }
        .onChange(of: instructions) { _, _ in requestId = UUID().uuidString }
        .onChange(of: owner) { _, _ in requestId = UUID().uuidString }
        .task(id: "\(groupId):\(attempt)") {
            guard let client = model.live else { failure = "Computer is offline"; return }
            do {
                let response: Tasks = try await client.call("groupTasks", ["groupId": groupId])
                if !Task.isCancelled { items = response.items; loaded = true; failure = nil; if owner.isEmpty { owner = members.first?.id ?? "" } }
            } catch { if !Task.isCancelled { failure = error.localizedDescription } }
        }
    }
    private func taskCard(_ entry: TaskEntry) -> some View {
        let task = entry.data.groupTask
        let stopped = ["draft", "paused", "failed", "interrupted"].contains(task.status)
        return VStack(alignment: .leading, spacing: 8) {
            Text(task.title).font(.headline)
            Text("\(model.bots[task.owner]?.name ?? "Unavailable owner") · \(task.status)").font(.caption).foregroundStyle(Palette.secondary)
            Text(task.text).textSelection(.enabled)
            if stopped {
                Button(task.status == "draft" ? "Start task" : "Resume task") { update(entry, action: task.status == "draft" ? "start" : "resume") }.buttonStyle(SecondaryButtonStyle())
                Field("Reassign owner") { ChoicePicker(selection: Binding(get: { task.owner }, set: { update(entry, action: "reassign", owner: $0) }), options: members.map { (id: $0.id, label: $0.name) }) }
            }
            if ["queued", "running"].contains(task.status) { Button("Pause task") { update(entry, action: "pause") }.buttonStyle(SecondaryButtonStyle()) }
            Disclosure(isExpanded: Binding(get: { expanded.contains(entry.id) }, set: { if $0 { expanded.insert(entry.id) } else { expanded.remove(entry.id) } })) {
                ForEach(Array(task.history.enumerated()), id: \.offset) { _, event in
                    VStack(alignment: .leading) {
                        Text("\(model.bots[event.owner]?.name ?? event.owner) · \(event.status)").font(.caption)
                        Text(Date(milliseconds: event.at), style: .time).font(.caption)
                        Text(event.detail).font(.caption).textSelection(.enabled)
                    }.frame(maxWidth: .infinity, alignment: .leading)
                }
            } label: { Text("Task history") }
        }.frame(maxWidth: .infinity, alignment: .leading)
    }
    private func create() {
        guard !busy, let client = model.live else { return }
        busy = true
        Task {
            defer { busy = false }
            do {
                let _: Saved = try await client.call("createGroupTask", Creation(groupId: groupId, title: title, text: instructions, owner: owner, requestId: requestId))
                title = ""; instructions = ""; requestId = UUID().uuidString; attempt += 1
            } catch { failure = error.localizedDescription }
        }
    }
    private func update(_ entry: TaskEntry, action: String, owner: String? = nil) {
        guard !busy, let client = model.live else { return }
        busy = true
        Task {
            defer { busy = false }
            do { let _: Saved = try await client.call("updateGroupTask", Update(taskId: entry.id, revision: entry.rev, action: action, owner: owner)); attempt += 1 }
            catch { failure = error.localizedDescription; attempt += 1 }
        }
    }
    private struct Creation: Encodable { let groupId: String; let title: String; let text: String; let owner: String; let requestId: String }
    private struct Update: Encodable { let taskId: String; let revision: Int64; let action: String; let owner: String? }
    private struct Tasks: Decodable { let items: [TaskEntry] }
    private struct Saved: Decodable { let entry: TaskEntry }
    private struct TaskEntry: Decodable, Identifiable { let id: String; let rev: Int64; let data: Data }
    private struct Data: Decodable { let groupTask: Assignment }
    private struct Assignment: Decodable { let title: String; let text: String; let owner: String; let status: String; let history: [Event] }
    private struct Event: Decodable { let at: Int64; let status: String; let owner: String; let detail: String }
}

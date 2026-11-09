import CodyncKit
import SwiftUI

/// The user's messages waiting behind the bot's work, above the composer: each one line, sent now into the running
/// turn (`steer`) or taken back. An older computer has neither action, so the rows only show.
struct QueuedMessages: View {
    let entries: [Entry]
    @State private var queueState: QueueState?
    @State private var queueFailure: String?
    @State private var controlling = false
    @State private var editing: Entry?
    @Environment(BotStore.self) private var model
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    var body: some View {
        // A long queue scrolls, so every row stays in reach and the chat keeps its room.
        ScrollViewReader { proxy in
            CappedHeight(max: 190) {
                ScrollView {
                    VStack(spacing: 0) {
                        if let first = entries.first, model.bots[first.botId]?.isGroup != true {
                            HStack {
                                Text(queueState?.reason ?? "Queued messages").font(.caption).foregroundStyle(Palette.secondary)
                                Spacer()
                                Button(queueState?.paused == true ? "Resume queue" : "Pause queue") { control(queueState?.paused == true ? "resume" : "pause") }.buttonStyle(SecondaryButtonStyle()).disabled(controlling)
                            }.padding(.horizontal, 14)
                        }
                        if let queueFailure { Text(queueFailure).font(.caption).foregroundStyle(Palette.danger) }
                        ForEach(entries) { entry in
                            row(entry).id(entry.id).transition(.move(edge: .bottom).combined(with: .opacity))
                        }
                    }
                    .padding(.vertical, 2)
                }
                .scrollIndicators(.hidden)
                .scrollBounceBehavior(.basedOnSize)
            }
            // It stays where it is, the oldest (next to go) in view; a message sent from here shows that it arrived.
            .onChange(of: entries.last?.id) {
                guard let newest = entries.last, let nonce = newest.data.clientNonce, nonce == model.lastSentNonce else { return }
                withAnimation(Motion.reduced(Motion.layout, reduceMotion)) { proxy.scrollTo(newest.id, anchor: .bottom) }
            }
        }
        .task(id: entries.map { "\($0.id):\($0.rev)" }.joined(separator: ",")) { control("state") }
        .codyncSheet(item: $editing) { QueuedMessageEditor(entry: $0) }
        .background(Palette.surface)
        .clipShape(RoundedRectangle(cornerRadius: 18, style: .continuous))
        .animation(Motion.reduced(Motion.layout, reduceMotion), value: entries.map(\.id))
    }

    private func row(_ entry: Entry) -> some View {
        HStack(spacing: 2) {
            Text(MessagePreview.text(entry.replyQuote.text))
                .font(.subheadline)
                .foregroundStyle(Palette.text)
                .lineLimit(1)
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(.vertical, model.supportsQueueActions ? 0 : 9)
            if model.supportsQueueActions {
                Group {
                    if model.bots[entry.botId]?.isGroup != true {
                        IconButton("Move earlier", systemImage: "arrow.up") { control("earlier", entryId: entry.id) }.disabled(controlling)
                        IconButton("Move later", systemImage: "arrow.down") { control("later", entryId: entry.id) }.disabled(controlling)
                        IconButton("Edit queued message", systemImage: "pencil") { editing = entry }
                            .disabled(model.answering[entry.id] != nil)
                    }
                    IconButton("Send now", systemImage: "arrow.turn.down.right") { model.steer(entry) }
                    IconButton("Remove", systemImage: "trash") { model.unqueue(entry) }
                }
                .disabled(model.answering[entry.id] != nil)
            }
        }
        .padding(.leading, 14)
        .padding(.trailing, 4)
    }
    private struct QueueState: Decodable { let paused: Bool; let reason: String }
    private struct Control: Encodable { let botId: String; let action: String; let entryId: String? }
    private func control(_ action: String, entryId: String? = nil) {
        guard !controlling, let first = entries.first, model.bots[first.botId]?.isGroup != true, let client = model.live else { return }
        controlling = true
        Task {
            defer { controlling = false }
            do {
                queueState = try await client.call("queueControl", Control(botId: first.botId, action: action, entryId: entryId))
                queueFailure = nil
            } catch { queueFailure = error.localizedDescription }
        }
    }

}

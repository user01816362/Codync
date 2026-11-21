import CodyncKit
import SwiftUI

struct QueuedMessageEditor: View {
    let entry: Entry
    @Environment(BotStore.self) private var model
    @Environment(\.dismissModal) private var dismiss
    @State private var text: String
    @State private var saving = false
    @State private var failure: String?

    init(entry: Entry) {
        self.entry = entry
        _text = State(initialValue: entry.data.text ?? "")
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            ModalHeader("Edit queued message") {
                if saving { Spinner() }
                else { IconButton("Save", systemImage: "checkmark") { save() }.disabled(text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty || text.utf8.count > 65536) }
            }
            VStack(alignment: .leading, spacing: 12) {
                Text(L10n.tr("Changes apply only while the message is waiting. Files and reply context stay attached."))
                    .font(.caption).foregroundStyle(Palette.secondary)
                TextField("Message", text: $text, axis: .vertical).lineLimit(4...12).disabled(saving)
                if let failure { Text(failure).font(.caption).foregroundStyle(Palette.danger) }
            }.padding(.horizontal, 20).padding(.bottom, 24)
        }.background(Palette.background)
    }

    private func save() {
        guard let client = model.live else { failure = L10n.tr("Computer is offline"); return }
        saving = true
        failure = nil
        Task {
            defer { saving = false }
            do {
                let _: Empty = try await client.call("editQueued", ["botId": entry.botId, "entryId": entry.id, "text": text])
                dismiss()
            } catch { failure = error.localizedDescription }
        }
    }
}

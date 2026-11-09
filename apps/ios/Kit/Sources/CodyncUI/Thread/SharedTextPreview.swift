import CodyncKit
import SwiftUI

/// A bounded, verified snapshot; Markdown is rendered and other source stays plain text.
struct SharedTextPreview: View {
    private let name: String
    private let id: String
    private let size: Int64
    private var entryId: String? = nil
    private let load: @MainActor (BotStore) async -> Data?
    @Environment(BotStore.self) private var model
    @State private var text: String?
    @State private var failed = false
    @State private var quoteFailure: String?
    @Environment(\.dismissModal) private var dismiss
    @State private var attempt = 0

    init(file: SharedFile, entryId: String) {
        name = file.name; id = file.id; size = file.size
        self.entryId = entryId
        load = { await $0.sharedFileData(file, entryId: entryId) }
    }

    init(attachment: Attachment, botId: String, entryId: String? = nil) {
        name = attachment.name; id = attachment.id; size = attachment.size
        self.entryId = entryId
        load = { await $0.attachmentData(attachment, bot: botId, bounded: true) }
    }

    var body: some View {
        if let kind = SharedFile.mediaKind(name: name, size: size) {
            MediaFilePreview(name: name, size: size, kind: kind) { await load(model) }
        } else { textPreview }
    }

    private var quoteAction: ((String) -> Void)? {
        guard let entryId else { return nil }
        return { passage in
            Task {
                do {
                    try await model.selectFileReply(entryId: entryId, fileId: id, text: passage)
                    dismiss()
                } catch { quoteFailure = error.localizedDescription }
            }
        }
    }

    private var textPreview: some View {
        TextPreviewBody(preferenceId: id, name: name, text: text, failure: failed ? L10n.tr("Couldn't open this file. The file must be valid UTF-8 text up to 1 MiB and pass its integrity checks.") : nil, quote: quoteAction) {
            attempt += 1
        }
        .overlay(alignment: .bottom) {
            if let quoteFailure { Text(quoteFailure).font(.caption).foregroundStyle(Palette.danger).padding() }
        }
        .id(id)
        .task(id: "\(id):\(attempt)") {
            text = nil
            failed = false
            guard SharedFile.canPreview(name: name, size: size), let data = await load(model), Int64(data.count) == size,
                  let decoded = String(data: data, encoding: .utf8), !decoded.contains("\0") else {
                if !Task.isCancelled { failed = true }
                return
            }
            if !Task.isCancelled { text = decoded }
        }
    }
}

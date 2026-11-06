import CodyncKit
import SwiftUI

/// A bounded, verified snapshot; Markdown is rendered and other source stays plain text.
struct SharedTextPreview: View {
    private let name: String
    private let id: String
    private let size: Int64
    private let load: @MainActor (BotStore) async -> Data?
    @Environment(BotStore.self) private var model
    @State private var text: String?
    @State private var failed = false

    init(file: SharedFile, entryId: String) {
        name = file.name; id = file.id; size = file.size
        load = { await $0.sharedFileData(file, entryId: entryId) }
    }

    init(attachment: Attachment, botId: String) {
        name = attachment.name; id = attachment.id; size = attachment.size
        load = { await $0.attachmentData(attachment, bot: botId, bounded: true) }
    }

    var body: some View {
        VStack(spacing: 0) {
            ModalHeader(name)
            ScrollView([.vertical, .horizontal]) {
                Group {
                    if let text {
                        if name.lowercased().hasSuffix(".md") || name.lowercased().hasSuffix(".markdown") {
                            MarkdownText(text)
                        } else {
                            Text(text).font(.system(.footnote, design: .monospaced)).textSelection(.enabled)
                        }
                    } else if failed {
                        Text(L10n.tr("Couldn't open this file.")).foregroundStyle(Palette.secondary)
                    } else { Spinner() }
                }.frame(maxWidth: .infinity, alignment: .leading).padding(20)
            }
        }
        .background(Palette.background)
        .task(id: id) {
            guard SharedFile.canPreview(name: name, size: size), let data = await load(model), Int64(data.count) == size,
                  let decoded = String(data: data, encoding: .utf8), !decoded.contains("\0") else {
                if !Task.isCancelled { failed = true }
                return
            }
            if !Task.isCancelled { text = decoded }
        }
    }
}

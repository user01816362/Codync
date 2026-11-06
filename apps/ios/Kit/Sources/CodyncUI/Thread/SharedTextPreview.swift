import CodyncKit
import SwiftUI

/// A bounded, verified snapshot; Markdown is rendered and other source stays plain text.
struct SharedTextPreview: View {
    let file: SharedFile
    let entryId: String
    @Environment(BotStore.self) private var model
    @State private var text: String?
    @State private var failed = false

    var body: some View {
        VStack(spacing: 0) {
            ModalHeader(file.name)
            ScrollView([.vertical, .horizontal]) {
                Group {
                    if let text {
                        if file.name.lowercased().hasSuffix(".md") || file.name.lowercased().hasSuffix(".markdown") {
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
        .task(id: file.id) {
            guard file.showsAsText, let data = await model.sharedFileData(file, entryId: entryId),
                  let decoded = String(data: data, encoding: .utf8), !decoded.contains("\0") else {
                if !Task.isCancelled { failed = true }
                return
            }
            if !Task.isCancelled { text = decoded }
        }
    }
}

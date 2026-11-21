import CodyncKit
import SwiftUI

/// Shared presentation for verified snapshots and host-confined workspace text.
struct TextPreviewBody: View {
    var preferenceId: String? = nil
    let name: String
    let text: String?
    let failure: String?
    var quote: ((String) -> Void)? = nil
    let retry: () -> Void
    @Environment(BotStore.self) private var model
    @State private var table: String?
    @State private var tableNotice: String?
    @State private var selectingQuote = false
    @State private var source = false
    @State private var wrap = true
    @State private var query = ""
    private var preferenceKey: String { preferenceId ?? name }

    private var markdown: Bool {
        ["md", "markdown"].contains((name as NSString).pathExtension.lowercased())
    }

    private var tabular: Bool { ["csv", "tsv"].contains((name as NSString).pathExtension.lowercased()) }
    private struct TableResponse: Decodable { let markdown: String; let truncated: Bool }

    var body: some View {
        VStack(spacing: 0) {
            ModalHeader(name)
            HStack(spacing: 4) {
                Spacer(minLength: 0)
                if quote != nil {
                    IconButton("Select text / Quote", systemImage: "arrowshape.turn.up.left") { selectingQuote = true }
                        .disabled(text == nil)
                }
                if markdown || tabular {
                    Button(source ? "Read" : "Source") {
                        withAnimation(Motion.layout) { source.toggle() }
                    }
                    .buttonStyle(.plain).font(.caption).foregroundStyle(Palette.secondary)
                    .accessibilityLabel(source ? "Read Markdown" : "Show source")
                }
                if !markdown && !tabular || source {
                    IconButton(wrap ? "Disable line wrapping" : "Wrap lines", systemImage: "text.alignleft") {
                        withAnimation(Motion.layout) { wrap.toggle() }
                    }
                    .accessibilityValue(wrap ? "On" : "Off")
                }
                IconButton("Copy", systemImage: "square.on.square") {
                    if let text { Pasteboard.copy(text) }
                }.disabled(text == nil)
                IconButton("Retry", systemImage: "arrow.clockwise", action: retry)
                    .disabled(text == nil && failure == nil)
            }.padding(.horizontal, 16).padding(.bottom, 8)
            SearchField("Find in file", text: $query).padding(.horizontal, 16).padding(.bottom, 8)
            if !query.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty, let text {
                let matches = text.components(separatedBy: "\n").enumerated().filter { $0.element.localizedCaseInsensitiveContains(query.trimmingCharacters(in: .whitespacesAndNewlines)) }
                ScrollView {
                    VStack(alignment: .leading, spacing: 8) {
                        Text("\(matches.count) matching lines" + (matches.count > 100 ? " · showing first 100" : "")).font(.caption)
                        ForEach(Array(matches.prefix(100)), id: \.offset) { match in
                            Text("\(match.offset + 1): \(match.element)").font(.system(.caption, design: .monospaced)).textSelection(.enabled)
                        }
                    }.frame(maxWidth: .infinity, alignment: .leading).padding(.horizontal, 20)
                }.frame(maxHeight: 180)
            }
            ScrollView((markdown || table != nil) && !source || wrap ? .vertical : [.vertical, .horizontal]) {
                Group {
                    if let text {
                        if !source, let tableNotice { Text(tableNotice).font(.caption).foregroundStyle(Palette.secondary) }
                        if (markdown || table != nil) && !source {
                            MarkdownText(table ?? text)
                        } else {
                            Text(text).font(.system(.footnote, design: .monospaced)).textSelection(.enabled)
                                .fixedSize(horizontal: !wrap, vertical: true)
                        }
                    } else if let failure {
                        Text(failure).foregroundStyle(Palette.secondary).textSelection(.enabled)
                    } else { Spinner() }
                }.frame(maxWidth: .infinity, alignment: .leading).padding(20)
            }
        }.background(Palette.background)
        .onAppear {
            let saved = UserDefaults.standard.dictionary(forKey: "text-preview-preferences")?[preferenceKey] as? [String: Bool]
            source = saved?["source"] ?? false
            wrap = saved?["wrap"] ?? true
        }
        .onChange(of: source) { _, _ in savePreferences() }
        .onChange(of: wrap) { _, _ in savePreferences() }
        .codyncSheet(isPresented: $selectingQuote) {
            QuoteSelectionSheet(text: text ?? "", author: name, structured: true) { quote?($0) }
        }
        .task(id: text) {
            table = nil
            tableNotice = nil
            guard tabular, let text else { return }
            guard text.utf8.count <= 65536 else {
                tableNotice = L10n.tr("Table previews are limited to 64 KiB. Source shows the full text.")
                return
            }
            guard let client = model.live else {
                tableNotice = L10n.tr("Connect to the computer for a table preview. Source remains available.")
                return
            }
            do {
                let result: TableResponse = try await client.call("formatTable", ["name": name, "text": text])
                if !Task.isCancelled {
                    table = result.markdown
                    if result.truncated { tableNotice = L10n.tr("Table preview truncated. Source shows the full text.") }
                }
            } catch { if !Task.isCancelled { tableNotice = error.localizedDescription } }
        }
    }
    private func savePreferences() {
        var saved = UserDefaults.standard.dictionary(forKey: "text-preview-preferences") ?? [:]
        if saved[preferenceKey] == nil, saved.count >= 100, let oldest = saved.keys.sorted().first { saved.removeValue(forKey: oldest) }
        saved[preferenceKey] = ["source": source, "wrap": wrap]
        UserDefaults.standard.set(saved, forKey: "text-preview-preferences")
    }

}

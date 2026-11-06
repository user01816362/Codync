import CodyncKit
import SwiftUI

struct WorkspacePreviewTarget: Identifiable {
    let path: String
    var id: String { path }
}

/// Accept explicit local Markdown links only; the host confines the request to the workspace.
func workspaceLink(_ url: URL) -> String? {
    let host = url.host?.lowercased()
    let local = (url.scheme == nil && host == nil) ||
        (url.scheme?.lowercased() == "file" && (host == nil || host == "" || host == "localhost"))
    guard local, url.user == nil, url.password == nil, url.port == nil,
          url.query == nil, url.fragment == nil, !url.path.isEmpty else { return nil }
    return url.path
}

struct WorkspaceTextPreview: View {
    let botId: String
    let path: String
    @Environment(BotStore.self) private var model
    @State private var result: Response?
    @State private var failed = false

    private struct Request: Encodable { let botId: String; let path: String }
    private struct Response: Decodable { let name: String; let text: String }

    var body: some View {
        VStack(spacing: 0) {
            ModalHeader(result?.name ?? (path as NSString).lastPathComponent)
            ScrollView([.vertical, .horizontal]) {
                Group {
                    if let result {
                        if result.name.lowercased().hasSuffix(".md") || result.name.lowercased().hasSuffix(".markdown") {
                            MarkdownText(result.text)
                        } else {
                            Text(result.text).font(.system(.footnote, design: .monospaced)).textSelection(.enabled)
                        }
                    } else if failed {
                        Text(L10n.tr("Couldn't open this file.")).foregroundStyle(Palette.secondary)
                    } else { Spinner() }
                }.frame(maxWidth: .infinity, alignment: .leading).padding(20)
            }
        }
        .background(Palette.background)
        .task(id: path) {
            guard let client = model.live else { failed = true; return }
            do { result = try await client.call("previewWorkspaceFile", Request(botId: botId, path: path)) }
            catch { if !Task.isCancelled { failed = true } }
        }
    }
}

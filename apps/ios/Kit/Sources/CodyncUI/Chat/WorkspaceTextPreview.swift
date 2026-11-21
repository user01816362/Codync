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
    @Environment(\.dismissModal) private var dismiss
    @State private var quoteFailure: String?
    @State private var result: Response?
    @State private var failure: String?
    @State private var attempt = 0

    private struct Request: Encodable { let botId: String; let path: String }
    private struct Response: Decodable { let name: String; let text: String }

    var body: some View {
        let name = result?.name ?? (path as NSString).lastPathComponent
        TextPreviewBody(preferenceId: "\(botId):\(path)", name: name, text: result?.text, failure: quoteFailure ?? failure, quote: { text in
            Task {
                do { try await model.selectWorkspaceReply(botId: botId, path: path, text: text); dismiss() }
                catch { quoteFailure = error.localizedDescription }
            }
        }) {
            attempt += 1
        }
        .overlay(alignment: .bottom) { if let quoteFailure { Text(quoteFailure).font(.caption).foregroundStyle(Palette.danger).padding() } }
        .id("\(botId):\(path)")
        .task(id: "\(botId):\(path):\(attempt)") {
            result = nil
            failure = nil
            guard let client = model.live else { failure = L10n.tr("Computer is offline"); return }
            do {
                let loaded: Response = try await client.call("previewWorkspaceFile", Request(botId: botId, path: path))
                if !Task.isCancelled { result = loaded }
            }
            catch { if !Task.isCancelled { failure = L10n.tr("Couldn't open this file.") + " " + error.localizedDescription } }
        }
    }
}

import CodyncKit
import SwiftUI

struct WorkspaceBrowser: View {
    let botId: String
    @Environment(BotStore.self) private var model
    @State private var path = "."
    @State private var hidden = false
    @State private var attempt = 0
    @State private var directory: WorkspaceDirectory?
    @State private var failure: String?
    @State private var selected: WorkspaceFile?

    var body: some View {
        VStack(spacing: 0) {
            ModalHeader("Workspace files")
            HStack {
                IconButton("Workspace root", systemImage: "house") { path = "." }.disabled(path == ".")
                IconButton("Parent folder", systemImage: "arrow.up") { if let parent = directory?.parent { path = parent } }
                    .disabled(directory?.parent == nil)
                Text(directory?.path ?? path).font(.caption).lineLimit(2)
                Spacer(minLength: 0)
                IconButton("Retry", systemImage: "arrow.clockwise") { attempt += 1 }.disabled(directory == nil && failure == nil)
            }.padding(.horizontal, 16)
            SwitchRow("Hidden files", isOn: $hidden).padding(.horizontal, 20)
            ScrollView {
                VStack(alignment: .leading, spacing: 8) {
                    if let failure { Text(failure).foregroundStyle(Palette.danger).textSelection(.enabled) }
                    else if let directory {
                        if directory.truncated { Text(L10n.tr("Listing limited to 500 items from at most 2,000 entries.")).font(.caption).foregroundStyle(Palette.secondary) }
                        if directory.entries.isEmpty { Text(L10n.tr("This folder is empty.")).foregroundStyle(Palette.secondary) }
                        ForEach(directory.entries) { item in
                            let folder = item.kind == "directory"
                            let readable = item.kind == "file" && SharedFile.canPreview(name: item.name, size: item.size)
                            Button {
                                if folder { withAnimation(Motion.layout) { path = item.path } }
                                else { selected = WorkspaceFile(botId: directory.botId, path: item.path) }
                            } label: {
                                HStack(spacing: 12) {
                                    Image(systemName: folder ? "folder" : "doc.text")
                                    VStack(alignment: .leading, spacing: 2) {
                                        Text(item.name).lineLimit(2)
                                        Text(folder ? L10n.tr("Folder") : readable ? ByteCountFormatter.string(fromByteCount: item.size, countStyle: .file) : L10n.tr("Ask the bot to share this file for preview."))
                                            .font(.caption).foregroundStyle(Palette.secondary)
                                    }
                                    Spacer(minLength: 0)
                                }.contentShape(Rectangle()).padding(.vertical, 6)
                            }.buttonStyle(.plain).disabled(!folder && !readable)
                        }
                    } else { Spinner() }
                }.padding(20).frame(maxWidth: .infinity, alignment: .leading)
            }
        }
        .background(Palette.background)
        .task(id: "\(botId):\(path):\(hidden):\(attempt)") {
            directory = nil
            failure = nil
            guard let client = model.live else { failure = L10n.tr("Computer is offline"); return }
            do {
                let result: WorkspaceDirectory = try await client.call("listWorkspaceFiles", Request(botId: botId, path: path, showHidden: hidden))
                if !Task.isCancelled { directory = result }
            } catch { if !Task.isCancelled { failure = error.localizedDescription } }
        }
        .codyncSheet(item: $selected) { WorkspaceTextPreview(botId: $0.botId, path: $0.path) }
    }

    private struct Request: Encodable { let botId: String; let path: String; let showHidden: Bool }
}

private struct WorkspaceDirectory: Decodable {
    let botId: String
    let path: String
    let parent: String?
    let entries: [WorkspaceItem]
    let truncated: Bool
}
private struct WorkspaceItem: Decodable, Identifiable {
    let name: String
    let path: String
    let kind: String
    let size: Int64
    var id: String { path }
}
private struct WorkspaceFile: Identifiable {
    let botId: String
    let path: String
    var id: String { path }
}

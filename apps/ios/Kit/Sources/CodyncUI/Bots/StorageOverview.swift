import CodyncKit
import SwiftUI

struct StorageOverview: View {
    @Environment(BotStore.self) private var model
    @State private var value: Usage?
    @State private var failure: String?
    @State private var busy = false
    @State private var confirm = false
    @State private var attempt = 0
    var body: some View {
        VStack(spacing: 0) {
            ModalHeader("Host storage") { IconButton("Refresh storage", systemImage: "arrow.clockwise") { attempt += 1 }.disabled(busy) }
            ScrollView {
                VStack(alignment: .leading, spacing: 20) {
                    Text("Stored on the connected computer. File sizes are a bounded scan of managed file roots; workspace project files, logs, caches and database journals are excluded.").font(.caption).foregroundStyle(Palette.secondary)
                    if let failure { Text(failure).foregroundStyle(Palette.danger).textSelection(.enabled) }
                    if busy { Spinner() }
                    if let value {
                        Text("Conversation database: \(bytes(value.databaseBytes)) · \(value.entries) entries")
                        Text("Reusable database pages: \(bytes(value.reusableBytes))")
                        category("Shared files", value.sharedFiles)
                        category("Uploads", value.uploads)
                        category("Artifacts", value.artifacts)
                        Button("Compact database") { confirm = true }.buttonStyle(SecondaryButtonStyle()).disabled(busy || value.reusableBytes == 0)
                    }
                }.padding(20).frame(maxWidth: .infinity, alignment: .leading)
            }
        }.background(Palette.background)
        .task(id: attempt) { await load() }
        .codyncDialog("Compact database?", isPresented: $confirm, message: "This preserves saved messages and files. SQLite may need up to twice the database size in temporary free space and briefly blocks host database operations. File caches are not deleted.") {
            [DialogAction("Compact database") { Task { await load(compact: true) } }]
        }
    }
    private func category(_ label: String, _ category: Category) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            Text("\(label): \(bytes(category.bytes)) · \(category.count) files")
            if category.truncated || category.unavailable > 0 { Text("Partial scan · \(category.unavailable) unavailable paths").font(.caption).foregroundStyle(Palette.secondary) }
        }
    }
    private func bytes(_ count: Int64) -> String { ByteCountFormatter.string(fromByteCount: count, countStyle: .file) }
    private func load(compact: Bool = false) async {
        guard !busy, let client = model.live else { if model.live == nil { failure = "Computer is offline" }; return }
        busy = true; failure = nil
        defer { busy = false }
        do { value = try await client.call(compact ? "compactStorage" : "storageOverview", ["confirm": compact]) }
        catch { failure = error.localizedDescription }
    }
    private struct Category: Decodable { let count: Int64; let bytes: Int64; let truncated: Bool; let unavailable: Int }
    private struct Usage: Decodable { let databaseBytes: Int64; let reusableBytes: Int64; let entries: Int64; let sharedFiles: Category; let uploads: Category; let artifacts: Category }
}

import CodyncKit
import SwiftUI

/// A chat's Library (the details sheet's Library tab): the artifacts its bot published and the files shared in it,
/// grouped by when. A row does what tapping it in the chat does; an artifact's menu also renames or shares it.
struct LibraryView: View {
    let botId: String
    /// Opens an artifact in the chat's viewer (the details sheet closes first).
    let openArtifact: (ArtifactTarget) -> Void
    @Environment(BotStore.self) private var model
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    /// The files shared in the chat; its artifacts come from the store, where their events keep them current.
    @State private var files: [ChatLibraryItem] = []
    @State private var loaded = false
    @State private var error: String?
    @State private var renaming: Artifact?
    @State private var sharing: ArtifactFile?
    @State private var viewing: ImageViewing?
    @State private var textAttachment: Attachment?
    @State private var textFile: LibraryTextFile?
    @Environment(\.dismissModal) private var dismiss
    @State private var nextBefore: Int64?
    @State private var paging = false
    @State private var query = ""
    @State private var attachmentEntryId: String?
    @State private var filesOnly = false
    @State private var attempt = 0

    var body: some View {
        let items = files + model.artifacts.artifacts(of: botId).map(ChatLibraryItem.init)
        let fileItems = items.filter { if case .artifact = $0.kind { return false }; return true }
        let visible = (filesOnly ? fileItems : items).filter { item in
            guard !query.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else { return true }
            switch item.kind {
            case .artifact(let artifact): return artifact.title.localizedCaseInsensitiveContains(query)
            case .file(let file, _): return file.name.localizedCaseInsensitiveContains(query)
            case .attachment(let file, _): return file.name.localizedCaseInsensitiveContains(query)
            }
        }
        VStack(alignment: .leading, spacing: 28) {
            HStack {
                TabChoice(selection: $filesOnly, options: [(false, "All"), (true, "Files")])
                Text("\(fileItems.count)").font(.caption).foregroundStyle(Palette.secondary)
                Spacer(minLength: 0)
                IconButton("Retry", systemImage: "arrow.clockwise") { attempt += 1 }.disabled(!loaded)
            }
            SearchField("Find files and artifacts", text: $query)
            if let error {
                Text(L10n.tr(error)).font(.caption).foregroundStyle(Palette.danger).textSelection(.enabled)
            }
            if !visible.isEmpty {
                ForEach(ChatLibraryItem.sections(visible), id: \.period) { section in
                    CardSection(section.period.rawValue) {
                        ForEach(section.items) { row($0) }
                    }
                }
            } else if loaded && error == nil {
                if filesOnly { Text(L10n.tr("No files shared in this chat")).foregroundStyle(Palette.secondary) }
                else { empty }
            } else if !loaded {
                Spinner(size: 20).frame(maxWidth: .infinity).padding(.top, 40)
            }
            if nextBefore != nil { Button("Load older files") { older() }.buttonStyle(SecondaryButtonStyle()).disabled(paging) }
        }
        .animation(Motion.reduced(Motion.fade, reduceMotion), value: loaded)
        .task(id: "\(botId):\(attempt)") {
            loaded = false
            error = nil
            do {
                let page = try await model.loadLibraryPage(botId)
                if !Task.isCancelled { files = page.items; nextBefore = page.nextBefore }
            } catch {
                if !Task.isCancelled { self.error = error.localizedDescription }
            }
            if !Task.isCancelled { loaded = true }
        }
        .codyncSheet(item: $renaming) { ArtifactRenameSheet(artifact: $0) }
        .artifactShareSheet($sharing)
        .imageViewer($viewing)
        // The picture viewer presents a download that finishes while it's open.
        .fileExportSheet(model, botId: botId, active: viewing == nil)
        .codyncSheet(item: $textAttachment) { SharedTextPreview(attachment: $0, botId: botId, entryId: attachmentEntryId) }
        .codyncSheet(item: $textFile) { SharedTextPreview(file: $0.file, entryId: $0.entryId) }
    }

    @ViewBuilder private func row(_ item: ChatLibraryItem) -> some View {
        switch item.kind {
        case .artifact(let artifact):
            Button { show(artifact) } label: {
                LibraryRow(title: artifact.title) { symbol(ArtifactCard.symbol) }
            }
            .buttonStyle(.plain)
            .contextActions {
                [MenuItem("Open", icon: "arrow.up.forward.square") { show(artifact) },
                 MenuItem("Rename artifact", icon: "pencil") { renaming = artifact },
                 MenuItem("Share artifact", icon: "square.and.arrow.up") { share(artifact) }]
            }
        case .file(let file, let entryId):
            Button {
                if file.showsAsPreview { textFile = LibraryTextFile(file: file, entryId: entryId) }
                else { download(file, entryId: entryId) }
            } label: {
                LibraryRow(title: file.name, detail: (model.bots[botId]?.name ?? "") + " · " + FileDownloadCard.detail(model.fileDownloads.states[file.id], size: file.size) + " · " + item.date.formatted()) {
                    symbol(AttachmentIcon.symbol(file.name))
                }
            }
            .buttonStyle(.plain)
            .contextActions {
                [MenuItem("Show in chat", icon: "bubble.left") { show(entryId) }, MenuItem("Download", icon: "arrow.down.to.line") { download(file, entryId: entryId) }]
            }
        case .attachment(let file, let entryId) where file.isImage:
            // No message here: the viewer's menu is just Copy image and Copy name.
            Button { viewing = ImageViewing(source: .sent(file, botId: botId)) } label: {
                LibraryRow(title: file.name) {
                    AttachmentImage(.sent(file, botId: botId), maxPixels: 120, contentMode: .fill)
                        .frame(width: 36, height: 36)
                        .clipShape(RoundedRectangle(cornerRadius: 8, style: .continuous))
                }
            }
            .buttonStyle(.plain)
            .contextActions { [MenuItem("Show in chat", icon: "bubble.left") { show(entryId) }] }
        case .attachment(let file, let entryId):
            Button { attachmentEntryId = entryId; textAttachment = file } label: {
                LibraryRow(title: file.name, detail: L10n.tr("You") + " · " + ByteCountFormatter.string(fromByteCount: file.size, countStyle: .file) + " · " + item.date.formatted()) {
                    symbol(AttachmentIcon.symbol(file.name))
                }
            }.buttonStyle(.plain).disabled(!SharedFile.canOpenPreview(name: file.name, size: file.size))
            .contextActions { [MenuItem("Show in chat", icon: "bubble.left") { show(entryId) }] }
        }
    }

    private func show(_ entry: String) {
        Task {
            do { try await model.revealMessage(botId: botId, entryId: entry); dismiss() }
            catch { self.error = error.localizedDescription }
        }
    }
    private func older() {
        guard let nextBefore, !paging else { return }
        paging = true
        Task {
            defer { paging = false }
            do {
                let page = try await model.loadLibraryPage(botId, before: nextBefore)
                var seen = Set(files.map(\.id))
                files += page.items.filter { seen.insert($0.id).inserted }
                self.nextBefore = page.nextBefore
            } catch { self.error = error.localizedDescription }
        }
    }

    private func symbol(_ name: String) -> some View {
        Image(systemName: name).font(.system(size: 18)).foregroundStyle(Palette.secondary)
    }

    private var empty: some View {
        VStack(spacing: 8) {
            Image(systemName: "books.vertical").font(.system(size: 34)).foregroundStyle(Palette.tertiary)
            Text(L10n.tr("Library is empty")).font(.headline).foregroundStyle(Palette.text)
            Text(L10n.tr("Artifacts {name} makes and files shared in this chat show here.")
                .replacingOccurrences(of: "{name}", with: model.bots[botId]?.name ?? ""))
                .font(.subheadline)
                .foregroundStyle(Palette.secondary)
        }
        .multilineTextAlignment(.center)
        .padding(.vertical, 32)
        .frame(maxWidth: .infinity)
    }

    private func show(_ artifact: Artifact) {
        openArtifact(ArtifactTarget(botId: artifact.botId, id: artifact.id, title: artifact.title))
    }

    private func share(_ artifact: Artifact) {
        Task {
            do {
                sharing = ArtifactFile(url: try await model.artifactFile(artifact.id, botId: botId))
            } catch {
                self.error = error.localizedDescription
            }
        }
    }

    /// As the file's chat card does: download it for the share sheet, or stop the download under way.
    private func download(_ file: SharedFile, entryId: String) {
        Motion.animate { model.fileDownloads.toggle(client: model.client, entryId: entryId, botId: botId, file: file) }
    }
}

/// One Library row: its picture (a symbol or a thumbnail), the name, and an optional line under it.
private struct LibraryRow<Leading: View>: View {
    let title: String
    var detail: String?
    @ViewBuilder let leading: Leading

    var body: some View {
        HStack(spacing: 12) {
            leading.frame(width: 36, height: 36)
            VStack(alignment: .leading, spacing: 2) {
                Text(title).foregroundStyle(Palette.text).lineLimit(2)
                if let detail { Text(L10n.tr(detail)).font(.caption).foregroundStyle(Palette.secondary) }
            }
            Spacer(minLength: 0)
        }
        .contentShape(Rectangle())
    }
}

private struct LibraryTextFile: Identifiable {
    let file: SharedFile
    let entryId: String
    var id: String { "\(entryId):\(file.id)" }
}

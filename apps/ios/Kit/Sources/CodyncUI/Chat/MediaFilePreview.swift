import AVKit
import CodyncKit
import PDFKit
import SwiftUI

/// Local, size-checked snapshots. Playback starts only through the native controls.
struct MediaFilePreview: View {
    let name: String
    let size: Int64
    let kind: String
    let load: @MainActor () async -> Data?
    @State private var document: PDFDocument?
    @State private var player: AVPlayer?
    @State private var temporary: URL?
    @State private var failure: String?
    @State private var attempt = 0

    var body: some View {
        VStack(spacing: 0) {
            ModalHeader(name) {
                IconButton("Retry", systemImage: "arrow.clockwise") { attempt += 1 }
                    .disabled(document == nil && player == nil && failure == nil)
            }
            if let failure { Text(failure).foregroundStyle(Palette.secondary).padding(20) }
            else if let document { ReadOnlyPDF(document: document) }
            else if let player { VideoPlayer(player: player) }
            else { Spinner().padding(20) }
        }
        .background(Palette.background)
        .task(id: attempt) {
            clear()
            failure = nil
            guard size >= 0, size <= 32 * 1024 * 1024, let bytes = await load(), Int64(bytes.count) == size else {
                if !Task.isCancelled { failure = L10n.tr("Couldn't verify this file. Download it from its card instead.") }
                return
            }
            guard !Task.isCancelled else { return }
            if kind == "pdf" {
                guard bytes.starts(with: Data("%PDF-".utf8)), let pdf = PDFDocument(data: bytes), pdf.pageCount > 0, pdf.pageCount <= 500 else {
                    failure = L10n.tr("This PDF cannot be previewed, or exceeds 500 pages.")
                    return
                }
                document = pdf
            } else {
                let url = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
                    .appendingPathExtension((name as NSString).pathExtension)
                do {
                    try bytes.write(to: url, options: .atomic)
                    temporary = url
                    player = AVPlayer(url: url)
                } catch { failure = error.localizedDescription }
            }
        }
        .onDisappear { clear() }
    }

    private func clear() {
        player?.pause()
        player = nil
        document = nil
        if let temporary { try? FileManager.default.removeItem(at: temporary) }
        temporary = nil
    }
}

private struct ReadOnlyPDF: UIViewRepresentable {
    let document: PDFDocument
    func makeUIView(context: Context) -> PDFView {
        let view = PDFView()
        view.autoScales = true
        view.displayMode = .singlePageContinuous
        return view
    }
    func updateUIView(_ view: PDFView, context: Context) { view.document = document }
}

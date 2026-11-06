import CodyncKit
import Foundation
import SwiftUI
import WebKit

struct RichContentPreview: View {
    let source: String
    let language: String
    var allowMarkdownPreview = true
    @State private var showSource: Bool?

    private var markdown: Bool { allowMarkdownPreview && ["md", "markdown"].contains(language) }
    private var canPreview: Bool { !source.isEmpty && (markdown || (source.utf16.count <= 16000 && !["md", "markdown"].contains(language))) }
    private var preview: Bool { canPreview && !(showSource ?? !(markdown || ["mermaid", "math", "latex", "tex"].contains(language))) }
    private var height: CGFloat {
        if ["mermaid", "math", "latex", "tex"].contains(language) { return 280 }
        return CGFloat(min(360, max(64, source.components(separatedBy: "\n").count * 18 + 20)))
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack {
                Text(language.isEmpty ? "text" : language).font(.caption).foregroundStyle(Palette.secondary).lineLimit(1)
                Spacer()
                if canPreview {
                    Button(preview ? "Code" : "Preview") { withAnimation { showSource = preview } }
                        .buttonStyle(.plain).font(.caption).foregroundStyle(Palette.secondary)
                        .accessibilityLabel(preview ? "Show code" : "Show preview")
                }
                IconButton("Copy", systemImage: "square.on.square") { Pasteboard.copy(source) }
            }
            .padding(.leading, 10).padding(.trailing, 4)
            if !preview {
                ScrollView(.horizontal, showsIndicators: false) {
                    Text(source).font(.system(.footnote, design: .monospaced))
                        .foregroundStyle(Palette.text).textSelection(.enabled).padding(10)
                }
            } else if markdown {
                AnyView(MarkdownText(source, allowMarkdownPreview: false)).padding(12)
            } else if let html = RichRenderer.html(source: source, language: language) {
                RichContentWebView(html: html).frame(height: height)
            } else {
                Text("Preview couldn’t load. Choose Code to view the source.").font(.caption).padding(12)
            }
        }
        .background(Palette.codeBackground, in: RoundedRectangle(cornerRadius: 8))
    }
}

private enum RichRenderer {
    static let template: String? = {
        guard let url = Bundle.module.url(forResource: "rich-renderer", withExtension: "html") else { return nil }
        return try? String(contentsOf: url, encoding: .utf8)
    }()

    static func html(source: String, language: String) -> String? {
        let kind = language == "mermaid" ? "mermaid" : ["math", "latex", "tex"].contains(language) ? "math" : "code"
        guard let template, let data = try? JSONSerialization.data(withJSONObject: ["kind": kind, "source": source, "language": language]) else { return nil }
        return template.replacingOccurrences(of: "__CODYNC_PAYLOAD__", with: data.base64EncodedString())
    }
}

private struct RichContentWebView: UIViewRepresentable {
    let html: String

    func makeCoordinator() -> Coordinator { Coordinator() }
    func makeUIView(context: Context) -> WKWebView {
        let configuration = WKWebViewConfiguration()
        configuration.websiteDataStore = .nonPersistent()
        let view = WKWebView(frame: .zero, configuration: configuration)
        view.navigationDelegate = context.coordinator
        view.isOpaque = false
        view.backgroundColor = .clear
        return view
    }
    func updateUIView(_ view: WKWebView, context: Context) {
        guard context.coordinator.shown != html else { return }
        context.coordinator.shown = html
        view.loadHTMLString(html, baseURL: nil)
    }
    static func dismantleUIView(_ view: WKWebView, coordinator: Coordinator) {
        view.stopLoading()
        view.navigationDelegate = nil
    }

    final class Coordinator: NSObject, WKNavigationDelegate {
        var shown: String?
        func webView(_ webView: WKWebView, decidePolicyFor action: WKNavigationAction,
                     decisionHandler: @escaping @MainActor @Sendable (WKNavigationActionPolicy) -> Void) {
            decisionHandler(action.request.url?.scheme == "about" ? .allow : .cancel)
        }
    }
}

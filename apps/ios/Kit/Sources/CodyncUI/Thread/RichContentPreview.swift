import CodyncKit
import Foundation
import SwiftUI
import WebKit

struct RichContentPreview: View {
    let source: String
    let language: String
    var allowMarkdownPreview = true
    @ScaledMetric(relativeTo: .body) private var inlineFontSize: CGFloat = 17
    @State private var showSource: Bool?
    @State private var expanded = false
    @State private var measuredHeight: CGFloat = 48
    private var inlineMath: Bool { language == "inline-math" }

    private var markdown: Bool { allowMarkdownPreview && ["md", "markdown"].contains(language) }
    private var canPreview: Bool { !source.isEmpty && (markdown || (source.utf16.count <= 16000 && !["md", "markdown"].contains(language))) }
    private var preview: Bool { canPreview && !(showSource ?? !(markdown || ["mermaid", "math", "latex", "tex", "inline-math"].contains(language))) }
    private var height: CGFloat {
        if ["mermaid", "math", "latex", "tex", "inline-math"].contains(language) { return 280 }
        return CGFloat(min(360, max(64, source.components(separatedBy: "\n").count * 18 + 20)))
    }

    private var layoutHandler: (@MainActor (CGFloat) -> Void)? {
        guard inlineMath else { return nil }
        return { height in measuredHeight = height }
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            if !inlineMath { toolbar }
            content
        }
        .background(inlineMath ? Color.clear : Palette.codeBackground, in: RoundedRectangle(cornerRadius: 8))
        .codyncSheet(isPresented: $expanded) { expandedPreview }
    }

    private var toolbar: some View {
        HStack {
            Text(language.isEmpty ? "text" : language).font(.caption).foregroundStyle(Palette.secondary).lineLimit(1)
            Spacer()
            if canPreview {
                Button(preview ? "Code" : "Preview") { withAnimation { showSource = preview } }
                    .buttonStyle(.plain).font(.caption).foregroundStyle(Palette.secondary)
                    .accessibilityLabel(preview ? "Show code" : "Show preview")
            }
            if preview && !markdown {
                IconButton("Expand preview", systemImage: "arrow.up.left.and.arrow.down.right") { expanded = true }
            }
            IconButton("Copy", systemImage: "square.on.square") { Pasteboard.copy(source) }
        }
        .padding(.leading, 10).padding(.trailing, 4)
    }

    @ViewBuilder private var content: some View {
        if !preview {
            ScrollView(.horizontal, showsIndicators: false) {
                Text(source).font(.system(.footnote, design: .monospaced))
                    .foregroundStyle(Palette.text).textSelection(.enabled).padding(10)
            }
        } else if markdown {
            AnyView(MarkdownText(source, allowMarkdownPreview: false)).padding(12)
        } else if let html = RichRenderer.html(source: source, language: language, fontSize: inlineMath ? Int(inlineFontSize) : 17) {
            RichContentWebView(html: html, layout: layoutHandler)
                .frame(height: inlineMath ? measuredHeight : height)
        } else {
            Text(inlineMath ? source : "Preview couldn’t load. Choose Code to view the source.").font(inlineMath ? .body : .caption).textSelection(.enabled).padding(12)
        }
    }

    private var expandedPreview: some View {
        VStack(spacing: 0) {
            ModalHeader("Preview") {
                IconButton("Copy", systemImage: "square.on.square") { Pasteboard.copy(source) }
            }
            if let html = RichRenderer.html(source: source, language: language, fontSize: inlineMath ? Int(inlineFontSize) : 17) {
                RichContentWebView(html: html).frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }.background(Palette.background)
    }

}

private enum RichRenderer {
    static let template: String? = {
        guard let url = Bundle.module.url(forResource: "rich-renderer", withExtension: "html") else { return nil }
        return try? String(contentsOf: url, encoding: .utf8)
    }()

    static func html(source: String, language: String, fontSize: Int) -> String? {
        let kind = language == "inline-math" ? "inline-math" : language == "mermaid" ? "mermaid" : ["math", "latex", "tex"].contains(language) ? "math" : "code"
        guard let template, let data = try? JSONSerialization.data(withJSONObject: ["kind": kind, "source": source, "language": language, "fontSize": fontSize]) else { return nil }
        return template.replacingOccurrences(of: "__CODYNC_PAYLOAD__", with: data.base64EncodedString())
    }
}

private struct RichContentWebView: UIViewRepresentable {
    let html: String
    var layout: (@MainActor (CGFloat) -> Void)? = nil

    func makeCoordinator() -> Coordinator { Coordinator(layout: layout) }
    func makeUIView(context: Context) -> WKWebView {
        let configuration = WKWebViewConfiguration()
        configuration.websiteDataStore = .nonPersistent()
        if layout != nil { configuration.userContentController.add(context.coordinator, name: "layout") }
        let view = WKWebView(frame: .zero, configuration: configuration)
        view.navigationDelegate = context.coordinator
        view.isOpaque = false
        view.backgroundColor = .clear
        return view
    }
    func updateUIView(_ view: WKWebView, context: Context) {
        context.coordinator.layout = layout
        guard context.coordinator.shown != html else { return }
        context.coordinator.shown = html
        view.loadHTMLString(html, baseURL: nil)
    }
    static func dismantleUIView(_ view: WKWebView, coordinator: Coordinator) {
        view.stopLoading()
        view.navigationDelegate = nil
        view.configuration.userContentController.removeScriptMessageHandler(forName: "layout")
    }

    @MainActor final class Coordinator: NSObject, WKNavigationDelegate, WKScriptMessageHandler {
        var shown: String?
        var layout: (@MainActor (CGFloat) -> Void)?
        init(layout: (@MainActor (CGFloat) -> Void)?) { self.layout = layout }
        func userContentController(_ controller: WKUserContentController, didReceive message: WKScriptMessage) {
            guard message.frameInfo.isMainFrame, let value = message.body as? NSNumber else { return }
            let height = value.doubleValue
            guard height.isFinite, height >= 24, height <= 1200 else { return }
            layout?(CGFloat(height))
        }
        func webView(_ webView: WKWebView, decidePolicyFor action: WKNavigationAction,
                     decisionHandler: @escaping @MainActor @Sendable (WKNavigationActionPolicy) -> Void) {
            decisionHandler(action.request.url?.scheme == "about" ? .allow : .cancel)
        }
    }
}

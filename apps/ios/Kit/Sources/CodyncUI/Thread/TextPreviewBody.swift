import CodyncKit
import SwiftUI

/// Shared presentation for verified snapshots and host-confined workspace text.
struct TextPreviewBody: View {
    let name: String
    let text: String?
    let failure: String?
    let retry: () -> Void
    @State private var source = false
    @State private var wrap = true

    private var markdown: Bool {
        ["md", "markdown"].contains((name as NSString).pathExtension.lowercased())
    }

    var body: some View {
        VStack(spacing: 0) {
            ModalHeader(name)
            HStack(spacing: 4) {
                Spacer(minLength: 0)
                if markdown {
                    Button(source ? "Read" : "Source") {
                        withAnimation(Motion.layout) { source.toggle() }
                    }
                    .buttonStyle(.plain).font(.caption).foregroundStyle(Palette.secondary)
                    .accessibilityLabel(source ? "Read Markdown" : "Show source")
                }
                if !markdown || source {
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
            ScrollView(markdown && !source || wrap ? .vertical : [.vertical, .horizontal]) {
                Group {
                    if let text {
                        if markdown && !source {
                            MarkdownText(text)
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
    }
}

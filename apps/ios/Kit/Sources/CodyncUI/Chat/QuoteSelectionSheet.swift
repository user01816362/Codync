import CodyncKit
import SwiftUI
import UIKit

struct QuoteSelectionTarget: Identifiable {
    let id = UUID()
    let text: String
    let author: String
    var part: String? = nil
}

/// A native selection surface preserves the exact selected source and the platform's Copy menu.
struct QuoteSelectionSheet: View {
    let text: String
    let author: String
    var structured = false
    let quote: (String) -> Void
    @Environment(\.dismissModal) private var dismiss
    @State private var selected = ""

    var body: some View {
        VStack(spacing: 12) {
            ModalHeader("Select text")
            Text(author).font(.subheadline).foregroundStyle(Palette.secondary).padding(.horizontal, 20)
            SelectionTextView(text: text, selected: $selected).frame(maxWidth: .infinity, maxHeight: .infinity)
            Button("Quote selected text") {
                let passage = String(selected.trimmingCharacters(in: .whitespacesAndNewlines).prefix(4000))
                guard !passage.isEmpty else { return }
                quote(structured ? passage : author + ":\n" + passage.components(separatedBy: .newlines).map { "> " + $0 }.joined(separator: "\n") + "\n\n")
                dismiss()
            }
            .buttonStyle(SecondaryButtonStyle())
            .disabled(selected.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
            .padding(.bottom, 16)
        }
        .background(Palette.background)
    }
}

private struct SelectionTextView: UIViewRepresentable {
    let text: String
    @Binding var selected: String
    func makeCoordinator() -> Coordinator { Coordinator(selected: $selected) }
    func makeUIView(context: Context) -> UITextView {
        let view = UITextView()
        view.isEditable = false
        view.isSelectable = true
        view.font = .preferredFont(forTextStyle: .body)
        view.adjustsFontForContentSizeCategory = true
        view.textColor = UIColor(Palette.text)
        view.backgroundColor = .clear
        view.textContainerInset = UIEdgeInsets(top: 12, left: 20, bottom: 12, right: 20)
        view.delegate = context.coordinator
        view.text = text
        return view
    }
    func updateUIView(_ view: UITextView, context: Context) {
        context.coordinator.selected = $selected
        if view.text != text { view.text = text }
    }
    final class Coordinator: NSObject, UITextViewDelegate {
        var selected: Binding<String>
        init(selected: Binding<String>) { self.selected = selected }
        func textViewDidChangeSelection(_ textView: UITextView) {
            guard let range = textView.selectedTextRange else { selected.wrappedValue = ""; return }
            selected.wrappedValue = textView.text(in: range) ?? ""
        }
    }
}

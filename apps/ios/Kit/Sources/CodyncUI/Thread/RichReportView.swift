import CodyncKit
import SwiftUI

struct RichReportView: View {
    let report: RichReport
    let submit: (String) -> Void
    @State private var answers: [String: String] = [:]
    @State private var sent: Set<Int> = []
    @State private var closed: Set<Int> = []

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text(report.title).font(.headline)
            ForEach(Array(report.sections.prefix(24).enumerated()), id: \.offset) { index, section in
                VStack(alignment: .leading, spacing: 8) {
                    Button {
                        withAnimation { if closed.contains(index) { closed.remove(index) } else { closed.insert(index) } }
                    } label: {
                        HStack {
                            Image(systemName: closed.contains(index) ? "chevron.right" : "chevron.down")
                            Text(section.heading).font(.subheadline.bold())
                        }
                    }
                    .buttonStyle(.plain)
                    .accessibilityValue(closed.contains(index) ? "Collapsed" : "Expanded")
                    if !closed.contains(index) {
                        if let text = section.text, !text.isEmpty { Text(text).font(.body) }
                        let items = Array((section.items ?? []).prefix(30))
                        let maximum = items.compactMap { Double($0.value) }.filter { $0.isFinite }.max() ?? 0
                        ForEach(Array(items.enumerated()), id: \.offset) { itemIndex, item in
                            VStack(alignment: .leading, spacing: 4) {
                                HStack(alignment: .firstTextBaseline) {
                                    Text(item.label).font(.subheadline)
                                    Spacer()
                                    if section.kind == "form" {
                                        TextField(item.value, text: Binding(get: { answers["\(index):\(itemIndex)", default: ""] }, set: { answers["\(index):\(itemIndex)"] = String($0.prefix(500)) }))
                                            .textFieldStyle(.plain).accessibilityLabel(item.label).disabled(sent.contains(index))
                                    } else {
                                        Text(item.value).font(.subheadline).foregroundStyle(Palette.secondary)
                                    }
                                }
                                if section.kind == "bar" {
                                    GeometryReader { proxy in
                                        Capsule().fill(Palette.secondary).frame(width: proxy.size.width * ratio(item.value, maximum))
                                    }
                                    .frame(height: 6)
                                    .accessibilityHidden(true)
                                }
                            }
                        }
                        if section.kind == "form" {
                            Button(sent.contains(index) ? "Sent" : "Send answers") {
                                let lines = items.enumerated().map { i, item in "\(item.label): \(answers["\(index):\(i)", default: ""])" }
                                submit("\(report.title) — \(section.heading)\n" + lines.joined(separator: "\n"))
                                sent.insert(index)
                            }
                            .buttonStyle(SecondaryButtonStyle())
                            .disabled(sent.contains(index) || items.indices.contains { answers["\(index):\($0)", default: ""].trimmingCharacters(in: .whitespacesAndNewlines).isEmpty })
                        }
                        ForEach(section.citations ?? [], id: \.self) { citation in
                            if citation > 0, let sources = report.sources, citation <= sources.count {
                                sourceLink(sources[citation - 1], number: citation)
                            }
                        }
                    }
                }
            }
            if let sources = report.sources, !sources.isEmpty {
                Text("Sources supplied by the agent").font(.caption).foregroundStyle(Palette.secondary)
                ForEach(Array(sources.prefix(30).enumerated()), id: \.offset) { index, source in
                    sourceLink(source, number: index + 1)
                }
            }
        }
        .foregroundStyle(Palette.text)
        .textSelection(.enabled)
    }

    @ViewBuilder private func sourceLink(_ source: RichReport.Source, number: Int) -> some View {
        if let url = URL(string: source.url), ["http", "https"].contains(url.scheme?.lowercased() ?? "") {
            Link("[\(number)] \(source.title)", destination: url).font(.caption).underline()
        }
    }

    private func ratio(_ value: String, _ maximum: Double) -> Double {
        guard let number = Double(value), number.isFinite, maximum > 0 else { return 0 }
        return min(1, max(0, number / maximum))
    }
}

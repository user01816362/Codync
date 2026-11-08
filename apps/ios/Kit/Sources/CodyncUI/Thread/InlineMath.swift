import Foundation

/// Plain prose plus bounded formulas; styled/link/code paragraphs retain the native Markdown path.
enum InlineMath {
    private static let expression = try? NSRegularExpression(pattern: #"\\\(([^\n]{1,1024}?)\\\)|(?<!\\)\$([^$\n]{1,1024}?)\$"#)
    static func eligible(_ text: String) -> Bool {
        guard text.utf16.count <= 4000, !text.contains("$$"), let expression else { return false }
        let original = text as NSString
        let remaining = NSMutableString(string: text)
        var found = false
        for match in expression.matches(in: text, range: NSRange(location: 0, length: original.length)).reversed() {
            let explicit = match.range(at: 1).location != NSNotFound
            let range = match.range(at: explicit ? 1 : 2)
            let math = original.substring(with: range)
            let valid = explicit || (math.trimmingCharacters(in: .whitespacesAndNewlines) == math &&
                (math.range(of: #"[\\^_=+*/{}]"#, options: .regularExpression) != nil ||
                 math.range(of: #"^[A-Za-z]$"#, options: .regularExpression) != nil))
            if valid { found = true; remaining.replaceCharacters(in: match.range, with: "") }
        }
        return found && (remaining as String).range(of: #"[`*_~\[\]<>\\]"#, options: .regularExpression) == nil
    }
}

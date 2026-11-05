import CodyncKit

/// What the reader chose to see: a version, and how many versions the message had when they chose it.
struct VersionPick: Equatable {
    let version: String
    /// A newer edit (more versions) shows itself instead of leaving an old pick in place.
    let of: Int
}

/// One message the user edited: every version, oldest first, and the one that keeps its place in the chat.
struct MessageVersions {
    /// The first version the chat holds; the message is shown where it is.
    let anchor: Entry
    let versions: [Entry]

    /// The edited messages among `entries`, by the id of their anchor. Each edit is its own entry (so its turn
    /// and answers stay together), and points at the first version with `editOf`.
    static func groups(_ entries: [Entry]) -> [String: MessageVersions] {
        let held = Dictionary(entries.map { ($0.id, $0) }, uniquingKeysWith: { a, _ in a })
        var byRoot: [String: [Entry]] = [:]
        for entry in entries where entry.kind == "user" {
            guard let root = entry.data.editOf else { continue }
            byRoot[root, default: []].append(entry)
        }
        var groups: [String: MessageVersions] = [:]
        for (root, edits) in byRoot {
            var versions = edits
            if let first = held[root] { versions.insert(first, at: 0) }
            versions.sort { ($0.data.version ?? 1, $0.seq) < ($1.data.version ?? 1, $1.seq) }
            if let anchor = versions.first {
                groups[anchor.id] = MessageVersions(anchor: anchor, versions: versions)
            }
        }
        return groups
    }

    /// The version a chat shows: the one the reader picked, or the newest.
    static func selected(_ group: MessageVersions, pick: VersionPick?) -> Entry {
        let newest = group.versions.last!
        guard let pick, pick.of == group.versions.count,
              let chosen = group.versions.first(where: { $0.id == pick.version }) else { return newest }
        return chosen
    }
}

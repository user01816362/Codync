import Foundation

/// An immutable host snapshot published by a bot. Separate from user uploads.
public struct SharedFile: Codable, Hashable, Sendable, Identifiable {
    public let id: String
    public let name: String
    public let size: Int64
    public let sha256: String

    public init(id: String, name: String, size: Int64, sha256: String) {
        self.id = id
        self.name = name
        self.size = size
        self.sha256 = sha256
    }

    /// An image small enough to show in the chat (the phone fetches all of it).
    public var showsAsImage: Bool { Attachment.isImage(name) && size <= 20 * 1024 * 1024 }

    /// A small text file can be opened without handing its contents to another app.
    public var showsAsText: Bool {
        Self.canPreview(name: name, size: size)
    }

    public static func canPreview(name: String, size: Int64) -> Bool {
        let extensions = Set("md markdown txt log json jsonl yaml yml toml xml csv tsv html css js jsx ts tsx py rs go swift kt kts java c h cpp hpp sh bash zsh sql diff patch".split(separator: " "))
        let suffix = name.lowercased().split(separator: ".").last ?? ""
        return name.contains(".") && size >= 0 && size <= 1024 * 1024 && extensions.contains(suffix)
    }

    public func validate() throws {
        guard size >= 0, size <= 100 * 1024 * 1024,
              sha256.count == 64, sha256.allSatisfy({ "0123456789abcdef".contains($0) }),
              !name.isEmpty, name.utf8.count <= 200, name != ".", name != "..",
              !name.contains("/"), !name.contains("\\"),
              !name.unicodeScalars.contains(where: { CharacterSet.controlCharacters.contains($0) }) else {
            throw FileDownloadError.invalidMetadata
        }
    }
}

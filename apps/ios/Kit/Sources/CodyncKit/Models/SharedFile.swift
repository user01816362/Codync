import Foundation

/// An immutable host snapshot published by a bot. Separate from user uploads.
public struct SharedFile: Codable, Hashable, Sendable, Identifiable {
    public let id: String
    public let name: String
    public let size: Int64
    public let sha256: String
    /// A sound the chat plays in place, as the computer stamps it; missing (an older computer) means a plain file card.
    public let playsInChat: Bool

    public init(id: String, name: String, size: Int64, sha256: String, playsInChat: Bool = false) {
        self.id = id
        self.name = name
        self.size = size
        self.sha256 = sha256
        self.playsInChat = playsInChat
    }

    public init(from decoder: any Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        self.init(id: try c.decode(String.self, forKey: .id), name: try c.decode(String.self, forKey: .name),
                  size: try c.decode(Int64.self, forKey: .size), sha256: try c.decode(String.self, forKey: .sha256),
                  playsInChat: try c.decodeIfPresent(Bool.self, forKey: .playsInChat) ?? false)
    }

    /// The largest shared image the phone fetches whole to show.
    private static let inlineBytes: Int64 = 20 * 1024 * 1024
    /// An image small enough to show in the chat.
    public var showsAsImage: Bool { Attachment.isImage(name) && size <= Self.inlineBytes }

    /// A small text file can be opened without handing its contents to another app.
    public var showsAsText: Bool {
        Self.canPreview(name: name, size: size)
    }

    public static func canPreview(name: String, size: Int64) -> Bool {
        let extensions = Set("md markdown txt log json jsonl yaml yml toml xml csv tsv html css js jsx ts tsx py rs go swift kt kts java c h cpp hpp sh bash zsh sql diff patch".split(separator: " "))
        let suffix = name.lowercased().split(separator: ".").last ?? ""
        return name.contains(".") && size >= 0 && size <= 1024 * 1024 && extensions.contains(suffix)
    }

    public var showsAsPreview: Bool { Self.canOpenPreview(name: name, size: size) }

    public static func canOpenPreview(name: String, size: Int64) -> Bool {
        canPreview(name: name, size: size) || mediaKind(name: name, size: size) != nil
    }

    public static func mediaKind(name: String, size: Int64) -> String? {
        guard size >= 0, size <= 32 * 1024 * 1024 else { return nil }
        switch (name as NSString).pathExtension.lowercased() {
        case "pdf": return "pdf"
        case "mp3", "m4a", "wav", "ogg": return "audio"
        case "mp4", "m4v", "mov", "webm": return "video"
        default: return nil
        }
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

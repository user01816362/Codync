import Foundation

/// Inert, host-validated structured content; message text is the compatibility fallback.
public struct RichReport: Codable, Hashable, Sendable {
    public var version: Int
    public var title: String
    public var sections: [Section]
    public var sources: [Source]?

    public struct Section: Codable, Hashable, Sendable {
        public var kind: String
        public var heading: String
        public var text: String?
        public var items: [Item]?
        public var citations: [Int]?
    }
    public struct Item: Codable, Hashable, Sendable {
        public var label: String
        public var value: String
    }
    public struct Source: Codable, Hashable, Sendable {
        public var title: String
        public var url: String
    }
}

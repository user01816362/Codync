import Foundation

/// Tailscale on a computer, as its host sees it (`reachStatus`). The phone's own
/// Tailscale state is [`Tailscale.isConnected`], read from this device's interfaces.
public struct TailscaleReach: Codable, Hashable, Sendable {
    public var installed: Bool
    public var running: Bool
    public var canOpen: Bool
    /// The MagicDNS name, without the trailing dot (`mac.example.ts.net`).
    public var dnsName: String?
    /// The first IPv4 address on the tailnet (`100.x.y.z`).
    public var ipv4: String?
    public var download: String

    public init(installed: Bool, running: Bool, canOpen: Bool = false, dnsName: String? = nil, ipv4: String? = nil, download: String) {
        self.installed = installed
        self.running = running
        self.canOpen = canOpen
        self.dnsName = dnsName
        self.ipv4 = ipv4
        self.download = download
    }
}

/// A named Cloudflare Tunnel on a computer (`reachStatus`). Locally managed, the way
/// Cloudflare documents it: the account certificate and tunnel credentials live on the
/// computer, and only the phone channel is served through it.
public struct CloudflareReach: Codable, Hashable, Sendable {
    public var installed: Bool
    public var loggedIn: Bool
    public var configured: Bool
    public var enabled: Bool
    public var tunnel: String?
    public var hostname: String?
    public var url: String?
    public var running: Bool
    /// A browser sign-in is waiting; its link can be opened on this phone or the host computer.
    public var loginPending: Bool
    public var loginUrl: String?
    public var lastError: String?
    public var download: String

    public init(installed: Bool, loggedIn: Bool, configured: Bool, enabled: Bool, tunnel: String? = nil,
                hostname: String? = nil, url: String? = nil, running: Bool, loginPending: Bool,
                loginUrl: String? = nil, lastError: String? = nil, download: String) {
        self.installed = installed
        self.loggedIn = loggedIn
        self.configured = configured
        self.enabled = enabled
        self.tunnel = tunnel
        self.hostname = hostname
        self.url = url
        self.running = running
        self.loginPending = loginPending
        self.loginUrl = loginUrl
        self.lastError = lastError
        self.download = download
    }
}

/// Reaching a computer without the Codync cloud (`reachStatus`).
public struct ReachStatus: Codable, Hashable, Sendable {
    public var tailscale: TailscaleReach
    public var cloudflare: CloudflareReach

    public init(tailscale: TailscaleReach, cloudflare: CloudflareReach) {
        self.tailscale = tailscale
        self.cloudflare = cloudflare
    }
}

public extension HostClient {
    func reachStatus() async throws -> ReachStatus {
        try await call("reachStatus")
    }

    /// Starts `cloudflared tunnel login` on the computer; `loginPending` and `loginUrl` follow.
    func reachCloudflareLogin() async throws -> ReachStatus {
        try await call("reachCloudflareLogin", timeout: 30)
    }

    /// Creates the tunnel and its DNS record, then runs it against the loopback channel listener.
    func reachCloudflareCreate(hostname: String) async throws -> ReachStatus {
        try await call("reachCloudflareCreate", ["hostname": hostname], timeout: 120)
    }

    func reachCloudflareSetEnabled(_ enabled: Bool) async throws -> ReachStatus {
        try await call("reachCloudflareSetEnabled", ["enabled": enabled], timeout: 30)
    }

    /// Stops it here and forgets the hostname; the tunnel and DNS record stay in the Cloudflare account.
    func reachCloudflareForget() async throws -> ReachStatus {
        try await call("reachCloudflareForget", timeout: 30)
    }

    /// Opens Tailscale on the computer, for a person to sign in there.
    func reachTailscaleOpen() async throws -> ReachStatus {
        try await call("reachTailscaleOpen", timeout: 30)
    }
}

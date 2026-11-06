import CodyncKit
import CodyncUI
import SwiftUI

/// Reaching this computer away from home without a Codync account: Tailscale's private
/// network, or a named Cloudflare Tunnel. Both end at the same end-to-end encrypted
/// channel the phone already uses; neither goes through Codync's cloud.
struct ReachSheet: View {
    @Environment(BotStore.self) private var store
    @Environment(\.openURL) private var openURL
    @Environment(\.scenePhase) private var scenePhase
    @State private var status: ReachStatus?
    @State private var hostname = ""
    @State private var busy = false
    @State private var error: String?
    @State private var confirmForget = false
    @State private var tailscaleOn = Tailscale.isConnected

    private var cloudflare: CloudflareReach? { status?.cloudflare }

    var body: some View {
        VStack(spacing: 0) {
            ModalHeader("Reach from anywhere") {
                if busy { Spinner() }
                IconButton("Check again", systemImage: "arrow.clockwise") { Task { await refresh() } }
                    .disabled(busy)
            }
            CardForm {
                CardSection("Tailscale", footer: "Install Tailscale on this computer and on this iPhone, then sign in to both. Codync connects straight over your tailnet; no Codync account is involved.") {
                    row {
                        VStack(alignment: .leading, spacing: 6) {
                            Text(hostLine)
                                .foregroundStyle(Palette.text)
                                .fixedSize(horizontal: false, vertical: true)
                            Text(phoneLine)
                                .font(.footnote)
                                .foregroundStyle(tailscaleOn ? Palette.secondary : Palette.warning)
                                .fixedSize(horizontal: false, vertical: true)
                            HStack(spacing: 8) {
                                if status?.tailscale.installed == false {
                                    Button("Get Tailscale") { openURL(Tailscale.downloadURL) }
                                        .buttonStyle(.secondary)
                                } else if status?.tailscale.running == false, status?.tailscale.canOpen == true {
                                    Button("Open on this computer") { openTailscale() }
                                        .buttonStyle(.secondary)
                                        .disabled(busy || store.connection != .online)
                                }
                                if !tailscaleOn {
                                    Button("Get Tailscale for iPhone") {
                                        if let url = URL(string: "https://tailscale.com/docs/install/ios") { openURL(url) }
                                    }
                                        .buttonStyle(.secondary)
                                }
                            }
                        }
                    }
                }

                CardSection("Cloudflare Tunnel", footer: "A named tunnel from a domain in your Cloudflare account. Only Codync's end-to-end encrypted phone channel is served through it, never the computer's files, terminals or local API.") {
                    row {
                        VStack(alignment: .leading, spacing: 6) {
                            Text(cloudflareLine)
                                .foregroundStyle(cloudflare?.lastError == nil ? Palette.text : Palette.warning)
                                .fixedSize(horizontal: false, vertical: true)
                            cloudflareActions
                        }
                    }
                }

                if let error {
                    Text(L10n.tr(error))
                        .font(.caption)
                        .foregroundStyle(Palette.danger)
                        .textSelection(.enabled)
                }
            }
        }
        .background(Palette.background)
        .task { await refresh() }
        .onChange(of: scenePhase) { _, phase in
            // Back from the Tailscale app: its VPN interface is visible now.
            if phase == .active { tailscaleOn = Tailscale.isConnected }
        }
        .task(id: cloudflare?.loginPending) {
            guard cloudflare?.loginPending == true else { return }
            while !Task.isCancelled {
                try? await Task.sleep(for: .seconds(2))
                await refresh()
                if cloudflare?.loginPending != true { break }
            }
        }
        .codyncDialog("Forget this Cloudflare Tunnel here?", isPresented: $confirmForget,
                      message: "The tunnel stops and this computer forgets its hostname. The tunnel and DNS record stay in your Cloudflare account; delete them there if you no longer want them.") {
            [DialogAction("Forget", destructive: true) {
                run("Forget this tunnel") { try await $0.reachCloudflareForget() }
            }]
        }
    }

    private var hostLine: String {
        guard let ts = status?.tailscale else { return L10n.tr("Checking…") }
        if !ts.installed { return L10n.tr("Tailscale isn't installed on this computer.") }
        if !ts.running { return L10n.tr("Tailscale is installed on this computer, but not signed in or not running.") }
        if let name = ts.dnsName, !name.isEmpty { return L10n.tr("This computer is on your tailnet at {name}.").replacingOccurrences(of: "{name}", with: name) }
        if let ip = ts.ipv4, !ip.isEmpty { return L10n.tr("This computer is on your tailnet at {ip}.").replacingOccurrences(of: "{ip}", with: ip) }
        return L10n.tr("This computer is running on your tailnet.")
    }

    private var phoneLine: String {
        tailscaleOn
            ? L10n.tr("This iPhone is on Tailscale.")
            : L10n.tr("This iPhone isn't on Tailscale yet. Install it to connect away from home without Cloudflare.")
    }

    private var cloudflareLine: String {
        guard let cf = cloudflare else { return L10n.tr("Checking…") }
        if !cf.installed { return L10n.tr("Install cloudflared on this computer first.") }
        if cf.loginPending { return L10n.tr("Finish signing in in a browser on this device or the computer running Codync.") }
        if let lastError = cf.lastError, !lastError.isEmpty { return lastError }
        if !cf.loggedIn { return L10n.tr("Sign in to the Cloudflare account that holds your domain.") }
        if !cf.configured { return L10n.tr("Choose a hostname from a domain in that account.") }
        if cf.running, let url = cf.url { return L10n.tr("Tunnel process running for {url}.").replacingOccurrences(of: "{url}", with: url) }
        return L10n.tr("Stopped. Turn it on to reach this computer from anywhere.")
    }

    @ViewBuilder private var cloudflareActions: some View {
        if let cf = cloudflare {
            if !cf.installed {
                Button("Install cloudflared") {
                    if let url = URL(string: cf.download) { openURL(url) }
                }
                .buttonStyle(.secondary)
            } else if cf.loginPending {
                Button("Continue Cloudflare sign-in") {
                    if let url = cf.loginUrl.flatMap(URL.init(string:)) { openURL(url) }
                }
                .buttonStyle(.secondary)
                .disabled(cf.loginUrl == nil)
            } else if !cf.loggedIn {
                Button("Sign in to Cloudflare") {
                    run("Sign in") { try await $0.reachCloudflareLogin() }
                }
                .buttonStyle(.secondary)
                .disabled(busy)
            } else if !cf.configured {
                HStack(spacing: 8) {
                    TextField("codync.example.com", text: $hostname)
                        .textInputAutocapitalization(.never)
                        .autocorrectionDisabled()
                        .textFieldStyle(.roundedBorder)
                    Button("Create tunnel") {
                        let host = hostname.trimmingCharacters(in: .whitespacesAndNewlines)
                        guard !host.isEmpty else { return }
                        run("Create the tunnel") { try await $0.reachCloudflareCreate(hostname: host) }
                    }
                    .buttonStyle(.secondary)
                    .disabled(busy || hostname.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                }
            } else {
                Toggle(isOn: Binding(
                    get: { cf.enabled },
                    set: { on in run(on ? "Turn the tunnel on" : "Turn the tunnel off") { try await $0.reachCloudflareSetEnabled(on) } }
                )) {
                    VStack(alignment: .leading, spacing: 2) {
                        Text(cf.hostname ?? L10n.tr("Cloudflare Tunnel"))
                        Text(cf.running ? L10n.tr("Tunnel running") : L10n.tr("Tunnel stopped"))
                            .font(.footnote)
                            .foregroundStyle(Palette.secondary)
                    }
                }
                .disabled(busy)
                HStack(spacing: 8) {
                    Button("Restart tunnel") {
                        run("Restart the tunnel") { try await $0.reachCloudflareSetEnabled(true) }
                    }
                    .buttonStyle(.secondary)
                    .disabled(busy)
                    Button("Forget this tunnel") { confirmForget = true }
                        .buttonStyle(.secondary)
                        .disabled(busy)
                }
            }
        }
    }

    private func refresh() async {
        guard let client = store.client, store.connection == .online else { return }
        do {
            status = try await client.reachStatus()
            error = nil
        } catch {
            self.error = error.localizedDescription
        }
    }

    private func openTailscale() {
        run("Open Tailscale") { try await $0.reachTailscaleOpen() }
    }

    private func run(_ what: String, _ action: @escaping (HostClient) async throws -> ReachStatus) {
        guard let client = store.client else {
            error = "This computer isn't connected."
            return
        }
        busy = true
        error = nil
        Task {
            defer { busy = false }
            do {
                status = try await action(client)
            } catch {
                self.error = error.localizedDescription
            }
        }
    }

    /// One card row, without the hairline padding a list row gets.
    private func row<Content: View>(@ViewBuilder _ content: () -> Content) -> some View {
        content()
    }
}

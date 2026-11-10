import AuthenticationServices
import ClerkKit
import CodyncKit
import CodyncUI
import Foundation
import Observation

/// Clerk owns OAuth, session restoration and Keychain persistence. An account session
/// never grants access to a computer by itself: each one still approves this device (spec §4.2).
@MainActor
@Observable
final class AccountSession {
    private var clerk: Clerk?
    private let publishableKey: String?
    private static let consentKey = "accountServicesEnabled"
    private(set) var isBusy = false
    var errorMessage: String?

    var showSwitcher = false

    struct Account: Identifiable {
        let id: String
        let sessionID: String
        let email: String
        let avatarURL: URL?
    }

    var userID: String? { clerk?.session?.status == .active ? clerk?.user?.id : nil }
    var accounts: [Account] {
        var seen = Set<String>()
        return (clerk?.auth.sessions ?? []).compactMap { session in
            guard session.status == .active, session.expireAt > .now,
                  let user = session.user, seen.insert(user.id).inserted else { return nil }
            return Account(id: user.id, sessionID: session.id,
                           email: user.primaryEmailAddress?.emailAddress ?? "Codync account",
                           avatarURL: URL(string: user.imageUrl).flatMap { $0.scheme == "https" ? $0 : nil })
        }
    }
    var supportsMultipleAccounts: Bool { clerk?.environment?.authConfig.singleSessionMode == false }

    var isConfigured: Bool { publishableKey != nil }
    /// Clerk restores the persisted Keychain session during configuration. Don't offer a fresh sign-in
    /// until that restore has completed, or an update can briefly look like a signed-out install.
    var isReady: Bool { clerk?.isLoaded ?? true }
    /// Waits up to `timeout` for that restore; false when it didn't finish in time or the task was cancelled.
    func waitUntilReady(timeout: Duration) async -> Bool {
        let deadline = ContinuousClock.now + timeout
        while !isReady, !Task.isCancelled, ContinuousClock.now < deadline {
            try? await Task.sleep(for: .milliseconds(100))
        }
        return isReady
    }
    /// Clerk gave up restoring (launched offline, an outage) and tries again only when the app comes back to the
    /// foreground, so the welcome offers `retryRestore()`. Clears itself once Clerk loads.
    var restoreFailed: Bool { restoreGaveUp && !isReady }
    private var restoreGaveUp = false

    /// Clerk's launch refresh again: the environment and the client it restores sign-in from.
    func retryRestore() async {
        guard let clerk, restoreFailed else { return }
        restoreGaveUp = false
        _ = try? await clerk.refreshEnvironment()
        _ = try? await clerk.refreshClient()
        restoreGaveUp = !isReady
    }
    /// The Codync cloud (accounts and the encrypted relay); nil in builds without one.
    let cloudURL: URL?
    var isSignedIn: Bool { userID != nil }
    var email: String? { clerk?.user?.primaryEmailAddress?.emailAddress }
    var avatarURL: URL? {
        guard let image = clerk?.user?.imageUrl, let url = URL(string: image), url.scheme == "https" else { return nil }
        return url
    }

    init() {
        let env = ProcessInfo.processInfo.environment
        let config = Bundle.main.url(forResource: "AccountConfig", withExtension: "plist")
            .flatMap { try? Data(contentsOf: $0) }
            .flatMap { try? PropertyListDecoder().decode(Configuration.self, from: $0) }
        cloudURL = (env["CODYNC_CLOUD_URL"] ?? config?.cloudURL)
            .flatMap { URL(string: $0.trimmingCharacters(in: .whitespacesAndNewlines)) }
            .flatMap { Pairing.isCloudURL($0) ? $0 : nil }
        let key = (env["CODYNC_CLERK_PUBLISHABLE_KEY"] ?? config?.clerkPublishableKey ?? "").trimmingCharacters(in: .whitespacesAndNewlines)
        publishableKey = key.hasPrefix("pk_test_") || key.hasPrefix("pk_live_") ? key : nil
        clerk = nil
        if (UserDefaults.standard.bool(forKey: Self.consentKey) && !UserDefaults.standard.bool(forKey: "withoutAccount"))
            || env["CODYNC_DEV_SIGN_IN_TICKET"] != nil {
            enableAccountServices()
        }
    }

    /// Account services start only after the user chooses sign-in, or restores that explicit choice.
    private func enableAccountServices() {
        guard clerk == nil, let key = publishableKey else { return }
        UserDefaults.standard.set(true, forKey: Self.consentKey)
        let scheme = Bundle.main.bundleIdentifier ?? "com.pokai.Codync"
        clerk = Clerk.configure(publishableKey: key, options: .init(
            telemetryEnabled: false,
            redirectConfig: .init(redirectUrl: "\(scheme)://callback", callbackUrlScheme: scheme)
        ))
        Task { restoreGaveUp = !(await waitUntilReady(timeout: .seconds(10))) }
        if key.hasPrefix("pk_test_"), let ticket = ProcessInfo.processInfo.environment["CODYNC_DEV_SIGN_IN_TICKET"], !ticket.isEmpty {
            Task { await signInForDevelopment(ticket: ticket) }
        }
    }

    /// A worktree's simulator signs into its sandbox's Clerk test account with a one-time token from
    /// `npm run phone-account`, as the desktop sandbox does; only against Clerk's development instance, and only while signed out.
    private func signInForDevelopment(ticket: String) async {
        guard let clerk, await waitUntilReady(timeout: .seconds(20)), !isSignedIn else { return }
        do {
            let signIn = try await clerk.auth.signInWithTicket(ticket)
            if let session = signIn.createdSessionId { try await clerk.auth.setActive(sessionId: session) }
        } catch {
            errorMessage = "Couldn't sign in to the test account: \(error.localizedDescription)"
        }
    }

    /// The signed-in session's JWT for the Codync cloud (Clerk caches it for a minute).
    func sessionToken() async throws -> String {
        guard let clerk, isSignedIn, let token = try await clerk.auth.getToken() else { throw SessionError.signedOut }
        return token
    }

    /// The Codync cloud for `userID`'s context; nil when signed out or the build has no cloud.
    /// Requests stop working once another account becomes active, so one account's calls never carry another's token.
    func cloudClient(for userID: String?, identity: DeviceIdentity?) -> CloudClient? {
        guard let userID, let cloudURL, isConfigured else { return nil }
        return CloudClient(baseURL: cloudURL, identity: identity) { [weak self] in
            guard let self, await self.userID == userID else { throw SessionError.signedOut }
            return try await self.sessionToken()
        }
    }

    enum SessionError: LocalizedError {
        case signedOut
        var errorDescription: String? { "Sign in again to reach your account." }
    }

    func signIn(provider: SignInProvider) async {
        guard !isBusy else { return }
        enableAccountServices()
        guard let clerk else { return }
        isBusy = true
        errorMessage = nil
        defer { isBusy = false }
        do {
            guard await waitUntilReady(timeout: .seconds(20)) else {
                errorMessage = "Couldn't load sign-in. Check your connection and try again."
                return
            }
            // Every provider uses Clerk's transfer flow and activates the same kind of session.
            let name: String
            let result: TransferFlowResult
            switch provider {
            case .apple:
                name = "Apple"
                result = try await clerk.auth.signInWithApple()
            case .google:
                name = "Google"
                result = try await clerk.auth.signInWithOAuth(provider: .google, prefersEphemeralWebBrowserSession: isSignedIn)
            case .github:
                name = "GitHub"
                result = try await clerk.auth.signInWithOAuth(provider: .github, prefersEphemeralWebBrowserSession: isSignedIn)
            }
            switch result {
            case .signIn(let signIn):
                if signIn.status != .complete {
                    errorMessage = "\(name) sign-in needs additional verification. Your account is not signed in yet."
                }
            case .signUp(let signUp):
                if signUp.status != .complete {
                    errorMessage = "\(name) sign-up needs additional account information. Your account is not signed in yet."
                }
            }
        } catch where Self.isCancellation(error) {
            // The user closed the sign-in sheet.
        } catch {
            errorMessage = "Couldn't sign in. Check your connection and try again."
        }
    }

    /// The user closed the sign-in sheet.
    private static func isCancellation(_ error: any Error) -> Bool {
        switch error {
        case let error as ASWebAuthenticationSessionError: error.code == .canceledLogin
        case let error as ASAuthorizationError: error.code == .canceled
        default: error is CancellationError
        }
    }

    func signOut() async {
        guard !isBusy, let clerk, let sessionID = clerk.session?.id else { return }
        isBusy = true
        errorMessage = nil
        defer { isBusy = false }
        do {
            try await clerk.auth.signOut(sessionId: sessionID)
            if !isSignedIn { UserDefaults.standard.set(false, forKey: Self.consentKey) }
        } catch {
            errorMessage = "Couldn't sign out. Check your connection and try again."
        }
    }

    /// Deletes the signed-in user in Clerk (App Store 5.1.1(v)); the cloud cleans up after Clerk's `user.deleted`.
    /// Throws when Clerk refused or couldn't be reached, and then nothing changed.
    func deleteUser() async throws {
        guard let clerk, let user = clerk.user, let sessionID = clerk.session?.id else { throw SessionError.signedOut }
        isBusy = true
        errorMessage = nil
        defer { isBusy = false }
        try await user.delete()
        // Clerk ends the session with the user; make sure it's gone here too.
        if userID == user.id { try? await clerk.auth.signOut(sessionId: sessionID) }
    }

    /// Every account on this device, for starting over.
    func signOutAll() async {
        guard let clerk else { return }
        isBusy = true
        errorMessage = nil
        defer { isBusy = false }
        for session in clerk.auth.sessions {
            try? await clerk.auth.signOut(sessionId: session.id)
        }
    }

    func switchAccount(_ account: Account) async {
        guard !isBusy, let clerk, account.id != userID else { return }
        isBusy = true
        errorMessage = nil
        defer { isBusy = false }
        do {
            try await clerk.auth.setActive(sessionId: account.sessionID)
        } catch {
            errorMessage = "Couldn't switch accounts. Check your connection or sign in again."
        }
    }

    #if DEBUG
    /// End-to-end tests (`CODYNC_TEST_SIGN_IN`, docs/guides/cloud-computer-testing.md): signs in a Clerk test address
    /// (`…+clerk_test@…`, whose code is always 424242 on a development instance), creating it the first time.
    func signInTestAccount(_ email: String) async {
        enableAccountServices()
        guard let clerk, !isSignedIn, email.contains("+clerk_test@") else { return }
        do {
            let sessionID: String?
            do {
                let signIn = try await clerk.auth.signInWithEmailCode(emailAddress: email).verifyCode("424242")
                NSLog("CODYNC_TEST_SIGN_IN: sign-in %@", String(describing: signIn.status))
                sessionID = signIn.createdSessionId
            } catch {
                NSLog("CODYNC_TEST_SIGN_IN: no account yet (%@), signing up", String(describing: error))
                // The instance asks for a password at sign-up; later sign-ins use the email code.
                let password = UUID().uuidString + UUID().uuidString
                let signUp = try await clerk.auth.signUp(emailAddress: email, password: password).sendEmailCode().verifyEmailCode("424242")
                NSLog("CODYNC_TEST_SIGN_IN: sign-up %@, missing %@", String(describing: signUp.status), String(describing: signUp.missingFields))
                sessionID = signUp.createdSessionId
            }
            if let sessionID { try await clerk.auth.setActive(sessionId: sessionID) }
            NSLog("CODYNC_TEST_SIGN_IN: done, session %@", sessionID ?? "none")
        } catch {
            errorMessage = "Test sign-in failed: \(error.localizedDescription)"
            NSLog("CODYNC_TEST_SIGN_IN: failed: %@", String(describing: error))
        }
    }
    #endif

    func handle(_ url: URL) async {
        guard let clerk else { return }
        do { try await clerk.handle(url) }
        catch { errorMessage = "Couldn't finish signing in. Please try again." }
    }

    /// `AccountConfig.plist` (`CODYNC_CLERK_PUBLISHABLE_KEY` / `CODYNC_CLOUD_URL` override it).
    private struct Configuration: Decodable {
        let clerkPublishableKey: String?
        let cloudURL: String?
    }
}

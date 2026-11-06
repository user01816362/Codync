import CodyncKit
import SwiftUI

/// App-owned authentication intents used by the shared icon editor.
public struct CustomIconAccount: Sendable {
    public let configured: Bool
    public let signedIn: Bool
    public let loading: Bool
    public let error: String?
    let signIn: @MainActor @Sendable (SignInProvider) -> Void

    public init(configured: Bool, signedIn: Bool, loading: Bool, error: String?,
                signIn: @escaping @MainActor @Sendable (SignInProvider) -> Void) {
        self.configured = configured
        self.signedIn = signedIn
        self.loading = loading
        self.error = error
        self.signIn = signIn
    }
}

public extension EnvironmentValues {
    @Entry var customIconAccount: CustomIconAccount?
}

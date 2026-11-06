import CodyncKit
import CodyncUI
import SwiftUI

/// The Settings tab: what belongs to this iPhone alone (appearance, language, the lead shortcuts call,
/// notification permission, Start over). Each computer's own settings (voice chat, push, connection, hidden bots) are in
/// Computer management (`ComputersView`).
struct SettingsView: View {
    @Environment(AppStore.self) private var app
    @Environment(\.openURL) private var openURL
    @Environment(\.scenePhase) private var scenePhase
    @State private var notificationsAllowed: Bool?
    @State private var confirmStartOver = false

    var body: some View {
        CardForm {
            Text(L10n.tr("Settings"))
                .font(.largeTitle.bold())
                .foregroundStyle(Palette.text)
                .accessibilityAddTraits(.isHeader)

            AppearanceSettings()

            LanguageSettings()

            ThinkingDetailsSettings()

            ShortcutLeadSettings()

            CardSection("Notifications", footer: "Get a result summary, a request for input, or a failure notice. Notification previews follow your iOS settings.") {
                notificationsRow
            }

            CardSection {
                Button {
                    openURL(AppStoreRelease.reviewPage)
                } label: {
                    Text(L10n.tr("Rate on the App Store"))
                }
                .buttonStyle(.plain)
                .foregroundStyle(Palette.text)
                Button {
                    openURL(URL(string: "https://github.com/leepokai/Codync/issues")!)
                } label: {
                    Text(L10n.tr("Report an issue"))
                }
                .buttonStyle(.plain)
                .foregroundStyle(Palette.text)
                ValueRow("App version", value: Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String ?? "")
                Button {
                    openURL(URL(string: "https://github.com/leepokai/Codync")!)
                } label: {
                    Text(L10n.tr("Source code"))
                }
                    .buttonStyle(.plain)
                    .foregroundStyle(Palette.text)
            }

            CardSection {
                Button { confirmStartOver = true } label: {
                    Label(L10n.tr("Start over"), systemImage: "arrow.counterclockwise")
                        .foregroundStyle(Palette.danger)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
            }
        }
        .codyncDialog("Start over?", isPresented: $confirmStartOver,
                      message: "Forgets every computer paired with this iPhone and shows the welcome again. Your computers keep their bots and chats; pair again to use them.") {
            [DialogAction("Start over", destructive: true) { Task { await app.startOver() } }]
        }
        .task { notificationsAllowed = await PushRegistrar.shared.isAllowed() }
        // The tab stays alive: coming back from iOS Settings reads the permission again.
        .onChange(of: scenePhase) { _, phase in
            if phase == .active { Task { notificationsAllowed = await PushRegistrar.shared.isAllowed() } }
        }
    }

    private var notificationsRow: some View {
        Button {
            if notificationsAllowed != nil {
                UIApplication.shared.open(URL(string: UIApplication.openNotificationSettingsURLString)!)
            } else {
                Task { notificationsAllowed = await PushRegistrar.shared.requestAuthorization() }
            }
        } label: {
            ValueRow("Notifications", value: notificationsAllowed == true ? "On" : notificationsAllowed == false ? "Off in Settings" : "Turn on")
                .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
    }
}

import CodyncKit
import SwiftUI
import UIKit

/// One endless conversation with a bot or a group chat. Only deliberate messages show
/// here; tool calls and thinking live in the "Full conversation" sheet. Any message can be
/// quoted in a reply.
public struct ThreadView: View {
    let botId: String

    public init(botId: String) { self.botId = botId }
    @Environment(BotStore.self) var model
    @State var showTrace = false
    /// The message the composer's next send quotes.
    @State var replyingTo: Entry?
    /// A sent message being edited in the composer; sending asks the bot again.
    @State var editingMessage: Entry?
    @State var botChat: BotChatTarget?
    @State var openArtifact: ArtifactTarget?
    @State var openCall: Entry?
    @State var editingGroup = false
    @State var details: DetailsRequest?
    /// The version of an edited message the reader walked back to, by the message's first entry.
    @State var versionPicks: [String: VersionPick] = [:]
    @State var calling = false
    @State var callSpeaking = false
    @State var interruptCall: (() -> Void)?
    /// The chat stays on the newest message until the reader scrolls away.
    @State var following = true
    @State var loadingEarlier = false
    @Environment(\.accessibilityReduceMotion) var reduceMotion

    var bot: Bot? { model.bots[botId] }

    /// The messages (each platform's own `transcript`, in its `ThreadView+` file) and the
    /// composer, without the platform's top chrome.
    private var chat: some View {
        transcript
        .background(Palette.background)
        .safeAreaInset(edge: .bottom) {
            VStack(spacing: 6) {
                if let mismatch = model.mismatch {
                    // Nothing sent now could be read on the other side.
                    UpdateNeededCard(store: model, mismatch: mismatch)
                        .padding(.horizontal, 16)
                        .padding(.bottom, 8)
                        .transition(.opacity)
                } else {
                    Composer(botId: botId, replyingTo: $replyingTo, editingMessage: $editingMessage,
                             queued: model.chat(botId).hidingCalls().queue(working: busy),
                             onCall: !calling && bot != nil ? { calling = true } : nil,
                             onInterrupt: callSpeaking ? interruptCall : nil)
                }
            }
            .animation(Motion.reduced(Motion.layout, reduceMotion), value: model.mismatch)
        }
        // Grok Bot's call: a bar floating over the chat, which stays readable and usable.
        .overlay(alignment: .top) {
            if calling {
                CallView(botId: botId, isSpeaking: $callSpeaking, interrupt: $interruptCall) {
                    withAnimation(Motion.reduced(Motion.layout, reduceMotion)) { calling = false }
                }
                    .padding(.top, 4)
                    .transition(.move(edge: .top).combined(with: .opacity))
            }
        }
        .animation(Motion.reduced(Motion.layout, reduceMotion), value: calling)
    }

    var conversation: some View {
        chrome(chat)
            .readingConversation(botId)
            .onChange(of: model.voiceCallRequest, initial: true) { _, _ in
                if model.takeVoiceCallRequest(for: botId) { startShortcutCall() }
            }
            // The details sheet's Library presents its own downloads.
            .fileExportSheet(model, botId: botId, active: details == nil)
            .codyncSheet(isPresented: $showTrace) {
                TraceView(botId: botId)
            }
            .codyncSheet(item: $botChat) { BotConversationView(botId: botId, peerId: $0.id) }
            .codyncSheet(item: $openArtifact) { ArtifactView(target: $0) }
            .codyncSheet(item: $openCall) { CallConversationView(line: $0) }
            .codyncSheet(item: $details) { request in
                BotDetailsView(botId: request.botId ?? botId, request: request) { artifact in
                    details = nil
                    openArtifact = artifact
                }
            }
            .codyncSheet(isPresented: $editingGroup) {
                if let bot { GroupEditorView(group: bot) }
            }
    }

    /// Open Live Mode from Shortcuts: closes whatever covers the chat, then starts the call.
    private func startShortcutCall() {
        showTrace = false
        botChat = nil
        openArtifact = nil
        editingGroup = false
        details = nil
        if model.fileDownloads.export?.botId == botId { model.fileDownloads.dismissExport() }
        withAnimation(Motion.reduced(Motion.layout, reduceMotion)) { calling = true }
    }

    // MARK: rows

    @ViewBuilder func row(_ item: ChatItem) -> some View {
        switch item.kind {
        case .separator(let date):
            TimeSeparator(date: date)
        case .exchanges(let group):
            BotMessageRow(group: group) { botChat = BotChatTarget(id: group.peerId) }
        case .entry(let e, let groupStart):
            if e.kind == "notice", let id = e.data.routineId {
                RoutineNoticeRow(entry: e) { openRoutine(id) }
            } else {
                ChatRow(entry: e, groupStart: groupStart, chat: bot, openTrace: { showTrace = true },
                        // An update card takes the composer's place, so there's nothing to reply with.
                        reply: e.isReplyable && model.mismatch == nil ? { reply(to: e) } : nil,
                        // Only a sent message of a one-to-one chat can be edited and answered again.
                        edit: e.isEditable && bot?.isGroup != true && model.mismatch == nil ? { beginEditing(e) } : nil,
                        // In a group, the bot whose agent failed.
                        addFallback: { details = DetailsRequest(botId: e.data.author ?? e.botId) },
                        openArtifact: { openArtifact = ArtifactTarget(botId: e.botId, id: $0.id, title: $0.title) },
                        openCall: { openCall = e },
                        versions: item.versions, versionIndex: item.versionIndex,
                        pickVersion: item.versions.map { versions in
                            { version in pickVersion(versions, version) }
                        })
                    .equatable()
            }
        }
    }

    private func reply(to entry: Entry) {
        withAnimation(Motion.reduced(Motion.layout, reduceMotion)) { replyingTo = entry }
    }

    /// Walks to another version of an edited message: its own answer comes with it.
    private func pickVersion(_ versions: [Entry], _ version: Entry) {
        guard let first = versions.first else { return }
        versionPicks[first.id] = VersionPick(version: version.id, of: versions.count)
    }

    /// Opens a sent message in the composer; sending edits it and asks the bot again.
    private func beginEditing(_ entry: Entry) {
        model.setComposerDraft(entry.data.text ?? "", for: "edit:\(entry.id)")
        withAnimation(Motion.reduced(Motion.layout, reduceMotion)) {
            replyingTo = nil
            editingMessage = entry
        }
    }

    // MARK: chrome

    var connectionSubtitle: some View {
        HStack(spacing: 6) {
            Group {
                if model.shownConnection == .connecting {
                    Spinner(size: 8)
                } else {
                    Image(systemName: connectionSymbol).font(.system(size: 8, weight: .medium))
                }
            }
            .frame(width: 12)
            .accessibilityHidden(true)
            Text(L10n.tr(model.connectionLabel))
                .lineLimit(1)
                .truncationMode(.middle)
        }
        .font(.system(size: 10, weight: .medium))
        .foregroundStyle(Palette.secondary)
        .accessibilityElement(children: .ignore)
        .accessibilityLabel("\(model.hostName), \(L10n.tr(connectionDescription))")
        .help("\(model.hostName) · \(L10n.tr(connectionDescription))")
    }

    private var connectionSymbol: String {
        guard model.connection == .online else { return "wifi.slash" }
        switch model.hostRoute {
        case .relay: return "cloud"
        case .direct: return "wifi"
        case nil: return "network"
        }
    }

    private var connectionDescription: String {
        switch model.shownConnection {
        case .connecting: return "Connecting"
        case .computerOffline, .offline: return "Offline"
        case .unauthorized: return "No access"
        case .unpaired: return "Not paired"
        case .online: break
        }
        if model.mismatch != nil { return "Needs update" }
        switch model.hostRoute {
        case .relay: return RelayCopy.connected(cloud: model.computer.cloud)
        case .direct: return "Connected over Wi-Fi or Tailscale"
        case nil: return "Connected"
        }
    }

    var title: some View {
        HStack(spacing: 7) {
            if let bot {
                let size: CGFloat = 22
                if bot.isGroup { GroupAvatar(members: model.members(of: bot), size: size) } else { CharacterAvatar(bot: bot, size: size) }
            }
            Text(bot?.name ?? "")
                .font(.system(size: 13, weight: .semibold))
                .foregroundStyle(Palette.text)
                .lineLimit(1)
        }
    }

    func openRoutine(_ id: String) {
        withAnimation(Motion.reduced(Motion.layout, reduceMotion)) {
            details = DetailsRequest(tab: .details, routineId: id)
        }
    }
}

// MARK: - Chat model

struct ChatItem: Identifiable {
    enum Kind {
        case separator(Date)
        case entry(Entry, groupStart: Bool)
        /// Main chat only: consecutive bot-to-bot exchanges with one peer.
        case exchanges(BotExchangeGroup)
    }

    let id: String
    let kind: Kind
    /// An edited message's versions and the one shown.
    var versions: [Entry]? = nil
    var versionIndex: Int? = nil

    /// Chat-visible entries plus time separators (gaps > 1 h) and author grouping. A bot's
    /// messages arrive whole (Grok Bot's `send_message`); what it writes along the way is trace. Runs of
    /// bot exchanges with one peer fold into one item.
    ///
    /// An edited message is one slot: it shows the version the reader picked (`picked`, by its first entry's
    /// id; the newest by default) and that version's answers, where the message first was.
    static func build(_ entries: [Entry], picked: [String: VersionPick] = [:]) -> [ChatItem] {
        var out: [ChatItem] = []
        var lastDate: Date?
        var lastAuthor: String?
        let groups = MessageVersions.groups(entries)
        let editedTurns = Set(groups.values.flatMap { $0.versions.map(\.turn) })
        // Each edited version's answer shares its turn, and is shown with the version instead of at the end.
        var answers: [Int64: [Entry]] = [:]
        for entry in entries where entry.kind != "user" && editedTurns.contains(entry.turn) {
            answers[entry.turn, default: []].append(entry)
        }
        let visible = entries.filter { entry in
            // Where the message is shown, even when its first version is older than what the chat loaded.
            if groups[entry.id] != nil { return true }
            if entry.kind == "user", entry.data.editOf != nil { return false }
            // Left behind by an edit: its version and answer stand behind the arrows.
            if entry.data.superseded == true { return false }
            return !editedTurns.contains(entry.turn)
        }

        func emit(_ item: BotExchangeGroup.Item, at date: Date, versions: [Entry]? = nil, index: Int? = nil) {
            let entryId: String
            switch item {
            case .entry(let e):
                // Bot-originated messages carry an empty nonce. Their entry IDs distinguish them; only
                // real nonces identify optimistic echoes.
                if e.kind == "user", let nonce = e.data.clientNonce, !nonce.isEmpty { entryId = "user-\(nonce)" } else { entryId = e.id }
            case .exchanges(let g):
                entryId = g.id
            }
            if lastDate.map({ date.timeIntervalSince($0) > 3600 }) ?? true {
                out.append(ChatItem(id: "sep-\(entryId)", kind: .separator(date)))
                lastAuthor = nil
            }
            lastDate = date
            switch item {
            case .exchanges(let group):
                out.append(ChatItem(id: entryId, kind: .exchanges(group)))
                lastAuthor = nil
            case .entry(let entry):
                // In a group each bot is its own author.
                let author = entry.kind == "user" ? "user" : entry.kind == "agent" ? "agent:\(entry.data.author ?? "")" : entry.kind
                out.append(ChatItem(id: entryId, kind: .entry(entry, groupStart: author != lastAuthor),
                                    versions: versions, versionIndex: index))
                lastAuthor = (author == "user" || entry.kind == "agent") ? author : nil
            }
        }

        for slot in BotExchangeGroup.collapse(visible) {
            guard case .entry(let entry) = slot, let group = groups[entry.id] else {
                emit(slot, at: slot.date)
                continue
            }
            // The edited message and the answer of the version shown, in the place the message was first sent.
            let chosen = MessageVersions.selected(group, pick: picked[group.anchor.id])
            let index = group.versions.firstIndex(of: chosen)!
            for (i, part) in BotExchangeGroup.collapse([chosen] + (answers[chosen.turn] ?? [])).enumerated() {
                emit(part, at: i == 0 ? group.anchor.date : part.date,
                     versions: i == 0 ? group.versions : nil, index: i == 0 ? index : nil)
            }
        }
        return out
    }

    var isUserMessage: Bool {
        if case let .entry(entry, _) = kind { entry.kind == "user" } else { false }
    }

}

/// The start of an empty group chat: who's in it and how the room works.
struct GroupIntroCard: View {
    let group: Bot
    @Environment(BotStore.self) private var model

    var body: some View {
        VStack(spacing: 12) {
            GroupAvatar(members: model.members(of: group), size: 72)
            Text(group.name).font(.title2.weight(.semibold)).foregroundStyle(Palette.text)
            let names = model.members(of: group).map(\.name)
            if group.name != names.joined(separator: ", ") {
                Text(names.joined(separator: " · "))
                    .font(.subheadline)
                    .foregroundStyle(Palette.secondary)
                    .multilineTextAlignment(.center)
            }
            if !group.description.isEmpty {
                Text(group.description)
                    .font(.subheadline)
                    .foregroundStyle(Palette.secondary)
                    .multilineTextAlignment(.center)
            }
            Text(L10n.tr("Everyone answers in turn. @mention a bot to ask just that one."))
                .font(.subheadline)
                .foregroundStyle(Palette.secondary)
                .multilineTextAlignment(.center)
                .padding(.top, 4)
        }
        .frame(maxWidth: .infinity)
        .padding(.horizontal, 24)
    }
}

struct IntroCard: View {
    let bot: Bot
    @Environment(BotStore.self) private var model

    var body: some View {
        VStack(spacing: 12) {
            CharacterAvatar(bot: bot, size: 72)
            Text(bot.name).font(.title2.weight(.semibold)).foregroundStyle(Palette.text)
            Text(bot.managedWorkspace
                 ? "\(model.backendName(bot.backend)) · \(L10n.tr("Personal workspace"))"
                 : L10n.fill(L10n.tr("{agent} in {path}"), ["agent": model.backendName(bot.backend), "path": bot.cwd]))
                .font(.subheadline)
                .foregroundStyle(Palette.secondary)
                .multilineTextAlignment(.center)
            if !bot.description.isEmpty {
                Text(bot.description)
                    .font(.subheadline)
                    .foregroundStyle(Palette.secondary)
                    .multilineTextAlignment(.center)
            }
            Text(L10n.tr("Tell it what you need. You'll get a notification when it's done or needs you."))
                .font(.subheadline)
                .foregroundStyle(Palette.secondary)
                .multilineTextAlignment(.center)
                .padding(.top, 4)
        }
        .frame(maxWidth: .infinity)
        .padding(.horizontal, 24)
    }
}

extension ThreadView {
    public var body: some View {
        conversation
            .tracked(.screenViewed(.chat))
    }

    private var working: Bool { bot?.isWorking(in: botId) == true && !model.isOffline }
    /// The bot works anywhere (a room, a routine): the user's messages here wait above the composer.
    private var busy: Bool { bot?.isWorking == true }

    /// iPhone's messages: all this device has, earlier ones from the computer as the reader
    /// scrolls up; the chat follows the newest message until they scroll away, and a reply is
    /// revealed steadily as it's written.
    var transcript: some View {
        // What voice calls carried shows in each call's conversation, not in the chat.
        let items = ChatItem.build(model.chat(botId).hidingCalls().hidingQueue(working: busy), picked: versionPicks)
        var rows: [ConversationRow] = []
        if moreOnComputer {
            rows.append(ConversationRow("earlier") {
                Spinner(size: 14)
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, 14)
                    .accessibilityLabel("Loading earlier messages")
                    // A chat shorter than the screen never scrolls, so showing the row is the trigger.
                    .onAppear { showEarlier() }
            })
        }
        if items.isEmpty, let bot {
            rows.append(ConversationRow("intro") {
                if bot.isGroup { GroupIntroCard(group: bot).padding(.top, 40) } else { IntroCard(bot: bot).padding(.top, 40) }
            })
        }
        rows += items.map { item in ConversationRow(item.id, isUserMessage: item.isUserMessage) { row(item) } }
        // Offline, "working" is only what the computer last said; don't show it as live.
        if let bot, working {
            rows.append(ConversationRow("working") {
                // Each turn starts folded.
                WorkingIndicator(bot: bot, steps: Entry.workingSteps(model.chat(botId)))
                    .id(bot.startedAt)
                    .padding(.top, 6)
            })
        }
        // Pull to refresh reconnects, like Reconnect in the computer menu, until the link is back.
        return ConversationList(rows: rows, following: $following, nearTop: { showEarlier() }, refresh: {
            model.restartStream()
            _ = try? await model.ready()
        })
    }

    private var moreOnComputer: Bool {
        model.canLoadOlder(botId)
    }

    /// The reader scrolled up to the top: fetch earlier messages from the computer.
    private func showEarlier() {
        guard moreOnComputer, !loadingEarlier else { return }
        loadingEarlier = true
        Task {
            await model.loadOlder(botId)
            loadingEarlier = false
        }
    }

    /// The system navigation bar: the title opens the bot's details; the corner is the computer.
    func chrome(_ content: some View) -> some View {
        content
            .navigationBarTitleDisplayMode(.inline)
            .toolbar(.visible, for: .navigationBar)
            .toolbar {
                ToolbarItem(placement: .principal) {
                    VStack(spacing: 0) {
                        header
                        connectionSubtitle
                    }
                }
                if model.screen != nil {
                    ToolbarItem(placement: .topBarTrailing) { computerButton }
                }
            }
    }

    /// Like Grok Bot: the bot at the top opens its details (a group's editor for a group).
    var header: some View {
        Button(action: openDetails) { title }
            .accessibilityLabel(L10n.tr("{name}, view details").replacingOccurrences(of: "{name}", with: bot?.name ?? ""))
    }

    private func openDetails() {
        if bot?.isGroup == true { editingGroup = true } else { details = DetailsRequest() }
    }

    /// Opens the computer's screen; pulses while this bot is operating it (watch, then take over).
    private var computerButton: some View {
        let operating = model.screen?.agentBot == botId
        return Button(L10n.tr("Computer"), systemImage: "desktopcomputer") {
            model.screenRequest = operating ? ScreenRequest(watching: botId) : ScreenRequest()
        }
        .symbolEffect(.pulse, options: .repeating, isActive: operating)
    }
}

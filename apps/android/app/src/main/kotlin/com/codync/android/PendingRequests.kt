package com.codync.android

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.codync.android.core.*
import com.codync.android.design.*
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.booleanOrNull

private data class WaitingRequest(val state: AppState, val entry: Entry) {
    val key: String get() = "${state.computer.id}/${entry.id}"
    val author: String get() = state.bots.firstOrNull { it.id == (entry.data.text("author") ?: entry.botId) }?.name ?: "Agent"
    val chat: String get() = state.bots.firstOrNull { it.id == entry.botId }?.name ?: "Conversation"
}

/** One request at a time, without navigating away or granting permissions to several agents at once. */
@Composable internal fun PendingRequests(states: List<AppState>, accounts: AccountStore) {
    var showing by remember { mutableStateOf(false) }
    var selected by remember { mutableStateOf<String?>(null) }
    val requests = states.flatMap { state -> state.entries.filter { entry ->
        (entry.data["removed"] as? JsonPrimitive)?.booleanOrNull != true && entry.kind == "permission" && entry.data.text("status") == "pending" && entry.botId != state.selectedBot
    }.map { WaitingRequest(state, it) } }.sortedWith(compareBy({ it.entry.createdAt }, { it.key }))
    if (requests.isNotEmpty()) Row(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 6.dp), horizontalArrangement = Arrangement.End) {
        SecondaryButton("${tr("Requests")} · ${requests.size}") { selected = requests.first().key; showing = true }
    }
    CodyncSheet(showing, { showing = false }) {
        Column(Modifier.fillMaxSize()) {
            ModalHeader("Requests")
            Column(Modifier.verticalScroll(rememberScrollState()).padding(16.dp), verticalArrangement = Arrangement.spacedBy(16.dp)) {
                val current = requests.firstOrNull { it.key == selected }
                if (current != null) {
                    ChoicePicker(selected.orEmpty(), requests.map { it.key to "${it.author} · ${it.chat} · ${it.state.hostName}" }, { selected = it }, label = "Choose request")
                    Text(current.state.hostName, style = Type.caption, color = Palette.secondary)
                    val store = accounts.store(current.state.computer.id)
                    val online = current.state.actualConnection is LinkState.Ready
                    key(current.key) {
                        PermissionCard(current.entry, current.state.hostName,
                            answering = !online || "permission:${current.entry.id}" in current.state.busy) { option -> store?.respond(current.entry, option) }
                    }
                    if (!online) Text(tr("Reconnect to this computer before responding."), style = Type.footnote, color = Palette.warning)
                    current.state.error?.let { Text(it, style = Type.footnote, color = Palette.danger) }
                    SecondaryButton("Open chat") {
                        accounts.openBot(BotDestination(current.state.contextId, current.state.computer.id, current.entry.botId))
                        showing = false
                    }
                } else {
                    Text(tr(if (requests.isEmpty()) "No requests waiting." else "This request is no longer waiting."), color = Palette.text)
                    if (requests.isNotEmpty()) SecondaryButton("Review next request") { selected = requests.first().key }
                }
                Text(tr("Each approval applies to the selected request. Always allow uses the agent’s supplied permission scope."), style = Type.caption, color = Palette.secondary)
            }
        }
    }
}

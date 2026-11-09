package com.codync.android

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.codync.android.design.*
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.launch
import kotlinx.serialization.json.*

@Composable internal fun GroupInbox(groupId: String, state: AppState, store: AppStore) {
    val members = state.bots.firstOrNull { it.id == groupId }?.members.orEmpty().mapNotNull { id -> state.bots.firstOrNull { it.id == id } }
    var items by remember(groupId) { mutableStateOf<JsonArray?>(null) }
    var failure by remember(groupId) { mutableStateOf<String?>(null) }
    var attempt by remember(groupId) { mutableStateOf(0) }
    var busy by remember(groupId) { mutableStateOf<String?>(null) }
    var handlers by remember(groupId) { mutableStateOf<Map<String, String>>(emptyMap()) }
    val scope = rememberCoroutineScope()
    LaunchedEffect(groupId, attempt) {
        failure = null
        try { items = store.hostQuery("groupInbox", buildJsonObject { put("groupId", groupId) }).jsonObject["items"]?.jsonArray }
        catch (cancelled: CancellationException) { throw cancelled }
        catch (error: Exception) { failure = error.message ?: "Couldn't load group inbox." }
    }
    fun decide(id: String, approve: Boolean) {
        if (busy != null) return
        busy = id
        failure = null
        scope.launch {
            try {
                store.hostQuery("decideGroupMail", buildJsonObject {
                    put("mailId", id); put("approve", approve)
                    (handlers[id] ?: members.firstOrNull()?.id)?.let { put("handler", it) }
                })
                attempt += 1
            } catch (cancelled: CancellationException) { throw cancelled }
            catch (error: Exception) { failure = error.message ?: "Couldn't decide group mail." }
            finally { busy = null }
        }
    }
    Column(Modifier.fillMaxSize()) {
        ModalHeader("Group inbox") { IconButton("Refresh inbox", "arrow.clockwise", enabled = busy == null) { attempt += 1 } }
        Column(Modifier.weight(1f).fillMaxWidth().verticalScroll(rememberScrollState()).padding(20.dp), verticalArrangement = Arrangement.spacedBy(20.dp)) {
            Text(tr("Approve an external request and choose who handles it. The sender receives only that handler’s reply, with no access to this conversation."), style = Type.caption, color = Palette.secondary)
            failure?.let { Text(it, color = Palette.danger) }
            val current = items
            if (current == null && failure == null) Spinner()
            if (current?.isEmpty() == true) Text(tr("No group mail."), color = Palette.secondary)
            current.orEmpty().forEach { value ->
                val item = value.jsonObject
                val id = item.string("id")
                val mail = item.getValue("mail").jsonObject
                val status = mail.string("status")
                val delivered = item["delivered"]?.jsonPrimitive?.booleanOrNull == true
                val replied = item["replied"]?.jsonPrimitive?.booleanOrNull == true
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text(mail.string("subject"), style = Type.subheadline.semibold, color = Palette.text)
                    Text(tr("From") + " " + item.string("from"), style = Type.caption, color = Palette.secondary)
                    SelectionContainer { Text(mail.string("text"), color = Palette.text) }
                    Text(tr(if (replied) "Replied" else if (delivered) "Delivered to handler" else if (status == "approved") "Approved · delivery pending" else if (status == "rejected") "Rejected" else "Awaiting your approval"), style = Type.caption, color = Palette.secondary)
                    if (status == "pending") {
                        Text(tr("Handler"), style = Type.caption, color = Palette.secondary)
                        members.forEach { member ->
                            LinkButton((if ((handlers[id] ?: members.firstOrNull()?.id) == member.id) "✓ " else "") + member.name) {
                                if (busy == null) handlers = handlers + (id to member.id)
                            }
                        }
                        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                            SecondaryButton("Approve delivery", enabled = busy == null && members.isNotEmpty()) { decide(id, true) }
                            SecondaryButton("Reject", enabled = busy == null) { decide(id, false) }
                        }
                    } else if (status == "approved" && !delivered) SecondaryButton("Retry approved delivery", enabled = busy == null) { decide(id, true) }
                }
            }
            if (current?.size == 100) Text(tr("Showing up to 100 requests, pending first."), style = Type.caption, color = Palette.secondary)
        }
    }
}

private fun JsonObject.string(key: String) = this[key]?.jsonPrimitive?.content.orEmpty()

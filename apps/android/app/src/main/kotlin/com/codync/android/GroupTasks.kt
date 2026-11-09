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

@Composable internal fun GroupTasks(groupId: String, state: AppState, store: AppStore) {
    val members = state.bots.firstOrNull { it.id == groupId }?.members.orEmpty().mapNotNull { id -> state.bots.firstOrNull { it.id == id } }
    var items by remember(groupId) { mutableStateOf<JsonArray?>(null) }
    var title by remember(groupId) { mutableStateOf("") }
    var instructions by remember(groupId) { mutableStateOf("") }
    var owner by remember(groupId) { mutableStateOf(members.firstOrNull()?.id.orEmpty()) }
    var request by remember(groupId) { mutableStateOf(java.util.UUID.randomUUID().toString()) }
    var attempt by remember(groupId) { mutableStateOf(0) }
    var busy by remember(groupId) { mutableStateOf(false) }
    var failure by remember(groupId) { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()
    LaunchedEffect(groupId, attempt) {
        try { items = store.hostQuery("groupTasks", body("groupId" to groupId)).jsonObject["items"]?.jsonArray }
        catch (cancelled: CancellationException) { throw cancelled }
        catch (error: Exception) { failure = error.message }
    }
    fun run(method: String, args: JsonObject) {
        if (busy) return
        busy = true
        scope.launch {
            try {
                store.hostQuery(method, args)
                if (method == "createGroupTask") { title = ""; instructions = ""; request = java.util.UUID.randomUUID().toString() }
                attempt += 1
            } catch (cancelled: CancellationException) { throw cancelled }
            catch (error: Exception) { failure = error.message; attempt += 1 }
            finally { busy = false }
        }
    }
    fun update(entry: JsonObject, action: String, owner: String? = null) = run("updateGroupTask", buildJsonObject {
        put("taskId", entry.getValue("id")); put("revision", entry.getValue("rev")); put("action", action); owner?.let { put("owner", it) }
    })
    Column(Modifier.fillMaxSize()) {
        ModalHeader("Group tasks") { IconButton("Refresh tasks", "arrow.clockwise", enabled = !busy) { failure = null; attempt += 1 } }
        Column(Modifier.weight(1f).fillMaxWidth().verticalScroll(rememberScrollState()).padding(20.dp), verticalArrangement = Arrangement.spacedBy(20.dp)) {
            Text(tr("Attempts run in the owner's chat. Pause waits for cancellation; resume asks the bot to inspect earlier work first."), style = Type.caption, color = Palette.secondary)
            Field("Task title") { FieldBox(title, { if (!busy) { title = it; request = java.util.UUID.randomUUID().toString() } }, "Task title") }
            Field("Instructions") { FieldBox(instructions, { if (!busy) { instructions = it; request = java.util.UUID.randomUUID().toString() } }, "Instructions", minLines = 3, maxLines = 8) }
            ChoicePicker(owner, members.map { it.id to it.name }, { owner = it; request = java.util.UUID.randomUUID().toString() }, label = "Owner", enabled = !busy)
            SecondaryButton("Create task", enabled = !busy && title.isNotBlank() && instructions.isNotBlank() && members.any { it.id == owner }) {
                run("createGroupTask", body("groupId" to groupId, "owner" to owner, "title" to title, "text" to instructions, "requestId" to request))
            }
            failure?.let { Text(it, color = Palette.danger) }
            if (items == null && failure == null) Spinner()
            if (items?.isEmpty() == true) Text(tr("No tasks yet."), color = Palette.secondary)
            items.orEmpty().forEach { value ->
                val entry = value.jsonObject
                val task = entry.getValue("data").jsonObject.getValue("groupTask").jsonObject
                val status = task.getValue("status").jsonPrimitive.content
                val assigned = task.getValue("owner").jsonPrimitive.content
                var expanded by remember(entry.getValue("id")) { mutableStateOf(false) }
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text(task.getValue("title").jsonPrimitive.content, style = Type.headline, color = Palette.text)
                    Text("${state.bots.firstOrNull { it.id == assigned }?.name ?: "Unavailable owner"} · $status", style = Type.caption, color = Palette.secondary)
                    SelectionContainer { Text(task.getValue("text").jsonPrimitive.content, color = Palette.text) }
                    if (status in listOf("draft", "paused", "failed", "interrupted")) {
                        SecondaryButton(if (status == "draft") "Start task" else "Resume task", enabled = !busy) { update(entry, if (status == "draft") "start" else "resume") }
                        ChoicePicker(assigned, members.map { it.id to it.name }, { update(entry, "reassign", it) }, label = "Reassign owner", enabled = !busy)
                    }
                    if (status in listOf("queued", "running")) SecondaryButton("Pause task", enabled = !busy) { update(entry, "pause") }
                    LinkButton("Task history") { expanded = !expanded }
                    if (expanded) task["history"]?.jsonArray.orEmpty().forEach { event ->
                        val data = event.jsonObject
                        Text("${java.util.Date(data.getValue("at").jsonPrimitive.long)} · ${data.getValue("status").jsonPrimitive.content}", style = Type.caption, color = Palette.secondary)
                        SelectionContainer { Text(data.getValue("detail").jsonPrimitive.content, style = Type.caption, color = Palette.text) }
                    }
                }
            }
        }
    }
}

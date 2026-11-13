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
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.launch
import kotlinx.serialization.json.*

/** A native view of the same host-owned meeting used by the other clients. */
@Composable internal fun MeetingRoom(groupId: String, state: AppState, store: AppStore) {
    val group = state.bots.firstOrNull { it.id == groupId }
    var room by remember(groupId) { mutableStateOf<JsonObject?>(null) }
    var name by remember(groupId) { mutableStateOf("${group?.name.orEmpty()} meeting".take(40)) }
    var agenda by remember(groupId) { mutableStateOf(group?.description.orEmpty()) }
    var kind by remember(groupId) { mutableStateOf("consultation") }
    var selected by remember(groupId) { mutableStateOf<Map<String, String>>(emptyMap()) }
    var department by remember(groupId) { mutableStateOf(group?.takeIf { it.kind == "group" && !it.meeting }?.id.orEmpty()) }
    var chair by remember(groupId) { mutableStateOf(selected.keys.firstOrNull().orEmpty()) }
    LaunchedEffect(selected.keys) { if (chair !in selected) chair = selected.keys.firstOrNull().orEmpty() }
    var historyOpen by remember(groupId) { mutableStateOf(false) }
    var proposal by remember(groupId) { mutableStateOf("") }
    var attempt by remember(groupId) { mutableStateOf(0) }
    var busy by remember(groupId) { mutableStateOf(false) }
    var notice by remember(groupId) { mutableStateOf<String?>(null) }
    var failure by remember(groupId) { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()
    val disabled = busy || state.actualConnection !is LinkState.Ready
    val roomId = room?.text("groupId") ?: groupId
    val revision = state.entries.firstOrNull { it.id == "meeting:$roomId" }?.rev
    LaunchedEffect(roomId, attempt, revision) {
        if (group?.meeting != true && room == null) return@LaunchedEffect
        try { room = store.hostQuery("meetingState", buildJsonObject { put("groupId", roomId) }).jsonObject }
        catch (cancelled: CancellationException) { throw cancelled }
        catch (error: Exception) { failure = error.message }
    }
    fun run(action: String) {
        if (disabled || room == null) return
        busy = true; failure = null
        scope.launch {
            try {
                val result = store.hostQuery("meetingAction", buildJsonObject {
                    put("groupId", roomId); put("rev", room!!["rev"]!!); put("action", action)
                    if (action in listOf("propose", "finish")) put("text", proposal)
                })
                result.jsonObject["deliveredTo"]?.jsonArray?.let { notice = "Outcome published to ${it.size} departments. No automatic follow-up was requested." }
                attempt += 1
            } catch (cancelled: CancellationException) { throw cancelled }
            catch (error: Exception) { failure = error.message; attempt += 1 }
            finally { busy = false }
        }
    }
    Column(Modifier.fillMaxSize()) {
        ModalHeader(if (room == null) "New meeting room" else "Meeting room")
        Column(Modifier.verticalScroll(rememberScrollState()).padding(14.dp), verticalArrangement = Arrangement.spacedBy(24.dp)) {
            Text(tr("Participants keep their existing permissions and shared agent-session history. Share only material intended for this room."), style = Type.caption, color = Palette.secondary)
            notice?.let { Text(it, style = Type.caption, color = Palette.secondary) }
            failure?.let { Text(it, color = Palette.danger) }
            val meeting = room?.get("meeting")?.jsonObject
            if (meeting == null && group?.meeting != true) {
                CardSection("Room") {
                    row { Field("Name") { FieldBox(name, { name = it.take(40) }, "Name") } }
                    row { Field("Agenda") { FieldBox(agenda, { agenda = it.take(4000) }, "Agenda", minLines = 3, maxLines = 8) } }
                    row { ValueRow("Purpose") { ChoicePicker(kind, listOf("consultation" to "Consultation", "meeting" to "Meeting", "decision" to "Decision"), { kind = it }, enabled = !disabled) } }
                }
                CardSection("Delegates · ${selected.size}/5") {
                    row { ValueRow("Department", detail = "Choose a group to see its members, or invite individual bots.") {
                        ChoicePicker(department, listOf("" to "Individual bots") + state.bots.filter { it.kind == "group" && !it.meeting }.map { it.id to it.name }, { department = it }, enabled = !disabled)
                    } }
                    val candidates = state.bots.filter { it.kind != "group" && !it.deleted && (!it.hidden || it.id in selected) && (department.isEmpty() || state.bots.firstOrNull { group -> group.id == department }?.members?.contains(it.id) == true) }
                    if (candidates.isEmpty()) row { Text(tr("No available members in this department."), color = Palette.secondary) }
                    for (bot in candidates) row {
                        Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                            SecondaryButton("${if (bot.id in selected) "✓" else "+"} ${bot.name}", enabled = !disabled && (bot.id in selected || selected.size < 5)) {
                                selected = if (bot.id in selected) selected - bot.id else selected + (bot.id to department)
                            }
                            if (bot.id in selected) ChoicePicker(selected[bot.id].orEmpty(), listOf("" to "Personal") + state.bots.filter { it.kind == "group" && !it.meeting && bot.id in it.members }.map { it.id to it.name }, { selected = selected + (bot.id to it) }, enabled = !disabled)
                        }
                    }
                }
                if (selected.isNotEmpty()) CardSection("Invited delegates") {
                    for ((id, origin) in selected) row {
                        ValueRow(state.bots.firstOrNull { it.id == id }?.name ?: "Delegate", detail = state.bots.firstOrNull { it.id == origin }?.name ?: tr("Personal")) {
                            IconButton("Remove delegate", "minus.circle", enabled = !disabled) { selected = selected - id }
                        }
                    }
                }
                CardSection("Coordination") { row { ValueRow("Chair", detail = "Organizes proposals. Each delegate records their own response; you approve the final decision.") {
                    if (selected.isEmpty()) Text(tr("Choose delegates first"), color = Palette.secondary)
                    else ChoicePicker(chair, selected.keys.map { id -> id to (state.bots.firstOrNull { it.id == id }?.name ?: "Delegate") }, { chair = it }, enabled = !disabled)
                } } }
                Text(tr("Five delegates share a ten-turn budget. Groups are represented by bots; full chat histories are not imported. Delegates can read recent messages and tasks in departments they still belong to."), style = Type.caption, color = Palette.secondary)
                Text(tr("Creating the room approves these invitations. Ten turns and ten minutes maximum; all delegates must approve the current proposal before final user approval. Starts paused."), style = Type.caption, color = Palette.secondary)
                PrimaryButton("Create room and approve invitations", enabled = !disabled && selected.size >= 2 && chair in selected && name.isNotBlank() && agenda.isNotBlank()) {
                    busy = true
                    scope.launch {
                        try {
                            room = store.hostQuery("createMeeting", buildJsonObject {
                                put("name", name); put("agenda", agenda); put("kind", kind); put("chair", chair)
                                putJsonArray("delegates") { selected.forEach { (id, origin) -> add(buildJsonObject { put("botId", id); if (origin.isNotEmpty()) put("groupId", origin) }) } }
                            }).jsonObject
                        } catch (cancelled: CancellationException) { throw cancelled }
                        catch (error: Exception) { failure = error.message }
                        finally { busy = false }
                    }
                }
            } else if (meeting != null) {
                val status = meeting.text("status").orEmpty()
                CardSection("${meeting.text("kind")} · $status") {
                    row { MarkdownText(meeting.text("agenda").orEmpty()) }
                    row { Text("${meeting["spent"]}/${meeting["limit"]} ${tr("discussion turns used")}", color = Palette.secondary) }
                }
                CardSection("Proposal · ${meeting["proposalRevision"]}") {
                    row { MarkdownText(meeting.text("proposal").orEmpty().ifEmpty { tr("No proposal recorded.") }) }
                    for (delegate in meeting["delegates"]?.jsonArray.orEmpty()) row {
                        val d = delegate.jsonObject
                        val position = meeting["positions"]?.jsonObject?.get(d.text("botId").orEmpty())?.jsonObject
                        Column { Text("${d.text("name")} · ${d.text("department")}", color = Palette.text); Text(position?.let { "${it.text("position")} · ${it.text("reason")}" } ?: tr("Not answered"), color = Palette.secondary) }
                    }
                }
                if (status in listOf("paused", "running", "awaiting_owner")) CardSection("Owner proposal") {
                    row { FieldBox(proposal, { proposal = it.take(8000) }, "Proposal", minLines = 2, maxLines = 8) }
                    row { SecondaryButton("Publish proposal and reset responses", enabled = !disabled && proposal.isNotBlank()) { run("propose") } }
                }
                val history = meeting["history"]?.jsonArray.orEmpty()
                if (history.isNotEmpty()) CardSection("Previous proposals") {
                    row { SecondaryButton("${history.size} recorded versions") { historyOpen = !historyOpen } }
                    if (historyOpen) for (saved in history) row {
                        val version = saved.jsonObject
                        Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                            Text("Revision ${version["proposalRevision"]} · ${version.text("status")}", color = Palette.text)
                            MarkdownText(version.text("proposal").orEmpty())
                            for (delegate in meeting["delegates"]?.jsonArray.orEmpty()) {
                                val d = delegate.jsonObject
                                Text("${d.text("name")}: ${version["positions"]?.jsonObject?.get(d.text("botId").orEmpty())?.jsonObject?.text("position") ?: tr("Not answered")}", style = Type.caption, color = Palette.secondary)
                            }
                        }
                    }
                }
                CardSection("Room actions") {
                    if (status in listOf("paused", "unresolved")) row { SecondaryButton("Start / Resume", enabled = !disabled && (meeting["spent"]?.jsonPrimitive?.intOrNull ?: 10) < 10) { run("resume") } }
                    if (status == "running") row { SecondaryButton("Pause", enabled = !disabled) { run("pause") } }
                    if (meeting.text("kind") != "decision" && status !in listOf("completed", "resolved", "archived", "cancelled")) row { SecondaryButton("Complete with summary", enabled = !disabled && proposal.isNotBlank()) { run("finish") } }
                    if (status == "awaiting_owner") row { PrimaryButton("Approve final decision", enabled = !disabled) { run("approve") } }
                    if (status in listOf("resolved", "completed", "unresolved", "cancelled")) row { SecondaryButton("Publish outcome to departments", enabled = !disabled) { run("report") } }
                    row { SecondaryButton(if (status == "archived") "Reopen" else "Archive", enabled = !disabled) { run(if (status == "archived") "reopen" else "archive") } }
                    if (status !in listOf("resolved", "completed", "archived", "cancelled")) row { SecondaryButton("Cancel meeting", enabled = !disabled) { run("cancel") } }
                }
            }
        }
    }
}

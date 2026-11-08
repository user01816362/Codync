package com.codync.android

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.codync.android.core.Attachment
import com.codync.android.design.*
import kotlinx.coroutines.CancellationException
import kotlinx.serialization.json.*

@Composable internal fun WorkspaceBrowser(botId: String, store: AppStore) {
    var path by remember(botId) { mutableStateOf(".") }
    var hidden by remember(botId) { mutableStateOf(false) }
    var attempt by remember(botId) { mutableStateOf(0) }
    var directory by remember(botId) { mutableStateOf<JsonObject?>(null) }
    var failure by remember(botId) { mutableStateOf<String?>(null) }
    var selected by remember(botId) { mutableStateOf<Pair<String, String>?>(null) }
    LaunchedEffect(botId, path, hidden, attempt) {
        directory = null
        failure = null
        try {
            directory = store.hostQuery("listWorkspaceFiles", buildJsonObject {
                put("botId", botId); put("path", path); put("showHidden", hidden)
            }).jsonObject
        } catch (cancelled: CancellationException) { throw cancelled }
        catch (error: Exception) { failure = error.message ?: "Workspace unavailable" }
    }
    Column(Modifier.fillMaxSize()) {
        ModalHeader("Workspace files")
        Row(Modifier.fillMaxWidth().padding(horizontal = 16.dp), verticalAlignment = Alignment.CenterVertically) {
            IconButton("Workspace root", "house", enabled = path != ".") { path = "." }
            val parent = directory?.get("parent")?.jsonPrimitive?.contentOrNull
            IconButton("Parent folder", "arrow.up", enabled = parent != null) { parent?.let { path = it } }
            Text(directory?.value("path") ?: path, Modifier.weight(1f), style = Type.caption, color = Palette.secondary, maxLines = 2)
            IconButton("Retry", "arrow.clockwise", enabled = directory != null || failure != null) { attempt += 1 }
        }
        Row(Modifier.fillMaxWidth().padding(horizontal = 20.dp), verticalAlignment = Alignment.CenterVertically) {
            Text(tr("Hidden files"), Modifier.weight(1f), style = Type.caption, color = Palette.secondary)
            CodyncSwitch(hidden, { hidden = it })
        }
        Column(Modifier.weight(1f).fillMaxWidth().verticalScroll(rememberScrollState()).padding(20.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            val listing = directory
            if (failure != null) Text(failure.orEmpty(), color = Palette.danger)
            else if (listing == null) Spinner()
            else {
                if (listing["truncated"]?.jsonPrimitive?.booleanOrNull == true) Text(tr("Listing limited to 500 items from at most 2,000 entries."), style = Type.caption, color = Palette.secondary)
                val entries = listing["entries"]?.jsonArray.orEmpty()
                if (entries.isEmpty()) Text(tr("This folder is empty."), color = Palette.secondary)
                for (entry in entries) {
                    val item = entry.jsonObject
                    val name = item.value("name")
                    val folder = item.value("kind") == "directory"
                    val size = item["size"]?.jsonPrimitive?.longOrNull ?: 0
                    val readable = item.value("kind") == "file" && Attachment(item.value("path"), name, size).showsAsText
                    Row((if (folder || readable) Modifier.plainButton {
                        if (folder) path = item.value("path") else selected = listing.value("botId") to item.value("path")
                    } else Modifier).fillMaxWidth().padding(vertical = 6.dp), verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                        Symbol(if (folder) "folder" else "doc.text", size = 18.dp, tint = Palette.secondary)
                        Column(Modifier.weight(1f)) {
                            Text(name, style = Type.body, color = Palette.text, maxLines = 2)
                            Text(if (folder) tr("Folder") else if (readable) formatBytes(size) else tr("Ask the bot to share this file for preview."), style = Type.caption, color = Palette.secondary)
                        }
                    }
                }
            }
        }
    }
    CodyncSheet(selected, { selected = null }) { WorkspaceTextPreview(it.first, it.second, store) }
}

private fun JsonObject.value(key: String) = this[key]?.jsonPrimitive?.contentOrNull.orEmpty()

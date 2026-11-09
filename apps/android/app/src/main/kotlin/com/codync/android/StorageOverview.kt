package com.codync.android

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.codync.android.design.*
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.launch
import kotlinx.serialization.json.*

@Composable internal fun StorageOverview(store: AppStore) {
    var value by remember { mutableStateOf<JsonObject?>(null) }
    var busy by remember { mutableStateOf(false) }
    var failure by remember { mutableStateOf<String?>(null) }
    var confirm by remember { mutableStateOf(false) }
    var attempt by remember { mutableStateOf(0) }
    val scope = rememberCoroutineScope()
    suspend fun load(compact: Boolean = false) {
        if (busy) return
        busy = true; failure = null
        try { value = store.hostQuery(if (compact) "compactStorage" else "storageOverview", buildJsonObject { put("confirm", compact) }).jsonObject }
        catch (cancelled: CancellationException) { throw cancelled }
        catch (error: Exception) { failure = error.message }
        finally { busy = false }
    }
    LaunchedEffect(attempt) { load() }
    Column(Modifier.fillMaxSize()) {
        ModalHeader("Host storage") { IconButton("Refresh storage", "arrow.clockwise", enabled = !busy) { attempt += 1 } }
        Column(Modifier.weight(1f).fillMaxWidth().verticalScroll(rememberScrollState()).padding(20.dp), verticalArrangement = Arrangement.spacedBy(20.dp)) {
            Text(tr("Stored on the connected computer. File sizes are a bounded scan of managed file roots; workspace project files, logs, caches and database journals are excluded."), style = Type.caption, color = Palette.secondary)
            failure?.let { Text(it, color = Palette.danger) }
            if (busy) Spinner()
            value?.let { data ->
                Text("Conversation database: ${formatBytes(data.getValue("databaseBytes").jsonPrimitive.long)} · ${data.getValue("entries").jsonPrimitive.long} entries", color = Palette.text)
                Text("Reusable database pages: ${formatBytes(data.getValue("reusableBytes").jsonPrimitive.long)}", color = Palette.text)
                listOf("sharedFiles" to "Shared files", "uploads" to "Uploads", "artifacts" to "Artifacts").forEach { (key, label) ->
                    val category = data.getValue(key).jsonObject
                    Text("${tr(label)}: ${formatBytes(category.getValue("bytes").jsonPrimitive.long)} · ${category.getValue("count").jsonPrimitive.long} files", color = Palette.text)
                    if (category.getValue("truncated").jsonPrimitive.boolean || category.getValue("unavailable").jsonPrimitive.int > 0) Text("Partial scan · ${category.getValue("unavailable")} unavailable paths", style = Type.caption, color = Palette.secondary)
                }
                SecondaryButton("Compact database", enabled = !busy && data.getValue("reusableBytes").jsonPrimitive.long > 0) { confirm = true }
            }
        }
    }
    CodyncDialog("Compact database?", confirm, { confirm = false }, message = "This preserves saved messages and files. SQLite may need up to twice the database size in temporary free space and briefly blocks host database operations. File caches are not deleted.") {
        listOf(DialogAction("Compact database") { confirm = false; scope.launch { load(true) } })
    }
}

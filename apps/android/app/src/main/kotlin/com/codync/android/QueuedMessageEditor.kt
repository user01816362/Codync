package com.codync.android

import androidx.compose.foundation.layout.*
import androidx.compose.material3.Text
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.codync.android.core.Entry
import com.codync.android.design.*
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.launch

@Composable internal fun QueuedMessageEditor(entry: Entry, store: AppStore) {
    val dismiss = LocalDismissModal.current
    val scope = rememberCoroutineScope()
    var text by remember(entry.id) { mutableStateOf(entry.text) }
    var saving by remember { mutableStateOf(false) }
    var failure by remember { mutableStateOf<String?>(null) }
    Column(verticalArrangement = Arrangement.spacedBy(16.dp)) {
        ModalHeader("Edit queued message") {
            if (saving) Spinner() else IconButton("Save", "checkmark", enabled = text.isNotBlank() && text.toByteArray().size <= 65536) {
                saving = true
                failure = null
                scope.launch {
                    try {
                        store.hostQuery("editQueued", body("botId" to entry.botId, "entryId" to entry.id, "text" to text))
                        dismiss()
                    } catch (cancelled: CancellationException) { throw cancelled }
                    catch (error: Exception) { failure = error.message }
                    finally { saving = false }
                }
            }
        }
        Column(Modifier.fillMaxWidth().padding(horizontal = 20.dp).padding(bottom = 24.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
            Text(tr("Changes apply only while the message is waiting. Files and reply context stay attached."), style = Type.caption, color = Palette.secondary)
            PlainTextField(text, { if (!saving) text = it }, Modifier.fillMaxWidth(), "Message", maxLines = 12)
            failure?.let { Text(it, style = Type.caption, color = Palette.danger) }
        }
    }
}

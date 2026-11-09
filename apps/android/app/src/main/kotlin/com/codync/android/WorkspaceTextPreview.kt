package com.codync.android

import androidx.compose.runtime.*
import com.codync.android.design.LocalDismissModal
import kotlinx.coroutines.launch
import java.net.URI
import kotlinx.coroutines.CancellationException
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive

/** Only explicit Markdown links become workspace requests; host enforces containment. */
internal fun workspaceLink(destination: String): String? = try {
    val uri = URI(destination)
    val local = uri.scheme == null && uri.rawAuthority == null ||
        uri.scheme.equals("file", ignoreCase = true) && (uri.rawAuthority == null || uri.rawAuthority.equals("localhost", ignoreCase = true))
    uri.path?.takeIf { local && uri.query == null && uri.fragment == null && it.isNotEmpty() }
} catch (_: Exception) { null }

@Composable internal fun WorkspaceTextPreview(botId: String, path: String, store: AppStore) {
    val scope = rememberCoroutineScope()
    val dismiss = LocalDismissModal.current
    var text by remember(botId, path) { mutableStateOf<String?>(null) }
    var name by remember(botId, path) { mutableStateOf(path.substringAfterLast('/')) }
    var failure by remember(botId, path) { mutableStateOf<String?>(null) }
    var attempt by remember(botId, path) { mutableStateOf(0) }
    LaunchedEffect(botId, path, attempt) {
        text = null
        failure = null
        try {
            val response = store.hostQuery("previewWorkspaceFile", body("botId" to botId, "path" to path)).jsonObject
            name = response.getValue("name").jsonPrimitive.content
            text = response.getValue("text").jsonPrimitive.content
        } catch (cancelled: CancellationException) { throw cancelled }
        catch (error: Exception) { failure = error.message ?: "Couldn't open this file." }
    }
    TextPreviewBody("$botId:$path", name, text, failure, store, quote = { selected ->
        scope.launch {
            try { store.selectWorkspaceReply(botId, path, selected); dismiss() }
            catch (cancelled: CancellationException) { throw cancelled }
            catch (error: Exception) { store.showError(error) }
        }
    }) { attempt += 1 }
}

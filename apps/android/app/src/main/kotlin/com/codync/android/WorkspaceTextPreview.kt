package com.codync.android

import androidx.compose.foundation.background
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.codync.android.design.*
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
    var text by remember(path) { mutableStateOf<String?>(null) }
    var name by remember(path) { mutableStateOf(path.substringAfterLast('/')) }
    var failure by remember(path) { mutableStateOf<String?>(null) }
    LaunchedEffect(botId, path) {
        try {
            val response = store.hostQuery("previewWorkspaceFile", body("botId" to botId, "path" to path)).jsonObject
            name = response.getValue("name").jsonPrimitive.content
            text = response.getValue("text").jsonPrimitive.content
        } catch (cancelled: CancellationException) { throw cancelled }
        catch (error: Exception) { failure = error.message ?: "Couldn't open this file." }
    }
    val markdown = name.endsWith(".md", true) || name.endsWith(".markdown", true)
    val horizontal = rememberScrollState()
    Column(Modifier.fillMaxSize().background(Palette.background)) {
        ModalHeader(name)
        Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState()).then(if (markdown) Modifier else Modifier.horizontalScroll(horizontal)).padding(20.dp)) {
            val content = text
            when {
                content != null && markdown -> MarkdownText(content)
                content != null -> Text(content, style = Type.footnote.monospaced, color = Palette.text)
                failure != null -> Text(failure.orEmpty(), color = Palette.secondary)
                else -> Spinner()
            }
        }
    }
}

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
import com.codync.android.core.SharedFile
import com.codync.android.core.Attachment
import com.codync.android.design.*
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.nio.ByteBuffer
import java.nio.charset.CodingErrorAction
import java.security.MessageDigest

/** A small verified snapshot, rendered without executing any file contents. */
@Composable internal fun SharedTextPreview(file: SharedFile, entryId: String, store: AppStore) {
    TextFilePreview(file.id, file.name, file.size, file.showsAsText, file.sha256) { store.sharedImage(entryId, file) }
}

@Composable internal fun AttachmentTextPreview(file: Attachment, botId: String, store: AppStore) {
    TextFilePreview(file.id, file.name, file.size, file.showsAsText, null) { store.attachment(botId, file) }
}

@Composable private fun TextFilePreview(id: String, name: String, size: Long, supported: Boolean, sha256: String?, load: suspend () -> java.io.File) {
    var text by remember(id) { mutableStateOf<String?>(null) }
    var failed by remember(id) { mutableStateOf(false) }
    LaunchedEffect(id) {
        try {
            require(supported)
            val cached = load()
            text = withContext(Dispatchers.IO) {
                require(cached.length() == size && cached.length() <= 1024 * 1024)
                val bytes = cached.readBytes()
                require(bytes.size.toLong() == size && bytes.size <= 1024 * 1024)
                if (sha256 != null) {
                    val digest = MessageDigest.getInstance("SHA-256").digest(bytes).joinToString("") { "%02x".format(it) }
                    require(digest == sha256)
                }
                val decoded = Charsets.UTF_8.newDecoder().onMalformedInput(CodingErrorAction.REPORT)
                    .onUnmappableCharacter(CodingErrorAction.REPORT).decode(ByteBuffer.wrap(bytes)).toString()
                require('\u0000' !in decoded)
                decoded
            }
        } catch (cancelled: CancellationException) { throw cancelled }
        catch (_: Exception) { failed = true }
    }
    Column(Modifier.fillMaxSize().background(Palette.background)) {
        ModalHeader(name)
        Column(Modifier.fillMaxSize().verticalScroll(rememberScrollState()).horizontalScroll(rememberScrollState()).padding(20.dp)) {
            val content = text
            when {
                content != null && (name.endsWith(".md", true) || name.endsWith(".markdown", true)) -> MarkdownText(content)
                content != null -> Text(content, style = Type.footnote.monospaced, color = Palette.text)
                failed -> Text(tr("Couldn't open this file."), color = Palette.secondary)
                else -> Spinner()
            }
        }
    }
}

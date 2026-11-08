package com.codync.android

import androidx.compose.runtime.*
import com.codync.android.core.SharedFile
import com.codync.android.core.Attachment
import com.codync.android.design.LocalDismissModal
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.nio.ByteBuffer
import java.nio.charset.CodingErrorAction
import java.security.MessageDigest

/** A small verified snapshot, rendered without executing any file contents. */
@Composable internal fun SharedTextPreview(file: SharedFile, entryId: String, store: AppStore) {
    val scope = rememberCoroutineScope()
    val dismiss = LocalDismissModal.current
    if (mediaPreviewKind(file.name, file.size) != null) MediaFilePreview(file.id, file.name, file.size, file.sha256) { store.sharedImage(entryId, file) }
    else TextFilePreview(file.id, file.name, file.size, file.showsAsText, file.sha256, store, quote = { passage ->
        scope.launch {
            try { store.selectFileReply(entryId, file.id, passage); dismiss() }
            catch (cancelled: CancellationException) { throw cancelled }
            catch (error: Exception) { store.showError(error) }
        }
    }) { store.sharedImage(entryId, file) }
}

@Composable internal fun AttachmentTextPreview(file: Attachment, botId: String, store: AppStore) {
    if (mediaPreviewKind(file.name, file.size) != null) MediaFilePreview(file.id, file.name, file.size, null) { store.attachment(botId, file) }
    else TextFilePreview(file.id, file.name, file.size, file.showsAsText, null, store) { store.attachment(botId, file) }
}

@Composable private fun TextFilePreview(id: String, name: String, size: Long, supported: Boolean, sha256: String?, store: AppStore, quote: ((String) -> Unit)? = null, load: suspend () -> java.io.File) {
    var text by remember(id) { mutableStateOf<String?>(null) }
    var failure by remember(id) { mutableStateOf<String?>(null) }
    var attempt by remember(id) { mutableStateOf(0) }
    LaunchedEffect(id, attempt) {
        text = null
        failure = null
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
        catch (_: Exception) { failure = "Couldn't open this file. The file must be valid UTF-8 text up to 1 MiB and pass its integrity checks." }
    }
    TextPreviewBody(id, name, text, failure, store, quote = quote) { attempt += 1 }
}

internal val SharedFile.canOpenPreview: Boolean get() = showsAsText || mediaPreviewKind(name, size) != null
internal val Attachment.canOpenPreview: Boolean get() = showsAsText || mediaPreviewKind(name, size) != null

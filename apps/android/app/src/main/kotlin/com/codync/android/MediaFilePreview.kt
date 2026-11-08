package com.codync.android

import android.graphics.Bitmap
import android.graphics.Color
import android.graphics.pdf.PdfRenderer
import android.os.ParcelFileDescriptor
import android.widget.MediaController
import android.widget.VideoView
import androidx.compose.foundation.Image
import androidx.compose.foundation.layout.*
import androidx.compose.material3.Text
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import com.codync.android.design.*
import java.io.File
import java.security.MessageDigest
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

internal fun mediaPreviewKind(name: String, size: Long): String? {
    if (size < 0 || size > 32 * 1024 * 1024) return null
    return when (name.substringAfterLast('.', "").lowercase()) {
        "pdf" -> "pdf"
        "mp3", "m4a", "wav", "ogg" -> "audio"
        "mp4", "m4v", "mov", "webm" -> "video"
        else -> null
    }
}

/** Native viewers consume only a bounded, locally verified cache file, never an agent URL. */
@Composable internal fun MediaFilePreview(id: String, name: String, size: Long, sha256: String?, load: suspend () -> File) {
    var file by remember(id) { mutableStateOf<File?>(null) }
    var failure by remember(id) { mutableStateOf<String?>(null) }
    var attempt by remember(id) { mutableStateOf(0) }
    LaunchedEffect(id, attempt) {
        file = null
        failure = null
        try {
            require(mediaPreviewKind(name, size) != null)
            val cached = load()
            withContext(Dispatchers.IO) {
                require(cached.length() == size && size <= 32 * 1024 * 1024)
                if (sha256 != null) {
                    val digest = MessageDigest.getInstance("SHA-256")
                    cached.inputStream().use { input ->
                        val buffer = ByteArray(16384)
                        var total = 0L
                        while (true) {
                            val count = input.read(buffer)
                            if (count < 0) break
                            total += count
                            require(total <= size)
                            digest.update(buffer, 0, count)
                        }
                        require(total == size)
                    }
                    require(digest.digest().joinToString("") { "%02x".format(it) } == sha256)
                }
            }
            file = cached
        } catch (cancelled: CancellationException) { throw cancelled }
        catch (error: Exception) { failure = error.message ?: "Couldn't verify this file." }
    }
    Column(Modifier.fillMaxSize()) {
        ModalHeader(name) { IconButton("Retry", "arrow.clockwise", enabled = file != null || failure != null) { attempt += 1 } }
        val cached = file
        when {
            failure != null -> Text(failure.orEmpty(), Modifier.padding(20.dp), color = Palette.secondary)
            cached == null -> Spinner()
            mediaPreviewKind(name, size) == "pdf" -> NativePDF(cached, name)
            else -> NativePlayback(cached)
        }
    }
}

@Composable private fun NativePDF(file: File, name: String) {
    var page by remember(file) { mutableStateOf(0) }
    var pages by remember(file) { mutableStateOf(0) }
    var image by remember(file) { mutableStateOf<Bitmap?>(null) }
    var failure by remember(file) { mutableStateOf<String?>(null) }
    LaunchedEffect(file, page) {
        image = null
        failure = null
        try {
            val result = withContext(Dispatchers.IO) {
                ParcelFileDescriptor.open(file, ParcelFileDescriptor.MODE_READ_ONLY).use { descriptor ->
                    PdfRenderer(descriptor).use { renderer ->
                        require(renderer.pageCount in 1..500)
                        val count = renderer.pageCount
                        renderer.openPage(page.coerceIn(0, count - 1)).use { current ->
                            val scale = 2048f / maxOf(current.width, current.height).coerceAtLeast(1)
                            val bitmap = Bitmap.createBitmap((current.width * scale).toInt().coerceAtLeast(1),
                                (current.height * scale).toInt().coerceAtLeast(1), Bitmap.Config.ARGB_8888)
                            bitmap.eraseColor(Color.WHITE)
                            current.render(bitmap, null, null, PdfRenderer.Page.RENDER_MODE_FOR_DISPLAY)
                            bitmap to count
                        }
                    }
                }
            }
            image = result.first
            pages = result.second
        } catch (cancelled: CancellationException) { throw cancelled }
        catch (error: Exception) { failure = error.message ?: "This PDF cannot be previewed." }
    }
    Column(Modifier.fillMaxSize()) {
        Row(Modifier.fillMaxWidth().padding(horizontal = 16.dp), horizontalArrangement = Arrangement.SpaceBetween) {
            IconButton("Previous page", "chevron.left", enabled = page > 0) { page -= 1 }
            Text("${page + 1} / $pages", style = Type.caption, color = Palette.secondary)
            IconButton("Next page", "chevron.right", enabled = page + 1 < pages) { page += 1 }
        }
        failure?.let { Text(it, Modifier.padding(20.dp), color = Palette.secondary) }
        image?.let { Image(it.asImageBitmap(), contentDescription = "$name, page ${page + 1}", modifier = Modifier.fillMaxWidth().weight(1f)) }
    }
}

@Composable private fun NativePlayback(file: File) {
    var failure by remember(file) { mutableStateOf(false) }
    if (failure) Text("This media format could not be played. Download it from its card instead.", Modifier.padding(20.dp), color = Palette.secondary)
    else AndroidView(factory = { context ->
        VideoView(context).apply {
            setMediaController(MediaController(context).also { it.setAnchorView(this) })
            setVideoPath(file.absolutePath)
            setOnErrorListener { _, _, _ -> failure = true; true }
        }
    }, modifier = Modifier.fillMaxWidth().heightIn(min = 160.dp), onReset = null, onRelease = { it.stopPlayback() })
}

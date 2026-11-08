package com.codync.android

import android.annotation.SuppressLint
import android.content.ClipData
import android.content.ClipboardManager
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.rememberScrollState
import android.util.Base64
import android.webkit.RenderProcessGoneDetail
import android.webkit.WebResourceRequest
import android.webkit.WebResourceResponse
import android.webkit.WebView
import android.webkit.WebViewClient
import androidx.compose.animation.animateContentSize
import androidx.compose.foundation.layout.*
import androidx.compose.material3.Text
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import com.codync.android.design.*
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put
import java.io.ByteArrayInputStream

/** Offline renderer with no app bridge, navigation, file access or network requests. */
@SuppressLint("SetJavaScriptEnabled")
@Composable internal fun RichContentPreview(source: String, language: String, allowMarkdownPreview: Boolean = true, fullSize: Boolean = false) {
    val context = LocalContext.current
    var expanded by remember { mutableStateOf(false) }
    val markdown = allowMarkdownPreview && language in listOf("md", "markdown")
    val canPreview = source.isNotEmpty() && (markdown || (source.length <= 16000 && language !in listOf("md", "markdown")))
    var showSource by remember(language, allowMarkdownPreview) { mutableStateOf<Boolean?>(null) }
    val preview = canPreview && !(showSource ?: !(markdown || language in listOf("mermaid", "math", "latex", "tex")))
    var failed by remember(source, language) { mutableStateOf(false) }
    Column((if (fullSize) Modifier.fillMaxSize() else Modifier.fillMaxWidth()).animateContentSize().background(Palette.codeBackground)) {
        if (fullSize) ModalHeader("Preview")
        Row(Modifier.fillMaxWidth().padding(start = 10.dp, end = 4.dp), verticalAlignment = androidx.compose.ui.Alignment.CenterVertically) {
            Text(language.ifEmpty { "text" }, style = Type.caption, color = Palette.secondary, maxLines = 1)
            Spacer(Modifier.weight(1f))
            if (canPreview) Text(tr(if (preview) "Code" else "Preview"), Modifier.plainButton { showSource = preview }.padding(4.dp), style = Type.caption, color = Palette.secondary)
            if (preview && !markdown && !fullSize) IconButton("Expand preview", "arrow.up.left.and.arrow.down.right") { expanded = true }
            IconButton("Copy", "square.on.square") {
                context.getSystemService(ClipboardManager::class.java).setPrimaryClip(ClipData.newPlainText("Code", source))
            }
        }
        if (!preview) Box(Modifier.horizontalScroll(rememberScrollState())) {
            Text(source, Modifier.padding(10.dp), style = Type.footnote.monospaced, color = Palette.text, softWrap = false)
        }
        else if (markdown) Box(Modifier.padding(12.dp)) { MarkdownText(source, allowMarkdownPreview = false) }
        else {
            val html by produceState<String?>(null, source, language) {
                value = withContext(Dispatchers.IO) {
                    try {
                        val kind = when (language) { "mermaid" -> "mermaid"; "math", "latex", "tex" -> "math"; else -> "code" }
                        val payload = buildJsonObject { put("kind", kind); put("source", source); put("language", language) }
                        val encoded = Base64.encodeToString(payload.toString().toByteArray(Charsets.UTF_8), Base64.NO_WRAP)
                        context.assets.open("rich-renderer.html").bufferedReader().use { it.readText() }
                            .replace("__CODYNC_PAYLOAD__", encoded)
                    } catch (_: java.io.IOException) { failed = true; null }
                }
            }
            if (failed) Text("Preview couldn’t load. Choose Code to view the source.", style = Type.caption2, color = Palette.secondary)
            else if (html == null) Text("Loading preview…", style = Type.caption2, color = Palette.secondary)
            else AndroidView(factory = { ctx ->
                WebView(ctx).apply {
                    setBackgroundColor(android.graphics.Color.TRANSPARENT)
                    settings.javaScriptEnabled = true
                    settings.domStorageEnabled = false
                    settings.allowFileAccess = false
                    settings.allowContentAccess = false
                    settings.blockNetworkLoads = true
                    settings.javaScriptCanOpenWindowsAutomatically = false
                    webViewClient = object : WebViewClient() {
                        override fun shouldOverrideUrlLoading(view: WebView, request: WebResourceRequest) = true
                        override fun shouldInterceptRequest(view: WebView, request: WebResourceRequest): WebResourceResponse =
                            WebResourceResponse("text/plain", "UTF-8", ByteArrayInputStream(ByteArray(0)))
                        override fun onRenderProcessGone(view: WebView, detail: RenderProcessGoneDetail): Boolean {
                            failed = true
                            return true
                        }
                    }
                }
            }, modifier = if (fullSize) Modifier.fillMaxWidth().weight(1f) else Modifier.fillMaxWidth().height(if (language in listOf("mermaid", "math", "latex", "tex")) 280.dp else (source.lineSequence().count() * 18 + 20).coerceIn(64, 360).dp), onRelease = { it.stopLoading(); it.destroy() }, update = { view ->
                if (view.tag != html) { view.tag = html; view.loadDataWithBaseURL(null, html!!, "text/html", "UTF-8", null) }
            })
        }
    }
    if (!fullSize) CodyncSheet(expanded, { expanded = false }) {
        RichContentPreview(source, language, allowMarkdownPreview, fullSize = true)
    }
}

package com.codync.android

import android.content.ClipData
import android.content.ClipboardManager
import androidx.compose.animation.animateContentSize
import androidx.compose.foundation.background
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Text
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import com.codync.android.design.*
import kotlinx.coroutines.CancellationException
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlinx.serialization.json.booleanOrNull

/** One presentation for verified attachments and host-confined workspace text. */
@Composable internal fun TextPreviewBody(id: String, name: String, text: String?, failure: String?, store: AppStore, quote: ((String) -> Unit)? = null, retry: () -> Unit) {
    val context = LocalContext.current
    var selectingQuote by remember(id) { mutableStateOf(false) }
    var source by remember(id) { mutableStateOf(false) }
    var wrap by remember(id) { mutableStateOf(true) }
    var table by remember(id) { mutableStateOf<String?>(null) }
    var notice by remember(id) { mutableStateOf<String?>(null) }
    val tabular = name.endsWith(".csv", true) || name.endsWith(".tsv", true)
    LaunchedEffect(id, text) {
        table = null
        notice = null
        if (tabular && text != null) {
            if (text.toByteArray(Charsets.UTF_8).size > 65536) notice = "Table previews are limited to 64 KiB. Source shows the full text."
            else try {
                val response = store.hostQuery("formatTable", body("name" to name, "text" to text)).jsonObject
                table = response.getValue("markdown").jsonPrimitive.content
                if (response["truncated"]?.jsonPrimitive?.booleanOrNull == true) notice = "Table preview truncated. Source shows the full text."
            } catch (cancelled: CancellationException) { throw cancelled }
            catch (error: Exception) { notice = error.message ?: "Table preview unavailable. Source remains available." }
        }
    }
    val markdown = name.endsWith(".md", true) || name.endsWith(".markdown", true)
    if (selectingQuote) CodyncSheet(true, { selectingQuote = false }) {
        QuoteSelectionSheet(QuoteSelectionTarget(text.orEmpty(), name), structured = true) {
            quote?.invoke(it)
            selectingQuote = false
        }
    }
    Column(Modifier.fillMaxSize().background(Palette.background)) {
        ModalHeader(name)
        Row(Modifier.fillMaxWidth().animateContentSize().padding(horizontal = 16.dp).padding(bottom = 8.dp),
            horizontalArrangement = Arrangement.End, verticalAlignment = Alignment.CenterVertically) {
            if (quote != null) IconButton("Select text / Quote", "arrowshape.turn.up.left", enabled = text != null) { selectingQuote = true }
            if (markdown || tabular) Text(tr(if (source) "Read" else "Source"),
                Modifier.plainButton { source = !source }.padding(8.dp), style = Type.caption, color = Palette.secondary)
            if (!markdown && !tabular || source) IconButton(if (wrap) "Disable line wrapping" else "Wrap lines", "text.alignleft") { wrap = !wrap }
            IconButton("Copy", "square.on.square", enabled = text != null) {
                text?.let { context.getSystemService(ClipboardManager::class.java).setPrimaryClip(ClipData.newPlainText(name, it)) }
            }
            IconButton("Retry", "arrow.clockwise", enabled = text != null || failure != null, action = retry)
        }
        Column(Modifier.weight(1f).fillMaxWidth().verticalScroll(rememberScrollState())
            .then(if ((markdown || table != null) && !source || wrap) Modifier else Modifier.horizontalScroll(rememberScrollState())).padding(20.dp)) {
            if (!source) notice?.let { Text(tr(it), style = Type.caption, color = Palette.secondary) }
            when {
                text != null && (markdown || table != null) && !source -> MarkdownText(table ?: text)
                text != null -> SelectionContainer { Text(text, style = Type.footnote.monospaced, color = Palette.text, softWrap = wrap) }
                failure != null -> SelectionContainer { Text(failure, color = Palette.secondary) }
                else -> Spinner()
            }
        }
    }
}

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

/** One presentation for verified attachments and host-confined workspace text. */
@Composable internal fun TextPreviewBody(id: String, name: String, text: String?, failure: String?, retry: () -> Unit) {
    val context = LocalContext.current
    var source by remember(id) { mutableStateOf(false) }
    var wrap by remember(id) { mutableStateOf(true) }
    val markdown = name.endsWith(".md", true) || name.endsWith(".markdown", true)
    Column(Modifier.fillMaxSize().background(Palette.background)) {
        ModalHeader(name)
        Row(Modifier.fillMaxWidth().animateContentSize().padding(horizontal = 16.dp).padding(bottom = 8.dp),
            horizontalArrangement = Arrangement.End, verticalAlignment = Alignment.CenterVertically) {
            if (markdown) Text(tr(if (source) "Read" else "Source"),
                Modifier.plainButton { source = !source }.padding(8.dp), style = Type.caption, color = Palette.secondary)
            if (!markdown || source) IconButton(if (wrap) "Disable line wrapping" else "Wrap lines", "text.alignleft") { wrap = !wrap }
            IconButton("Copy", "square.on.square", enabled = text != null) {
                text?.let { context.getSystemService(ClipboardManager::class.java).setPrimaryClip(ClipData.newPlainText(name, it)) }
            }
            IconButton("Retry", "arrow.clockwise", enabled = text != null || failure != null, action = retry)
        }
        Column(Modifier.weight(1f).fillMaxWidth().verticalScroll(rememberScrollState())
            .then(if (markdown && !source || wrap) Modifier else Modifier.horizontalScroll(rememberScrollState())).padding(20.dp)) {
            when {
                text != null && markdown && !source -> MarkdownText(text)
                text != null -> SelectionContainer { Text(text, style = Type.footnote.monospaced, color = Palette.text, softWrap = wrap) }
                failure != null -> SelectionContainer { Text(failure, color = Palette.secondary) }
                else -> Spinner()
            }
        }
    }
}

package com.codync.android

import android.content.Context
import android.widget.TextView
import androidx.compose.foundation.layout.*
import androidx.compose.material3.Text
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import com.codync.android.design.*

internal data class QuoteSelectionTarget(val text: String, val author: String)

/** Native selection and Copy handles; structured callers preserve message reply metadata. */
@Composable internal fun QuoteSelectionSheet(target: QuoteSelectionTarget, structured: Boolean = false, quote: (String) -> Unit) {
    var selected by remember(target) { mutableStateOf("") }
    Column(Modifier.fillMaxSize(), verticalArrangement = Arrangement.spacedBy(12.dp)) {
        ModalHeader("Select text")
        Text(target.author, Modifier.padding(horizontal = 20.dp), style = Type.subheadline, color = Palette.secondary)
        val ink = Palette.text.toArgb()
        AndroidView(factory = { context ->
            QuoteTextView(context).apply {
                setTextIsSelectable(true)
                textSize = 17f
                val density = resources.displayMetrics.density
                setPadding((20 * density).toInt(), (12 * density).toInt(), (20 * density).toInt(), (12 * density).toInt())
            }
        }, modifier = Modifier.fillMaxWidth().weight(1f), update = { view ->
            view.changed = { selected = it }
            view.setTextColor(ink)
            if (view.text.toString() != target.text) view.text = target.text
        })
        SecondaryButton("Quote selected text", Modifier.padding(horizontal = 20.dp, vertical = 16.dp), enabled = selected.isNotBlank()) {
            val passage = buildString { selected.trim().codePoints().limit(4000).forEach { appendCodePoint(it) } }
            quote(if (structured) passage else target.author + ":\n" + passage.lineSequence().joinToString("\n") { "> $it" } + "\n\n")
        }
    }
}

private class QuoteTextView(context: Context) : TextView(context) {
    var changed: ((String) -> Unit)? = null
    override fun onSelectionChanged(start: Int, end: Int) {
        super.onSelectionChanged(start, end)
        val content = text ?: return
        val from = minOf(start, end)
        val to = maxOf(start, end)
        changed?.invoke(if (from >= 0 && to <= content.length && from < to) content.subSequence(from, to).toString() else "")
    }
}

package com.codync.android

import androidx.compose.animation.animateContentSize
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.material3.Text
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.unit.dp
import com.codync.android.design.*
import kotlinx.serialization.json.*

/** Native, inert report sections. Only explicit HTTP(S) source links leave the app. */
@Composable internal fun RichReportView(report: JsonObject, submit: (String) -> Unit) {
    val uriHandler = LocalUriHandler.current
    val sources = (report["sources"] as? JsonArray).orEmpty().take(30)
    Column(Modifier.padding(16.dp).animateContentSize(), verticalArrangement = Arrangement.spacedBy(12.dp)) {
        Text(report.string("title"), style = Type.headline, color = Palette.text)
        for ((index, element) in (report["sections"] as? JsonArray).orEmpty().take(24).withIndex()) {
            val section = element as? JsonObject ?: continue
            key(index) {
                val answers = remember { mutableStateMapOf<Int, String>() }
                var sent by remember { mutableStateOf(false) }
                var expanded by remember { mutableStateOf(true) }
                Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
                    Text((if (expanded) "▾ " else "▸ ") + section.string("heading"),
                        Modifier.fillMaxWidth().clickable(role = Role.Button) { expanded = !expanded }
                            .semantics { stateDescription = if (expanded) "Expanded" else "Collapsed" },
                        style = Type.subheadline, color = Palette.text)
                    if (expanded) {
                        val text = section.string("text")
                        if (text.isNotEmpty()) Text(text, style = Type.body, color = Palette.text)
                        val items = (section["items"] as? JsonArray).orEmpty().take(30).mapNotNull { it as? JsonObject }
                        val maximum = items.mapNotNull { it.string("value").toDoubleOrNull()?.takeIf(Double::isFinite) }.maxOrNull() ?: 0.0
                        for ((itemIndex, item) in items.withIndex()) {
                            Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                                Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                                    Text(item.string("label"), Modifier.weight(1f), style = Type.subheadline, color = Palette.text)
                                    if (section.string("kind") == "form") PlainTextField(answers[itemIndex].orEmpty(), { answers[itemIndex] = it.take(500) },
                                        Modifier.weight(1f), prompt = item.string("value"), enabled = !sent)
                                    else Text(item.string("value"), style = Type.subheadline, color = Palette.secondary)
                                }
                                if (section.string("kind") == "bar") {
                                    val number = item.string("value").toDoubleOrNull()?.takeIf(Double::isFinite) ?: 0.0
                                    val fraction = if (maximum > 0) (number / maximum).coerceIn(0.0, 1.0).toFloat() else 0f
                                    Box(Modifier.fillMaxWidth(fraction).height(6.dp).background(Palette.secondary))
                                }
                            }
                        }
                        if (section.string("kind") == "form") SecondaryButton(if (sent) "Sent" else "Send answers",
                            enabled = !sent && items.indices.all { !answers[it].isNullOrBlank() }) {
                            submit(report.string("title") + " — " + section.string("heading") + "\n" +
                                items.mapIndexed { i, item -> item.string("label") + ": " + answers[i].orEmpty() }.joinToString("\n"))
                            sent = true
                        }
                        for (citation in (section["citations"] as? JsonArray).orEmpty()) {
                            val number = (citation as? JsonPrimitive)?.intOrNull ?: continue
                            val source = sources.getOrNull(number - 1) as? JsonObject ?: continue
                            ReportSource(source, number, uriHandler::openUri)
                        }
                    }
                }
            }
        }
        if (sources.isNotEmpty()) {
            Text("Sources supplied by the agent", style = Type.caption2, color = Palette.secondary)
            for ((index, element) in sources.withIndex()) {
                val source = element as? JsonObject ?: continue
                ReportSource(source, index + 1, uriHandler::openUri)
            }
        }
    }
}

@Composable private fun ReportSource(source: JsonObject, number: Int, open: (String) -> Unit) {
    val url = source.string("url")
    if (url.startsWith("https://", true) || url.startsWith("http://", true)) {
        Text("[$number] ${source.string("title")}", Modifier.clickable(role = Role.Button) { open(url) },
            style = Type.caption2, color = Palette.text)
    }
}

private fun JsonObject.string(key: String): String = (this[key] as? JsonPrimitive)?.contentOrNull.orEmpty()

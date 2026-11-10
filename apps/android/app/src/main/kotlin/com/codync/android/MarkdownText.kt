package com.codync.android

import androidx.compose.foundation.background
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.ui.Alignment
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.material3.Text
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.platform.LocalLayoutDirection
import androidx.compose.ui.unit.LayoutDirection
import java.text.Bidi
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.text.style.TextDirection
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.LinkAnnotation
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.TextLinkStyles
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.unit.dp
import coil3.ImageLoader
import com.codync.android.core.MarkdownBlocks
import com.codync.android.core.MarkdownBlocks.Block
import com.codync.android.design.*
import com.kyant.shapes.RoundedRectangle
import org.commonmark.ext.gfm.strikethrough.Strikethrough
import org.commonmark.ext.gfm.strikethrough.StrikethroughExtension
import org.commonmark.node.Code
import org.commonmark.node.Emphasis
import org.commonmark.node.HardLineBreak
import org.commonmark.node.HtmlInline
import org.commonmark.node.Image
import org.commonmark.node.Link
import org.commonmark.node.Node
import org.commonmark.node.SoftLineBreak
import org.commonmark.node.StrongEmphasis
import org.commonmark.parser.Parser

val LocalNetworkImages = staticCompositionLocalOf<ImageLoader> { error("Missing image loader") }

/**
 * The reply renderer of iOS `MarkdownText`: `MarkdownBlocks` for the blocks, inline styling for
 * bold, italic, code, strikethrough and links. Images show as their text: a reply never makes
 * the phone fetch an address.
 */
@Composable fun MarkdownText(source: String, selectable: Boolean = true, color: Color = Palette.text, openFile: ((String) -> Unit)? = null, allowMarkdownPreview: Boolean = true) {
    val blocks = remember(source) { MarkdownBlocks.parse(source) }
    val mathIndices = remember(blocks) { blocks.withIndex().filter { (_, block) -> block is Block.Paragraph && InlineMath.eligible(block.text) }.take(8).map { it.index }.toSet() }
    val content: @Composable () -> Unit = {
        Column(verticalArrangement = Arrangement.spacedBy(8.dp)) { for ((index, block) in blocks.withIndex()) MarkdownBlock(block, color, openFile, allowMarkdownPreview, index in mathIndices) }
    }
    if (selectable) SelectionContainer(content = content) else content()
}

@Composable private fun MarkdownBlock(block: Block, ink: Color, openFile: ((String) -> Unit)?, allowMarkdownPreview: Boolean, inlineMath: Boolean) {
    val code = Palette.text.copy(alpha = 0.08f)
    val prose = when (block) {
        is Block.Paragraph -> block.text
        is Block.Heading -> block.text
        is Block.Bullet -> block.text
        is Block.Quote -> block.text
        else -> ""
    }
    val direction = remember(prose) {
        if (Bidi(prose, Bidi.DIRECTION_DEFAULT_LEFT_TO_RIGHT).baseIsLeftToRight()) LayoutDirection.Ltr else LayoutDirection.Rtl
    }
    CompositionLocalProvider(LocalLayoutDirection provides direction) {
        when (block) {
            is Block.Paragraph -> Column {
                val text = block.text.trim()
                if (text.length > 4 && text.startsWith("$$") && text.endsWith("$$")) RichContentPreview(text.drop(2).dropLast(2), "math")
                else if (inlineMath) RichContentPreview(block.text, "inline-math")
                else Text(inline(block.text, code, openFile), style = Type.body.copy(textDirection = TextDirection.Content, textAlign = TextAlign.Start), color = ink)
            }
            is Block.Heading -> Text(inline(block.text, code, openFile), style = when (block.level) {
                1 -> Type.title3.bold; 2 -> Type.headline; else -> Type.subheadline.bold
            }, color = ink)
            is Block.Bullet -> Row(Modifier.padding(start = 16.dp * block.depth), horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                Text(block.marker, style = Type.body.monospacedDigit, color = Palette.secondary)
                Text(inline(block.text, code, openFile), style = Type.body.copy(textDirection = TextDirection.Content, textAlign = TextAlign.Start), color = ink)
            }
            Block.Rule -> Box(Modifier.padding(vertical = 4.dp).fillMaxWidth().height(1.dp).background(Palette.border))
            is Block.Table -> MarkdownTable(block, code, ink, openFile)
            is Block.Quote -> Row(Modifier.height(IntrinsicSize.Min)) {
                Box(Modifier.width(3.dp).fillMaxHeight().background(Palette.border))
                Text(inline(block.text, code, openFile), Modifier.padding(start = 10.dp), style = Type.body.copy(textDirection = TextDirection.Content, textAlign = TextAlign.Start), color = Palette.secondary)
            }
            is Block.Code -> RichContentPreview(block.text, block.language.lowercase(), allowMarkdownPreview)
        }
    }
}

/** A table: a bold header and hairlines between rows, scrolling sideways when wide. */
@Composable private fun MarkdownTable(table: Block.Table, code: Color, ink: Color, openFile: ((String) -> Unit)?) {
    val columns = maxOf(table.header.size, table.rows.maxOfOrNull { it.size } ?: 0)
    val rows = listOf(table.header) + table.rows
    Box(Modifier.horizontalScroll(rememberScrollState()).padding(vertical = 2.dp)) {
        Row(horizontalArrangement = Arrangement.spacedBy(16.dp)) {
            for (column in 0 until columns) Column(Modifier.width(IntrinsicSize.Max), verticalArrangement = Arrangement.spacedBy(7.dp)) {
                for ((index, row) in rows.withIndex()) {
                    if (index > 0) Box(Modifier.fillMaxWidth().height(1.dp).background(Palette.border))
                    Text(inline(row.getOrElse(column) { "" }, code, openFile), Modifier.heightIn(min = 22.dp),
                        style = if (index == 0) Type.body.semibold else Type.body, color = ink, softWrap = false)
                }
            }
        }
    }
}

private val inlineParser = Parser.builder().extensions(listOf(StrikethroughExtension.create())).build()

/** Inline Markdown: links underlined (the theme is monochrome), inline code on a soft tint. */
internal fun inline(source: String, codeTint: Color, openFile: ((String) -> Unit)? = null): AnnotatedString {
    // Inline only: indentation would otherwise read as a code block.
    val document = inlineParser.parse(source.lines().joinToString("\n") { it.trimStart() })
    return buildAnnotatedString {
        fun visit(node: Node) {
            val style = when (node) {
                is StrongEmphasis -> SpanStyle(fontWeight = FontWeight.SemiBold)
                is Emphasis -> SpanStyle(fontStyle = FontStyle.Italic)
                is Code -> SpanStyle(fontFamily = FontFamily.Monospace, background = codeTint)
                is Strikethrough -> SpanStyle(textDecoration = TextDecoration.LineThrough)
                else -> null
            }
            if (style != null) pushStyle(style)
            val link = (node as? Link)?.destination?.takeIf { it.startsWith("https://") || it.startsWith("http://") || it.startsWith("mailto:") }
            val path = if (openFile != null && node is Link) workspaceLink(node.destination) else null
            val styleLink = TextLinkStyles(SpanStyle(textDecoration = TextDecoration.Underline))
            if (link != null) pushLink(LinkAnnotation.Url(link, styleLink))
            else if (path != null && openFile != null) pushLink(LinkAnnotation.Clickable(path, styleLink) { openFile(path) })
            when (node) {
                is org.commonmark.node.Text -> append(node.literal)
                is Code -> append(node.literal)
                is SoftLineBreak, is HardLineBreak -> append("\n")
                is HtmlInline -> append(node.literal)
                is org.commonmark.node.HtmlBlock -> append(node.literal.trimEnd())
                is org.commonmark.node.FencedCodeBlock -> append(node.literal.trimEnd())
                is Image -> children(node).forEach(::visit)
                else -> children(node).forEach(::visit)
            }
            if (link != null || path != null) pop()
            if (style != null) pop()
        }
        // Paragraphs of a multi-line block keep their line breaks.
        children(document).forEachIndexed { index, paragraph -> if (index > 0) append("\n\n"); visit(paragraph) }
    }
}

private fun children(node: Node): List<Node> = buildList {
    var child = node.firstChild
    while (child != null) { add(child); child = child.next }
}

/** A monospaced box for commands and output (iOS `CodeBox`). */
@Composable fun CodeBox(text: String, style: TextStyle = Type.caption.monospaced) {
    Box(Modifier.fillMaxWidth().heightIn(max = 240.dp).clip(RoundedRectangle(8.dp)).background(Palette.codeBackground)
        .horizontalScroll(rememberScrollState())) {
        SelectionContainer { Text(text, Modifier.padding(8.dp), style = style, color = Palette.text, softWrap = false) }
    }
}

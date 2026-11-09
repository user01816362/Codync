package com.codync.android

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.codync.android.core.Attachment
import com.codync.android.core.Bot
import com.codync.android.core.LibraryItem
import com.codync.android.core.LibraryPeriod
import com.codync.android.core.LinkState
import com.codync.android.core.Artifact
import com.codync.android.design.*
import com.kyant.shapes.RoundedRectangle

/**
 * A chat's Library, on its bot's details (docs/features/artifacts.md): the artifacts its bot published and the files shared
 * in it, newest first by day. Each opens as it does in the chat.
 */
@Composable internal fun LibraryView(bot: Bot, state: AppState, store: AppStore, open: (OpenArtifact) -> Unit) {
    val online = state.actualConnection is LinkState.Ready
    AnalyticsOnAppear(AnalyticsEvent.ScreenViewed(AnalyticsScreen.Library))
    var attempt by remember(bot.id) { mutableStateOf(0) }
    var query by remember(bot.id) { mutableStateOf("") }
    var attachmentEntryId by remember(bot.id) { mutableStateOf<String?>(null) }
    var filesOnly by remember(bot.id) { mutableStateOf(false) }
    LaunchedEffect(bot.id, online, attempt) { if (online) store.loadLibrary(bot.id) }
    val items = state.artifacts.library(bot.id, state.library[bot.id].orEmpty())
    var renaming by remember { mutableStateOf<Artifact?>(null) }
    val fileItems = items.filter { it !is LibraryItem.ArtifactItem }
    val visible = (if (filesOnly) fileItems else items).filter { item ->
        val name = when (item) { is LibraryItem.ArtifactItem -> item.artifact.title; is LibraryItem.FileItem -> item.file.name; is LibraryItem.AttachmentItem -> item.file.name }
        name.contains(query.trim(), ignoreCase = true)
    }
    var textFile by remember(bot.id) { mutableStateOf<LibraryItem.FileItem?>(null) }
    var textAttachment by remember(bot.id) { mutableStateOf<Attachment?>(null) }
    var image by remember { mutableStateOf<Attachment?>(null) }
    Column(Modifier.fillMaxWidth()) {
        Row(Modifier.fillMaxWidth().padding(bottom = 12.dp), horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
            TabChoice(filesOnly, listOf(false to "All", true to "Files"), { filesOnly = it })
            Text(fileItems.size.toString(), style = Type.caption, color = Palette.secondary)
            IconButton("Retry", "arrow.clockwise", enabled = online && "library:${bot.id}" !in state.busy) { attempt += 1 }
        }
        SearchField(query, { query = it }, prompt = "Find files and artifacts")
        when {
            visible.isNotEmpty() -> Column(verticalArrangement = Arrangement.spacedBy(16.dp)) {
                for ((period, members) in LibraryPeriod.sections(visible)) Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
                    SectionHeading(period.title, Modifier.padding(start = 2.dp))
                    Column(Modifier.fillMaxWidth().cardGroup()) {
                        for ((index, item) in members.withIndex()) {
                            if (index > 0) Hairline(Modifier.padding(horizontal = 16.dp))
                            when (item) {
                                is LibraryItem.ArtifactItem -> ArtifactRow(item.artifact, { open(OpenArtifact(bot.id, item.artifact.id, item.artifact.title, item.artifact.entryId)) },
                                    { renaming = item.artifact }) { store.shareArtifact(bot.id, item.artifact.id, item.artifact.title) }
                                is LibraryItem.FileItem -> FileRow(item, bot.id, state, store) { textFile = item }
                                is LibraryItem.AttachmentItem -> AttachmentRow(item.file, bot.id, state, store, { image = item.file }) { attachmentEntryId = item.entryId; textAttachment = item.file }
                            }
                        }
                    }
                }
            }
            "library:${bot.id}" in state.busy -> Box(Modifier.fillMaxWidth().padding(top = 40.dp), contentAlignment = Alignment.Center) { Spinner(20.dp) }
            else -> if (filesOnly) Text(tr("No files shared in this chat"), color = Palette.secondary) else EmptyLibrary(bot.name)
        }
    }
    // The artifact as composed: Save closes the dialog (clearing `renaming`) before it renames.
    val target = renaming
    RenameArtifactDialog(target?.title, { renaming = null }) { title -> target?.let { store.renameArtifact(bot.id, it.id, title) } }
    AttachmentImageSheet(image, bot.id, state, store) { image = null }
    CodyncSheet(textFile, { textFile = null }) { SharedTextPreview(it.file, it.entryId, store) }
    CodyncSheet(textAttachment, { textAttachment = null }) { AttachmentTextPreview(it, bot.id, store, attachmentEntryId) }
}

/** One row of the Library: what it is on the left, its name and detail. */
@Composable private fun LibraryRow(modifier: Modifier, leading: @Composable () -> Unit, content: @Composable RowScope.() -> Unit) {
    Row(modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 12.dp), verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(12.dp)) {
        Box(Modifier.size(36.dp), contentAlignment = Alignment.Center) { leading() }
        content()
    }
}

/** An artifact: tapping opens it, holding opens its menu. */
@Composable private fun ArtifactRow(artifact: Artifact, open: () -> Unit, rename: () -> Unit, share: () -> Unit) {
    LibraryRow(Modifier.contextActions(onClick = open) {
        listOf(MenuItem("Open", icon = "arrow.up.right", action = open), MenuItem("Rename artifact", icon = "pencil", action = rename),
            MenuItem("Share artifact", icon = "square.and.arrow.up", action = share))
    }, { Symbol("chart.bar.doc.horizontal", size = 22.dp, tint = Palette.secondary) }) {
        Text(artifact.title, Modifier.weight(1f), style = Type.system(13f), color = Palette.text, maxLines = 2, overflow = TextOverflow.Ellipsis)
    }
}

/** A file the bot shared: tapping downloads it to the share sheet, as its card in the chat does (again: cancels). */
@Composable private fun FileRow(item: LibraryItem.FileItem, botId: String, state: AppState, store: AppStore, preview: () -> Unit) {
    val download = state.fileDownloads[item.file.id]
    val busy = download is FileDownloadState.Downloading
    LibraryRow(Modifier.plainButton { if (item.file.canOpenPreview) preview() else if (busy) store.cancelFile(item.file.id) else store.saveFile(item.entryId, botId, item.file) },
        { Symbol(AttachmentIcon.symbol(item.file.name), size = 22.dp, tint = Palette.secondary) }) {
        NameAndDetail(item.file.name, (state.bots.firstOrNull { it.id == botId }?.name ?: "") + " · " + detail(item.file, download))
        if (item.file.canOpenPreview) IconButton(if (busy) "Cancel download" else "Download", if (busy) "xmark" else "arrow.down.to.line") {
            if (busy) store.cancelFile(item.file.id) else store.saveFile(item.entryId, botId, item.file)
        }
    }
}

/** A file the user attached: an image opens full size, as in the chat; another file only shows. */
@Composable private fun AttachmentRow(file: Attachment, botId: String, state: AppState, store: AppStore, openImage: () -> Unit, openText: () -> Unit) {
    if (file.isImage) LibraryRow(Modifier.plainButton(action = openImage), {
        AttachmentImage(file, botId, 144, state, store, Modifier.size(36.dp).clip(RoundedRectangle(8.dp)), ContentScale.Crop)
    }) { NameAndDetail(file.name, tr("You") + " · " + formatBytes(file.size)) }
    else LibraryRow(if (file.canOpenPreview) Modifier.plainButton(action = openText) else Modifier.semantics(mergeDescendants = true) {}, { Symbol(AttachmentIcon.symbol(file.name), size = 22.dp, tint = Palette.secondary) }) {
        NameAndDetail(file.name, tr("You") + " · " + formatBytes(file.size))
    }
}

@Composable private fun RowScope.NameAndDetail(name: String, detail: String) {
    Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
        Text(name, style = Type.system(13f), color = Palette.text, maxLines = 1, overflow = TextOverflow.MiddleEllipsis)
        Text(detail, style = Type.caption2, color = Palette.secondary)
    }
}

@Composable private fun EmptyLibrary(name: String) {
    Column(Modifier.fillMaxWidth().padding(horizontal = 32.dp, vertical = 48.dp), horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Symbol("books.vertical", size = 36.dp, tint = Palette.tertiary)
        Text(tr("Library is empty"), style = Type.headline, color = Palette.text, textAlign = TextAlign.Center)
        Text(tr("Artifacts {name} makes and files shared in this chat show here.").replace("{name}", name), style = Type.subheadline,
            color = Palette.secondary, textAlign = TextAlign.Center)
    }
}

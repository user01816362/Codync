package com.codync.android

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import com.codync.android.core.AppVersion
import com.codync.android.core.Entry
import com.codync.android.design.MenuItem
import com.codync.android.design.ReactionPick

/** Message actions shared by text, attachment, image, and audio menus. */
internal class MessageMenu(
    val reactions: ReactionPick?,
    val reply: (() -> Unit)?,
    val trace: (() -> Unit)?,
    val selectText: (() -> Unit)? = null,
    val edit: (() -> Unit)? = null,
) {
    /** `copy` first, then the rest; `leave` runs before an item that opens another view. */
    fun items(copy: List<MenuItem>, leave: () -> Unit = {}): List<MenuItem> = copy + listOfNotNull(
        selectText?.let { MenuItem("Select text / Quote", icon = "arrowshape.turn.up.left") { leave(); it() } },
        edit?.let { MenuItem("Edit", icon = "pencil") { leave(); it() } },
        reply?.let { MenuItem("Reply", icon = "arrowshape.turn.up.left") { leave(); it() } },
        trace?.let { MenuItem("Show what it did", icon = "list.bullet") { leave(); it() } },
    )

    /** A message's text: copy, select a passage when supported, then its other actions. */
    fun textItems(copy: MenuItem, more: List<MenuItem> = emptyList()): List<MenuItem> = items(listOf(copy) + more)

    companion object {
        fun of(entry: Entry, store: AppStore, reply: (() -> Unit)?, trace: (() -> Unit)?,
            selectText: (() -> Unit)? = null, edit: (() -> Unit)? = null) = MessageMenu(
            reactionPick(entry, store), reply, trace,
            selectText.takeIf { offersSelectText(entry, replyable = reply != null) }, edit,
        )

        /** A message of a thread's own transcript: its copy items alone, no reactions, reply or steps. */
        val copyOnly = MessageMenu(reactions = null, reply = null, trace = null)

        /** Select Text goes where Reply does, on a message with text of its own (an artifact card's is only its title). */
        fun offersSelectText(entry: Entry, replyable: Boolean) = replyable && entry.text.isNotBlank() && entry.artifact == null
    }
}

/** The first host version that takes a reply to a part of a message (`send`'s `replyPart`). */
private const val REPLY_PART_HOST = "2.48.1"

/** Older hosts drop the selected part and quote the whole message; before hello, support is unknown. */
internal val AppState.supportsReplyPart: Boolean
    get() = hello?.text("version")?.let { !AppVersion.isBelow(it, REPLY_PART_HOST) } == true

/** Copy a message's text. */
internal fun textCopyItem(context: Context, text: String) = MenuItem("Copy", icon = "square.on.square") {
    context.getSystemService(ClipboardManager::class.java).setPrimaryClip(ClipData.newPlainText("Message", text))
}

/** Replace Copy with image actions, with a name fallback when copying image data is unavailable. */
internal fun imageCopyItems(copyImage: () -> Unit, copyName: () -> Unit) = listOf(
    MenuItem("Copy image", icon = "photo.on.rectangle", action = copyImage),
    MenuItem("Copy name", icon = "square.on.square", action = copyName),
)

internal fun imageCopyItems(store: AppStore, source: ImageSource) =
    imageCopyItems({ store.copyImageLater(source) }) { store.copyImageName(source, ImageFrom.Menu) }

internal val quickReactions = listOf("👍", "❤️", "😂", "🎉", "👀", "✅")

internal fun reactionPick(entry: Entry, store: AppStore) = ReactionPick(quickReactions, entry.reactions) { emoji ->
    store.command("react", body("entryId" to entry.id, "emoji" to emoji), "react:" + entry.id)
}

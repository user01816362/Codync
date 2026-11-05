package com.codync.android

import com.codync.android.core.Entry

/** What the reader chose to see: a version, and how many versions the message had when they chose it. */
internal data class VersionPick(val version: String, val of: Int)

/** One message the user edited: every version, oldest first, and the one that keeps its place in the chat. */
internal class MessageVersions(val anchor: Entry, val versions: List<Entry>)

/**
 * The edited messages among [entries], by the id of their first version. Each edit is its own entry (so its turn
 * and answers stay together), and points at the first version with `editOf`.
 */
internal fun messageVersions(entries: List<Entry>): Map<String, MessageVersions> {
    val held = entries.associateBy { it.id }
    val edits = mutableMapOf<String, MutableList<Entry>>()
    for (entry in entries) {
        val root = entry.editOf?.takeIf { entry.kind == "user" } ?: continue
        edits.getOrPut(root) { mutableListOf() } += entry
    }
    return edits.mapNotNull { (root, group) ->
        val versions = (listOfNotNull(held[root]) + group).sortedWith(compareBy({ it.version ?: 1 }, { it.seq }))
        versions.firstOrNull()?.let { it.id to MessageVersions(it, versions) }
    }.toMap()
}

/** The version a chat shows: the one the reader picked, or the newest. */
internal fun MessageVersions.selected(pick: VersionPick?): Entry {
    val newest = versions.last()
    if (pick == null || pick.of != versions.size) return newest
    return versions.firstOrNull { it.id == pick.version } ?: newest
}

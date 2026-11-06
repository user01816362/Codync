package com.codync.android

import com.codync.android.core.Entry
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put
import org.junit.Assert.assertEquals
import org.junit.Test

class MessageVersionsTest {
    private fun entry(id: String, kind: String, turn: Long, at: Long, data: JsonObject) = Entry(
        id = id, seq = at, botId = "b1", rev = 1, kind = kind, turn = turn, data = data, createdAt = at, updatedAt = at)

    private fun user(id: String, turn: Long, at: Long, data: JsonObject) = entry(id, "user", turn, at, data)
    private fun reply(id: String, turn: Long, at: Long, text: String) = entry(id, "agent", turn, at, buildJsonObject {
        put("text", text); put("final", true)
    })
    private fun edited(id: String, turn: Long, at: Long, text: String, editOf: String, version: Int) = user(id, turn, at, buildJsonObject {
        put("text", text); put("editedAt", at); put("editOf", editOf); put("version", version)
    })

    /** u1 (turn 1) answered by a1, u2 (turn 2), then u1 edited as e1 (turn 3) answered by a2. The host marks u1 and a1 superseded. */
    private fun chat() = listOf(
        user("u1", 1, 1_000, buildJsonObject { put("text", "first"); put("status", "sent"); put("superseded", true) }),
        entry("a1", "agent", 1, 1_100, buildJsonObject { put("text", "answer to first"); put("final", true); put("superseded", true) }),
        user("u2", 2, 1_500, buildJsonObject { put("text", "second"); put("status", "sent") }),
        edited("e1", 3, 3_000, "edited first", "u1", 2),
        reply("a2", 3, 3_100, "answer to the edit"),
    )

    private fun ids(items: List<ChatItem>) = items.mapNotNull {
        when (it) {
            is ChatItem.Message -> "${it.entry.id}:${it.entry.text}"
            is ChatItem.Pending -> "pending"
            is ChatItem.Separator -> null
            is ChatItem.Exchanges -> "exchanges"
        }
    }

    @Test
    fun `an edited message shows its newest version and answer where it was first sent`() {
        val items = chatItems(chat(), emptyList())
        assertEquals(listOf("e1:edited first", "a2:answer to the edit", "u2:second"), ids(items))
        val shown = items.filterIsInstance<ChatItem.Message>().first()
        assertEquals(listOf("u1", "e1"), shown.versions?.map { it.id })
        assertEquals(1, shown.versionIndex)
    }

    @Test
    fun `picking an older version brings back its own answer`() {
        val pick = mapOf("u1" to VersionPick("u1", 2))
        assertEquals(listOf("u1:first", "a1:answer to first", "u2:second"), ids(chatItems(chat(), emptyList(), picked = pick)))
    }

    @Test
    fun `a newer edit wins over a pick made before it`() {
        val pick = mapOf("u1" to VersionPick("u1", 1))
        assertEquals(listOf("e1:edited first", "a2:answer to the edit", "u2:second"), ids(chatItems(chat(), emptyList(), picked = pick)))
    }

    @Test
    fun `versions group by their first entry, oldest first, even when it is not loaded`() {
        assertEquals(listOf("u1", "e1"), messageVersions(chat())["u1"]?.versions?.map { it.id })

        val onlyEdit = chat().filter { it.id != "u1" }
        assertEquals(listOf("e1"), messageVersions(onlyEdit)["e1"]?.versions?.map { it.id })
        assertEquals(listOf("u2:second", "e1:edited first", "a2:answer to the edit"), ids(chatItems(onlyEdit, emptyList())))
    }

    @Test
    fun `a message with one version alone is the newest`() {
        val group = messageVersions(chat()).getValue("u1")
        assertEquals("e1", group.selected(null).id)
    }
}

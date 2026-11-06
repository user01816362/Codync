package com.codync.android.core

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class TextPreviewTest {
    @Test fun `only bounded text files can be previewed`() {
        fun file(name: String, size: Long = 5) = SharedFile("id", name, size, "a".repeat(64))
        assertTrue(file("README.MD").showsAsText)
        assertTrue(file("app.ts").showsAsText)
        assertFalse(file("md").showsAsText)
        assertFalse(file("photo.png").showsAsText)
        assertFalse(file("big.txt", 1024 * 1024 + 1).showsAsText)
        assertTrue(Attachment("id", "notes.md", 12).showsAsText)
        assertFalse(Attachment("id", "notes.md", 1024 * 1024 + 1).showsAsText)
    }
}

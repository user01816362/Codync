package com.codync.android

import android.content.Context
import android.content.SharedPreferences
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow

/** Device preference: the working pill unfolds the turn's thinking and steps. Off (the default) keeps the chat to its
 *  messages (iOS `ThinkingDetails`). */
internal object ThinkingDetails {
    private var stored: SharedPreferences? = null
    private val current = MutableStateFlow(false)
    val enabled = current.asStateFlow()
    private val collaboration = MutableStateFlow("compact")
    val collaborationMode = collaboration.asStateFlow()
    fun setCollaboration(mode: String) {
        if (mode !in listOf("hidden", "compact", "expanded")) return
        stored?.edit()?.putString("collaborationActivityMode", mode)?.apply()
        collaboration.value = mode
    }

    fun initialize(context: Context) {
        val preferences = context.getSharedPreferences("codync-chat", Context.MODE_PRIVATE)
        stored = preferences
        collaboration.value = preferences.getString("collaborationActivityMode", "compact")
            ?.takeIf { it in listOf("hidden", "compact", "expanded") } ?: "compact"
        current.value = preferences.getBoolean("showThinkingDetails", false)
    }

    fun set(on: Boolean) {
        stored?.edit()?.putBoolean("showThinkingDetails", on)?.apply()
        current.value = on
    }
}

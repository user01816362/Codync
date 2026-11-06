package com.codync.android

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import com.codync.android.core.Bot
import com.codync.android.core.BotExchange
import com.codync.android.core.Entry
import com.codync.android.core.groupCollaborations
import com.codync.android.design.*
import com.kyant.shapes.Capsule
import kotlinx.coroutines.CancellationException
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put
import java.util.Date

@Composable internal fun SidechatIndicator(bot: Bot, state: AppState, open: () -> Unit) {
    val exchanges = BotExchange.all(state.collaborationUpdates[bot.id].orEmpty(), state.entries.filter { it.botId == bot.id })
    val active = exchanges.filter { it.outcome == BotExchange.Outcome.Pending }.map { it.peerId }.toSet().size
    val title = if (active > 0) tr("{count} bots collaborating").replace("{count}", "${active + 1}") else tr("Collaboration activity")
    Box(Modifier.fillMaxWidth().padding(bottom = 6.dp), contentAlignment = Alignment.Center) {
        Row(Modifier.background(Palette.surface, Capsule()).plainButton(action = open).padding(horizontal = 12.dp, vertical = 7.dp),
            verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(7.dp)) {
            Symbol("person.2", size = 13.dp, tint = Palette.secondary)
            Text(title, style = Type.footnote.medium, color = Palette.secondary)
            Symbol("chevron.up", size = 10.dp, tint = Palette.tertiary)
        }
    }
}

/** Saved exchanges and live revisions, without adding historical entries to the chat mirror. */
@Composable internal fun SidechatView(bot: Bot, state: AppState, store: AppStore) {
    var fetched by remember(bot.id) { mutableStateOf<List<Entry>>(emptyList()) }
    var error by remember(bot.id) { mutableStateOf<String?>(null) }
    val clearGeneration = state.chatClearGenerations[bot.id] ?: 0
    val openedGeneration = remember(bot.id) { clearGeneration }
    LaunchedEffect(bot.id, clearGeneration) {
        fetched = emptyList()
        error = null
        if (clearGeneration != openedGeneration) return@LaunchedEffect
        try { fetched = entriesOf(store.hostCall("botExchanges", buildJsonObject { put("botId", bot.id) }, 20_000)) }
        catch (cancelled: CancellationException) { throw cancelled }
        catch (_: Exception) { error = "Couldn't load earlier collaboration activity." }
    }
    val exchanges = BotExchange.all(fetched, state.entries.filter { it.botId == bot.id } + state.collaborationUpdates[bot.id].orEmpty())
    val groups = groupCollaborations(exchanges)
    val list = rememberLazyListState()
    LaunchedEffect(exchanges.size, exchanges.lastOrNull()?.rev) {
        val layout = list.layoutInfo
        val last = layout.visibleItemsInfo.lastOrNull()?.index ?: -1
        if (exchanges.isNotEmpty() && (layout.totalItemsCount == 0 || last >= layout.totalItemsCount - 2)) {
            list.scrollToItem(groups.lastIndex + if (error != null) 1 else 0)
        }
    }
    Column(Modifier.fillMaxSize().background(Palette.background)) {
        ModalHeader("Collaboration")
        LazyColumn(Modifier.fillMaxSize(), state = list, contentPadding = PaddingValues(horizontal = 20.dp, vertical = 16.dp),
            verticalArrangement = Arrangement.spacedBy(20.dp)) {
            if (error != null) item("error") { Text(tr(error.orEmpty()), style = Type.footnote, color = Palette.danger) }
            items(groups, key = { it.first().id }) { group ->
                val first = group.first()
                val name = state.authorName(first.source, first.peerName.takeIf { first.source == first.peerId })
                Column(verticalArrangement = Arrangement.spacedBy(16.dp)) {
                    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                        Text(tr("Started by {name}").replace("{name}", name), style = Type.footnote.medium, color = Palette.tertiary)
                        Text(android.text.format.DateFormat.getDateFormat(LocalContext.current).format(Date(first.createdAt)), style = Type.footnote, color = Palette.tertiary)
                    }
                    group.forEach { SidechatExchange(it, state) }
                }
            }
        }
    }
}

@Composable private fun SidechatExchange(exchange: BotExchange, state: AppState) {
    val name = { id: String -> state.authorName(id, exchange.peerName.takeIf { id == exchange.peerId }) }
    val time = android.text.format.DateFormat.getTimeFormat(LocalContext.current).format(Date(exchange.createdAt))
    Column(Modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(7.dp)) {
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(6.dp)) {
            state.bots.firstOrNull { it.id == exchange.source }?.let { BotAvatar(it, state.bots, 20.dp) }
            Text(name(exchange.source), style = Type.footnote.semibold, color = Palette.text)
            Symbol("arrow.right", size = 11.dp, tint = Palette.tertiary)
            state.bots.firstOrNull { it.id == exchange.target }?.let { BotAvatar(it, state.bots, 20.dp) }
            Text(name(exchange.target), style = Type.footnote.semibold, color = Palette.text)
            Spacer(Modifier.weight(1f))
            Text(time, style = Type.footnote, color = Palette.tertiary)
        }
        SelectionContainer { Text(exchange.text, style = Type.body, color = Palette.text) }
        if (!exchange.reply.isNullOrEmpty()) {
            Text(tr("Reply from {name}").replace("{name}", name(exchange.target)), style = Type.footnote.semibold, color = Palette.secondary)
            SelectionContainer { Text(exchange.reply, style = Type.body, color = Palette.secondary) }
        }
        val status = when (val outcome = exchange.outcome) {
            BotExchange.Outcome.Pending -> tr("Waiting for a reply…")
            BotExchange.Outcome.Done -> tr("Completed")
            is BotExchange.Outcome.Failed -> outcome.detail.ifEmpty { tr("Didn't complete.") }
        }
        Text(status, style = Type.footnote, color = if (exchange.outcome is BotExchange.Outcome.Failed) Palette.danger else Palette.tertiary)
    }
}

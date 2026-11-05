import { useRef } from 'react'
import type { Entry } from '@shared/models'
import { groupExchanges, type BotExchange, type ChatSlot } from './bot-exchange.ts'
import { selectedVersion, versionGroups, type VersionPick } from './message-versions.ts'

export type ChatItem =
  | { id: string; kind: 'separator'; date: number }
  /** `versions`: a message the user edited, and where `entry` (the version shown) sits among them. */
  | { id: string; kind: 'entry'; entry: Entry; groupStart: boolean; versions?: Entry[]; versionIndex?: number }
  | { id: string; kind: 'exchanges'; entry: Entry; exchanges: BotExchange[] }

/**
 * Chat-visible entries plus time separators (gaps > 1 h) and author grouping. A bot's messages
 * arrive whole (Grok Bot's `send_message`); what it writes along the way is trace. Runs of bot
 * exchanges with one peer fold into one item.
 *
 * An edited message is one slot: it shows the version the reader picked (`picked`, by its first entry's id;
 * the newest by default) and that version's answers, where the message first was.
 */
export function buildChat(entries: Entry[], picked: ReadonlyMap<string, VersionPick> = new Map()): ChatItem[] {
  const out: ChatItem[] = []
  let lastDate: number | null = null
  let lastAuthor: string | null = null

  const groups = versionGroups(entries)
  const editedTurns = new Set<number>()
  for (const group of groups.values()) {
    for (const version of group.versions) editedTurns.add(version.turn)
  }
  // Each edited version's answer shares its turn, and is shown with the version instead of at the end of the chat.
  const answers = new Map<number, Entry[]>()
  for (const e of entries) {
    if (e.kind === 'user' || !editedTurns.has(e.turn)) continue
    const list = answers.get(e.turn)
    if (list) list.push(e)
    else answers.set(e.turn, [e])
  }
  const visible = entries.filter((e) => {
    // Where the message is shown, even when the first version is older than what the chat loaded.
    if (groups.has(e.id)) return true
    if (e.kind === 'user' && e.data.editOf) return false
    // Left behind by an edit: its version and answer stand behind the arrows.
    if (e.data.superseded) return false
    return !editedTurns.has(e.turn)
  })

  const emit = (e: Entry, date: number, versions?: Entry[], versionIndex?: number) => {
    const id = e.kind === 'user' && e.data.clientNonce ? `user-${e.data.clientNonce}` : e.id
    if (lastDate === null || date - lastDate > 3_600_000) {
      out.push({ id: `sep-${id}`, kind: 'separator', date })
      lastAuthor = null
    }
    // In a group each bot is its own author.
    const author = e.kind === 'user' ? 'user' : e.kind === 'agent' ? `agent:${e.data.author ?? ''}` : e.kind
    out.push({ id, kind: 'entry', entry: e, groupStart: author !== lastAuthor, versions, versionIndex })
    lastAuthor = author === 'user' || e.kind === 'agent' ? author : null
    lastDate = date
  }

  const emitSlot = (slot: ChatSlot, date: number, versions?: Entry[], versionIndex?: number) => {
    const { entry: e, exchanges } = slot
    if (!exchanges) {
      emit(e, date, versions, versionIndex)
      return
    }
    const id = e.kind === 'user' && e.data.clientNonce ? `user-${e.data.clientNonce}` : e.id
    if (lastDate === null || date - lastDate > 3_600_000) {
      out.push({ id: `sep-${id}`, kind: 'separator', date })
      lastAuthor = null
    }
    out.push({ id, kind: 'exchanges', entry: e, exchanges })
    lastAuthor = null
    lastDate = date
  }

  for (const slot of groupExchanges(visible)) {
    const { entry: e, exchanges } = slot
    const group = exchanges ? undefined : groups.get(e.id)
    if (!group) {
      emitSlot(slot, e.createdAt)
      continue
    }
    // The edited message and the answer of the version shown, in the place the message was first sent.
    const chosen = selectedVersion(group, picked.get(group.anchor.id))
    const index = group.versions.indexOf(chosen)
    const shown: ChatSlot[] = groupExchanges([chosen, ...(answers.get(chosen.turn) ?? [])])
    shown.forEach((part, i) => {
      const first = i === 0
      emitSlot(part, first ? group.anchor.createdAt : part.entry.createdAt,
        first ? group.versions : undefined, first ? index : undefined)
    })
  }
  return out
}

/**
 * Rows that arrived after the list first showed, while it follows the newest: they pop in
 * (`.row-in`, Grok Bot's). Earlier pages and rows that came in while reading history don't.
 */
export function useArrivals(ids: string[], following: boolean): Set<string> {
  const seen = useRef<Set<string> | null>(null)
  const fresh = useRef(new Set<string>())
  if (seen.current === null) seen.current = new Set(ids)
  for (const id of ids) {
    if (seen.current.has(id)) continue
    seen.current.add(id)
    if (following) fresh.current.add(id)
  }
  return fresh.current
}

/** A new row pops in from its bubble's side. */
export function rowIn(item: ChatItem): string {
  return item.kind === 'entry' && item.entry.kind === 'user' ? 'row-in mine' : 'row-in'
}

const time = new Intl.DateTimeFormat(undefined, { hour: 'numeric', minute: '2-digit' })
const monthDay = new Intl.DateTimeFormat(undefined, { month: 'short', day: 'numeric' })
const weekday = new Intl.DateTimeFormat(undefined, { weekday: 'long' })

const sameDay = (a: Date, b: Date) => a.getFullYear() === b.getFullYear() && a.getMonth() === b.getMonth() && a.getDate() === b.getDate()

export const relativeTime = {
  time: (ms: number) => time.format(new Date(ms)),
  /** Chat separator: Today 3:27 AM · Yesterday 5:20 PM · Sep 16 9:02 AM */
  separator(ms: number, tr: (text: string) => string = (text) => text) {
    const date = new Date(ms)
    const now = new Date()
    const yesterday = new Date(now.getTime() - 86_400_000)
    const day = sameDay(date, now) ? tr('Today') : sameDay(date, yesterday) ? tr('Yesterday') : monthDay.format(date)
    return `${day} ${time.format(date)}`
  },
  /** Roster stamp: 3:45 AM · Yesterday · Wednesday · Sep 16 */
  day(ms: number, tr: (text: string) => string = (text) => text) {
    const date = new Date(ms)
    const now = new Date()
    if (sameDay(date, now)) return time.format(date)
    if (sameDay(date, new Date(now.getTime() - 86_400_000))) return tr('Yesterday')
    if (now.getTime() - ms < 6 * 86_400_000) return weekday.format(date)
    return monthDay.format(date)
  },
  /** Grok-style compact age: now · 5m · 3h · 2d · Sep 3 */
  short(ms: number) {
    const s = Math.max(0, (Date.now() - ms) / 1000)
    if (s < 60) return 'now'
    if (s < 3600) return `${Math.floor(s / 60)}m`
    if (s < 86_400) return `${Math.floor(s / 3600)}h`
    if (s < 86_400 * 7) return `${Math.floor(s / 86_400)}d`
    return monthDay.format(new Date(ms))
  },
}

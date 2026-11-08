import { useEffect } from 'react'
import type { Entry, ReplyQuote } from '@shared/models'

/** Reuse the reply composer without modifying either the source message or the text draft. */
export function quoteSelection(entry: Pick<Entry, 'id' | 'botId' | 'kind' | 'data'>, selected: string, author?: string, part?: 'request' | 'reply' | `file:${string}`) {
  const text = Array.from(selected.trim()).slice(0, 4000).join('')
  if (!text) return
  const quote: ReplyQuote = { id: entry.id, author: author ?? (entry.kind === 'user' ? 'user' : entry.data.author ?? entry.botId), text, selected: true, ...(part ? { part } : {}) }
  window.dispatchEvent(new CustomEvent<Entry>('codync:quote-selection', { detail: { seq: 0, rev: 0, turn: 0, createdAt: 0, updatedAt: 0, ...entry, data: { ...entry.data, selectedQuote: quote } } }))
}

export function useSelectionQuote(botId: string, receive: (entry: Entry) => void, disabled: boolean) {
  useEffect(() => {
    const quote = (event: Event) => {
      const entry = (event as CustomEvent<Entry>).detail
      if (entry.botId === botId && !disabled) receive(entry)
    }
    window.addEventListener('codync:quote-selection', quote)
    return () => window.removeEventListener('codync:quote-selection', quote)
  }, [botId, receive, disabled])
}

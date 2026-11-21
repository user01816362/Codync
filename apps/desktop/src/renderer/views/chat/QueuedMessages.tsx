import { useEffect, useRef, useState } from 'react'
import { hidingCalls } from '@shared/calls'
import { isWorking, quoteOf, type Entry } from '@shared/models'
import { IconButton, Button } from '../../components/Controls'
import { usePresence } from '../../components/Overlay'
import { HStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { afterExit } from '../../lib/exit-delay'
import { messagePreview } from '../../lib/message-preview'
import { useStore } from '../../store/context'
import { Sheet } from '../../components/Overlay'
import { QueuedMessageEditor } from './QueuedMessageEditor'
import { ComposerReveal } from './Composer'
import { canSteer, waitingMessages } from './queued-messages'
import './queued-messages.css'

const EXIT_MS = 300

/**
 * The user's messages waiting while the bot works, oldest first, in a tray tucked under the composer's top edge: each
 * can go out now (`steer`) or be taken back (`unqueue`). A host older than steering only lists them.
 */
export function QueuedMessages({ botId }: { botId: string }) {
  const store = useStore()
  const bot = store.bots.get(botId)
  const waiting = waitingMessages(hidingCalls(store.allEntries(botId)), !!bot && isWorking(bot))
  const [queueState, setQueueState] = useState<{ paused: boolean; reason: string } | null>(null)
  const [queueError, setQueueError] = useState<string | null>(null)
  const [controlling, setControlling] = useState(false)
  const control = async (action: string, entryId?: string) => {
    if (!store.client || controlling) return
    setControlling(true)
    try { setQueueState(await store.client.call('queueControl', { botId, action, entryId })); setQueueError(null) }
    catch (error: unknown) { setQueueError(error instanceof Error ? error.message : String(error)) }
    finally { setControlling(false) }
  }
  const signature = waiting.map(entry => `${entry.id}:${entry.data.queuePaused}:${entry.data.queueOrder}`).join(',')
  useEffect(() => {
    if (!waiting.length || bot?.kind === 'group') return
    void control('state')
  }, [botId, signature, store.client])
  const rows = useLeaving(waiting)
  const list = useRevealSent(waiting.at(-1), store.lastSentNonce)
  const tray = usePresence(waiting.length > 0, EXIT_MS)
  const actions = canSteer(store.hostVersion?.version)
  if (!tray.mounted) return null
  return (
    <div className={`queued-tray ${tray.shown ? 'shown' : ''}`} inert={waiting.length === 0}>
      <div>
        {bot?.kind !== 'group' && <HStack gap={8}>
          <Text size="caption" tone="secondary" grow>{queueState?.reason ?? 'Queued messages'}</Text>
          <Button disabled={controlling} onClick={() => void control(queueState?.paused ? 'resume' : 'pause')}>{queueState?.paused ? 'Resume queue' : 'Pause queue'}</Button>
        </HStack>}
        {queueError && <Text role="alert" size="caption" tone="danger">{queueError}</Text>}
        <div ref={list} className="queued-list" role="list" aria-label="Queued messages">
          {rows.map(({ entry, leaving }) => (
            <QueuedRow key={entry.id} entry={entry} leaving={leaving} actions={actions} reorder={bot?.kind === 'group' ? undefined : (action) => void control(action, entry.id)} disabled={controlling} />
          ))}
        </div>
      </div>
    </div>
  )
}

function QueuedRow({ entry, leaving, actions, reorder, disabled }: { entry: Entry; leaving: boolean; actions: boolean; reorder?: (action: string) => void; disabled: boolean }) {
  const store = useStore()
  const answering = store.answering.has(entry.id)
  const [editing, setEditing] = useState(false)
  return (
    <div className="queued-row" role="listitem">
      <ComposerReveal open={!leaving}>
        <HStack gap={2} className="queued-row-body">
          <Text size="callout" tone="secondary" truncate grow>{messagePreview(quoteOf(entry).text)}</Text>
          {actions ? (
            <>
              {reorder && <><IconButton title="Move earlier" icon="arrow.up" size={26} disabled={answering || disabled} onClick={() => reorder('earlier')} /><IconButton title="Move later" icon="arrow.down" size={26} disabled={answering || disabled} onClick={() => reorder('later')} /></>}
              <IconButton title="Edit queued message" icon="pencil" size={26} disabled={answering || store.bots.get(entry.botId)?.kind === 'group'} onClick={() => setEditing(true)} />
              <IconButton title="Send now" icon="arrow.turn.down.right" size={26} disabled={answering} onClick={() => store.steer(entry)} />
              <IconButton title="Remove" icon="trash" size={26} disabled={answering} onClick={() => store.unqueue(entry)} />
            </>
          ) : null}
        </HStack>
      </ComposerReveal>
      <Sheet open={editing} onClose={() => setEditing(false)} width={480}>
        <QueuedMessageEditor entry={entry} />
      </Sheet>
    </div>
  )
}

/** The list stays where it is, the oldest (next to go) in view; a message sent from here shows that it arrived. */
function useRevealSent(newest: Entry | undefined, lastSentNonce: string | null) {
  const list = useRef<HTMLDivElement>(null)
  useEffect(() => {
    if (!newest?.data.clientNonce || newest.data.clientNonce !== lastSentNonce) return
    // Once its row has opened, so the scroll takes it in.
    return afterExit(EXIT_MS, () => list.current?.scrollTo({ top: list.current.scrollHeight, behavior: 'smooth' }))
  }, [newest?.id])
  return list
}

/** `items` plus the ones that just left, kept in their places until their row has folded away. */
function useLeaving(items: Entry[]): { entry: Entry; leaving: boolean }[] {
  const [previous, setPrevious] = useState(items)
  const [leaving, setLeaving] = useState<{ entry: Entry; index: number }[]>([])
  const keys = items.map((e) => e.id)
  // Worked out while rendering, so a row that leaves never unmounts before it folds away.
  if (keys.join('\n') !== previous.map((e) => e.id).join('\n')) {
    const gone = previous.flatMap((entry, index) => (keys.includes(entry.id) ? [] : [{ entry, index }]))
    setPrevious(items)
    setLeaving((list) => [...list.filter((x) => !keys.includes(x.entry.id)), ...gone])
  }
  useEffect(() => (leaving.length ? afterExit(EXIT_MS, () => setLeaving([])) : undefined), [leaving])
  const rows = items.map((entry) => ({ entry, leaving: false }))
  for (const { entry, index } of leaving) {
    if (!keys.includes(entry.id)) rows.splice(Math.min(index, rows.length), 0, { entry, leaving: true })
  }
  return rows
}

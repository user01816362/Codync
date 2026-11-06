import { useEffect, useRef, useState } from 'react'
import type { Entry } from '@shared/models'
import { BotAvatar } from '../../components/Avatar'
import { Icon } from '../../components/Icon'
import { ModalHeader } from '../../components/Overlay'
import { Text } from '../../components/Text'
import { useTr } from '../../lib/use-language'
import { useStore } from '../../store/context'
import { allBotExchanges, groupCollaborations, peerOf, type BotExchange } from './bot-exchange'
import './sidechat.css'

/** Opens a read-only view of the real exchanges in this chat. */
export function SidechatIndicator({ botId, open }: { botId: string; open: () => void }) {
  const store = useStore()
  const tr = useTr()
  const exchanges = allBotExchanges(store.collaborationUpdates.get(botId) ?? [], store.allEntries(botId))
  const active = new Set(exchanges.filter((x) => x.outcome.kind === 'pending').map(peerOf)).size
  const title = active ? tr('{count} bots collaborating').replace('{count}', String(active + 1)) : tr('Collaboration activity')
  return <button className="press sidechat-indicator" onClick={open} aria-label={title}>
    <Icon name="person.2" size={13} /> <span>{title}</span> <Icon name="chevron.up" size={10} />
  </button>
}

export function SidechatView({ botId, close }: { botId: string; close: () => void }) {
  const store = useStore()
  const tr = useTr()
  const [fetched, setFetched] = useState<Entry[]>([])
  const [error, setError] = useState(false)
  const scroller = useRef<HTMLDivElement>(null)
  const following = useRef(true)
  const clearGeneration = store.chatClearGenerations.get(botId) ?? 0
  useEffect(() => {
    let current = true
    setFetched([])
    setError(false)
    const generation = store.chatClearGenerations.get(botId) ?? 0
    void store.botExchanges(botId).then(
      (entries) => { if (current && generation === (store.chatClearGenerations.get(botId) ?? 0)) setFetched(entries) },
      () => { if (current && generation === (store.chatClearGenerations.get(botId) ?? 0)) setError(true) },
    )
    return () => { current = false }
  }, [store, botId])
  useEffect(() => { setFetched([]); setError(false) }, [clearGeneration])
  const exchanges = allBotExchanges(fetched, [...store.allEntries(botId), ...(store.collaborationUpdates.get(botId) ?? [])])
  useEffect(() => {
    const el = scroller.current
    if (el && following.current) el.scrollTop = el.scrollHeight
  }, [exchanges.length, exchanges[exchanges.length - 1]?.rev])
  return <div className="sidechat">
    <ModalHeader title={tr('Collaboration')} onClose={close} />
    <div className="sidechat-scroll" ref={scroller} onScroll={(event) => {
      const el = event.currentTarget
      following.current = el.scrollHeight - el.scrollTop - el.clientHeight < 48
    }}>
      {error ? <Text as="div" size="footnote" tone="danger">{tr("Couldn't load earlier collaboration activity.")}</Text> : null}
      {groupCollaborations(exchanges).map((group) => {
        const first = group[0]!
        const name = store.authorName(first.message.sourceBotId, first.message.sourceBotId === peerOf(first) ? first.message.peerName : undefined)
        return <section key={first.id} className="sidechat-group">
          <div className="sidechat-group-heading"><span>{tr('Started by {name}').replace('{name}', name)}</span><time>{new Date(first.createdAt).toLocaleDateString()}</time></div>
          {group.map((exchange) => <ExchangeRow key={exchange.id} exchange={exchange} />)}
        </section>
      })}
    </div>
  </div>
}

function ExchangeRow({ exchange }: { exchange: BotExchange }) {
  const store = useStore()
  const tr = useTr()
  const { sourceBotId: source, targetBotId: target, peerName, text, reply } = exchange.message
  const name = (id: string) => store.authorName(id, id === peerOf(exchange) ? peerName : undefined)
  const bot = (id: string) => store.bots.get(id)
  return <div className="sidechat-exchange">
    <div className="sidechat-heading">
      {bot(source) ? <BotAvatar bot={bot(source)!} size={20} animated={false} /> : null}
      <strong>{name(source)}</strong><Icon name="arrow.right" size={11} color="var(--tertiary)" />
      {bot(target) ? <BotAvatar bot={bot(target)!} size={20} animated={false} /> : null}
      <strong>{name(target)}</strong>
      <time>{new Date(exchange.createdAt).toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' })}</time>
    </div>
    <div className="sidechat-message">{text}</div>
    {reply ? <><div className="sidechat-reply-label">{tr('Reply from {name}').replace('{name}', name(target))}</div><div className="sidechat-reply">{reply}</div></> : null}
    <div className={exchange.outcome.kind === 'failed' ? 'sidechat-status failed' : 'sidechat-status'}>
      {exchange.outcome.kind === 'pending' ? tr('Waiting for a reply…') : exchange.outcome.kind === 'done' ? tr('Completed') : exchange.outcome.detail || tr("Didn't complete.")}
    </div>
  </div>
}

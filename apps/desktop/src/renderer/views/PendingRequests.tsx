import { useState } from 'react'
import type { Entry } from '@shared/models'
import { Button, DropdownMenu } from '../components/Controls'
import { Icon } from '../components/Icon'
import { ModalHeader, Sheet, SheetPage } from '../components/Overlay'
import { Text } from '../components/Text'
import { useModels } from '../lib/observable'
import { useTr } from '../lib/use-language'
import type { BotStore } from '../store/bot-store'
import { useApp } from '../store/context'
import { PermissionCard } from './thread/PermissionCard'
import './pending-requests.css'

type Request = { key: string; store: BotStore; entry: Entry; author: string; chat: string }

/** One quiet entry point for other chats' requests; each decision retains its original entry and host. */
export function PendingRequests() {
  const app = useApp()
  useModels([...app.stores.values()])
  const tr = useTr()
  const [open, setOpen] = useState(false)
  const [selected, setSelected] = useState<string | null>(null)
  const requests: Request[] = [...app.stores.values()].flatMap(store => [...store.entries.values()].flatMap(entries => entries
    .filter(entry => entry.kind === 'permission' && entry.data.status === 'pending' && !entry.data.removed
      && !(app.selection?.computerId === store.computer.id && app.selection.botId === entry.botId))
    .map(entry => ({ key: `${store.computer.id}/${entry.id}`, store, entry,
      author: store.bots.get(entry.data.author ?? entry.botId)?.name ?? tr('Agent'),
      chat: store.bots.get(entry.botId)?.name ?? tr('Conversation') }))))
    .sort((a, b) => a.entry.createdAt - b.entry.createdAt || a.key.localeCompare(b.key))
  const current = requests.find(request => request.key === selected)
  const review = () => { setSelected(requests[0]?.key ?? null); setOpen(true) }
  return <>
    {requests.length > 0 ? <button className="requests-chip" onClick={review} aria-label={`${tr('Review requests')} · ${requests.length}`} aria-haspopup="dialog">
      <Icon name="hand.raised" size={16} /><span>{tr('Requests')} · {requests.length}</span>
    </button> : null}
    <Sheet open={open} onClose={() => setOpen(false)} width={540} height={640}>
      <SheetPage>
        <ModalHeader title="Requests" />
        <div className="requests-content">
          {current ? <>
            <DropdownMenu title="Choose request" items={() => requests.map(request => ({ title: `${request.author} · ${request.chat} · ${request.store.hostName}`, action: () => setSelected(request.key) }))}>
              <span className="requests-source">{current.author} · {current.chat}</span><Icon name="chevron.down" size={12} />
            </DropdownMenu>
            <Text size="caption" tone="secondary">{current.store.hostName}</Text>
            <div className="requests-card" key={current.key}>
              <PermissionCard entry={current.entry} hostName={current.store.hostName}
                answering={current.store.connection.kind === 'online' ? current.store.answering.get(current.entry.id) ?? null : 'offline'} respond={option => current.store.respond(current.entry, option)} />
            </div>
            {current.store.connection.kind !== 'online' ? <Text tone="attention">{tr('Reconnect to this computer before responding.')}</Text> : null}
            {current.store.lastError ? <Text tone="danger" role="status">{current.store.lastError}</Text> : null}
            <Button kind="secondary" onClick={() => { app.select({ computerId: current.store.computer.id, botId: current.entry.botId }); setOpen(false) }}>Open chat</Button>
          </> : <>
            <Text role="status">{tr(requests.length ? 'This request is no longer waiting.' : 'No requests waiting.')}</Text>
            {requests.length ? <Button onClick={review}>Review next request</Button> : null}
          </>}
          <Text size="caption" tone="secondary">{tr('Each approval applies to the selected request. Always allow uses the agent’s supplied permission scope.')}</Text>
        </div>
      </SheetPage>
    </Sheet>
  </>
}

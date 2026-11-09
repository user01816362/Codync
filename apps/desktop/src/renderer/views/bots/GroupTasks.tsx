import { useEffect, useState } from 'react'
import type { Bot } from '@shared/models'
import { Button, ChoicePicker, IconButton, Spinner } from '../../components/Controls'
import { ModalHeader } from '../../components/Overlay'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useStore } from '../../store/context'
import { AutoTextArea, Field } from './parts'

interface TaskEntry { id: string; rev: number; data: { groupTask: { title: string; text: string; owner: string; status: string; history: { at: number; status: string; owner: string; detail: string }[] } } }
export function GroupTasks({ group }: { group: Bot }) {
  const store = useStore()
  const [items, setItems] = useState<TaskEntry[]>([])
  const [title, setTitle] = useState('')
  const [text, setText] = useState('')
  const [owner, setOwner] = useState(group.members[0] ?? '')
  const [busy, setBusy] = useState(false)
  const [loaded, setLoaded] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [attempt, setAttempt] = useState(0)
  const [requestId, setRequestId] = useState(() => crypto.randomUUID())
  useEffect(() => {
    let active = true
    if (!store.client) { setError('Computer is offline'); return }
    void store.client.call<{ items: TaskEntry[] }>('groupTasks', { groupId: group.id }).then(value => { if (active) { setItems(value.items); setLoaded(true) } }).catch((error: unknown) => { if (active) setError(String(error)) })
    return () => { active = false }
  }, [group.id, store.client, attempt])
  const run = async (method: string, body: object) => {
    if (!store.client || busy) return
    setBusy(true)
    setError(null)
    try {
      await store.client.call(method, body)
      if (method === 'createGroupTask') { setTitle(''); setText(''); setRequestId(crypto.randomUUID()) }
      setAttempt(value => value + 1)
    } catch (error: unknown) { setError(String(error)); setAttempt(value => value + 1) }
    finally { setBusy(false) }
  }
  return <div className="sidechat"><ModalHeader title="Group tasks" subtitle={group.name} trailing={<IconButton title="Refresh tasks" icon="arrow.clockwise" onClick={() => { setError(null); setAttempt(value => value + 1) }} />} />
    <div className="sidechat-scroll"><VStack gap={20}>
      <Text size="caption" tone="secondary">Assign work to a member. Attempts run in that bot’s chat. Pause waits for cancellation; resuming asks the bot to inspect previous work before continuing.</Text>
      <Field label="Task title"><input className="field-box" value={title} maxLength={160} disabled={busy} onChange={event => { setTitle(event.target.value); setRequestId(crypto.randomUUID()) }} /></Field>
      <Field label="Instructions"><AutoTextArea className="field-box" value={text} onChange={value => { setText(value); setRequestId(crypto.randomUUID()) }} minRows={3} maxRows={8} disabled={busy} maxLength={32768} /></Field>
      <Field label="Owner"><ChoicePicker selection={owner} options={group.members.map(id => ({ id, label: store.bots.get(id)?.name ?? 'Unavailable member' }))} onChange={value => { setOwner(value); setRequestId(crypto.randomUUID()) }} /></Field>
      <Button disabled={busy || !title.trim() || !text.trim() || !group.members.includes(owner)} onClick={() => void run('createGroupTask', { groupId: group.id, owner, title, text, requestId })}>Create task</Button>
      {error && <Text role="alert" tone="danger">{error}</Text>}
      {!loaded && !error && <Spinner />}
      {loaded && items.length === 0 && <Text tone="secondary">No tasks yet.</Text>}
      {items.map(entry => { const task = entry.data.groupTask; const stopped = ['draft', 'paused', 'failed', 'interrupted'].includes(task.status); return <VStack key={entry.id} gap={8}>
        <Text weight="semibold">{task.title}</Text><Text size="caption" tone="secondary">{store.bots.get(task.owner)?.name ?? 'Unavailable owner'} · {task.status}</Text>
        <Text style={{ whiteSpace: 'pre-wrap' }}>{task.text}</Text>
        <HStack gap={8}>{stopped && <Button disabled={busy} onClick={() => void run('updateGroupTask', { taskId: entry.id, revision: entry.rev, action: task.status === 'draft' ? 'start' : 'resume' })}>{task.status === 'draft' ? 'Start task' : 'Resume task'}</Button>}
          {['queued', 'running'].includes(task.status) && <Button disabled={busy} onClick={() => void run('updateGroupTask', { taskId: entry.id, revision: entry.rev, action: 'pause' })}>Pause task</Button>}</HStack>
        {stopped && <Field label="Reassign owner"><ChoicePicker selection={task.owner} options={group.members.map(id => ({ id, label: store.bots.get(id)?.name ?? 'Unavailable member' }))} onChange={owner => { if (!busy) void run('updateGroupTask', { taskId: entry.id, revision: entry.rev, action: 'reassign', owner }) }} /></Field>}
        <details><summary>Task history</summary>{task.history.map((event, index) => <div key={index}><Text size="caption" tone="secondary">{new Date(event.at).toLocaleString()} · {event.status} · {store.bots.get(event.owner)?.name ?? event.owner}</Text><Text as="div" style={{ whiteSpace: 'pre-wrap' }}>{event.detail}</Text></div>)}</details>
      </VStack> })}
    </VStack></div></div>
}

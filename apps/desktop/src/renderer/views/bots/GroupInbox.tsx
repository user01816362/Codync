import { useEffect, useState } from 'react'
import type { Bot } from '@shared/models'
import { Button, ChoicePicker, Switch, IconButton, Spinner } from '../../components/Controls'
import { ModalHeader } from '../../components/Overlay'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useStore } from '../../store/context'
import { Field } from './parts'
import { errorText } from './drafts'

interface MailItem {
  id: string
  from: string
  createdAt: number
  delivered: boolean
  replied: boolean
  mail: { subject: string; text: string; status: 'pending' | 'approved' | 'rejected'; handler?: string }
}

/** Approval never adds the sender to the group or exposes group history. */
export function GroupInbox({ group }: { group: Bot }) {
  const store = useStore()
  const [items, setItems] = useState<MailItem[] | null>(null)
  const [policy, setPolicy] = useState({ autoApprove: false, handler: '' })
  const [attempt, setAttempt] = useState(0)
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState<string | null>(null)
  const [handlers, setHandlers] = useState<Record<string, string>>({})
  useEffect(() => {
    let active = true
    setError(null)
    if (!store.client) { setError('Computer is offline'); return }
    void store.client.call<{ items: MailItem[]; policy: typeof policy }>('groupInbox', { groupId: group.id }).then(value => {
      if (active) { setItems(value.items); setPolicy(value.policy ?? { autoApprove: false, handler: '' }) }
    }).catch((error: unknown) => { if (active) setError(errorText(error)) })
    return () => { active = false }
  }, [store.client, group.id, attempt])
  const handlerFor = (id: string) => group.members.includes(handlers[id] ?? '') ? handlers[id]! : group.members[0] ?? ''
  const decide = async (item: MailItem, approve: boolean) => {
    if (!store.client || busy) return
    setBusy(item.id)
    setError(null)
    try {
      await store.client.call('decideGroupMail', { mailId: item.id, approve, handler: handlerFor(item.id) })
      setAttempt(value => value + 1)
    } catch (error: unknown) { setError(errorText(error)) }
    finally { setBusy(null) }
  }
  const savePolicy = async (next: typeof policy) => {
    if (!store.client || busy) return
    setBusy('policy')
    setError(null)
    try { await store.client.call('setGroupInboxPolicy', { groupId: group.id, ...next }); setPolicy(next) }
    catch (error: unknown) { setError(errorText(error)) }
    finally { setBusy(null) }
  }
  return <div className="sidechat">
    <ModalHeader title="Group inbox" subtitle={group.name} trailing={<IconButton title="Refresh inbox" icon="arrow.clockwise" disabled={busy !== null} onClick={() => setAttempt(value => value + 1)} />} />
    <div className="sidechat-scroll">
      <VStack gap={20}>
        <Text size="caption" tone="secondary">Approve an external request and choose who handles it. The sender receives only that handler’s reply, with no access to this conversation.</Text>
        <Switch on={policy.autoApprove} disabled={!items || busy !== null || !group.members.length} label="Automatically approve incoming mail" onChange={autoApprove => void savePolicy({ autoApprove, handler: group.members.includes(policy.handler) ? policy.handler : group.members[0] ?? '' })} />
        <Text size="caption" tone="secondary">New requests go directly to the selected bot. Existing pending requests still need approval. If the handler leaves, incoming requests wait for approval.</Text>
        <Field label="Automatic mail handler">
          <fieldset disabled={!items || busy !== null} style={{ border: 0, margin: 0, padding: 0, minWidth: 0 }}>
            <ChoicePicker selection={policy.handler} options={group.members.map(id => ({ id, label: store.bots.get(id)?.name ?? 'Unavailable member' }))} onChange={handler => void savePolicy({ ...policy, handler })} />
          </fieldset>
        </Field>
        {error && <Text role="alert" tone="danger">{error}</Text>}
        {!items && !error ? <Spinner /> : items?.length === 0 ? <Text tone="secondary">No group mail.</Text> : null}
        {items?.map(item => <VStack key={item.id} gap={8}>
          <Text weight="semibold">{item.mail.subject}</Text>
          <Text size="caption" tone="secondary">From {item.from} · {new Date(item.createdAt).toLocaleString()}</Text>
          <Text as="div" style={{ whiteSpace: 'pre-wrap', overflowWrap: 'anywhere' }}>{item.mail.text}</Text>
          <Text size="caption" tone="secondary">{item.replied ? 'Replied' : item.delivered ? 'Delivered to handler' : item.mail.status === 'approved' ? 'Approved · delivery pending' : item.mail.status === 'rejected' ? 'Rejected' : 'Awaiting your approval'}</Text>
          {item.mail.status === 'pending' ? <>
            <Field label="Handler">
              <fieldset disabled={busy !== null} style={{ border: 0, margin: 0, padding: 0, minWidth: 0 }}>
                <ChoicePicker selection={handlerFor(item.id)} options={group.members.map(id => ({ id, label: store.bots.get(id)?.name ?? 'Unavailable member' }))} onChange={id => { if (busy === null) setHandlers(value => ({ ...value, [item.id]: id })) }} />
              </fieldset>
            </Field>
            <HStack gap={8}>
              <Button disabled={busy !== null || group.members.length === 0} onClick={() => void decide(item, true)}>Approve delivery</Button>
              <Button disabled={busy !== null} onClick={() => void decide(item, false)}>Reject</Button>
            </HStack>
          </> : item.mail.status === 'approved' && !item.delivered ? <Button disabled={busy !== null} onClick={() => void decide(item, true)}>Retry approved delivery</Button> : null}
        </VStack>)}
        {items?.length === 100 && <Text size="caption" tone="secondary">Showing up to 100 requests, pending first.</Text>}
      </VStack>
    </div>
  </div>
}

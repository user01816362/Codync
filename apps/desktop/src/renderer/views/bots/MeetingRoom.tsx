import { MarkdownText } from '../thread/Markdown'
import { useEffect, useState } from 'react'
import { isGroup } from '@shared/models'
import { Button, CardSection, ChoicePicker, Disclosure, IconButton, ValueRow } from '../../components/Controls'
import { ModalHeader, SheetPage } from '../../components/Overlay'
import { Text } from '../../components/Text'
import { HStack, VStack } from '../../components/Stack'
import { useStore } from '../../store/context'
import { useModel } from '../../lib/observable'
import { AutoTextArea, Field } from './parts'

type Delegate = { botId: string; groupId?: string; name: string; department: string }
type Position = { position: string; reason: string }
type RoomState = { groupId: string; rev: number; meeting: { kind: string; status: string; agenda: string; proposal: string; proposalRevision: number; chair: string; delegates: Delegate[]; positions: Record<string, Position>; spent: number; limit: number; approved: boolean; history?: { proposalRevision: number; proposal: string; status: string; approved: boolean; positions: Record<string, Position> }[] } }

/** One host-owned meeting, rendered with ordinary profile cards and explicit owner actions. */
export function MeetingRoom({ groupId, created }: { groupId?: string; created?: (id: string) => void }) {
  const store = useStore()
  useModel(store)
  const [room, setRoom] = useState<RoomState | null>(null)
  const [name, setName] = useState('')
  const [agenda, setAgenda] = useState('')
  const [kind, setKind] = useState('consultation')
  const [delegates, setDelegates] = useState<{ botId: string; groupId?: string }[]>([])
  const [chair, setChair] = useState('')
  const [department, setDepartment] = useState('')
  useEffect(() => {
    if (!delegates.some(d => d.botId === chair)) setChair(delegates[0]?.botId ?? '')
  }, [delegates, chair])
  const [historyOpen, setHistoryOpen] = useState(false)
  const [proposal, setProposal] = useState('')
  const [busy, setBusy] = useState(false)
  const [notice, setNotice] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [attempt, setAttempt] = useState(0)
  const revision = groupId ? store.entries.get(groupId)?.find(entry => entry.id === `meeting:${groupId}`)?.rev : undefined
  useEffect(() => {
    let active = true
    if (!groupId || !store.client) return
    const origin = store.bots.get(groupId)
    if (origin && !origin.meeting) {
      setDepartment(origin.id)
      setName(current => current || `${origin.name} meeting`.slice(0, 40))
      setAgenda(current => current || origin.description || '')
      return
    }
    void store.client.call<RoomState>('meetingState', { groupId }).then(value => { if (active) setRoom(value) }).catch((e: unknown) => { if (active) setError(String(e)) })
    return () => { active = false }
  }, [groupId, store.client, revision, attempt])
  const run = async (action: string) => {
    if (!store.client || !room || busy) return
    setBusy(true); setError(null)
    try {
      const result = await store.client.call<{ deliveredTo?: string[] }>('meetingAction', { groupId: room.groupId, rev: room.rev, action, ...(['propose', 'finish'].includes(action) ? { text: proposal } : {}) })
      setNotice(result.deliveredTo ? `Outcome published to ${result.deliveredTo.length} departments. No automatic follow-up was requested.` : null)
      setAttempt(n => n + 1)
    }
    catch (e: unknown) { setError(String(e)); setAttempt(n => n + 1) }
    finally { setBusy(false) }
  }
  const create = async () => {
    if (!store.client || busy) return
    setBusy(true); setError(null)
    try {
      const result = await store.client.call<RoomState>('createMeeting', { name, agenda, kind, chair, delegates })
      setRoom(result); created?.(result.groupId)
    } catch (e: unknown) { setError(String(e)) }
    finally { setBusy(false) }
  }
  const meeting = room?.meeting
  const selected = new Set(delegates.map(d => d.botId))
  const disabled = busy || store.isOffline
  const departments = store.roster.filter(bot => isGroup(bot) && !bot.meeting)
  const candidates = store.roster.filter(bot => !isGroup(bot) && (!department || store.bots.get(department)?.members.includes(bot.id)))
  return <SheetPage>
    <ModalHeader title={meeting ? 'Meeting room' : 'New meeting room'} trailing={meeting ? <IconButton title="Refresh" icon="arrow.clockwise" onClick={() => setAttempt(n => n + 1)} /> : undefined} />
    <div className="bot-form"><div className="bot-form-content">
      <Text size="caption" tone="secondary">Participants keep their existing permissions and shared agent-session history. Only the agenda and material you explicitly share belong in this discussion.</Text>
      {error ? <Text role="status" tone="danger">{error}</Text> : null}
      {notice ? <Text role="status">{notice}</Text> : null}
      {!meeting ? <>
        <CardSection title="Room">
          <Field label="Name"><input dir="auto" className="field-box" value={name} maxLength={40} onChange={e => setName(e.target.value)} /></Field>
          <ValueRow label="Purpose"><ChoicePicker disabled={disabled} selection={kind} options={[{ id: 'consultation', label: 'Consultation' }, { id: 'meeting', label: 'Meeting' }, { id: 'decision', label: 'Decision' }]} onChange={setKind} /></ValueRow>
          <Field label="Agenda"><AutoTextArea value={agenda} onChange={setAgenda} minRows={3} maxRows={8} maxLength={4000} className="field-box" ariaLabel="Agenda" /></Field>
        </CardSection>
        <CardSection title={`Delegates · ${delegates.length}/5`} footer="Choose a department, then its delegates. Groups are represented by bots; their full chat histories are not imported. Five delegates share a ten-turn budget.">
          <ValueRow label="Department" detail="Choose a group to see its members, or invite individual bots."><ChoicePicker disabled={disabled} selection={department} options={[{ id: '', label: 'Individual bots' }, ...departments.map(group => ({ id: group.id, label: group.name }))]} onChange={setDepartment} /></ValueRow>
          {candidates.length === 0 ? <Text tone="secondary">No available members in this department.</Text> : null}
          {candidates.map(bot => <VStack key={bot.id} gap={8}>
            <button className="group-action-row press" role="checkbox" aria-checked={selected.has(bot.id)} disabled={disabled || (!selected.has(bot.id) && delegates.length >= 5)} onClick={() => {
              setDelegates(ds => selected.has(bot.id) ? ds.filter(d => d.botId !== bot.id) : [...ds, { botId: bot.id, ...(department ? { groupId: department } : {}) }])
              if (!chair) setChair(bot.id)
            }}><Text dir="auto" grow>{bot.name}</Text><Text>{selected.has(bot.id) ? '✓' : '+'}</Text></button>
            {selected.has(bot.id) ? <ValueRow label="Represents"><ChoicePicker disabled={disabled} selection={delegates.find(d => d.botId === bot.id)?.groupId ?? ''} options={[{ id: '', label: 'Personal' }, ...store.roster.filter(g => isGroup(g) && !g.meeting && g.members.includes(bot.id)).map(g => ({ id: g.id, label: g.name }))]} onChange={id => setDelegates(ds => ds.map(d => d.botId === bot.id ? { botId: d.botId, ...(id ? { groupId: id } : {}) } : d))} /></ValueRow> : null}
          </VStack>)}
        </CardSection>
        {delegates.length ? <CardSection title="Invited delegates">{delegates.map(d => <ValueRow key={d.botId} label={store.bots.get(d.botId)?.name ?? 'Delegate'} detail={d.groupId ? store.bots.get(d.groupId)?.name : 'Personal'}>
          <IconButton title="Remove delegate" icon="minus.circle" disabled={disabled} onClick={() => setDelegates(ds => ds.filter(value => value.botId !== d.botId))} />
        </ValueRow>)}</CardSection> : null}
        <CardSection title="Coordination"><ValueRow label="Chair" detail="Organizes proposals. Each delegate records their own response; you approve the final decision.">
          {delegates.length ? <ChoicePicker disabled={disabled} selection={chair} options={delegates.map(d => ({ id: d.botId, label: store.bots.get(d.botId)?.name ?? 'Delegate' }))} onChange={setChair} /> : <Text tone="secondary">Choose delegates first</Text>}
        </ValueRow></CardSection>
        <Text size="caption" tone="secondary">Delegates can read recent messages and tasks in departments they still belong to. Participation does not prove they read or remember the full history.</Text>
        <Text size="caption" tone="secondary">Ten discussion turns and ten minutes maximum. All delegates must approve the same proposal before final user approval. The room starts paused.</Text>
        <Button disabled={disabled || !name.trim() || !agenda.trim() || delegates.length < 2 || !selected.has(chair)} onClick={() => void create()}>Create room and approve invitations</Button>
      </> : <>
        <CardSection title={`${meeting.kind} · ${meeting.status}`}><MarkdownText text={meeting.agenda} /><Text tone="secondary">{meeting.spent}/{meeting.limit} discussion turns used</Text></CardSection>
        <CardSection title={`Proposal · revision ${meeting.proposalRevision}`}>
          <MarkdownText text={meeting.proposal || 'No proposal recorded.'} />
          {meeting.delegates.map(d => <VStack key={d.botId} gap={4}><Text>{d.name} · {d.department}</Text><Text tone="secondary">{meeting.positions[d.botId]?.position ?? 'Not answered'}{meeting.positions[d.botId]?.reason ? ` · ${meeting.positions[d.botId].reason}` : ''}</Text></VStack>)}
        </CardSection>
        {['paused', 'running', 'awaiting_owner'].includes(meeting.status) ? <CardSection title="Owner proposal"><AutoTextArea value={proposal} onChange={setProposal} minRows={2} maxRows={6} className="field-box" ariaLabel="Proposal" maxLength={8000} /><Button disabled={disabled || !proposal.trim()} onClick={() => void run('propose')}>Publish proposal and reset responses</Button></CardSection> : null}
        {meeting.history?.length ? <CardSection title="Previous proposals"><Disclosure expanded={historyOpen} onToggle={() => setHistoryOpen(value => !value)} label={`${meeting.history.length} recorded versions`}>
          {meeting.history.map(version => <VStack key={version.proposalRevision} gap={8}>
            <Text weight="semibold">Revision {version.proposalRevision} · {version.status}{version.approved ? ' · User approved' : ''}</Text>
            <MarkdownText text={version.proposal} />
            {meeting.delegates.map(d => <Text key={d.botId} size="caption">{d.name} · {d.department}: {version.positions[d.botId]?.position ?? 'Not answered'}</Text>)}
          </VStack>)}
        </Disclosure></CardSection> : null}
        <HStack gap={8} wrap>
          {['paused', 'unresolved'].includes(meeting.status) ? <Button disabled={disabled || meeting.spent >= meeting.limit} onClick={() => void run('resume')}>Start / Resume</Button> : null}
          {meeting.status === 'running' ? <Button disabled={disabled} onClick={() => void run('pause')}>Pause</Button> : null}
          {meeting.kind !== 'decision' && !['completed', 'resolved', 'archived', 'cancelled'].includes(meeting.status) ? <Button disabled={disabled || !proposal.trim()} onClick={() => void run('finish')}>Complete with summary</Button> : null}
          {meeting.status === 'awaiting_owner' ? <Button disabled={disabled} onClick={() => void run('approve')}>Approve final decision</Button> : null}
          {['resolved', 'completed', 'unresolved', 'cancelled'].includes(meeting.status) ? <Button disabled={disabled} onClick={() => void run('report')}>Publish outcome to departments</Button> : null}
          {meeting.status !== 'archived' ? <Button kind="secondary" disabled={disabled} onClick={() => void run('archive')}>Archive</Button> : <Button disabled={disabled} onClick={() => void run('reopen')}>Reopen</Button>}
          {!['resolved', 'completed', 'archived', 'cancelled'].includes(meeting.status) ? <Button kind="secondary" disabled={disabled} onClick={() => void run('cancel')}>Cancel meeting</Button> : null}
        </HStack>
      </>}
    </div></div>
  </SheetPage>
}

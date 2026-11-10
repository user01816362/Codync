import { MeetingRoom } from './MeetingRoom'
import { useRef, useState } from 'react'
import { isGroup, type Bot } from '@shared/models'
import { BotAvatar, BotOrGroupAvatar } from '../../components/Avatar'
import { CardSection, IconButton, Spinner, ValueRow } from '../../components/Controls'
import { Icon } from '../../components/Icon'
import { Dialog, ModalHeader, Sheet, SheetPage, useDismiss } from '../../components/Overlay'
import { Text } from '../../components/Text'
import { useStore } from '../../store/context'
import { GroupActionRow } from './GroupActionRow'
import { LibraryView } from '../library/LibraryView'
import { ArtifactView } from '../library/ArtifactView'
import { CustomIconEditor } from './CustomIconEditor'
import { StorageOverview } from './StorageOverview'
import { GroupTasks } from './GroupTasks'
import { GroupInbox } from './GroupInbox'
import { errorText } from './drafts'
import { AnchoredPanel, AutoTextArea, Field } from './parts'
import './bots.css'
import { useTr } from '../../lib/use-language'


/**
 * Change a group chat's name, what it's for and who's in it. Bots are added from a
 * search of the ones not in it yet. New groups start in `NewGroupView`.
 */
export function GroupEditorView({ group }: { group: Bot }) {
  const tr = useTr()
  const store = useStore()
  const isMeeting = (store.bots.get(group.id) ?? group).meeting === true
  const dismiss = useDismiss()
  const [name, setName] = useState(group.name)
  const [about, setAbout] = useState(group.description)
  const [members, setMembers] = useState<string[]>(group.members)
  const [query, setQuery] = useState('')
  const avatarRef = useRef<HTMLButtonElement>(null)
  const [avatar, setAvatar] = useState(false)
  const [library, setLibrary] = useState(false)
  const [artifact, setArtifact] = useState<string | null>(null)
  const [storage, setStorage] = useState(false)
  const [meeting, setMeeting] = useState(false)
  const [tasks, setTasks] = useState(false)
  const [inbox, setInbox] = useState(false)
  const [adding, setAdding] = useState(false)
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [confirmDelete, setConfirmDelete] = useState<Bot | null>(null)

  const picked = members.map((id) => store.bots.get(id)).filter((b): b is Bot => !!b)
  // Bots that can still join, filtered by the search.
  const q = query.trim().toLowerCase()
  const candidates = store.roster.filter((b) => !isMeeting && !isGroup(b) && !members.includes(b.id) && (!q || b.name.toLowerCase().includes(q)))
  const canSave = members.length > 0 && !saving && !store.isOffline
  // "Alice, Bob" when no name is typed.
  const defaultName = picked.length ? picked.map((b) => b.name).join(', ') : 'Name'
  const current = store.bots.get(group.id)

  const toggle = (id: string) => {
    setMembers((list) => (list.includes(id) ? list.filter((m) => m !== id) : [...list, id]))
    setQuery('')
  }

  const save = () => {
    if (!canSave) return
    setSaving(true)
    setError(null)
    store
      .updateGroup({ id: group.id, kind: 'group', name: name.trim() || defaultName, description: about.trim(), members, pinned: current?.pinned })
      .then(dismiss)
      .catch((e: unknown) => setError(errorText(e)))
      .finally(() => setSaving(false))
  }

  return (
    <SheetPage>
      <ModalHeader icon="person.2" title={tr(`Group chat`)} trailing={saving ? <Spinner /> : <IconButton title="Save" icon="checkmark" disabled={!canSave} onClick={save} />} />
      <div className="bot-form group-scroll">
        <div className="bot-form-content group-form">
          <CardSection title="Profile">
            <ValueRow label="Avatar">
              <button ref={avatarRef} className="avatar-button small" aria-label={tr('Edit group avatar')} title={tr('Edit group avatar')} onClick={() => setAvatar(value => !value)}>
                <BotOrGroupAvatar bot={current ?? group} members={picked} size={28} />
              </button>
              <AnchoredPanel open={avatar} onClose={() => setAvatar(false)} anchor={avatarRef}>
                <div className="group-icon-options"><CustomIconEditor target={`bot:${group.id}`} /></div>
              </AnchoredPanel>
            </ValueRow>
            <Field label={tr('Name')}>
              <input dir="auto" className="field-box" value={name} placeholder={defaultName} aria-label={tr('Name')} maxLength={40} disabled={saving} onChange={(e) => setName(e.target.value)} />
            </Field>
            <Field label={tr('Group instructions')} detail={tr('What this group works on. Each member keeps its own instructions.')}>
              <AutoTextArea className="field-box" ariaLabel={tr('Group instructions')} value={about} placeholder="What this group works on" minRows={3} maxRows={8} disabled={saving || isMeeting} maxLength={4000} onChange={setAbout} />
            </Field>
          </CardSection>
          <CardSection title="Group activity">
            {(current ?? group)?.meeting ? <GroupActionRow title="Meeting room" icon="person.2" onClick={() => setMeeting(true)} /> : null}
            <GroupActionRow title="Group tasks" icon="checklist" onClick={() => setTasks(true)} />
            <GroupActionRow title="Group inbox" icon="tray" onClick={() => setInbox(true)} />
            <GroupActionRow title="Library" icon="folder" onClick={() => setLibrary(true)} />
          </CardSection>
          <CardSection title={`${tr('Members')} · ${members.length}`} footer="Everyone answers in turn unless you @mention someone. Keep at least one member in the group.">
            {picked.map((bot) => (
              <div key={bot.id} className="member-row fade-in">
                <BotAvatar bot={bot} size={32} animated={false} />
                <span className="member-name">{bot.name}</span>
                <IconButton title={tr('Remove {name}').replaceAll('{name}', () => bot.name)} icon="xmark" disabled={saving || isMeeting || members.length <= 1} onClick={() => toggle(bot.id)} />
              </div>
            ))}
            {adding ? (
              <>
                <input
                  className="field-box fade-in group-search"
                  autoFocus
                  value={query}
                  placeholder={tr("Search bots")}
                  aria-label={tr("Search bots")}
                  onChange={(e) => setQuery(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter' && !e.nativeEvent.isComposing && candidates[0]) toggle(candidates[0].id)
                    if (e.key === 'Escape') setAdding(false)
                  }}
                />
                {candidates.map((bot) => (
                  <button key={bot.id} className="member-row press fade-in" aria-label={tr('Add {name}').replaceAll('{name}', () => bot.name)} onClick={() => toggle(bot.id)}>
                    <BotAvatar bot={bot} size={32} animated={false} />
                    <Text tone="secondary" className="member-name">{bot.name}</Text>
                    <Icon name="plus.circle" size={18} color="var(--secondary)" />
                  </button>
                ))}
              </>
            ) : candidates.length ? (
              <button className="member-row press" onClick={() => setAdding(true)}>
                <span className="member-add-icon">
                  <Icon name="plus" size={16} color="var(--secondary)" />
                </span>
                <Text tone="secondary" className="member-name">{tr(`Add Member`)}</Text>
              </button>
            ) : null}
          </CardSection>
          <CardSection title="Computer"><GroupActionRow title="Host storage" icon="internaldrive" onClick={() => setStorage(true)} /></CardSection>
          {error ? <Text size="footnote" tone="danger" className="fade-in">{error}</Text> : null}
          {current ? (
            <button className="press group-delete" onClick={() => setConfirmDelete(current)}>
              Delete group chat
            </button>
          ) : null}
        </div>
      </div>
      <Sheet open={library} onClose={() => setLibrary(false)} width={640} height={740}>
        <SheetPage><ModalHeader title="Group Library" /><div className="group-library"><LibraryView botId={group.id} openArtifact={setArtifact} /></div></SheetPage>
      </Sheet>
      <Sheet open={artifact !== null} onClose={() => setArtifact(null)} width={900} height={740}>{artifact ? <ArtifactView botId={group.id} artifactId={artifact} close={() => setArtifact(null)} /> : null}</Sheet>
      <Sheet open={storage} onClose={() => setStorage(false)} width={600} height={640}><StorageOverview /></Sheet>
      <Sheet open={meeting} onClose={() => setMeeting(false)} width={640} height={740}><MeetingRoom groupId={group.id} /></Sheet>
      <Sheet open={tasks} onClose={() => setTasks(false)} width={640} height={740}><GroupTasks group={current ?? group} /></Sheet>
      <Sheet open={inbox} onClose={() => setInbox(false)} width={600} height={640}><GroupInbox group={current ?? group} /></Sheet>
      <DeleteBotDialog bot={confirmDelete} onClose={() => setConfirmDelete(null)} onDeleted={dismiss} />
    </SheetPage>
  )
}

/** The one "Delete <bot>?" confirmation (kit `deleteBotConfirmation`). */
function DeleteBotDialog({ bot, onClose, onDeleted }: { bot: Bot | null; onClose: () => void; onDeleted: () => void }) {
  const store = useStore()
  const tr = useTr()
  return (
    <Dialog
      open={bot !== null}
      title={`Delete ${bot?.name ?? 'bot'}?`}
      message={tr(bot && isGroup(bot) ? 'Its bots and their own chats stay.' : 'Files it changed on your computer stay as they are.')}
      actions={[
        {
          title: bot && isGroup(bot) ? 'Delete group chat' : 'Delete bot and its conversation',
          destructive: true,
          action: () => {
            if (bot) store.delete(bot)
            onDeleted()
          },
        },
      ]}
      onClose={onClose}
    />
  )
}

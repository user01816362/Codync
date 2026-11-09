import { useState } from 'react'
import { BotAvatar, GroupAvatar } from '../../components/Avatar'
import { Button, ChoicePicker, IconButton, SearchField } from '../../components/Controls'
import { Icon } from '../../components/Icon'
import { ModalHeader } from '../../components/Overlay'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { sectionNames } from '../../lib/roster-sections'
import type { BotStore } from '../../store/bot-store'
import { isGroup } from '@shared/models'
import { Field } from './parts'

/** A discoverable entry point to the existing host-backed sections, not a second folder model. */
export function RosterOrganizer({ store }: { store: BotStore }) {
  const [selected, setSelected] = useState<Set<string>>(new Set())
  const [section, setSection] = useState('')
  const [query, setQuery] = useState('')
  const [busy, setBusy] = useState(false)
  const [notice, setNotice] = useState<string | null>(null)
  const bots = store.roster.filter(bot => bot.name.toLowerCase().includes(query.trim().toLowerCase()))
  const move = async (to: string | null) => {
    setBusy(true)
    setNotice(null)
    let moved = 0
    try {
      for (const id of selected) {
        const bot = store.bots.get(id)
        if (!bot) continue
        if (!await store.sections.move(bot, to)) {
          setNotice(`${moved} chats moved. A move was refused; refresh and try again.`)
          return
        }
        moved += 1
        setSelected(value => { const next = new Set(value); next.delete(id); return next })
      }
      setNotice(`${moved} chats moved.`)
    } catch (error: unknown) { setNotice(error instanceof Error ? error.message : String(error)) }
    finally { setBusy(false) }
  }
  return <div className="sidechat">
    <ModalHeader title="Organize chats" subtitle={store.computer.name} />
    <div className="sidechat-scroll">
      <VStack gap={16}>
        <Text size="caption" tone="secondary">Sections organize bots and group chats on this computer. They do not change group membership, files or permissions. Pinned chats remain above their sections.</Text>
        <Field label="Section">
          <input className="field-box" aria-label="Section" placeholder="Choose or name a section" value={section} disabled={busy} onChange={event => setSection(event.target.value)} />
          {sectionNames(store.bots.values()).length > 0 && <fieldset disabled={busy} style={{ border: 0, margin: 0, padding: 0, minWidth: 0 }}>
            <ChoicePicker selection={section} options={sectionNames(store.bots.values()).map(name => ({ id: name, label: name }))} onChange={name => { if (!busy) setSection(name) }} />
          </fieldset>}
        </Field>
        <SearchField value={query} onChange={setQuery} prompt="Search chats" />
        <HStack gap={8}>
          <Button disabled={busy || selected.size === 0 || !section.trim()} onClick={() => void move(section.trim())}>Move selected</Button>
          <Button kind="secondary" disabled={busy || selected.size === 0} onClick={() => void move(null)}>Remove from section</Button>
          <IconButton title="Select visible chats" icon="checkmark.circle" disabled={busy} onClick={() => setSelected(new Set(bots.map(bot => bot.id)))} />
        </HStack>
        {notice && <Text role="status" size="caption">{notice}</Text>}
        {bots.length === 0 && <Text tone="secondary">No matching chats.</Text>}
        {bots.map(bot => <button key={bot.id} className="member-row press" role="checkbox" aria-checked={selected.has(bot.id)} disabled={busy} onClick={() => setSelected(value => {
          const next = new Set(value)
          if (next.has(bot.id)) next.delete(bot.id); else next.add(bot.id)
          return next
        })}>
          {isGroup(bot) ? <GroupAvatar size={32} members={bot.members.flatMap(id => { const member = store.bots.get(id); return member ? [member] : [] })} /> : <BotAvatar bot={bot} size={32} animated={false} />}
          <VStack gap={2} grow><Text>{bot.name}</Text><Text size="caption" tone="secondary">{bot.section || 'No section'}</Text></VStack>
          <Icon name={selected.has(bot.id) ? 'checkmark.circle.fill' : 'circle'} size={20} />
        </button>)}
      </VStack>
    </div>
  </div>
}

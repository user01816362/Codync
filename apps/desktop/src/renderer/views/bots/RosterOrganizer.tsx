import { useState } from 'react'
import { BotOrGroupAvatar } from '../../components/Avatar'
import { Button, CardSection, ChoicePicker, IconButton, SearchField } from '../../components/Controls'
import { Icon } from '../../components/Icon'
import { ModalHeader, SheetPage } from '../../components/Overlay'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { limitSectionName, sectionName, sectionNames } from '../../lib/roster-sections'
import { useModel } from '../../lib/observable'
import { useTr } from '../../lib/use-language'
import type { BotStore } from '../../store/bot-store'
import { Field } from './parts'

/** Folder UI over the host's existing sections; assignments remain shared across devices. */
export function RosterOrganizer({ store, creating = false, close }: { store: BotStore; creating?: boolean; close: () => void }) {
  useModel(store)
  const tr = useTr()
  const [selected, setSelected] = useState<Set<string>>(new Set())
  const [section, setSection] = useState('')
  const [created, setCreated] = useState(false)
  const [query, setQuery] = useState('')
  const [busy, setBusy] = useState(false)
  const [notice, setNotice] = useState<string | null>(null)
  const bots = store.roster.filter(bot => bot.name.toLowerCase().includes(query.trim().toLowerCase()))
  const names = sectionNames(store.bots.values())
  const destination = sectionName(section)
  const duplicate = creating && !created && destination !== null && names.includes(destination)
  const count = [...selected].filter(id => store.bots.has(id)).length
  const disabled = busy || store.isOffline
  const allVisible = bots.length > 0 && bots.every(bot => selected.has(bot.id))
  const toggle = (id: string) => setSelected(value => {
    const next = new Set(value)
    if (next.has(id)) next.delete(id); else next.add(id)
    return next
  })
  const move = async (to: string | null) => {
    if (disabled || !count || (to !== null && (!destination || duplicate))) return
    setBusy(true)
    setNotice(null)
    let moved = 0
    try {
      for (const id of selected) {
        const bot = store.bots.get(id)
        if (!bot) continue
        if (!await store.sections.move(bot, to)) {
          setNotice(`${moved} chats moved. Remaining chats were not moved. Try again when connected.`)
          return
        }
        moved += 1
        if (creating) setCreated(true)
        setSelected(value => { const next = new Set(value); next.delete(id); return next })
      }
      if (creating) close()
      else setNotice(`${moved} chats moved.`)
    } catch (error: unknown) { setNotice(error instanceof Error ? error.message : String(error)) }
    finally { setBusy(false) }
  }
  return <SheetPage>
    <ModalHeader title={creating ? 'New folder' : 'Organize chats'} subtitle={store.hostName} />
    <div className="bot-form">
      <div className="bot-form-content">
        <CardSection title="Folder" footer="Folders organize chats on this computer. Pinned chats remain at the top. Group membership, files and permissions stay the same.">
          <Field label="Folder name">
            <input dir="auto" autoFocus={creating} className="field-box" aria-label={tr('Folder name')} placeholder={tr('Name your folder')} value={section}
              disabled={disabled} onChange={event => setSection(limitSectionName(event.target.value))} />
          </Field>
          {!creating && names.length > 0 ? <ChoicePicker selection={section} options={names.map(name => ({ id: name, label: name }))} onChange={name => { if (!disabled) setSection(name) }} /> : null}
        </CardSection>
        {duplicate ? <Text tone="danger" role="status">{tr('A folder with this name already exists. Use Organize chats to move chats into it.')}</Text> : null}
        {store.isOffline ? <Text tone="attention" role="status">{tr('Reconnect to this computer to organize chats.')}</Text> : null}
        <SearchField value={query} onChange={setQuery} prompt="Search chats" />
        <CardSection title={`${tr('Chats')} · ${count} ${tr('selected')}`} footer={creating ? 'Choose at least one chat to create this folder.' : undefined}
          accessory={<IconButton title={allVisible ? 'Deselect visible chats' : 'Select visible chats'} icon={allVisible ? 'minus.circle' : 'checkmark.circle'} disabled={disabled || !bots.length} onClick={() => setSelected(value => {
            const next = new Set(value)
            for (const bot of bots) { if (allVisible) next.delete(bot.id); else next.add(bot.id) }
            return next
          })} />}>
          {bots.length === 0 ? <Text tone="secondary">{tr('No matching chats.')}</Text> : null}
          {bots.map(bot => <button key={bot.id} className="group-action-row press" role="checkbox" aria-checked={selected.has(bot.id)} disabled={disabled} onClick={() => toggle(bot.id)}>
            <BotOrGroupAvatar bot={bot} members={store.members(bot)} size={28} />
            <VStack gap={2} grow><Text dir="auto">{bot.name}</Text><Text size="caption" tone="secondary">{bot.section || tr('Unassigned')}{bot.pinned ? ` · ${tr('Pinned')}` : ''}</Text></VStack>
            <Icon name={selected.has(bot.id) ? 'checkmark.circle.fill' : 'circle'} size={20} />
          </button>)}
        </CardSection>
        {notice ? <Text role="status" size="caption">{notice}</Text> : null}
        <HStack gap={8} wrap>
          <Button disabled={disabled || count === 0 || !destination || duplicate} onClick={() => void move(destination)}>{creating ? 'Create folder' : 'Move selected'}</Button>
          {!creating ? <Button kind="secondary" disabled={disabled || count === 0} onClick={() => void move(null)}>Move to Unassigned</Button> : null}
        </HStack>
      </div>
    </div>
  </SheetPage>
}

import type { Bot } from '@shared/models'
import type { MenuItem } from '../components/Menu'
import { sectionNames } from '../lib/roster-sections.ts'
import type { BotStore } from '../store/bot-store'
import type { Naming } from './NameDialog'

/** A bot row's "Move to section"; none while its computer runs a host without sections. */
export function moveToSectionItems(bot: Bot, store: BotStore, askName: (naming: Naming) => void): MenuItem[] {
  if (!store.hasRosterSections) return []
  const names = sectionNames(store.bots.values())
  const submenu: MenuItem[] = [
    ...names.map((name) => ({ title: name, icon: 'folder', verbatim: true, selected: name === bot.section, action: () => store.sections.move(bot, name) })),
    { title: 'Unassigned', selected: !bot.section, divider: names.length > 0, action: () => store.sections.move(bot, null) },
    { title: 'New folder…', icon: 'folder', divider: true, action: () => askName({ title: 'New folder', name: '', save: (name) => store.sections.move(bot, name) }) },
  ]
  return [{ title: 'Move to folder', icon: 'folder', submenu }]
}

/** A section header's right-click menu, `rename` taking the new name (null: remove it, its bots staying without one). None while the computer runs a host without sections. */
export function sectionHeaderItems(section: string, store: BotStore, askName: (naming: Naming) => void, rename: (to: string | null) => void): MenuItem[] {
  if (!store.hasRosterSections) return []
  return [
    { title: 'Rename folder…', icon: 'pencil', action: () => askName({ title: 'Rename folder', name: section, save: rename }) },
    { title: 'Ungroup chats', icon: 'xmark.circle', action: () => rename(null) },
  ]
}

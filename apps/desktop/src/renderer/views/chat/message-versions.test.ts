import assert from 'node:assert/strict'
import { test } from 'node:test'
import type { Entry, EntryData } from '../../../shared/models.ts'
import { buildChat } from './chat-items.ts'
import { selectedVersion, versionGroups, type VersionPick } from './message-versions.ts'

let seq = 0

function entry(id: string, kind: string, turn: number, createdAt: number, data: EntryData): Entry {
  return { id, seq: ++seq, botId: 'b1', rev: 1, kind, turn, data, createdAt, updatedAt: createdAt }
}

const user = (id: string, turn: number, createdAt: number, data: EntryData) => entry(id, 'user', turn, createdAt, data)
const reply = (id: string, turn: number, createdAt: number, text: string) =>
  entry(id, 'agent', turn, createdAt, { text, final: true })
const edited = (id: string, turn: number, createdAt: number, text: string, editOf: string, version: number) =>
  user(id, turn, createdAt, { text, editedAt: createdAt, editOf, version })

/** u1 (turn 1) answered by a1, u2 (turn 2), then u1 edited as e1 (turn 3) answered by a2. The host marks u1 and a1 superseded. */
function chat(): Entry[] {
  seq = 0
  return [
    user('u1', 1, 1_000, { text: 'first', status: 'sent', superseded: true }),
    reply('a1', 1, 1_100, 'answer to first'),
    user('u2', 2, 1_500, { text: 'second', status: 'sent' }),
    edited('e1', 3, 3_000, 'edited first', 'u1', 2),
    reply('a2', 3, 3_100, 'answer to the edit'),
  ].map((e) => (e.turn === 1 && e.kind === 'agent' ? { ...e, data: { ...e.data, superseded: true } } : e))
}

const ids = (items: ReturnType<typeof buildChat>) =>
  items.flatMap((item) => (item.kind === 'entry' ? [`${item.entry.id}:${item.entry.data.text}`] : []))

test('an edited message shows its newest version and answer where it was first sent', () => {
  const items = buildChat(chat())
  assert.deepEqual(ids(items), ['e1:edited first', 'a2:answer to the edit', 'u2:second'])
  const shown = items.find((item) => item.kind === 'entry')!
  assert.equal(shown.kind, 'entry')
  if (shown.kind !== 'entry') return
  assert.deepEqual(shown.versions?.map((v) => v.id), ['u1', 'e1'])
  assert.equal(shown.versionIndex, 1)
})

test('picking an older version brings back its own answer', () => {
  const pick = new Map<string, VersionPick>([['u1', { version: 'u1', of: 2 }]])
  assert.deepEqual(ids(buildChat(chat(), pick)), ['u1:first', 'a1:answer to first', 'u2:second'])
})

test('a newer edit wins over a pick made before it', () => {
  const pick = new Map<string, VersionPick>([['u1', { version: 'u1', of: 1 }]])
  assert.deepEqual(ids(buildChat(chat(), pick)), ['e1:edited first', 'a2:answer to the edit', 'u2:second'])
})

test('versions group by their first entry, oldest first, even when it is not loaded', () => {
  const all = chat()
  const groups = versionGroups(all)
  assert.deepEqual(groups.get('u1')?.versions.map((v) => v.id), ['u1', 'e1'])
  assert.equal(selectedVersion(groups.get('u1')!).id, 'e1')

  const onlyEdit = versionGroups(all.filter((e) => e.id !== 'u1'))
  assert.deepEqual(onlyEdit.get('e1')?.versions.map((v) => v.id), ['e1'])
  assert.deepEqual(ids(buildChat(all.filter((e) => e.id !== 'u1'))), ['u2:second', 'e1:edited first', 'a2:answer to the edit'])
})

import type { Entry } from '@shared/models'

/** One message the user edited: every version, oldest first, and the one that keeps its place in the chat. */
export interface VersionGroup {
  /** The first version the chat holds; the group is shown where it is. */
  anchor: Entry
  versions: Entry[]
}

/**
 * The edited messages among `entries`, by the id of their anchor. Each edit is its own entry (so its turn and
 * answers stay together), and points at the first version with `editOf`.
 */
export function versionGroups(entries: Entry[]): Map<string, VersionGroup> {
  const held = new Map(entries.map((e) => [e.id, e]))
  const byRoot = new Map<string, Entry[]>()
  for (const e of entries) {
    if (e.kind !== 'user' || !e.data.editOf) continue
    const edits = byRoot.get(e.data.editOf) ?? []
    edits.push(e)
    byRoot.set(e.data.editOf, edits)
  }
  const groups = new Map<string, VersionGroup>()
  for (const [root, edits] of byRoot) {
    const first = held.get(root)
    const versions = (first ? [first, ...edits] : edits)
      .sort((a, b) => (a.data.version ?? 1) - (b.data.version ?? 1) || a.seq - b.seq)
    const anchor = versions[0]!
    groups.set(anchor.id, { anchor, versions })
  }
  return groups
}

/** What the reader chose to see: a version, and how many versions the message had when they chose it. */
export interface VersionPick {
  version: string
  /** A newer edit (more versions) shows itself instead of leaving an old pick in place. */
  of: number
}

/** The version a chat shows: the one the reader picked, or the newest. */
export function selectedVersion(group: VersionGroup, pick?: VersionPick): Entry {
  const newest = group.versions[group.versions.length - 1]!
  if (!pick || pick.of !== group.versions.length) return newest
  return group.versions.find((v) => v.id === pick.version) ?? newest
}

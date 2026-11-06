import type { SharedFile } from './models.ts'

/** Preview only bounded text-like files; content is still validated as UTF-8 after download. */
const extensions = new Set('md markdown txt log json jsonl yaml yml toml xml csv tsv html css js jsx ts tsx py rs go swift kt kts java c h cpp hpp sh bash zsh sql diff patch'.split(' '))
export function canPreview(file: SharedFile): boolean {
  const dot = file.name.lastIndexOf('.')
  const ext = dot < 0 ? '' : file.name.slice(dot + 1).toLowerCase()
  return Number.isSafeInteger(file.size) && file.size >= 0 && file.size <= 1024 * 1024 && extensions.has(ext)
}

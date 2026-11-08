import type { Attachment } from './models.ts'

/** Preview only bounded text-like files; content is still validated as UTF-8 after download. */
const extensions = new Set('md markdown txt log json jsonl yaml yml toml xml csv tsv html css js jsx ts tsx py rs go swift kt kts java c h cpp hpp sh bash zsh sql diff patch'.split(' '))
export function canPreview(file: Attachment): boolean {
  const dot = file.name.lastIndexOf('.')
  const ext = dot < 0 ? '' : file.name.slice(dot + 1).toLowerCase()
  return Number.isSafeInteger(file.size) && file.size >= 0 && file.size <= 1024 * 1024 && extensions.has(ext)
}

const mediaTypes: Record<string, string> = {
  pdf: 'application/pdf', mp3: 'audio/mpeg', m4a: 'audio/mp4', wav: 'audio/wav', ogg: 'audio/ogg',
  mp4: 'video/mp4', m4v: 'video/mp4', mov: 'video/quicktime', webm: 'video/webm',
}
export function mediaMime(file: Attachment): string | null {
  if (!Number.isSafeInteger(file.size) || file.size < 0 || file.size > 32 * 1024 * 1024) return null
  return mediaTypes[file.name.split('.').pop()?.toLowerCase() ?? ''] ?? null
}
export const canOpenPreview = (file: Attachment) => canPreview(file) || mediaMime(file) !== null

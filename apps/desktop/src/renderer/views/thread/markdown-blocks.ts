// Block-level Markdown as replies use it (kit MarkdownBlocks): paragraphs, headings, nested
// lists, quotes, rules, tables and fenced code. Pure parsing, tested on its own.

export type Block =
  | { kind: 'paragraph'; text: string }
  | { kind: 'heading'; text: string; level: number }
  | { kind: 'bullet'; text: string; marker: string; depth: number }
  | { kind: 'quote'; text: string }
  | { kind: 'code'; text: string; language: string }
  | { kind: 'table'; header: string[]; rows: string[][] }
  | { kind: 'rule' }

const isTableRule = (line: string) => line.includes('-') && [...line].every((c) => '|-: '.includes(c))
const cells = (line: string) => {
  let row = line
  if (row.startsWith('|')) row = row.slice(1)
  if (row.endsWith('|')) row = row.slice(0, -1)
  return row.split('|').map((c) => c.trim())
}

export function parseMarkdown(source: string): Block[] {
  const blocks: Block[] = []
  let paragraph: string[] = []
  let table: string[] = []
  let code: string[] | null = null
  let language = ''
  let fence: { marker: string; length: number; indent: number } | null = null
  const flush = () => {
    if (paragraph.length) {
      blocks.push({ kind: 'paragraph', text: paragraph.join('\n') })
      paragraph = []
    }
    if (table.length) {
      // A header and its `|---|` line make a table; anything less stays text.
      if (table.length >= 2 && isTableRule(table[1]!)) blocks.push({ kind: 'table', header: cells(table[0]!), rows: table.slice(2).map(cells) })
      else blocks.push({ kind: 'paragraph', text: table.join('\n') })
      table = []
    }
  }
  for (const raw of source.split('\n')) {
    const line = raw.trim()
    const match = /^[ \t]*(`{3,}|~{3,})(.*)$/.exec(raw)
    if (code) {
      if (match && fence && match[1]![0] === fence.marker && match[1]!.length >= fence.length && !match[2]!.trim()) {
        blocks.push({ kind: 'code', text: code.join('\n'), language })
        code = null
        fence = null
      } else code.push(raw.replace(/^[ \t]*/, (indent) => indent.slice(Math.min(indent.length, fence?.indent ?? 0))))
      continue
    }
    if (match && !(match[1]![0] === '`' && match[2]!.includes('`'))) {
      flush()
      code = []
      fence = { marker: match[1]![0]!, length: match[1]!.length, indent: raw.length - raw.trimStart().length }
      language = match[2]!.trim()
      continue
    }
    if (line.startsWith('|')) {
      if (paragraph.length) flush()
      table.push(line)
      continue
    }
    if (table.length) flush()
    const depth = Math.min(Math.floor((/^[ \t]*/.exec(raw)?.[0].length ?? 0) / 2), 3)
    const heading = /^(#+) (.*)$/.exec(line)
    const ordered = /^(\d+\.) (.*)$/.exec(line)
    if (!line) flush()
    else if (line === '---' || line === '***' || line === '___') {
      flush()
      blocks.push({ kind: 'rule' })
    } else if (heading) {
      flush()
      blocks.push({ kind: 'heading', text: heading[2]!.trim(), level: heading[1]!.length })
    } else if (/^[-*+•] /.test(line)) {
      flush()
      blocks.push({ kind: 'bullet', text: line.slice(2), marker: depth > 0 ? '◦' : '•', depth })
    } else if (ordered) {
      flush()
      blocks.push({ kind: 'bullet', text: ordered[2]!, marker: ordered[1]!, depth })
    } else if (line.startsWith('>')) {
      flush()
      blocks.push({ kind: 'quote', text: line.slice(1).trim() })
    } else paragraph.push(raw)
  }
  if (code) blocks.push({ kind: 'code', text: (code as string[]).join('\n'), language })
  flush()
  return blocks
}

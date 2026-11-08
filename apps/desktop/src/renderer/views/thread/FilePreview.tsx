import { createContext, useContext, useEffect, useState } from 'react'
import type { Attachment, SharedFile } from '@shared/models'
import { mediaMime } from '../../../shared/text-preview'
import { useTablePreview } from './use-table-preview'
import { MediaFilePreview } from './MediaFilePreview'
import { sha256Hex } from '../../../shared/files'
import { IconButton } from '../../components/Controls'
import { HStack } from '../../components/Stack'
import { ModalHeader } from '../../components/Overlay'
import { useTr } from '../../lib/use-language'
import { useStore } from '../../store/context'
import { MarkdownText } from './Markdown'
import './sidechat.css'

export type PreviewFile = { entryId: string; botId: string; file: SharedFile } | { botId: string; file: Attachment } | { botId: string; path: string }
export const PreviewContext = createContext<((preview: PreviewFile) => void) | null>(null)
export const useFilePreview = () => useContext(PreviewContext)
export const previewKey = (preview: PreviewFile) => JSON.stringify('path' in preview
  ? [preview.botId, preview.path]
  : 'entryId' in preview ? [preview.botId, preview.entryId, preview.file.id] : [preview.botId, preview.file.id])

export function FilePreview({ preview, close }: { preview: PreviewFile; close: () => void }) {
  if (!('path' in preview) && mediaMime(preview.file)) return <MediaFilePreview preview={preview} close={close} />
  return <TextFilePreview preview={preview} close={close} />
}

function TextFilePreview({ preview, close }: { preview: PreviewFile; close: () => void }) {
  const store = useStore()
  const tr = useTr()
  const [result, setResult] = useState<{ text: string } | { error: string } | null>(null)
  const [attempt, setAttempt] = useState(0)
  const [source, setSource] = useState(false)
  const [wrap, setWrap] = useState(true)
  useEffect(() => {
    let active = true
    setResult(null)
    if ('path' in preview) {
      void store.previewWorkspaceFile(preview.botId, preview.path).then((value) => {
        if (active) setResult({ text: value.text })
      }).catch((error: unknown) => {
        if (active) setResult({ error: error instanceof Error && error.message ? `${tr("Couldn't open this file.")} ${error.message}` : tr("Couldn't open this file.") })
      })
      return () => { active = false }
    }
    if (preview.file.size > 1024 * 1024) {
      setResult({ error: tr('Text previews are limited to 1 MiB. Download the file to open it.') })
      return () => { active = false }
    }
    const dataPromise = 'entryId' in preview ? store.sharedFileData(preview.file, preview.entryId) : store.attachmentData(preview.file.id, preview.botId, preview.file.size)
    void dataPromise.then(async (data) => {
      if (!active) return
      try {
        if (!data || data.byteLength !== preview.file.size || ('entryId' in preview && await sha256Hex(new Uint8Array(data)) !== preview.file.sha256)) throw new Error('invalid')
        if (!active) return
        const text = new TextDecoder('utf-8', { fatal: true }).decode(data)
        if (text.includes('\0')) throw new Error('binary')
        setResult({ text })
      } catch { if (active) setResult({ error: tr("Couldn't open this file. The file must be valid UTF-8 text up to 1 MiB and pass its integrity checks.") }) }
    }).catch(() => { if (active) setResult({ error: tr("Couldn't open this file.") }) })
    return () => { active = false }
  }, [store, preview, attempt])
  const name = 'path' in preview ? preview.path.split(/[\\/]/).pop() || preview.path : preview.file.name
  const markdown = /\.(md|markdown)$/i.test(name)
  const text = result && 'text' in result ? result.text : null
  const table = useTablePreview(name, text)
  const tableText = table.result && 'markdown' in table.result ? table.result.markdown : null
  return <div className="sidechat">
    <ModalHeader title={name} onClose={close} />
    <HStack className="file-preview-toolbar" gap={4}>
      {(markdown || table.tabular) && <button className="md-preview-toggle" aria-pressed={source} onClick={() => setSource(!source)}>{tr(source ? 'Read' : 'Source')}</button>}
      {(!markdown && !table.tabular || source) && <IconButton title={tr(wrap ? 'Disable line wrapping' : 'Wrap lines')} icon="text.alignleft" selected={wrap} onClick={() => setWrap(!wrap)} />}
      <IconButton title="Copy" icon="square.on.square" disabled={text === null} onClick={() => { if (text !== null) window.codync.app.copy(text) }} />
      <IconButton title="Retry" icon="arrow.clockwise" disabled={!result} onClick={() => setAttempt(value => value + 1)} />
    </HStack>
    <div className="sidechat-scroll selectable" aria-busy={!result}>
      {table.tabular && !source && table.result && ('error' in table.result
        ? <div role="status">{table.result.error}</div>
        : table.result.truncated ? <div role="status">Table preview truncated. Source shows the full text.</div> : null)}
      {!result ? tr('Loading') : 'error' in result ? <div role="alert">{result.error}</div> : !source && (markdown || tableText !== null) ? <MarkdownText text={tableText ?? result.text} /> :
        <pre style={{ whiteSpace: wrap ? 'pre-wrap' : 'pre', overflowWrap: wrap ? 'anywhere' : undefined, overflowX: 'auto', margin: 0 }}>{result.text}</pre>}
    </div>
  </div>
}

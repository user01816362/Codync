import { createContext, useContext, useEffect, useState } from 'react'
import type { Attachment, Entry, SharedFile } from '@shared/models'
import { sha256Hex } from '../../../shared/files'
import { ModalHeader } from '../../components/Overlay'
import { useTr } from '../../lib/use-language'
import { useStore } from '../../store/context'
import { MarkdownText } from './Markdown'
import './sidechat.css'

export type PreviewFile = { entry: Entry; file: SharedFile } | { botId: string; file: Attachment } | { botId: string; path: string }
export const PreviewContext = createContext<((preview: PreviewFile) => void) | null>(null)
export const useFilePreview = () => useContext(PreviewContext)
export const previewKey = (preview: PreviewFile) => 'path' in preview ? preview.path : preview.file.id

export function FilePreview({ preview, close }: { preview: PreviewFile; close: () => void }) {
  const store = useStore()
  const tr = useTr()
  const [result, setResult] = useState<{ text: string } | { error: string } | null>(null)
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
    const dataPromise = 'entry' in preview ? store.sharedFileData(preview.file, preview.entry.id) : store.attachmentData(preview.file.id, preview.botId, preview.file.size)
    void dataPromise.then(async (data) => {
      if (!active) return
      try {
        if (!data || data.byteLength !== preview.file.size || ('entry' in preview && await sha256Hex(new Uint8Array(data)) !== preview.file.sha256)) throw new Error('invalid')
        if (!active) return
        const text = new TextDecoder('utf-8', { fatal: true }).decode(data)
        if (text.includes('\0')) throw new Error('binary')
        setResult({ text })
      } catch { setResult({ error: tr("Couldn't open this file.") }) }
    }).catch(() => { if (active) setResult({ error: tr("Couldn't open this file.") }) })
    return () => { active = false }
  }, [store, preview])
  const name = 'path' in preview ? preview.path.split(/[\\/]/).pop() || preview.path : preview.file.name
  const markdown = /\.(md|markdown)$/i.test(name)
  return <div className="sidechat">
    <ModalHeader title={name} onClose={close} />
    <div className="sidechat-scroll selectable">
      {!result ? tr('Loading') : 'error' in result ? result.error : markdown ? <MarkdownText text={result.text} /> :
        <pre style={{ whiteSpace: 'pre', overflowX: 'auto', margin: 0 }}>{result.text}</pre>}
    </div>
  </div>
}

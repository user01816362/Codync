import { createContext, useContext, useEffect, useState } from 'react'
import type { Entry, SharedFile } from '@shared/models'
import { sha256Hex } from '../../../shared/files'
import { ModalHeader } from '../../components/Overlay'
import { useTr } from '../../lib/use-language'
import { useStore } from '../../store/context'
import { MarkdownText } from './Markdown'
import './sidechat.css'

export type PreviewFile = { entry: Entry; file: SharedFile }
export const PreviewContext = createContext<((preview: PreviewFile) => void) | null>(null)
export const useFilePreview = () => useContext(PreviewContext)

export function FilePreview({ preview, close }: { preview: PreviewFile; close: () => void }) {
  const store = useStore()
  const tr = useTr()
  const [result, setResult] = useState<{ text: string } | { error: string } | null>(null)
  useEffect(() => {
    let active = true
    setResult(null)
    void store.sharedFileData(preview.file, preview.entry.id).then(async (data) => {
      if (!active) return
      try {
        if (!data || data.byteLength !== preview.file.size || await sha256Hex(new Uint8Array(data)) !== preview.file.sha256) throw new Error('invalid')
        if (!active) return
        const text = new TextDecoder('utf-8', { fatal: true }).decode(data)
        if (text.includes('\0')) throw new Error('binary')
        setResult({ text })
      } catch { setResult({ error: tr("Couldn't open this file.") }) }
    })
    return () => { active = false }
  }, [store, preview.entry.id, preview.file.id])
  const markdown = /\.(md|markdown)$/i.test(preview.file.name)
  return <div className="sidechat">
    <ModalHeader title={preview.file.name} onClose={close} />
    <div className="sidechat-scroll selectable">
      {!result ? tr('Loading') : 'error' in result ? result.error : markdown ? <MarkdownText text={result.text} /> :
        <pre style={{ whiteSpace: 'pre', overflowX: 'auto', margin: 0 }}>{result.text}</pre>}
    </div>
  </div>
}

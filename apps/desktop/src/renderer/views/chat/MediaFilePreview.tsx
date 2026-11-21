import { useEffect, useState } from 'react'
import { mediaMime } from '../../../shared/text-preview'
import { sha256Hex } from '../../../shared/files'
import { IconButton } from '../../components/Controls'
import { ModalHeader } from '../../components/Overlay'
import { useStore } from '../../store/context'
import type { PreviewFile } from './FilePreview'

/** Verified local bytes only; browser media decoders never receive an agent-supplied URL. */
export function MediaFilePreview({ preview, close }: { preview: Exclude<PreviewFile, { path: string }>; close: () => void }) {
  const store = useStore()
  const [url, setURL] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [attempt, setAttempt] = useState(0)
  const mime = mediaMime(preview.file)
  useEffect(() => {
    let active = true
    let objectURL: string | null = null
    setURL(null)
    setError(null)
    void (async () => {
      try {
        if (!mime) throw new Error('This file type or size cannot be previewed.')
        const bytes = 'entryId' in preview
          ? await store.sharedFileData(preview.file, preview.entryId)
          : await store.attachmentData(preview.file.id, preview.botId, preview.file.size)
        if (!bytes || bytes.byteLength !== preview.file.size) throw new Error('The file size could not be verified.')
        if ('entryId' in preview && await sha256Hex(new Uint8Array(bytes)) !== preview.file.sha256) throw new Error('The file integrity check failed.')
        if (mime === 'application/pdf' && new TextDecoder().decode(bytes.slice(0, 5)) !== '%PDF-') throw new Error('This file is not a PDF document.')
        if (!active) return
        objectURL = URL.createObjectURL(new Blob([new Uint8Array(bytes)], { type: mime }))
        setURL(objectURL)
      } catch (error) { if (active) setError(error instanceof Error ? error.message : String(error)) }
    })()
    return () => { active = false; if (objectURL) URL.revokeObjectURL(objectURL) }
  }, [store, preview, mime, attempt])
  return <div className="sidechat">
    <ModalHeader title={preview.file.name} onClose={close} trailing={<IconButton title="Retry" icon="arrow.clockwise" disabled={!url && !error} onClick={() => setAttempt(value => value + 1)} />} />
    {error ? <div className="sidechat-scroll" role="alert">{error} Use the file card to download it.</div> : !url ? <div className="sidechat-scroll" role="status">Loading preview…</div> : mime === 'application/pdf'
      ? <iframe title={preview.file.name} src={url} referrerPolicy="no-referrer" style={{ width: '100%', flex: 1, border: 0, minHeight: 0 }} />
      : mime?.startsWith('audio/') ? <audio controls preload="metadata" src={url} style={{ width: '100%' }} onError={() => setError('This audio format could not be played.')} />
        : <video controls playsInline preload="metadata" src={url} style={{ width: '100%', minHeight: 0 }} onError={() => setError('This video format could not be played.')} />}
  </div>
}

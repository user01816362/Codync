import { useEffect, useMemo, useRef, useState, type ReactNode } from 'react'
import { artifactFrameURL } from '@shared/artifacts'
import { ModalHeader, Sheet } from '../../components/Overlay'
import { IconButton } from '../../components/Controls'
import template from './rich-renderer.html?raw'

/** Markdown stays native. Trusted browser renderers get their own origin, rather than inheriting the app's CSP. */
export function RichContentPreview({ source, language, renderMarkdown }: { source: string; language: string; renderMarkdown?: (source: string) => ReactNode }) {
  const frame = useRef<HTMLIFrameElement>(null)
  const [measuredHeight, setMeasuredHeight] = useState(48)
  const inlineMath = language === 'inline-math'
  useEffect(() => {
    const measure = (event: MessageEvent) => {
      if (!inlineMath || event.source !== frame.current?.contentWindow || event.data?.kind !== 'codync-layout') return
      const height = event.data.height
      if (typeof height === 'number' && Number.isFinite(height) && height >= 24 && height <= 1200) setMeasuredHeight(Math.ceil(height))
    }
    window.addEventListener('message', measure)
    return () => window.removeEventListener('message', measure)
  }, [inlineMath])
  const markdown = ['md', 'markdown'].includes(language) && !!renderMarkdown
  const defaultPreview = markdown || ['mermaid', 'math', 'latex', 'tex', 'inline-math'].includes(language)
  const [expanded, setExpanded] = useState(false)
  const [choice, setChoice] = useState<boolean | null>(null)
  const [id] = useState(() => crypto.randomUUID())
  const [revision, setRevision] = useState(0)
  const [shown, setShown] = useState<string | null>(null)
  const [failed, setFailed] = useState(false)
  const canPreview = !!source && (markdown || (source.length <= 16000 && !['md', 'markdown'].includes(language)))
  const preview = canPreview && (choice ?? defaultPreview)
  const html = useMemo(() => {
    if (!preview || markdown) return ''
    const kind = inlineMath ? 'inline-math' : language === 'mermaid' ? 'mermaid' : ['math', 'latex', 'tex'].includes(language) ? 'math' : 'code'
    const bytes = new TextEncoder().encode(JSON.stringify({ kind, source, language, fontSize: 14 }))
    return template.replace('__CODYNC_PAYLOAD__', btoa(Array.from(bytes, byte => String.fromCharCode(byte)).join('')))
  }, [preview, markdown, source, language, inlineMath])
  useEffect(() => {
    let current = true
    setFailed(false)
    if (!html) { setShown(null); return }
    void window.codync.artifacts.show(id, new TextEncoder().encode(html)).then(
      () => { if (current) { setRevision(value => value + 1); setShown(html) } },
      () => { if (current) setFailed(true) },
    )
    return () => { current = false }
  }, [id, html])
  return <div className={inlineMath ? 'md-inline-math' : 'md-code'}>
    {!inlineMath && <div className="md-code-header">
      <span>{language || 'text'}</span>
      <div className="md-code-actions">
        {canPreview && <button className="md-preview-toggle" aria-label={preview ? 'Show code' : 'Show preview'} onClick={() => setChoice(!preview)}>{preview ? 'Code' : 'Preview'}</button>}
        {preview && !markdown && shown === html && !failed && <IconButton title="Expand preview" icon="arrow.up.left.and.arrow.down.right" size={26} onClick={() => setExpanded(true)} />}
        <IconButton title="Copy" icon="square.on.square" size={26} onClick={() => window.codync.app.copy(source)} />
      </div>
    </div>}
    {!preview ? <div className="md-code-scroll"><pre className="md-code-text">{source}</pre></div> : markdown ? <div className="md-readable-preview">{renderMarkdown?.(source)}</div> :
      failed ? <div className="md-readable-preview">{inlineMath ? source : 'Preview couldn’t load. Choose Code to view the source.'}</div> : shown !== html ? <div className="md-readable-preview" role="status">Loading preview…</div> :
        <iframe ref={frame} className="md-rendered-frame" title={`${language || 'Code'} preview`} sandbox="allow-scripts" referrerPolicy="no-referrer" style={inlineMath ? { height: measuredHeight } : ['mermaid', 'math', 'latex', 'tex'].includes(language) ? undefined : { height: Math.min(360, Math.max(64, source.split('\n').length * 18 + 20)) }} src={artifactFrameURL(id, revision, true)} />}
    <Sheet open={expanded} onClose={() => setExpanded(false)} width={960} height={760}>
      <div className="sidechat">
        <ModalHeader title={`${language || 'Code'} preview`} trailing={<IconButton title="Copy" icon="square.on.square" onClick={() => window.codync.app.copy(source)} />} />
        {shown === html && <iframe title={`${language || 'Code'} expanded preview`} sandbox="allow-scripts" referrerPolicy="no-referrer" style={{ width: '100%', flex: 1, minHeight: 0, border: 0 }} src={artifactFrameURL(id, revision, true)} />}
      </div>
    </Sheet>
  </div>
}

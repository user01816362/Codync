import { useEffect, useMemo, useState, type ReactNode } from 'react'
import { artifactFrameURL } from '@shared/artifacts'
import { IconButton } from '../../components/Controls'
import template from './rich-renderer.html?raw'

/** Markdown stays native. Trusted browser renderers get their own origin, rather than inheriting the app's CSP. */
export function RichContentPreview({ source, language, renderMarkdown }: { source: string; language: string; renderMarkdown?: (source: string) => ReactNode }) {
  const markdown = ['md', 'markdown'].includes(language) && !!renderMarkdown
  const defaultPreview = markdown || ['mermaid', 'math', 'latex', 'tex'].includes(language)
  const [choice, setChoice] = useState<boolean | null>(null)
  const [id] = useState(() => crypto.randomUUID())
  const [revision, setRevision] = useState(0)
  const [shown, setShown] = useState<string | null>(null)
  const [failed, setFailed] = useState(false)
  const canPreview = !!source && (markdown || (source.length <= 16000 && !['md', 'markdown'].includes(language)))
  const preview = canPreview && (choice ?? defaultPreview)
  const html = useMemo(() => {
    if (!preview || markdown) return ''
    const kind = language === 'mermaid' ? 'mermaid' : ['math', 'latex', 'tex'].includes(language) ? 'math' : 'code'
    const bytes = new TextEncoder().encode(JSON.stringify({ kind, source, language }))
    return template.replace('__CODYNC_PAYLOAD__', btoa(Array.from(bytes, byte => String.fromCharCode(byte)).join('')))
  }, [preview, markdown, source, language])
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
  return <div className="md-code">
    <div className="md-code-header">
      <span>{language || 'text'}</span>
      <div className="md-code-actions">
        {canPreview && <button className="md-preview-toggle" aria-label={preview ? 'Show code' : 'Show preview'} onClick={() => setChoice(!preview)}>{preview ? 'Code' : 'Preview'}</button>}
        <IconButton title="Copy" icon="square.on.square" size={26} onClick={() => window.codync.app.copy(source)} />
      </div>
    </div>
    {!preview ? <div className="md-code-scroll"><pre className="md-code-text">{source}</pre></div> : markdown ? <div className="md-readable-preview">{renderMarkdown?.(source)}</div> :
      failed ? <div className="md-readable-preview">Preview couldn’t load. Choose Code to view the source.</div> : shown !== html ? <div className="md-readable-preview" role="status">Loading preview…</div> :
        <iframe className="md-rendered-frame" title={`${language || 'Code'} preview`} sandbox="allow-scripts" referrerPolicy="no-referrer" style={['mermaid', 'math', 'latex', 'tex'].includes(language) ? undefined : { height: Math.min(360, Math.max(64, source.split('\n').length * 18 + 20)) }} src={artifactFrameURL(id, revision, true)} />}
  </div>
}

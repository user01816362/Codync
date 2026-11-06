import { Button } from '../../components/Controls'
import { useState } from 'react'
import type { RichReport } from '@shared/models'

/** Host-validated, inert native content. The message text remains the copy/old-client fallback. */
export function RichReportView({ report, submit }: { report: RichReport; submit: (text: string) => void }) {
  const [answers, setAnswers] = useState<Record<string, string>>({})
  const [sent, setSent] = useState<Set<number>>(new Set())
  const [closed, setClosed] = useState<Set<number>>(new Set())
  return <div className="rich-report">
    <strong>{report.title}</strong>
    {report.sections.slice(0, 24).map((section, index) => {
      const items = section.items?.slice(0, 30) ?? []
      const maximum = Math.max(0, ...items.map(item => Number(item.value)).filter(Number.isFinite))
      return <section key={index}>
        <button className="rich-report-heading" aria-expanded={!closed.has(index)} onClick={() => setClosed(current => {
          const next = new Set(current)
          if (next.has(index)) next.delete(index)
          else next.add(index)
          return next
        })}>{closed.has(index) ? '▸' : '▾'} {section.heading}</button>
        {!closed.has(index) && <div className="rich-report-content">
          {section.text && <p>{section.text}</p>}
          {items.map((item, i) => <div key={i} className={`rich-report-item rich-report-${section.kind}`}>
            <span>{item.label}</span>{section.kind === 'form' ? <input aria-label={item.label} placeholder={item.value} maxLength={500} disabled={sent.has(index)} value={answers[`${index}:${i}`] ?? ''} onChange={event => setAnswers(current => ({ ...current, [`${index}:${i}`]: event.target.value }))} /> : <span>{item.value}</span>}
            {section.kind === 'bar' && <meter min={0} max={maximum || 1} value={Math.max(0, Number(item.value) || 0)} aria-label={`${item.label}: ${item.value}`} />}
          </div>)}
          {section.kind === 'form' && <Button disabled={sent.has(index) || items.some((_, i) => !answers[`${index}:${i}`]?.trim())} onClick={() => {
            submit(`${report.title} — ${section.heading}\n${items.map((item, i) => `${item.label}: ${answers[`${index}:${i}`] ?? ''}`).join('\n')}`)
            setSent(current => new Set([...current, index]))
          }}>{sent.has(index) ? 'Sent' : 'Send answers'}</Button>}
          {!!section.citations?.length && <p>{section.citations.map(citation => {
            const source = report.sources?.[citation - 1]
            return source && /^https?:\/\//i.test(source.url) ? <a key={citation} href={source.url} onClick={event => { event.preventDefault(); window.codync.app.openExternal(source.url) }}>[{citation}] {source.title} </a> : null
          })}</p>}
        </div>}
      </section>
    })}
    {!!report.sources?.length && <details><summary>Sources supplied by the agent</summary>{report.sources.slice(0, 30).map((source, i) => /^https?:\/\//i.test(source.url) ? <p key={i}><a href={source.url} onClick={event => { event.preventDefault(); window.codync.app.openExternal(source.url) }}>{i + 1}. {source.title}</a></p> : null)}</details>}
  </div>
}

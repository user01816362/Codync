import { useEffect, useState } from 'react'
import { useStore } from '../../store/context'

export function useTablePreview(name: string, text: string | null) {
  const store = useStore()
  const [result, setResult] = useState<{ markdown: string; truncated: boolean } | { error: string } | null>(null)
  const tabular = /\.(csv|tsv)$/i.test(name)
  useEffect(() => {
    let active = true
    setResult(null)
    if (!tabular || text === null) return
    if (new TextEncoder().encode(text).length > 65536) {
      setResult({ error: 'Table previews are limited to 64 KiB. Source shows the full text.' })
      return
    }
    if (!store.client) { setResult({ error: 'Connect to the computer for a table preview. Source remains available.' }); return }
    void store.client.call<{ markdown: string; truncated: boolean }>('formatTable', { name, text }).then(
      result => { if (active) setResult(result) },
      (error: unknown) => { if (active) setResult({ error: error instanceof Error ? error.message : String(error) }) },
    )
    return () => { active = false }
  }, [store.client, name, text, tabular])
  return { tabular, result }
}

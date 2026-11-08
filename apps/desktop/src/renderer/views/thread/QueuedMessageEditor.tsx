import { useState } from 'react'
import type { Entry } from '@shared/models'
import { Button, Spinner } from '../../components/Controls'
import { ModalHeader, useDismiss } from '../../components/Overlay'
import { VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useStore } from '../../store/context'

/** Keeps the edit intent distinct from editing a message that has already been dispatched. */
export function QueuedMessageEditor({ entry }: { entry: Entry }) {
  const store = useStore()
  const dismiss = useDismiss()
  const [text, setText] = useState(entry.data.text ?? '')
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const save = async () => {
    if (!store.client || saving) return
    setSaving(true)
    setError(null)
    try {
      await store.client.call('editQueued', { botId: entry.botId, entryId: entry.id, text })
      dismiss()
    } catch (error) { setError(error instanceof Error ? error.message : String(error)) }
    finally { setSaving(false) }
  }
  return <VStack gap={16}>
    <ModalHeader title="Edit queued message" />
    <VStack gap={12} style={{ padding: '0 20px 20px' }}>
      <Text tone="secondary" size="caption">Changes apply only while the message is waiting. Files and reply context stay attached.</Text>
      <textarea className="field-box" aria-label="Message" rows={6} value={text} onChange={event => setText(event.target.value)} disabled={saving} />
      {error && <Text tone="danger" role="alert">{error}</Text>}
      {saving ? <Spinner /> : <Button disabled={!text.trim() || !store.client || new TextEncoder().encode(text).length > 65536} onClick={() => void save()}>Save</Button>}
    </VStack>
  </VStack>
}

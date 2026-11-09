import { useEffect, useState } from 'react'
import { Button, IconButton, Spinner } from '../../components/Controls'
import { Dialog, ModalHeader } from '../../components/Overlay'
import { VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useStore } from '../../store/context'
import { byteCount } from '../thread/Attachments'
interface Category { count: number; bytes: number; truncated: boolean; unavailable: number }
interface Usage { databaseBytes: number; reusableBytes: number; entries: number; sharedFiles: Category; uploads: Category; artifacts: Category }
export function StorageOverview() {
  const store = useStore()
  const [value, setValue] = useState<Usage | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const [confirm, setConfirm] = useState(false)
  const [attempt, setAttempt] = useState(0)
  const load = async (compact = false) => {
    if (!store.client || busy) { if (!store.client) setError('Computer is offline'); return }
    setBusy(true); setError(null)
    try { setValue(await store.client.call<Usage>(compact ? 'compactStorage' : 'storageOverview', { confirm: compact })) }
    catch (error: unknown) { setError(String(error)) }
    finally { setBusy(false) }
  }
  useEffect(() => { void load() }, [store.client, attempt])
  return <div className="sidechat"><ModalHeader title="Host storage" trailing={<IconButton title="Refresh storage" icon="arrow.clockwise" disabled={busy} onClick={() => setAttempt(value => value + 1)} />} />
    <div className="sidechat-scroll"><VStack gap={20}>
      <Text size="caption" tone="secondary">Stored on the connected computer. File sizes are a bounded scan of managed file roots; workspace project files, logs, caches and database journals are excluded.</Text>
      {error && <Text role="alert" tone="danger">{error}</Text>}{busy && <Spinner />}
      {value && <><Text>Conversation database: {byteCount(value.databaseBytes)} · {value.entries.toLocaleString()} entries</Text><Text>Reusable database pages: {byteCount(value.reusableBytes)}</Text>
        {(['sharedFiles', 'uploads', 'artifacts'] as const).map(key => <VStack key={key} gap={4}><Text>{({ sharedFiles: 'Shared files', uploads: 'Uploads', artifacts: 'Artifacts' })[key]}: {byteCount(value[key].bytes)} · {value[key].count} files</Text>{(value[key].truncated || value[key].unavailable > 0) && <Text size="caption" tone="secondary">Partial scan · {value[key].unavailable} unavailable paths</Text>}</VStack>)}
        <Button disabled={busy || value.reusableBytes === 0} onClick={() => setConfirm(true)}>Compact database</Button></>}
    </VStack></div>
    <Dialog open={confirm} title="Compact database?" message="This preserves saved messages and files. SQLite may need up to twice the database size in temporary free space and briefly blocks host database operations. File caches are not deleted." onClose={() => setConfirm(false)} actions={[{ title: 'Compact database', action: () => { setConfirm(false); void load(true) } }]} />
  </div>
}

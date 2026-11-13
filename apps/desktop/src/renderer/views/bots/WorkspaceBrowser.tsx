import { useEffect, useState } from 'react'
import { canPreview } from '../../../shared/text-preview'
import { IconButton, Spinner, Switch } from '../../components/Controls'
import { Icon } from '../../components/Icon'
import { ModalHeader } from '../../components/Overlay'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useStore } from '../../store/context'
import { byteCount } from '../thread/file-card'

interface Directory {
  botId: string
  path: string
  parent: string | null
  truncated: boolean
  entries: { name: string; path: string; kind: string; size: number }[]
}

/** Reads only registered workspace directories; opening a file keeps the same host confinement. */
export function WorkspaceBrowser({ botId, open }: { botId: string; open: (botId: string, path: string) => void }) {
  const store = useStore()
  const [path, setPath] = useState('.')
  const [hidden, setHidden] = useState(false)
  const [attempt, setAttempt] = useState(0)
  const [directory, setDirectory] = useState<Directory | null>(null)
  const [error, setError] = useState<string | null>(null)
  useEffect(() => {
    let active = true
    setDirectory(null)
    setError(null)
    if (!store.client) { setError('Computer is offline'); return }
    void store.client.call<Directory>('listWorkspaceFiles', { botId, path, showHidden: hidden }).then(
      value => { if (active) setDirectory(value) },
      (error: unknown) => { if (active) setError(error instanceof Error ? error.message : String(error)) },
    )
    return () => { active = false }
  }, [store.client, botId, path, hidden, attempt])
  return <div className="sidechat">
    <ModalHeader title="Workspace files" subtitle={store.bots.get(botId)?.name} />
    <HStack gap={8} style={{ padding: '0 20px 12px' }}>
      <IconButton title="Workspace root" icon="house" disabled={path === '.'} onClick={() => setPath('.')} />
      <IconButton title="Parent folder" icon="arrow.up" disabled={!directory?.parent} onClick={() => { if (directory?.parent) setPath(directory.parent) }} />
      <Text size="caption" grow style={{ overflowWrap: 'anywhere' }}>{directory?.path || path}</Text>
      <IconButton title="Retry" icon="arrow.clockwise" disabled={!directory && !error} onClick={() => setAttempt(value => value + 1)} />
    </HStack>
    <HStack style={{ padding: '0 20px' }}><Text size="caption" grow>Hidden files</Text><Switch label="Hidden files" on={hidden} onChange={setHidden} /></HStack>
    <div className="sidechat-scroll">
      {error ? <Text role="alert" tone="danger">{error}</Text> : !directory ? <Spinner /> : <VStack gap={6}>
        {directory.entries.length === 0 && <Text tone="secondary">This folder is empty.</Text>}
        {directory.truncated && <Text size="caption" tone="secondary">Listing limited to 500 items from at most 2,000 entries.</Text>}
        {directory.entries.map(entry => {
          const folder = entry.kind === 'directory'
          const readable = entry.kind === 'file' && canPreview({ id: entry.path, ...entry })
          return <button key={entry.path} className="member-row press" disabled={!folder && !readable} onClick={() => folder ? setPath(entry.path) : open(directory.botId, entry.path)}>
            <Icon name={folder ? 'folder' : 'doc.text'} size={18} />
            <VStack grow gap={2}>
              <Text size="callout" truncate title={entry.name}>{entry.name}</Text>
              <Text size="caption" tone="secondary">{folder ? 'Folder' : readable ? byteCount(entry.size) : entry.kind === 'link' ? 'Symbolic link' : 'Ask the bot to share this file for preview.'}</Text>
            </VStack>
          </button>
        })}
      </VStack>}
    </div>
  </div>
}

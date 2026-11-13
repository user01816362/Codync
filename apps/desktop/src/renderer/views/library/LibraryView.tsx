import { useEffect, useState, type ReactNode } from 'react'
import type { Artifact, LibraryItem } from '@shared/artifacts'
import { isImageName, type Attachment, type SharedFile } from '@shared/models'
import { Button, DropdownMenu, IconButton, Spinner, TabChoice, SearchField } from '../../components/Controls'
import { Icon } from '../../components/Icon'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { libraryGroups } from '../../lib/library-groups'
import { useTr } from '../../lib/use-language'
import { useScreenViewed } from '../../store/analytics'
import { useStore } from '../../store/context'
import { AttachmentImage, AttachmentViewer, attachmentSymbol } from '../thread/Attachments'
import { byteCount, downloadAction, downloadDetail } from '../thread/file-card'
import { useFilePreview } from '../thread/FilePreview'
import { canOpenPreview } from '../../../shared/text-preview'
import { artifactShareItems, renameArtifact, TitleField } from './ArtifactParts'

/**
 * A chat's Library in its details panel (Grok Bot's): the artifacts its bot made and the files shared in it, newest
 * first, by day. Each opens the way it does in the chat.
 */
export function LibraryView({ botId, openArtifact }: { botId: string; openArtifact: (artifactId: string) => void }) {
  const tr = useTr()
  const store = useStore()
  const [nextBefore, setNextBefore] = useState<number | null>(null)
  const [query, setQuery] = useState('')
  const [items, setItems] = useState<LibraryItem[]>([])
  const [error, setError] = useState<string | null>(null)
  const [viewing, setViewing] = useState<Attachment | null>(null)
  const [loading, setLoading] = useState(true)
  const [attempt, setAttempt] = useState(0)
  const [filter, setFilter] = useState<'all' | 'files'>('all')
  useScreenViewed('library')
  // Read again when the chat gets an artifact card or a file.
  const shared = store.allEntries(botId).filter((e) => e.data.artifact || e.data.files?.length || e.data.attachments?.length).length
  useEffect(() => {
    let live = true
    setLoading(true)
    setError(null)
    void (store.client ? store.client.call<{ items: LibraryItem[]; nextBefore: number | null }>('libraryPage', { botId }).then(value => { setNextBefore(value.nextBefore); for (const item of value.items) if (item.kind === 'artifact') store.artifacts.upsert(item.artifact); return value.items }) : Promise.reject(new Error('Computer is offline'))).then(
      (items) => live && (setItems(items), setError(null)),
      (e: unknown) => live && setError(e instanceof Error ? e.message : String(e)),
    ).finally(() => { if (live) setLoading(false) })
    return () => {
      live = false
    }
  }, [store, store.client, botId, shared, attempt])
  const older = async () => {
    if (!store.client || nextBefore === null || loading) return
    setLoading(true)
    try {
      const page = await store.client.call<{ items: LibraryItem[]; nextBefore: number | null }>('libraryPage', { botId, beforeSeq: nextBefore })
      setItems(items => [...items, ...page.items]); setNextBefore(page.nextBefore); setError(null)
    } catch (error: unknown) { setError(String(error)) }
    finally { setLoading(false) }
  }
  const show = (entryId: string) => { void store.revealMessage(botId, entryId).catch((error: unknown) => setError(String(error))) }
  const visible = (filter === 'files' ? items.filter(item => item.kind !== 'artifact') : items).filter(item => (item.kind === 'artifact' ? item.artifact.title : item.file.name).toLocaleLowerCase().includes(query.trim().toLocaleLowerCase()))
  return (
    <VStack gap={20}>
      <HStack gap={8}>
        <TabChoice label="Library items" selection={filter} options={[{ id: 'all', label: 'All' }, { id: 'files', label: 'Files' }]} onChange={setFilter} />
        <Text size="caption" tone="secondary" grow>{items.filter(item => item.kind !== 'artifact').length} {tr('files')}</Text>
        <IconButton title="Retry" icon="arrow.clockwise" disabled={loading} onClick={() => setAttempt(value => value + 1)} />
      </HStack>
      <SearchField value={query} onChange={setQuery} prompt="Find files and artifacts" />
      {error ? <Text size="caption" tone="danger" role="alert">{error}</Text> : null}
      {loading && items.length === 0 ? <Spinner /> : !error && visible.length === 0 ? (
        <VStack gap={4} className="outlined-card library-empty">
          <Text size="callout" weight="semibold">{tr(filter === 'files' ? 'No files shared in this chat' : 'Library is empty')}</Text>
          <Text size="callout" tone="secondary">{tr('Artifacts {name} makes and files shared in this chat show here.').replace('{name}', store.bots.get(botId)?.name ?? '')}</Text>
        </VStack>
      ) : (
        libraryGroups(visible).map(({ group, items }) => (
          <section key={group} className="details-section">
            <Text className="details-section-title">{tr(group)}</Text>
            <VStack gap={6}>
              {items.map((item) =>
                item.kind === 'artifact' ? (
                  <ArtifactRow key={item.artifact.id} artifact={store.artifacts.byId.get(item.artifact.id) ?? item.artifact} open={() => openArtifact(item.artifact.id)} onError={setError} />
                ) : item.kind === 'file' ? (
                  <FileRow key={item.file.id} botId={botId} entryId={item.entryId} file={item.file} at={item.at} show={() => show(item.entryId)} />
                ) : (
                  <AttachmentRow key={item.file.id} botId={botId} entryId={item.entryId} file={item.file} at={item.at} show={() => show(item.entryId)} view={() => setViewing(item.file)} />
                ),
              )}
            </VStack>
          </section>
        ))
      )}
      {nextBefore !== null && <Button disabled={loading} onClick={() => void older()}>Load older files</Button>}
      <AttachmentViewer source={viewing && { file: viewing, botId }} close={() => setViewing(null)} />
    </VStack>
  )
}

/** One line of the Library: what it is, its name, and what can be done with it. */
function Row({ leading, children, trailing }: { leading: ReactNode; children: ReactNode; trailing?: ReactNode }) {
  return (
    <HStack gap={4} className="outlined-card library-row">
      <HStack gap={10} grow className="library-row-main">
        <span className="library-row-icon">{leading}</span>
        {children}
      </HStack>
      {trailing}
    </HStack>
  )
}

function ArtifactRow({ artifact, open, onError }: { artifact: Artifact; open: () => void; onError: (message: string | null) => void }) {
  const tr = useTr()
  const store = useStore()
  const [renaming, setRenaming] = useState(false)
  const menu = (
    <DropdownMenu
      className="icon-button library-row-menu"
      title={tr('More actions')}
      items={() => [
        { title: 'Open', icon: 'doc.text', action: open },
        { title: 'Rename artifact', icon: 'pencil', action: () => setRenaming(true) },
        ...artifactShareItems(store, artifact, onError),
      ]}
    >
      <Icon name="ellipsis" size={14} weight="medium" />
    </DropdownMenu>
  )
  const icon = <Icon name="chart.bar.doc.horizontal" size={16} color="var(--secondary)" />
  if (renaming) {
    return (
      <Row leading={icon} trailing={menu}>
        <TitleField
          initial={artifact.title}
          className="library-row-field"
          done={(title) => {
            setRenaming(false)
            renameArtifact(store, artifact, title, onError)
          }}
        />
      </Row>
    )
  }
  return (
    <Row leading={icon} trailing={menu}>
      <button className="library-row-open" onClick={open}>
        <Name name={artifact.title} detail={tr('Artifact')} />
      </button>
    </Row>
  )
}

/** A file the bot shared: saved like its card in the chat. */
function FileRow({ botId, entryId, file, at, show }: { botId: string; entryId: string; file: SharedFile; at: number; show: () => void }) {
  const openPreview = useFilePreview()
  const store = useStore()
  const state = store.fileDownloads.states.get(file.id)
  const busy = state?.kind === 'downloading'
  const detail = `${store.bots.get(botId)?.name ?? ''} · ${downloadDetail(state, file)} · ${new Date(at).toLocaleString()}`
  return (
    <Row
      leading={<Icon name={attachmentSymbol(file.name)} size={16} color="var(--secondary)" />}
      trailing={<><IconButton title="Show in chat" icon="bubble.left" onClick={show} /><IconButton title={downloadAction(state)} icon={busy ? 'xmark' : 'arrow.down.to.line'} onClick={() => (busy ? store.fileDownloads.cancel(file.id) : void store.fileDownloads.save(store.client, entryId, file))} /></>}
    >
      {openPreview && canOpenPreview(file) ? <button className="library-row-open" onClick={() => openPreview({ botId, entryId, file })}>
        <Name name={file.name} detail={detail} failed={state?.kind === 'failed'} />
      </button> : <Name name={file.name} detail={detail} failed={state?.kind === 'failed'} />}
    </Row>
  )
}

/** A file the user sent: an image opens full size, like in the chat. */
function AttachmentRow({ botId, entryId, file, view, at, show }: { botId: string; entryId: string; file: Attachment; view: () => void; at: number; show: () => void }) {
  const openPreview = useFilePreview()
  if (!isImageName(file.name)) {
    return (
      <Row leading={<Icon name={attachmentSymbol(file.name)} size={16} color="var(--secondary)" />} trailing={<IconButton title="Show in chat" icon="bubble.left" onClick={show} />}>
        {openPreview && canOpenPreview(file) ? <button className="library-row-open" onClick={() => openPreview({ botId, file, uploadEntryId: entryId })}>
          <Name name={file.name} detail={`You · ${byteCount(file.size)} · ${new Date(at).toLocaleString()}`} />
        </button> : <Name name={file.name} detail={`You · ${byteCount(file.size)} · ${new Date(at).toLocaleString()}`} />}
      </Row>
    )
  }
  return (
    <Row leading={<AttachmentImage source={{ file, botId }} fit="icon" />} trailing={<IconButton title="Show in chat" icon="bubble.left" onClick={show} />}>
      <button className="library-row-open" onClick={view}>
        <Name name={file.name} detail={`You · ${byteCount(file.size)} · ${new Date(at).toLocaleString()}`} />
      </button>
    </Row>
  )
}

function Name({ name, detail, failed = false }: { name: string; detail: string; failed?: boolean }) {
  return (
    <VStack gap={1} shrink>
      <Text size="callout" truncate title={name}>{name}</Text>
      <Text size="caption2" tone={failed ? 'danger' : 'tertiary'} truncate>{detail}</Text>
    </VStack>
  )
}

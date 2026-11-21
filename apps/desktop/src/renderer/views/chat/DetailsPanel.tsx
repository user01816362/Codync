import { MeetingRoom } from '../bots/MeetingRoom'
import { useState } from 'react'
import { Sheet } from '../../components/Overlay'
import { StorageOverview } from '../bots/StorageOverview'
import { GroupTasks } from '../bots/GroupTasks'
import { GroupActionRow } from '../bots/GroupActionRow'
import { GroupInbox } from '../bots/GroupInbox'
import { WorkspaceBrowser } from '../bots/WorkspaceBrowser'
import { useFilePreview } from './FilePreview'
import { hasLibrary } from '@shared/artifacts'
import { isGroup, isWorking, type Bot } from '@shared/models'
import { AgentIcon } from '../../components/AgentIcon'
import { BotAvatar, BotOrGroupAvatar } from '../../components/Avatar'
import { CardSection, IconButton, TabChoice } from '../../components/Controls'
import { LeadAgentBadge } from '../../components/LeadAgentBadge'
import { Icon } from '../../components/Icon'
import { VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useStore } from '../../store/context'
import { useScreenViewed } from '../../store/analytics'
import { BotSettingsPanel } from '../bots/BotEditorView'
import { MemoryCard } from '../bots/MemoryView'
import { RoutinesView } from '../routines/RoutinesView'
import { LibraryView } from '../library/LibraryView'
import { DevServersSection } from './DevServersSection'
import { ComputerTab } from './ComputerTab'
import { WorkThread } from './WorkThread'
import { WorkTimeline } from './WorkTimeline'
import { MacChatButton } from './MacChatButton'
import { useThreadPage } from './use-thread-page'
import '../bots/bots.css'
import './details-panel.css'
import { useTr } from '../../lib/use-language'


export type DetailsTab = 'details' | 'computer' | 'library' | 'memory' | 'settings'

const TABS: { id: DetailsTab; label: string }[] = [
  { id: 'details', label: 'Details' },
  { id: 'computer', label: 'Computer' },
  { id: 'library', label: 'Library' },
  { id: 'memory', label: 'Memory' },
  { id: 'settings', label: 'Settings' },
]

/** The inspector beside a conversation (Grok Bot's): who the bot is, then what it does (Details), its computer, library,
 * memory and how it is set up (Settings). A row of a task a thread has slides the panel to that task's thread
 * (`thread`); its Back returns to the page as it was left. */
export function DetailsPanel({ botId, routineId, tab, setTab, thread, openThread, closeThread, editGroup, openArtifact, openCard, showInChat, share, close }: {
  botId: string
  routineId: string | null
  tab: DetailsTab
  setTab: (tab: DetailsTab) => void
  /** The task whose thread shows in place of the pages; null for the pages. */
  thread: string | null
  openThread: (taskId: string) => void
  closeThread: () => void
  editGroup: () => void
  openArtifact: (artifactId: string) => void
  /** Shows the chat at the approval card of a task that needs the user. */
  openCard: (taskId: string) => void
  /** Shows the chat at one of its entries: what a thread put there. */
  showInChat: (entryId: string) => void
  share: () => void
  close: () => void
}) {
  const tr = useTr()
  const [storage, setStorage] = useState(false)
  const [meeting, setMeeting] = useState(false)
  const [tasks, setTasks] = useState(false)
  const [inbox, setInbox] = useState(false)
  const [browsingWorkspace, setBrowsingWorkspace] = useState(false)
  const preview = useFilePreview()
  const store = useStore()
  const bot = store.bots.get(botId) ?? null
  const group = !!bot && isGroup(bot)
  useScreenViewed('bot_details')
  const { reading, page, shown, opening } = useThreadPage(thread, bot, closeThread)
  return (
    <div className="details-panel">
      <div className={reading ? 'details-page away' : 'details-page'} inert={reading}>
        <div className="details-drag" />
        <div className="details-actions">
          {group ? <MacChatButton className="ringed" title="Edit group" icon="person.2" onClick={editGroup} /> : null}
          {bot && !group ? <MacChatButton className="ringed" title="Create template" icon="square.and.arrow.up" onClick={share} /> : null}
          <MacChatButton className="ringed" title="Close details" icon="xmark" onClick={close} />
        </div>
        {/* The column is the sheet's scroller, so it takes the focus when the sheet opens: the card around it
            never scrolls, and a first PageDown aimed at the card would move nothing. */}
        <div className="details-scroll" tabIndex={-1} data-autofocus>
          {bot ? <Identity bot={bot} /> : null}
          {bot && group ? (
            <div className="group-details-content">
              <CardSection title="Group activity">
                {bot.meeting ? <GroupActionRow title="Meeting room" icon="person.2" onClick={() => setMeeting(true)} /> : null}
                <GroupActionRow title="Group tasks" icon="checklist" onClick={() => setTasks(true)} />
                <GroupActionRow title="Group inbox" icon="tray" onClick={() => setInbox(true)} />
                <GroupActionRow title="Host storage" icon="internaldrive" onClick={() => setStorage(true)} />
              </CardSection>
              <TabChoice label="Group details" selection={tab === 'library' ? 'library' : 'details'} options={[{ id: 'details', label: 'Members' }, { id: 'library', label: 'Library' }]} onChange={setTab} />
              {tab === 'library' ? <LibraryView botId={botId} openArtifact={openArtifact} /> : <CardSection title="Members"><GroupMembersList group={bot} editGroup={editGroup} /></CardSection>}
            </div>
          ) : (
            <>
              <TabChoice label="Conversation details" selection={tab} options={hasLibrary(store.hostVersion?.version) ? TABS : TABS.filter((t) => t.id !== 'library')} onChange={setTab} />
              <div key={tab} className="details-tab fade-in">
                {tab === 'details' ? (
                  bot ? (
                    <>
                      <CardSection><GroupActionRow title="Host storage" icon="internaldrive" onClick={() => setStorage(true)} /></CardSection>
                      <section className="details-section">
                        <Text className="details-section-title">{tr('Reported context')}</Text>
                        <Text as="div" size="callout" tone="secondary">{bot.contextUsage
                          ? `${bot.contextUsage.used.toLocaleString()} / ${bot.contextUsage.size.toLocaleString()} · ${new Date(bot.contextUsage.reportedAt).toLocaleTimeString()}`
                          : tr('Not reported by this agent')}</Text>
                      </section>
                      {bot.cwd ? <button className="member-row press" onClick={() => setBrowsingWorkspace(true)}><Icon name="folder" size={18} /> Browse workspace</button> : null}
                      <WorkTimeline bot={bot} openCard={openCard} openThread={(itemId) => { opening(); openThread(itemId) }}>
                        <RoutinesView botId={botId} initialId={routineId} />
                        <DevServersSection bot={bot} />
                      </WorkTimeline>
                      <BotSettingsPanel botId={botId} part="often" />
                    </>
                  ) : null
                ) : tab === 'computer' ? (
                  bot ? <ComputerTab bot={bot} /> : null
                ) : tab === 'library' ? (
                  <LibraryView botId={botId} openArtifact={openArtifact} />
                ) : tab === 'memory' ? (
                  <MemoryCard botId={botId} />
                ) : (
                  <BotSettingsPanel botId={botId} part="rarely" />
                )}
              </div>
            </>
          )}
        </div>
      </div>
      {page.mounted && shown ? (
        <div className={page.shown && reading ? 'details-page thread shown' : 'details-page thread'} inert={!reading}>
          <WorkThread key={shown.id} botId={botId} task={shown} openArtifact={openArtifact} showInChat={showInChat} close={closeThread} />
        </div>
      ) : null}
      <Sheet open={storage} onClose={() => setStorage(false)} width={600} height={640}><StorageOverview /></Sheet>
      <Sheet open={meeting} onClose={() => setMeeting(false)} width={640} height={740}><MeetingRoom groupId={botId} /></Sheet>
      <Sheet open={tasks} onClose={() => setTasks(false)} width={640} height={740}>{bot && group ? <GroupTasks group={bot} /> : null}</Sheet>
      <Sheet open={inbox} onClose={() => setInbox(false)} width={600} height={640}>{bot && group ? <GroupInbox group={bot} /> : null}</Sheet>
      <Sheet open={browsingWorkspace} onClose={() => setBrowsingWorkspace(false)} width={600} height={640}>
        <WorkspaceBrowser botId={botId} open={(owner, path) => { setBrowsingWorkspace(false); preview?.({ botId: owner, path }) }} />
      </Sheet>
    </div>
  )
}

/** The bot's avatar and name, and what runs it (a group: how many bots it has). */
function Identity({ bot }: { bot: Bot }) {
  const store = useStore()
  const group = isGroup(bot)
  return (
    <div className="details-identity">
      {group ? <BotOrGroupAvatar bot={bot} members={store.members(bot)} size={48} /> : <div className="avatar-frame"><BotAvatar bot={bot} size={48} />{bot.lead ? <LeadAgentBadge size={24} interactive /> : null}</div>}
      <span className="details-name">{bot.name}</span>
      <span className="details-subtitle">
        {group ? (
          `${bot.meeting ? "Meeting · " : ""}${bot.members.length} ${bot.members.length === 1 ? 'bot' : 'bots'}`
        ) : (
          <>
            <AgentIcon registry={store.agentRegistry(bot.backend)} size={12} />
            {store.backendName(bot.backend)}
            {bot.model ? ` · ${bot.model}` : null}
          </>
        )}
      </span>
    </div>
  )
}

/** A group's bots (Grok Bot's member list); clicking one opens its own chat. */
function GroupMembersList({ group, editGroup }: { group: Bot; editGroup: () => void }) {
  const tr = useTr()
  const store = useStore()
  return (
    <div className="details-members">
      {store.members(group).map((bot) => (
        <div key={bot.id} className="member-row removable">
          <button className="member-open press" title={tr("Open {name}'s own chat").replaceAll('{name}', () => bot.name)} onClick={() => store.setSelection(bot.id)}>
            <BotAvatar bot={bot} size={32} />
            <VStack as="span" gap={1} grow>
              <span className="member-name">{bot.name}</span>
              {isWorking(bot) ? <Text size="compactSecondary" tone="secondary" className="member-name">{tr(bot.activity || 'Working…')}</Text> : null}
            </VStack>
          </button>
          {group.members.length > 1 ? <IconButton title={tr('Remove {name} from the group').replaceAll('{name}', () => bot.name)} icon="xmark" onClick={() => store.removeMember(group, bot.id)} /> : null}
        </div>
      ))}
      <button className="member-row press" onClick={editGroup}>
        <span className="member-add-icon">
          <Icon name="plus" size={16} color="var(--secondary)" />
        </span>
        <Text tone="secondary" className="member-name">{tr(`Add Member`)}</Text>
      </button>
    </div>
  )
}

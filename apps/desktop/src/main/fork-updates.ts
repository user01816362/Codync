import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { app, BrowserWindow, ipcMain, shell } from 'electron'
import { identity } from './environment'
import { prefs, type HostController } from './host-controller'
import { Updates } from './updates'
import type { UpdateState } from '../shared/ipc'

const repository = 'user01816362/Codync'
interface ForkBuild { runId: number; commit: string }

/** CI marks only its ad-hoc Mac bundle; development and signed packages retain their updater. */
export function forkBuild(): ForkBuild | null {
  if (!app.isPackaged || !identity.releaseUpdates || process.platform !== 'darwin') return null
  try {
    const build = JSON.parse(readFileSync(join(process.resourcesPath, 'fork-updates.json'), 'utf8')) as ForkBuild
    return Number.isSafeInteger(build.runId) && build.runId > 0 && /^[a-f0-9]{40}$/.test(build.commit) ? build : null
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === 'ENOENT') return null
    throw error
  }
}

/** GitHub checks for ad-hoc builds. Installation stays with macOS, rather than bypassing Squirrel's signing checks. */
export class ForkUpdates extends Updates {
  private releaseDownload: string | null = null

  constructor(host: HostController, private readonly build: ForkBuild) {
    super(host)
    this.state = { ...this.state, supported: true, manualInstall: true, autoDownload: false }
  }

  override register(windows: () => BrowserWindow[]) {
    ipcMain.handle('updates:state', () => this.state)
    ipcMain.on('updates:check', () => this.check())
    ipcMain.on('updates:install', () => this.installNow())
    ipcMain.on('updates:autoCheck', (_event, on: boolean) => this.setAutoCheck(on))
    ipcMain.on('updates:autoDownload', () => this.setAutoDownload(false))
    this.on('setAutoCheck', on => this.setAutoCheck(on))
    this.on('setAutoDownload', () => this.setAutoDownload(false))
    this.on('change', () => {
      for (const window of windows()) window.webContents.send('updates:change', this.state)
    })
    setTimeout(() => this.scheduleForkCheck(), 30_000)
    setInterval(() => this.scheduleForkCheck(), 3600_000)
  }

  override setAutoDownload(_on: boolean) {
    this.publishForkState({ autoDownload: false })
  }

  override check() {
    if (this.releaseDownload && !this.state.error) return this.installNow()
    void this.checkForkRelease(true)
  }

  override installNow() {
    if (!this.releaseDownload) return
    void shell.openExternal(this.releaseDownload).catch(error => {
      this.publishForkState({ error: error instanceof Error ? error.message : String(error) })
    })
  }

  private publishForkState(change: Partial<UpdateState>) {
    this.state = { ...this.state, ...change }
    this.emit('change')
  }

  private scheduleForkCheck() {
    if (!this.state.autoCheck || this.state.checking) return
    if (this.state.lastCheck && Date.now() - this.state.lastCheck < 24 * 3600_000 - 60_000) return
    void this.checkForkRelease(false)
  }

  private async checkForkRelease(manual: boolean) {
    if (this.state.checking) return
    this.publishForkState({ checking: true, error: null })
    try {
      const response = await fetch(`https://api.github.com/repos/${repository}/releases/latest`, {
        headers: { Accept: 'application/vnd.github+json', 'User-Agent': `Codync-fork/${app.getVersion()}`, 'X-GitHub-Api-Version': '2026-03-10' },
        signal: AbortSignal.timeout(15_000),
      })
      if (!response.ok && response.status !== 404) throw new Error(`Couldn't check fork releases (HTTP ${response.status}).`)
      const release = response.status === 404 ? null : await response.json() as {
        tag_name?: string; draft?: boolean; prerelease?: boolean
        assets?: { name: string; browser_download_url: string }[]
      }
      const tag = release?.tag_name?.match(/^v(\d+\.\d+\.\d+)(?:-fork\.(\d+))?$/)
      const asset = release?.assets?.find(asset => asset.name === 'codync-macos.dmg')
      const current = app.getVersion().split('.').map(Number)
      const published = tag?.[1]?.split('.').map(Number) ?? []
      const difference = published.findIndex((part, index) => part !== current[index])
      const newerVersion = difference >= 0 && published[difference]! > current[difference]!
      // Keep old build-tag releases readable during the transition; normal releases compare app versions.
      const newerBuild = !!tag && tag[1] === app.getVersion() && Number.isSafeInteger(Number(tag[2])) && Number(tag[2]) > this.build.runId
      const newer = newerVersion || newerBuild
      const validURL = asset?.browser_download_url.startsWith(`https://github.com/${repository}/releases/download/`)
      this.releaseDownload = newer && validURL && !release?.draft && !release?.prerelease ? asset!.browser_download_url : null
      const now = Date.now()
      prefs.set('updatesLastCheck', now)
      this.publishForkState({
        checking: false, lastCheck: now,
        availableVersion: this.releaseDownload ? tag![1]! : null,
      })
      if (manual && !this.releaseDownload) this.emit('upToDate')
    } catch (error) {
      this.publishForkState({ checking: false, error: manual ? (error instanceof Error ? error.message : String(error)) : null })
    }
  }
}

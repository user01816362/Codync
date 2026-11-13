import assert from 'node:assert/strict'
import test from 'node:test'
import { hostUpdateFixture } from './test-support/host-update.ts'
import { settle } from './test-support/main-module.ts'

for (const platform of ['darwin', 'win32']) {
  test(`${platform}: updating stops the bundled host after unregistering screen and persists restart`, async () => {
    const f = hostUpdateFixture({ platform })
    await f.host.prepareForUpdate()
    await settle()
    assert.deepEqual(f.order, ['unregisterScreen', 'stop'])
    assert.equal(f.persisted.hostRestartAfterAppUpdate, true)
    await f.host.refresh()
    assert.deepEqual(f.calls, [['stop']], 'refresh cannot restart the host during replacement')
    assert.equal(await f.host.isIdleForUpdate(), false)
  })
}

for (const options of [{ platform: 'linux' }, { devPort: '19333' }]) {
  test(`app replacement leaves externally managed hosts running: ${JSON.stringify(options)}`, async () => {
    const f = hostUpdateFixture(options)
    await f.host.prepareForUpdate()
    assert.equal(f.calls.length, 0)
    assert.equal(f.order.length, 0)
    assert.equal(f.prefs.get('hostRestartAfterAppUpdate'), false)
  })
}

test('an install already in progress prevents update preparation from stopping the host', async () => {
  const f = hostUpdateFixture()
  const installing = f.host.install()
  await settle()
  await assert.rejects(f.host.prepareForUpdate(), /being installed/)
  assert.equal(f.calls.some((args) => args[0] === 'stop'), false)
  await f.tick(1000)
  await installing
})

test('missing bundled binary rejects preparation without pretending stop succeeded', async () => {
  const f = hostUpdateFixture({ missingBinary: true })
  await assert.rejects(f.host.prepareForUpdate(), /bundled host is missing/)
  assert.equal(f.calls.length, 0)
})

test('failed stop reports the error and cancellation reinstalls the service once', async () => {
  const f = hostUpdateFixture()
  f.behavior.stopStatus = 1
  await assert.rejects(f.host.prepareForUpdate(), /service command failed/)
  f.host.resumeAfterCancelledUpdate()
  f.host.resumeAfterCancelledUpdate()
  await settle()
  assert.deepEqual(f.calls, [['stop'], ['install', '--port', '19222']])
  await f.tick(1000)
})

test('new app launch reinstalls the host and clears restart marker only after health matches', async () => {
  const f = hostUpdateFixture({ restartMarker: true })
  const starting = f.host.refresh()
  await settle()
  assert.deepEqual(f.calls, [['install', '--port', '19222']])
  assert.equal(f.prefs.get('hostRestartAfterAppUpdate'), true)
  f.behavior.reachable = false
  await f.tick(1000)
  await starting
  assert.equal(f.prefs.get('hostRestartAfterAppUpdate'), true)
  f.behavior.reachable = true
  await f.tick(5000)
  assert.equal(f.host.state.kind, 'running')
  assert.equal(f.host.local?.computerId, 'fixture-computer')
  assert.equal(f.prefs.get('hostRestartAfterAppUpdate'), false)
})

test('a stale host binary cannot satisfy the restart marker', async () => {
  const f = hostUpdateFixture({ restartMarker: true })
  f.behavior.wrongBinary = true
  const starting = f.host.refresh()
  await settle()
  await f.tick(1000)
  await starting
  assert.equal(f.prefs.get('hostRestartAfterAppUpdate'), true)
  assert.notEqual(f.host.state.kind, 'running')
})

test('failed service installation keeps the restart marker and exposes the error', async () => {
  const f = hostUpdateFixture({ restartMarker: true })
  f.behavior.installStatus = 1
  await f.host.refresh()
  assert.equal(f.prefs.get('hostRestartAfterAppUpdate'), true)
  assert.equal(f.host.state.kind, 'failed')
  if (f.host.state.kind === 'failed') assert.match(f.host.state.message, /service command failed/)
})

test('an uninstalled host does not acquire a restart marker', async () => {
  const f = hostUpdateFixture({ installed: false })
  await f.host.prepareForUpdate()
  assert.equal(f.prefs.get('hostRestartAfterAppUpdate'), false)
})

test('idle detection distinguishes working, offline and idle hosts', async () => {
  const f = hostUpdateFixture()
  assert.equal(await f.host.isIdleForUpdate(), true)
  f.behavior.busy = true
  assert.equal(await f.host.isIdleForUpdate(), false)
  f.behavior.reachable = false
  assert.equal(await f.host.isIdleForUpdate(), false)
})


test('an update another device asks for reaches the updater once per request', async () => {
  const f = hostUpdateFixture()
  let requests = 0
  f.host.on('updateRequested', () => requests++)
  const installing = f.host.install()
  await settle()
  await f.tick(1000)
  await installing
  assert.equal(requests, 0)
  f.behavior.updateRequested = 1_000
  await f.tick(5000)
  await f.tick(5000)
  assert.equal(requests, 1, 'a request the poll sees again is passed on once')
  f.behavior.updateRequested = 2_000
  await f.tick(5000)
  assert.equal(requests, 2)
  assert.ok(f.fetched.length > 0 && f.fetched.every((url) => url.endsWith('/health?app=true')), 'the poll tells the host this app is open')
})

test('development install and stop reject a production binary before touching its service', async () => {
  const f = hostUpdateFixture({ environment: 'dev' })
  f.behavior.binaryEnvironment = 'main'
  await f.host.install()
  assert.equal(f.host.state.kind, 'failed')
  assert.deepEqual(f.calls, [])
  assert.equal(await f.host.uninstall(), false)
  assert.deepEqual(f.calls, [])
})

test('development service installs on its own port', async () => {
  const f = hostUpdateFixture({ environment: 'dev' })
  const installing = f.host.install()
  await settle()
  assert.deepEqual(f.calls, [['install', '--port', '19223']])
  await f.tick(1000)
  await installing
  assert.equal(f.host.state.kind, 'running')
})


test('macOS default runs an app-owned host without installing a login service', async () => {
  const f = hostUpdateFixture({ background: false, installed: false })
  delete f.persisted.hostKeepRunningAfterQuit
  const starting = f.host.install()
  await settle()
  await f.tick(1000)
  await starting
  assert.equal(f.calls.some(args => args[0] === 'install'), false)
  assert.equal(f.behavior.installed, false)
  assert.deepEqual(f.spawned.map(child => child.args), [['serve', '--port', '19222']])
  await f.host.stopForQuit()
  assert.equal(f.spawned[0]?.child.exitCode, 0)
})

test('macOS off migrates the owned launch agent into an app-owned host', async () => {
  const f = hostUpdateFixture({ background: false })
  const starting = f.host.install()
  await settle()
  await f.tick(1000)
  await starting
  assert.deepEqual(f.calls, [['stop'], ['uninstall']])
  assert.equal(f.behavior.installed, false)
  assert.equal(f.spawned.length, 1)
})

test('macOS background toggle applies immediately in both directions', async () => {
  const f = hostUpdateFixture({ background: true })
  const disabling = f.host.setKeepRunningAfterQuit(false)
  await settle()
  await f.tick(1000)
  await disabling
  assert.equal(f.behavior.installed, false)
  assert.equal(f.spawned.length, 1)
  const enabling = f.host.setKeepRunningAfterQuit(true)
  await settle()
  await f.tick(1000)
  await enabling
  assert.equal(f.spawned[0]?.child.exitCode, 0)
  assert.equal(f.behavior.installed, true)
  assert.equal(f.host.keepRunningAfterQuit(), true)
})


test('macOS foreground host is stopped for updates and busy work postpones them', async () => {
  const f = hostUpdateFixture({ background: false, installed: false })
  await f.host.install()
  const environment = f.spawned[0]?.options.env as Record<string, string>
  assert.equal(environment.CODYNC_DESKTOP_PARENT_PID, '123')
  f.behavior.busy = true
  assert.equal(await f.host.isIdleForUpdate(), false)
  await f.host.prepareForUpdate()
  assert.equal(f.spawned[0]?.child.exitCode, 0)
  assert.equal(f.persisted.hostRestartAfterAppUpdate, false)
})

test('failed macOS background removal keeps the previous setting and surfaces failure', async () => {
  const f = hostUpdateFixture({ background: true })
  f.behavior.stopStatus = 1
  await assert.rejects(f.host.setKeepRunningAfterQuit(false), /service command failed/)
  assert.equal(f.host.keepRunningAfterQuit(), true)
  assert.equal(f.behavior.installed, true)
  assert.equal(f.spawned.length, 0)
})


test('restarting the app-owned host replaces its process without installing a service', async () => {
  const f = hostUpdateFixture({ background: false, installed: false })
  await f.host.install()
  await f.host.install()
  assert.equal(f.spawned.length, 2)
  assert.equal(f.spawned[0]?.child.exitCode, 0)
  assert.equal(f.behavior.installed, false)
  assert.equal(f.calls.length, 0)
})

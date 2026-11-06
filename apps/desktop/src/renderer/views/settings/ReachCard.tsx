import { useEffect, useState } from 'react'
import { Button, CardSection, Switch } from '../../components/Controls'
import { Icon } from '../../components/Icon'
import { Dialog } from '../../components/Overlay'
import { HStack, VStack } from '../../components/Stack'
import { Text } from '../../components/Text'
import { useModel } from '../../lib/observable'
import { useTr } from '../../lib/use-language'
import type { BotStore } from '../../store/bot-store'
import { errorText } from './parts'

/**
 * Reaching this computer away from home without a Codync account: Tailscale's private
 * network, or a named Cloudflare Tunnel. Both end at the same end-to-end encrypted
 * phone channel; neither goes through Codync's cloud.
 */
export function ReachCard({ store, manageable }: { store: BotStore; manageable: boolean }) {
  const tr = useTr()
  useModel(store)
  const [busy, setBusy] = useState(false)
  const [hostname, setHostname] = useState('')
  const [confirmForget, setConfirmForget] = useState(false)
  const reach = store.reach
  const cf = reach?.cloudflare
  const ts = reach?.tailscale

  useEffect(() => {
    if (manageable) void store.refreshReach()
  }, [store, manageable])

  // A Cloudflare sign-in finishes in the browser, not through this app: read the
  // status again until cloudflared has written the account certificate.
  useEffect(() => {
    if (!cf?.loginPending) return
    const timer = setInterval(() => void store.refreshReach(), 2000)
    return () => clearInterval(timer)
  }, [store, cf?.loginPending])

  const run = (work: () => Promise<unknown>) => {
    setBusy(true)
    work()
      .catch((error: unknown) => store.setError(errorText(error)))
      .finally(() => setBusy(false))
  }
  const call = (method: string, body: object = {}, timeoutMs = 20_000) =>
    run(async () => {
      await (await store.ready()).call(method, body, timeoutMs)
      await store.refreshReach()
    })

  const open = (url: string | null | undefined) => {
    if (url) window.codync.app.openExternal(url)
  }

  return (
    <CardSection
      title={tr('Tailscale and Cloudflare Tunnel')}
      accessory={
        <Button kind="secondary" disabled={busy || !manageable} onClick={() => void store.refreshReach()}>
          {tr('Check again')}
        </Button>
      }
      footer={tr('No Codync account needed. Your phone connects straight to this computer, end to end encrypted.')}
    >
      <VStack gap={10} className="settings-appear">
        <HStack gap={8}>
          <Icon name="network" size={13} color="var(--secondary)" />
          <VStack grow className="settings-stack">
            <Text>{tr('Tailscale')}</Text>
            <Text size="caption" tone={ts?.running ? 'secondary' : 'attention'}>
              {!reach
                ? tr('Checking…')
                : !ts?.installed
                  ? tr('Install Tailscale on this computer and on your phone, then sign in to both.')
                  : !ts.running
                    ? tr('Installed, but not signed in or not running.')
                    : ts.dnsName
                      ? tr('On your tailnet at {name}.').replace('{name}', ts.dnsName)
                      : ts.ipv4
                        ? tr('On your tailnet at {ip}.').replace('{ip}', ts.ipv4)
                        : tr('Running on your tailnet.')}
            </Text>
          </VStack>
          {!ts?.installed ? (
            <Button kind="secondary" onClick={() => open(ts?.download ?? 'https://tailscale.com/download')}>{tr('Get Tailscale')}</Button>
          ) : !ts.running && ts.canOpen ? (
            <Button kind="secondary" disabled={busy || !manageable} onClick={() => void call('reachTailscaleOpen')}>{tr('Open Tailscale')}</Button>
          ) : null}
        </HStack>
      </VStack>

      <VStack gap={10} className="settings-appear">
        <HStack gap={8}>
          <Icon name="cloud" size={13} color="var(--secondary)" />
          <VStack grow className="settings-stack">
            <Text>{tr('Cloudflare Tunnel')}</Text>
            <Text size="caption" tone={cf?.lastError ? 'attention' : 'secondary'}>
              {!reach
                ? tr('Checking…')
                : !cf?.installed
                  ? tr('Install cloudflared on this computer first.')
                  : cf.loginPending
                    ? tr('Finish signing in to Cloudflare in the browser on this computer.')
                    : cf.lastError
                      ? cf.lastError
                      : !cf.loggedIn
                      ? tr('Sign in to the Cloudflare account that holds your domain.')
                      : !cf.configured
                        ? tr('Choose a hostname from a domain in that account.')
                        : cf.lastError
                          ? cf.lastError
                          : cf.running
                            ? tr('Tunnel process running for {url}.').replace('{url}', cf.url ?? '')
                            : tr('Stopped. Turn it on to reach this computer from anywhere.')}
            </Text>
          </VStack>
          {!cf?.installed ? (
            <Button kind="secondary" onClick={() => open(cf?.download)}>{tr('Install cloudflared')}</Button>
          ) : cf.loginPending ? (
            <Button kind="secondary" disabled={!cf.loginUrl} onClick={() => open(cf?.loginUrl)}>{tr('Open sign-in')}</Button>
          ) : !cf.loggedIn ? (
            <Button kind="secondary" disabled={busy || !manageable} onClick={() => void call('reachCloudflareLogin')}>{tr('Sign in to Cloudflare')}</Button>
          ) : null}
        </HStack>

        {cf?.loggedIn && !cf.configured ? (
          <HStack gap={8}>
            <input
              className="settings-field"
              aria-label={tr('Cloudflare hostname')}
              placeholder="codync.example.com"
              value={hostname}
              spellCheck={false}
              disabled={busy || !manageable}
              onChange={(event) => setHostname(event.currentTarget.value)}
              onKeyDown={(event) => {
                if (event.key === 'Enter' && hostname.trim()) void call('reachCloudflareCreate', { hostname: hostname.trim() }, 120_000)
              }}
            />
            <Button disabled={busy || !manageable || !hostname.trim()} onClick={() => void call('reachCloudflareCreate', { hostname: hostname.trim() }, 120_000)}>
              {tr('Create tunnel')}
            </Button>
          </HStack>
        ) : null}

        {cf?.configured ? (
          <Switch on={cf.enabled} disabled={busy || !manageable} onChange={(on) => void call('reachCloudflareSetEnabled', { enabled: on })}>
            <span className="settings-stack">
              <Text>{cf.hostname}</Text>
              <Text size="caption" tone="secondary">{cf.running ? tr('Tunnel running') : tr('Tunnel stopped')}</Text>
            </span>
          </Switch>
        ) : null}

        {cf?.configured ? (
          <HStack gap={8}>
            <Button kind="secondary" disabled={busy || !manageable} onClick={() => void call('reachCloudflareSetEnabled', { enabled: true })}>{tr('Restart tunnel')}</Button>
            <Button kind="secondary" disabled={busy || !manageable} onClick={() => setConfirmForget(true)}>{tr('Forget this tunnel')}</Button>
          </HStack>
        ) : null}
      </VStack>

      <Dialog
        open={confirmForget}
        title={tr('Forget this Cloudflare Tunnel here?')}
        message={tr('The tunnel stops and this computer forgets its hostname. The tunnel and DNS record stay in your Cloudflare account; delete them there if you no longer want them.')}
        actions={[{ title: tr('Forget'), destructive: true, action: () => void call('reachCloudflareForget') }]}
        onClose={() => setConfirmForget(false)}
      />
    </CardSection>
  )
}

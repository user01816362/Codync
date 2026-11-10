import { useEffect } from 'react'
import type { AnalyticsEvent, Link, Screen } from '@shared/analytics'
import type { AnalyticsSender } from '../lib/analytics-queue'

/** Telemetry remains disabled for both local and account-connected hosts. */
export const analytics = {
  track(_event: AnalyticsEvent) {},
  connect(_sender: AnalyticsSender | null) {},
}

/** A screen, sheet or tab is shown: once when it appears, and again when `key` changes. */
export function useScreenViewed(screen: Screen | null, key: unknown = null) {
  useEffect(() => {
    if (screen) analytics.track({ event: 'screen_viewed', screen })
  }, [screen, key])
}

/** Opens an outbound link in the browser, counted by what it leads to. */
export function openLink(url: string, link: Link) {
  analytics.track({ event: 'link_opened', link })
  window.codync.app.openExternal(url)
}

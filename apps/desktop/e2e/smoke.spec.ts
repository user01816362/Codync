import { test, expect } from '@playwright/test'
import type { HostSnapshot } from '../src/shared/ipc.ts'
import { attach, sandbox, setPref } from './sandbox.ts'

test('the sandbox app renders and runs against its own host', async () => {
  const { browser, page } = await attach()
  try {
    await expect(page.locator('#root')).not.toBeEmpty()
    const host = await page.evaluate(
      () => (window as unknown as { codync: { host: { snapshot(): Promise<HostSnapshot> } } }).codync.host.snapshot(),
    )
    // The sandbox host, never Codync Dev's shared one on 19223.
    expect(host.state.kind).toBe('running')
    expect(host.dev).toBe(true)
    expect(host.local?.baseURL).toBe(`http://127.0.0.1:${sandbox().port}`)
  } finally {
    await browser.close()
  }
})


test('the compact sidebar opens without a render error', async () => {
  const { browser, page } = await attach()
  const previous = await page.evaluate(() => localStorage.getItem('sidebarCompact'))
  try {
    await setPref(page, 'sidebarCompact', true)
    await expect(page.locator('.rail-profile')).toBeVisible()
    await expect(page.getByText('Something went wrong', { exact: true })).toHaveCount(0)
    await page.locator('.rail-profile').click()
    await expect(page.locator('.account-panel')).toBeVisible()
  } finally {
    await setPref(page, 'sidebarCompact', previous === null ? false : JSON.parse(previous))
    await browser.close()
  }
})

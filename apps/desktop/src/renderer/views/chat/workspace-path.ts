/** A local path explicitly linked by an agent; the host still checks its workspace boundary. */
export function workspacePath(url: string): string | null {
  if (!url || url.startsWith('#') || url.startsWith('//') || /[?#]/.test(url)) return null
  if (/^file:/i.test(url)) {
    try {
      const parsed = new URL(url)
      if (parsed.hostname && parsed.hostname !== 'localhost') return null
      const path = decodeURIComponent(parsed.pathname)
      return path && !path.includes('\0') ? path : null
    } catch { return null }
  }
  if (/^[a-z][a-z0-9+.-]*:/i.test(url) || url.includes('\0')) return null
  try { return decodeURIComponent(url) } catch { return null }
}

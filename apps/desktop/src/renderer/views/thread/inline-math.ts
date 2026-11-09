/** Keep native Markdown for links/styles/code; only plain prose and bounded formulas use the math paragraph view. */
export function hasInlineMath(source: string): boolean {
  if (source.length > 4000 || source.includes('$$')) return false
  let found = false
  const rest = source.replace(/`[^`\n]*`/g, '').replace(/\\\(([^\n]{1,1024}?)\\\)|(?<!\\)\$([^$\n]{1,1024}?)\$/g, (raw, explicit: string | undefined, dollar: string | undefined) => {
    const math = explicit ?? dollar ?? ''
    const valid = explicit !== undefined || (math.trim() === math && (/[\\^_=+*/{}]/.test(math) || /^[A-Za-z]$/.test(math)))
    if (valid) { found = true; return '' }
    return raw
  })
  return found && !/[\[\]<>\\]/.test(rest)
}

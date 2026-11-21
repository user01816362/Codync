import { useRef } from 'react'
import { AnchoredMenu, useContextMenu, type MenuItem, type ReactionPick } from '../../components/Menu'

export function Bubble({ className, items, reactions, children, quote }: { className: string; items: () => MenuItem[]; reactions?: ReactionPick; children: React.ReactNode; quote?: (selected: string) => void }) {
  const menu = useContextMenu()
  const body = useRef<HTMLDivElement>(null)
  const selected = useRef('')
  return (
    <>
      <div ref={body} className={className} onContextMenu={event => {
        const selection = window.getSelection()
        selected.current = selection && body.current?.contains(selection.anchorNode) && body.current?.contains(selection.focusNode) ? selection.toString() : ''
        menu.onContextMenu(event)
      }}>
        {children}
      </div>
      <AnchoredMenu open={menu.open} onClose={menu.close} point={menu.point} items={() => [
        ...(quote && selected.current.trim() ? [{ title: 'Quote selection', icon: 'arrowshape.turn.up.left', action: () => quote?.(selected.current) }] : []),
        ...items(),
      ]} reactions={reactions} />
    </>
  )
}

import { useWhatsNew } from '../lib/use-whats-new'
import { DeleteAccountPrompt } from './DeleteAccountPrompt'
import { JoinAccountPrompt } from './JoinAccountPrompt'
import { MoveComputerPrompt } from './MoveComputerPrompt'
import { StarPrompt } from './StarPrompt'
import { WhatsNewSheet } from './WhatsNewSheet'

/** What may come up over the chat window once it's `ready` (past the welcome and setup). */
export function MainPrompts({ ready }: { ready: boolean }) {
  // Decided here, mounted from the start: at mount, before the welcome can finish.
  const whatsNew = useWhatsNew(ready)
  return (
    <>
      {ready ? <><StarPrompt /><MoveComputerPrompt /><JoinAccountPrompt /><DeleteAccountPrompt /></> : null}
      <WhatsNewSheet {...whatsNew} />
    </>
  )
}

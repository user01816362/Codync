import { Icon } from '../../components/Icon'
import { useTr } from '../../lib/use-language'

/** A group action inside a shared settings card; the whole row is the hit target. */
export function GroupActionRow({ title, icon, onClick, disabled = false }: { title: string; icon: string; onClick: () => void; disabled?: boolean }) {
  const tr = useTr()
  return <button className="group-action-row press" onClick={onClick} disabled={disabled}>
    <Icon name={icon} size={18} />
    <span>{tr(title)}</span>
    <Icon name="chevron.right" size={11} color="var(--tertiary)" />
  </button>
}

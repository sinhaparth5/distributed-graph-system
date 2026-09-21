import { RadialMenu } from 'reagraph'
import type { ContextMenuEvent } from 'reagraph'

interface Props {
  event: ContextMenuEvent
  isPinned: (id: string) => boolean
  onDeleteNode: (id: string) => void
  onDeleteEdge: (id: string) => void
  onTogglePin: (id: string) => void
  onStartConnect: (id: string) => void
}

export default function NodeRadialMenu({ event, isPinned, onDeleteNode, onDeleteEdge, onTogglePin, onStartConnect }: Props) {
  const isNode = 'position' in event.data
  const id = event.data.id

  if (!isNode) {
    return (
      <RadialMenu
        onClose={event.onClose}
        items={[
          {
            label: 'Delete edge',
            onClick: () => { onDeleteEdge(id); event.onClose() },
          },
        ]}
      />
    )
  }

  const pinned = isPinned(id)

  return (
    <RadialMenu
      onClose={event.onClose}
      items={[
        {
          label: 'Connect from here',
          onClick: () => { onStartConnect(id); event.onClose() },
        },
        {
          label: pinned ? 'Unpin' : 'Pin in place',
          onClick: () => { onTogglePin(id); event.onClose() },
        },
        {
          label: 'Copy ID',
          onClick: () => { navigator.clipboard?.writeText(id); event.onClose() },
        },
        {
          label: 'Delete node',
          className: 'radial-danger',
          onClick: () => { onDeleteNode(id); event.onClose() },
        },
      ]}
    />
  )
}

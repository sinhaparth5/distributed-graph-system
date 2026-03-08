import { NEEDS_START, NEEDS_END } from '../types'
import type { Algorithm } from '../types'

interface Props {
  algorithm: Algorithm
  startNode: string
  endNode: string
  onStartChange: (value: string) => void
  onEndChange: (value: string) => void
}

export default function NodeInputs({
  algorithm,
  startNode,
  endNode,
  onStartChange,
  onEndChange,
}: Props) {
  const needsStart = NEEDS_START.has(algorithm)
  const needsEnd   = NEEDS_END.has(algorithm)

  if (!needsStart && !needsEnd) return null

  return (
    <div className="grid grid-cols-2 gap-4">
      {needsStart && (
        <div>
          <p className="text-xs font-mono-display text-zinc-500 uppercase tracking-widest mb-2.5">
            Start Node
          </p>
          <input
            type="number"
            min={0}
            value={startNode}
            onChange={e => onStartChange(e.target.value)}
            placeholder="0"
            className="node-input w-full bg-zinc-900/50 border border-zinc-700 rounded-lg px-4 py-3 text-white font-mono-display text-sm placeholder-zinc-600"
          />
        </div>
      )}

      {needsEnd && (
        <div>
          <p className="text-xs font-mono-display text-zinc-500 uppercase tracking-widest mb-2.5">
            End Node
          </p>
          <input
            type="number"
            min={0}
            value={endNode}
            onChange={e => onEndChange(e.target.value)}
            placeholder="1"
            className="node-input w-full bg-zinc-900/50 border border-zinc-700 rounded-lg px-4 py-3 text-white font-mono-display text-sm placeholder-zinc-600"
          />
        </div>
      )}
    </div>
  )
}

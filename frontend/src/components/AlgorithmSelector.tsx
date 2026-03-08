import { ALGORITHMS } from '../types'
import type { Algorithm } from '../types'

interface Props {
  value: Algorithm | null
  onChange: (value: Algorithm) => void
}

export default function AlgorithmSelector({ value, onChange }: Props) {
  return (
    <div>
      <p className="text-xs font-mono-display text-zinc-500 uppercase tracking-widest mb-2.5">
        Algorithm
      </p>

      <div className="grid grid-cols-3 gap-2">
        {ALGORITHMS.map(({ id, label }) => (
          <button
            key={id}
            onClick={() => onChange(id)}
            className={[
              'px-2 py-2.5 rounded-lg border text-xs font-mono-display',
              'transition-all duration-100',
              value === id
                ? 'border-cyan-600/70 bg-cyan-950/40 text-cyan-300'
                : 'border-zinc-800 bg-zinc-900/20 text-zinc-500 hover:border-zinc-600 hover:text-zinc-300',
            ].join(' ')}
          >
            {label}
          </button>
        ))}
      </div>
    </div>
  )
}

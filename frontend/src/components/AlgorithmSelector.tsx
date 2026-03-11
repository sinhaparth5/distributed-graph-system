import { ALGORITHMS } from '../types'
import type { Algorithm } from '../types'

interface Props {
  value: Algorithm | null
  onChange: (value: Algorithm) => void
}

const GROUPS = [
  { key: 'traversal',     label: 'Traversal'     },
  { key: 'shortest-path', label: 'Shortest Path' },
  { key: 'graph',         label: 'Graph'         },
] as const

export default function AlgorithmSelector({ value, onChange }: Props) {
  return (
    <div>
      <p className="text-xs font-mono-display text-zinc-500 uppercase tracking-widest mb-2.5">
        Algorithm
      </p>
      <div className="space-y-2.5">
        {GROUPS.map(({ key, label }) => {
          const algs = ALGORITHMS.filter(a => a.group === key)
          return (
            <div key={key}>
              <p className="text-[10px] font-mono-display text-zinc-700 uppercase tracking-wider mb-1.5">
                {label}
              </p>
              <div className="grid grid-cols-3 gap-1.5">
                {algs.map(({ id, label: alabel }) => (
                  <button
                    key={id}
                    onClick={() => onChange(id)}
                    className={[
                      'px-2 py-2 rounded-lg border text-xs font-mono-display',
                      'transition-all duration-100',
                      value === id
                        ? 'border-cyan-600/70 bg-cyan-950/40 text-cyan-300'
                        : 'border-zinc-800 bg-zinc-900/20 text-zinc-500 hover:border-zinc-600 hover:text-zinc-300',
                    ].join(' ')}
                  >
                    {alabel}
                  </button>
                ))}
              </div>
            </div>
          )
        })}
      </div>
    </div>
  )
}

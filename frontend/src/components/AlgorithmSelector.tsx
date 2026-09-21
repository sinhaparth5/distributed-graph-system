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
      <p className="text-xs font-semibold text-slate-500 uppercase tracking-wide mb-2.5">
        Algorithm
      </p>
      <div className="space-y-2.5">
        {GROUPS.map(({ key, label }) => {
          const algs = ALGORITHMS.filter(a => a.group === key)
          return (
            <div key={key}>
              <p className="text-[10px] font-semibold text-slate-400 uppercase tracking-wide mb-1.5">
                {label}
              </p>
              <div className="grid grid-cols-3 gap-1.5">
                {algs.map(({ id, label: alabel }) => (
                  <button
                    key={id}
                    onClick={() => onChange(id)}
                    className={[
                      'cursor-pointer px-2 py-2 rounded-lg border text-xs font-medium',
                      'transition-all duration-100',
                      value === id
                        ? 'border-cyan-300 bg-cyan-50 text-cyan-800'
                        : 'border-slate-200 bg-white text-slate-600 hover:border-slate-300 hover:bg-slate-50',
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

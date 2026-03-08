import type { FileFormat } from '../types'

interface Props {
  value: FileFormat
  onChange: (value: FileFormat) => void
}

const OPTIONS: { value: FileFormat; label: string; hint: string }[] = [
  { value: 'edgeList',      label: 'Edge List',      hint: 'u v w  per line' },
  { value: 'adjacencyList', label: 'Adjacency List', hint: 'u: v1 v2 …'      },
]

export default function FormatSelector({ value, onChange }: Props) {
  return (
    <div>
      <p className="text-xs font-mono-display text-zinc-500 uppercase tracking-widest mb-2.5">
        File Format
      </p>

      <div className="space-y-2">
        {OPTIONS.map(opt => (
          <label
            key={opt.value}
            className={[
              'flex items-center gap-3 px-4 py-3 rounded-lg border cursor-pointer',
              'transition-colors duration-100',
              value === opt.value
                ? 'border-cyan-700/70 bg-cyan-950/25 text-white'
                : 'border-zinc-800 bg-zinc-900/20 text-zinc-400 hover:border-zinc-700',
            ].join(' ')}
          >
            {/* Custom radio dot */}
            <span
              className={[
                'w-4 h-4 rounded-full border-2 flex items-center justify-center flex-shrink-0',
                value === opt.value ? 'border-cyan-400' : 'border-zinc-600',
              ].join(' ')}
            >
              {value === opt.value && (
                <span className="w-1.5 h-1.5 rounded-full bg-cyan-400 block" />
              )}
            </span>

            <input
              type="radio"
              name="format"
              value={opt.value}
              checked={value === opt.value}
              onChange={() => onChange(opt.value)}
              className="hidden"
            />

            <div>
              <p className="font-mono-display text-sm leading-none">{opt.label}</p>
              <p className="text-zinc-600 text-xs mt-1 font-mono-display">{opt.hint}</p>
            </div>
          </label>
        ))}
      </div>
    </div>
  )
}

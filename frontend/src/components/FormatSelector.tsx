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
      <p className="text-xs font-semibold text-slate-500 uppercase tracking-wide mb-2.5">
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
                ? 'border-cyan-300 bg-cyan-50 text-slate-900'
                : 'border-slate-200 bg-white text-slate-600 hover:border-slate-300',
            ].join(' ')}
          >
            {/* Custom radio dot */}
            <span
              className={[
                'w-4 h-4 rounded-full border-2 flex items-center justify-center flex-shrink-0',
                value === opt.value ? 'border-cyan-600' : 'border-slate-300',
              ].join(' ')}
            >
              {value === opt.value && (
                <span className="w-1.5 h-1.5 rounded-full bg-cyan-600 block" />
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
              <p className="text-sm font-medium leading-none">{opt.label}</p>
              <p className="text-slate-400 text-xs mt-1 font-mono-display">{opt.hint}</p>
            </div>
          </label>
        ))}
      </div>
    </div>
  )
}

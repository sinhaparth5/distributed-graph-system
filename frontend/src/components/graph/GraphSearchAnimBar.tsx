import { SURFACE } from '../../theme'

interface Props {
  searchInput: string
  searchError: boolean
  onSearchInputChange: (value: string) => void
  onSearchSubmit: (e: React.FormEvent) => void

  hasResult: boolean
  isPlaying: boolean
  animStep: number
  animSpeed: 'fast' | 'medium' | 'slow'
  animProgress: number
  animTotal: number
  onTogglePlayPause: () => void
  onReset: () => void
  onSpeedChange: (speed: 'fast' | 'medium' | 'slow') => void
}

const SPEEDS = ['fast', 'medium', 'slow'] as const

export default function GraphSearchAnimBar({
  searchInput, searchError, onSearchInputChange, onSearchSubmit,
  hasResult, isPlaying, animStep, animSpeed, animProgress, animTotal,
  onTogglePlayPause, onReset, onSpeedChange,
}: Props) {
  return (
    <div className="flex items-center gap-3 px-4 py-2 border-b border-slate-200 flex-wrap"
         style={{ background: SURFACE.panel }}>

      {/* Node search */}
      <form onSubmit={onSearchSubmit} className="flex items-center gap-2">
        <input
          type="text"
          value={searchInput}
          onChange={e => onSearchInputChange(e.target.value)}
          placeholder="Jump to node ID…"
          className={[
            'h-7 px-3 rounded text-xs border bg-white outline-none transition-colors w-36 font-mono-display',
            searchError
              ? 'border-red-400 text-red-600 placeholder-red-300'
              : 'border-slate-300 text-slate-900 placeholder-slate-400 focus:border-cyan-500',
          ].join(' ')}
        />
        <button
          type="submit"
          className="cursor-pointer h-7 px-3 rounded text-xs font-medium text-slate-500 border border-slate-300 hover:text-cyan-700 hover:border-cyan-400 transition-colors"
        >
          find
        </button>
        {searchError && (
          <span className="text-[10px] text-red-600">not found</span>
        )}
      </form>

      <div className="w-px h-4 bg-slate-200" />

      {/* Animation controls */}
      {hasResult ? (
        <div className="flex items-center gap-2 flex-wrap">
          <span className="text-[10px] font-semibold text-slate-400 uppercase tracking-wide">animate</span>

          <button
            onClick={onTogglePlayPause}
            className={[
              'cursor-pointer h-7 px-3 rounded text-xs font-medium border transition-colors',
              isPlaying
                ? 'bg-amber-50 text-amber-700 border-amber-300'
                : 'bg-cyan-50 text-cyan-700 border-cyan-300 hover:bg-cyan-100',
            ].join(' ')}
          >
            {isPlaying ? '⏸ pause' : animStep >= 0 ? '▶ resume' : '▶ play'}
          </button>

          {animStep >= 0 && (
            <button
              onClick={onReset}
              className="cursor-pointer h-7 px-2.5 rounded text-xs font-medium text-slate-500 border border-slate-200 hover:text-slate-800 transition-colors"
            >
              ↺ reset
            </button>
          )}

          <div className="flex items-center gap-1">
            {SPEEDS.map(s => (
              <button
                key={s}
                onClick={() => onSpeedChange(s)}
                className={[
                  'cursor-pointer h-6 px-2 rounded text-[10px] font-medium border transition-colors',
                  animSpeed === s
                    ? 'bg-slate-800 text-white border-slate-800'
                    : 'text-slate-500 border-slate-200 hover:text-slate-800',
                ].join(' ')}
              >{s}</button>
            ))}
          </div>

          {animStep >= 0 && (
            <span className="text-[10px] text-slate-400 font-mono-display">
              {animProgress} / {animTotal}
            </span>
          )}
        </div>
      ) : (
        <span className="text-[10px] text-slate-400">
          run an algorithm to animate the result
        </span>
      )}
    </div>
  )
}

import { SURFACE } from '../../theme'
import { MAX_DISPLAY_NODES } from '../../utils/parseGraph'
import type { ApiResult, Algorithm } from '../../types'

interface Props {
  maxDegree: number
  avgDegree: number
  density: string
  truncated: boolean
  result: ApiResult | null
  algorithm: Algorithm | null
}

export default function GraphStatsFooter({ maxDegree, avgDegree, density, truncated, result, algorithm }: Props) {
  return (
    <div className="flex items-center justify-between px-4 py-2 border-t border-slate-200 flex-wrap gap-3"
         style={{ background: SURFACE.raised }}>
      <div className="flex items-center gap-4 flex-wrap">
        <StatPill label="renderer" value="WebGL+WASM" />
        <StatPill label="max deg"  value={maxDegree} />
        <StatPill label="avg deg"  value={avgDegree.toFixed(1)} />
        <StatPill label="density"  value={density} />
        {truncated && (
          <StatPill label="cap" value={MAX_DISPLAY_NODES.toLocaleString()} />
        )}
      </div>
      <div className="flex items-center gap-3 flex-wrap">
        <LegendItem color="#0e7490" bg="#ecfeff" label="leaf"      />
        <LegendItem color="#06b6d4" bg="#cffafe" label="mid"       />
        <LegendItem color="#d97706" bg="#fef3c7" label="hub"       />
        <LegendItem color="#ea580c" bg="#fed7aa" label="super-hub" />
        <LegendItem color="#7c3aed" bg="#ede9fe" label="added"     />
        {result && !result.error && <>
          <LegendItem color="#059669" bg="#d1fae5" label="start" />
          {algorithm === 'astar' && <LegendItem color="#dc2626" bg="#fee2e2" label="end" />}
          {algorithm === 'kruskal'
            ? <LegendItem color="#059669" bg="#d1fae5" label="MST"  />
            : <LegendItem color="#0891b2" bg="#cffafe" label="path" />
          }
        </>}
      </div>
    </div>
  )
}

function StatPill({ label, value }: { label: string; value: string | number }) {
  return (
    <span className="flex items-center gap-1.5">
      <span className="text-slate-400 text-[10px] font-semibold uppercase tracking-wide">{label}</span>
      <span className="text-slate-700 text-xs font-mono-display">{value}</span>
    </span>
  )
}

function LegendItem({ color, bg, label }: { color: string; bg: string; label: string }) {
  return (
    <span className="flex items-center gap-1.5">
      <span className="w-3 h-3 rounded-full flex-shrink-0"
            style={{ background: bg, boxShadow: `0 0 0 1.5px ${color}` }} />
      <span className="text-[10px] text-slate-500">{label}</span>
    </span>
  )
}

import { SURFACE } from '../../theme'
import { LAYOUT_BUTTONS, type LayoutMode } from './graphTheme'

interface Props {
  totalNodes: number
  totalEdges: number
  nodeCount: number
  is3D: boolean
  truncated: boolean
  resultLabel: string | null
  showEdges: boolean
  onToggleEdges: () => void
  layoutMode: LayoutMode
  onLayoutChange: (mode: LayoutMode) => void
  onFit: () => void
  onZoomIn: () => void
  onZoomOut: () => void
  onExportPNG: () => void
}

export default function GraphToolbar({
  totalNodes, totalEdges, nodeCount, is3D, truncated, resultLabel,
  showEdges, onToggleEdges, layoutMode, onLayoutChange,
  onFit, onZoomIn, onZoomOut, onExportPNG,
}: Props) {
  return (
    <div className="flex items-center justify-between px-4 py-2 border-b border-slate-200 flex-wrap gap-2"
         style={{ background: SURFACE.raised }}>

      {/* Left: graph stats */}
      <div className="flex items-center gap-2.5 flex-wrap">
        <span className="text-xs font-semibold text-slate-500 uppercase tracking-wide">Graph</span>
        <span className="text-slate-300 text-xs">·</span>
        <span className="text-xs text-slate-600">
          {totalNodes.toLocaleString()} nodes
        </span>
        <span className="text-slate-300 text-xs">·</span>
        <span className="text-xs text-slate-600">
          {totalEdges.toLocaleString()} edges
        </span>
        {is3D && (
          <span className="text-[10px] font-medium text-violet-700 bg-violet-50 border border-violet-200 px-2 py-0.5 rounded-full">
            3D · WebGL
          </span>
        )}
        {truncated && (
          <span className="text-[10px] font-medium text-amber-700 bg-amber-50 border border-amber-200 px-2 py-0.5 rounded-full">
            showing first {nodeCount.toLocaleString()}
          </span>
        )}
        {resultLabel && (
          <span className="text-[10px] font-medium text-cyan-700 bg-cyan-50 border border-cyan-200 px-2 py-0.5 rounded-full">
            {resultLabel}
          </span>
        )}
      </div>

      {/* Right: layout + zoom + export */}
      <div className="flex items-center gap-1.5 flex-wrap">
        {nodeCount > 100 && (
          <button
            onClick={onToggleEdges}
            className={[
              'cursor-pointer px-2.5 py-1 rounded text-xs font-medium border transition-colors',
              showEdges
                ? 'bg-slate-100 text-slate-700 border-slate-300'
                : 'text-slate-500 border-slate-200 hover:text-slate-700',
            ].join(' ')}
          >
            {showEdges ? 'edges on' : 'edges off'}
          </button>
        )}

        {LAYOUT_BUTTONS.map(({ id, label }) => (
          <button
            key={id}
            onClick={() => onLayoutChange(id)}
            className={[
              'cursor-pointer px-2.5 py-1 rounded text-xs font-medium border transition-colors',
              layoutMode === id
                ? id.includes('3d')
                  ? 'bg-violet-50 text-violet-700 border-violet-300'
                  : 'bg-slate-800 text-white border-slate-800'
                : 'text-slate-500 border-slate-200 hover:text-slate-800 hover:border-slate-300',
            ].join(' ')}
          >
            {label}
          </button>
        ))}

        <div className="w-px h-4 bg-slate-200 mx-0.5" />

        <button
          onClick={onFit}
          title="Fit all nodes in view"
          className="cursor-pointer px-2.5 py-1 rounded text-xs font-medium text-slate-500 border border-slate-200 hover:text-slate-800 hover:border-slate-300"
        >fit</button>
        <button
          onClick={onZoomIn}
          className="cursor-pointer w-7 h-7 rounded text-xs font-medium text-slate-500 border border-slate-200 hover:text-slate-800 hover:border-slate-300 flex items-center justify-center"
        >+</button>
        <button
          onClick={onZoomOut}
          className="cursor-pointer w-7 h-7 rounded text-xs font-medium text-slate-500 border border-slate-200 hover:text-slate-800 hover:border-slate-300 flex items-center justify-center"
        >−</button>

        <div className="w-px h-4 bg-slate-200 mx-0.5" />

        <button
          onClick={onExportPNG}
          title="Export as PNG"
          className="cursor-pointer px-2.5 py-1 rounded text-xs font-medium text-slate-500 border border-slate-200 hover:text-emerald-700 hover:border-emerald-300 transition-colors"
        >
          ↓ PNG
        </button>
      </div>
    </div>
  )
}

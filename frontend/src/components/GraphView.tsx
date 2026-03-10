import { useRef, useState, useMemo, useCallback } from 'react'
import { GraphCanvas } from 'reagraph'
import type { GraphCanvasRef, GraphNode, GraphEdge } from 'reagraph'

import type { ParsedGraph } from '../utils/parseGraph'
import { MAX_DISPLAY_NODES } from '../utils/parseGraph'
import type { ApiResult, Algorithm } from '../types'

// ── Types ───────────────────────────────────────────────────────────────────────

interface SelectedNode { id: string; degree: number; distance: string | null }

interface Props {
  parsedGraph: ParsedGraph
  result: ApiResult | null
  algorithm: Algorithm | null
  startNode: string
  endNode: string
}

type LayoutMode =
  | '2d-force'
  | '3d-force'
  | '2d-circular'
  | '2d-radial'
  | '3d-radial'

const LAYOUT_TYPE = {
  '2d-force':   'forceDirected2d',
  '3d-force':   'forceDirected3d',
  '2d-circular':'circular2d',
  '2d-radial':  'radialOut2d',
  '3d-radial':  'radialOut3d',
} as const

const LAYOUT_BUTTONS: { id: LayoutMode; label: string }[] = [
  { id: '2d-force',    label: 'force 2D' },
  { id: '3d-force',    label: 'force 3D' },
  { id: '2d-circular', label: 'circular' },
  { id: '2d-radial',   label: 'radial 2D' },
  { id: '3d-radial',   label: 'radial 3D' },
]

// ── Color / size helpers ────────────────────────────────────────────────────────

function degreeColor(degree: number, maxDeg: number): string {
  const r = maxDeg > 0 ? degree / maxDeg : 0
  if (r > 0.95) return '#f97316'  // orange-red  — super-hub
  if (r > 0.80) return '#f59e0b'  // amber        — hub
  if (r > 0.40) return '#22d3ee'  // cyan-light
  return '#0891b2'                 // cyan-dark    — leaf
}

function degreeSize(degree: number, maxDeg: number): number {
  const r = maxDeg > 0 ? degree / maxDeg : 0
  return 2 + r * 8  // 2 → 10
}

// ── Reagraph dark theme ─────────────────────────────────────────────────────────

const DARK_THEME = {
  canvas: { background: '#020817', fog: false },
  node: {
    fill: '#0891b2',
    activeFill: '#22d3ee',
    opacity: 1,
    selectedOpacity: 1,
    inactiveOpacity: 0.08,
    label: {
      color: '#94a3b8',
      stroke: '#020817',
      activeColor: '#e2e8f0',
      fontSize: 6,
    },
    ring: { fill: '#a78bfa', activeFill: '#7c3aed' },
  },
  edge: {
    fill: '#1e3a5f',
    activeFill: '#22d3ee',
    opacity: 0.55,
    selectedOpacity: 1,
    inactiveOpacity: 0.03,
    label: {
      color: '#475569',
      stroke: '#020817',
      activeColor: '#94a3b8',
      fontSize: 5,
    },
  },
  ring: { fill: '#a78bfa', activeFill: '#7c3aed' },
  arrow: { fill: '#1e3a5f', activeFill: '#22d3ee' },
  lasso: { border: '#a78bfa', background: 'rgba(167,139,250,0.08)' },
  cluster: {
    stroke: '#1e3a5f',
    label: { color: '#475569', stroke: '#020817', fontSize: 10 },
  },
}

// ── Component ───────────────────────────────────────────────────────────────────

export default function GraphView({ parsedGraph, result, algorithm, startNode, endNode }: Props) {
  const graphRef = useRef<GraphCanvasRef | null>(null)

  const [layoutMode,   setLayoutMode]   = useState<LayoutMode>('2d-force')
  const [showEdges,    setShowEdges]    = useState(true)
  const [selectedNode, setSelectedNode] = useState<SelectedNode | null>(null)

  const maxDeg     = Math.max(parsedGraph.maxDegree, 1)
  const nodeCount  = parsedGraph.nodes.length
  const is3D       = layoutMode.startsWith('3d')
  const showLabels = nodeCount <= 60

  // ── Build path/edge sets from algorithm result ─────────────────────────────
  const { pathNodeSet, pathEdgeSet, mstEdgeSet } = useMemo(() => {
    const pathNodeSet = new Set<string>()
    const pathEdgeSet = new Set<string>()
    const mstEdgeSet  = new Set<string>()
    if (!result || result.error) return { pathNodeSet, pathEdgeSet, mstEdgeSet }

    const path = result.path ?? []
    if (algorithm === 'kruskal') {
      for (let i = 0; i + 1 < path.length; i += 2) {
        pathNodeSet.add(String(path[i]))
        pathNodeSet.add(String(path[i + 1]))
        mstEdgeSet.add(`${path[i]}-${path[i + 1]}`)
        mstEdgeSet.add(`${path[i + 1]}-${path[i]}`)
      }
    } else {
      path.forEach(id => pathNodeSet.add(String(id)))
      for (let i = 0; i + 1 < path.length; i++) {
        pathEdgeSet.add(`${path[i]}-${path[i + 1]}`)
        pathEdgeSet.add(`${path[i + 1]}-${path[i]}`)
      }
    }
    return { pathNodeSet, pathEdgeSet, mstEdgeSet }
  }, [result, algorithm])

  // ── Build Reagraph nodes ───────────────────────────────────────────────────
  const graphNodes: GraphNode[] = useMemo(() => {
    return parsedGraph.nodes.map(n => {
      const id     = n.data.id as string
      const degree = n.data.degree as number
      let fill     = degreeColor(degree, maxDeg)

      // Result highlight overrides
      if (pathNodeSet.has(id) && id !== startNode && !(id === endNode && algorithm === 'astar')) {
        fill = '#22d3ee'
      }
      if (startNode && id === startNode) fill = '#4ade80'
      if (endNode   && id === endNode && algorithm === 'astar') fill = '#f87171'

      return {
        id,
        label: id,
        fill,
        size: degreeSize(degree, maxDeg),
        labelVisible: showLabels,
        data: { degree },
      }
    })
  }, [parsedGraph, maxDeg, pathNodeSet, startNode, endNode, algorithm, showLabels])

  // ── Build Reagraph edges ───────────────────────────────────────────────────
  const graphEdges: GraphEdge[] = useMemo(() => {
    if (!showEdges) return []

    return parsedGraph.edges.map(e => {
      const src  = e.data.source as string
      const tgt  = e.data.target as string
      const fwd  = `${src}-${tgt}`
      const rev  = `${tgt}-${src}`
      const isPath = pathEdgeSet.has(fwd) || pathEdgeSet.has(rev)
      const isMst  = mstEdgeSet.has(fwd)  || mstEdgeSet.has(rev)

      return {
        id:     e.data.id as string,
        source: src,
        target: tgt,
        fill:   isMst ? '#4ade80' : isPath ? '#22d3ee' : '#1e3a5f',
        size:   isMst || isPath ? 2 : 0.7,
      }
    })
  }, [parsedGraph, showEdges, pathEdgeSet, mstEdgeSet])

  // ── Active set for dimming non-path nodes ──────────────────────────────────
  const actives = useMemo<string[]>(() => {
    if (!result || result.error || pathNodeSet.size === 0) return []
    return Array.from(pathNodeSet)
  }, [result, pathNodeSet])

  // ── Node click ─────────────────────────────────────────────────────────────
  const handleNodeClick = useCallback((node: GraphNode) => {
    const degree = (node.data as { degree: number }).degree
    let distance: string | null = null

    if (result && !result.error && result.distances) {
      const idx = parsedGraph.idToCompact.get(parseInt(node.id, 10))
      if (idx !== undefined && result.distances[idx] !== undefined) {
        const d = result.distances[idx]
        distance = isFinite(d) && d < 1e14 ? d.toFixed(2) : '∞'
      }
    }
    setSelectedNode({ id: node.id, degree, distance })
  }, [result, parsedGraph])

  // ── Stats ──────────────────────────────────────────────────────────────────
  const density = parsedGraph.totalEdges > 0 && parsedGraph.totalNodes > 1
    ? (parsedGraph.totalEdges / (parsedGraph.totalNodes * (parsedGraph.totalNodes - 1))).toFixed(4)
    : '0'

  const resultLabel = !result || result.error ? null
    : algorithm === 'kruskal'                    ? `${(result.path?.length ?? 0) / 2} MST edges`
    : algorithm === 'bfs' || algorithm === 'dfs' ? `${result.path?.length ?? 0} nodes visited`
    : (result.path?.length ?? 0) > 0            ? `Path: ${result.path!.length} nodes`
    : null

  // ── Render ─────────────────────────────────────────────────────────────────
  return (
    <div className="rounded-xl border border-zinc-800 overflow-hidden flex flex-col" style={{ background: '#0a0f1e' }}>

      {/* ── Top toolbar ── */}
      <div className="flex items-center justify-between px-4 py-2.5 border-b border-zinc-800/80 flex-wrap gap-2"
           style={{ background: '#0d1425' }}>
        <div className="flex items-center gap-2.5 flex-wrap">
          <span className="text-xs font-mono-display text-zinc-400 uppercase tracking-widest">Graph</span>
          <span className="text-zinc-700 text-xs">·</span>
          <span className="text-xs font-mono-display text-zinc-500">
            {parsedGraph.totalNodes.toLocaleString()} nodes
          </span>
          <span className="text-zinc-700 text-xs">·</span>
          <span className="text-xs font-mono-display text-zinc-500">
            {parsedGraph.totalEdges.toLocaleString()} edges
          </span>

          {is3D && (
            <span className="text-xs font-mono-display text-violet-400 bg-violet-950/40 border border-violet-800/50 px-2 py-0.5 rounded-full">
              3D · WebGL
            </span>
          )}
          {parsedGraph.truncated && (
            <span className="text-xs font-mono-display text-amber-400 bg-amber-950/40 border border-amber-800/50 px-2 py-0.5 rounded-full">
              showing first {nodeCount.toLocaleString()}
            </span>
          )}
          {resultLabel && (
            <span className="text-xs font-mono-display text-cyan-300 bg-cyan-950/40 border border-cyan-800/50 px-2 py-0.5 rounded-full">
              {resultLabel}
            </span>
          )}
        </div>

        {/* Controls */}
        <div className="flex items-center gap-1.5 flex-wrap">
          {nodeCount > 100 && (
            <button
              onClick={() => setShowEdges(v => !v)}
              className={[
                'px-2.5 py-1 rounded text-xs font-mono-display border transition-colors',
                showEdges
                  ? 'bg-zinc-700/60 text-zinc-200 border-zinc-600'
                  : 'text-zinc-500 border-zinc-800 hover:text-zinc-300',
              ].join(' ')}
            >
              {showEdges ? 'hide edges' : 'show edges'}
            </button>
          )}

          {LAYOUT_BUTTONS.map(({ id, label }) => (
            <button
              key={id}
              onClick={() => setLayoutMode(id)}
              className={[
                'px-2.5 py-1 rounded text-xs font-mono-display border transition-colors',
                layoutMode === id
                  ? id.includes('3d')
                    ? 'bg-violet-900/60 text-violet-200 border-violet-700'
                    : 'bg-zinc-700 text-zinc-100 border-zinc-500'
                  : 'text-zinc-500 border-zinc-800 hover:text-zinc-300 hover:border-zinc-700',
              ].join(' ')}
            >
              {label}
            </button>
          ))}

          <button
            onClick={() => graphRef.current?.fitNodesInView()}
            className="px-2.5 py-1 rounded text-xs font-mono-display text-zinc-500 border border-zinc-800 hover:text-zinc-200 hover:border-zinc-600 ml-1"
          >
            fit
          </button>
          <button
            onClick={() => graphRef.current?.zoomIn()}
            className="w-7 h-7 rounded text-xs font-mono-display text-zinc-400 border border-zinc-800 hover:text-zinc-200 hover:border-zinc-600 flex items-center justify-center"
          >+</button>
          <button
            onClick={() => graphRef.current?.zoomOut()}
            className="w-7 h-7 rounded text-xs font-mono-display text-zinc-400 border border-zinc-800 hover:text-zinc-200 hover:border-zinc-600 flex items-center justify-center"
          >−</button>
        </div>
      </div>

      {/* ── WebGL canvas ── */}
      <div className="relative" style={{ height: '580px', background: '#020817' }}>
        <GraphCanvas
          ref={graphRef}
          nodes={graphNodes}
          edges={graphEdges}
          layoutType={LAYOUT_TYPE[layoutMode] as Parameters<typeof GraphCanvas>[0]['layoutType']}
          actives={actives.length > 0 ? actives : undefined}
          theme={DARK_THEME as Parameters<typeof GraphCanvas>[0]['theme']}
          onNodeClick={handleNodeClick}
          onCanvasClick={() => setSelectedNode(null)}
        />

        {/* 3D mode hint */}
        {is3D && (
          <div className="absolute top-3 left-3 text-[10px] font-mono-display text-zinc-600 pointer-events-none">
            drag to orbit · scroll to zoom · right-drag to pan
          </div>
        )}

        {/* Node info panel */}
        {selectedNode && (
          <div
            className="absolute bottom-4 right-4 rounded-lg border border-zinc-700/80 px-4 py-3 text-xs font-mono-display space-y-1.5 z-10"
            style={{ background: 'rgba(13,20,37,0.95)', backdropFilter: 'blur(8px)' }}
          >
            <div className="flex items-center justify-between gap-6 mb-0.5">
              <span className="text-zinc-400 uppercase tracking-widest text-[10px]">Node Info</span>
              <button
                onClick={() => setSelectedNode(null)}
                className="text-zinc-600 hover:text-zinc-300 leading-none"
              >×</button>
            </div>
            <div className="flex justify-between gap-8">
              <span className="text-zinc-500">ID</span>
              <span className="text-zinc-200">{selectedNode.id}</span>
            </div>
            <div className="flex justify-between gap-8">
              <span className="text-zinc-500">Degree</span>
              <span className="text-cyan-300">{selectedNode.degree}</span>
            </div>
            {selectedNode.distance !== null && (
              <div className="flex justify-between gap-8">
                <span className="text-zinc-500">Distance</span>
                <span className="text-emerald-300">{selectedNode.distance}</span>
              </div>
            )}
          </div>
        )}
      </div>

      {/* ── Bottom stats + legend ── */}
      <div className="flex items-center justify-between px-4 py-2 border-t border-zinc-800/80 flex-wrap gap-3"
           style={{ background: '#0d1425' }}>
        <div className="flex items-center gap-4 flex-wrap">
          <StatPill label="renderer" value="WebGL" />
          <StatPill label="max deg"  value={parsedGraph.maxDegree} />
          <StatPill label="avg deg"  value={parsedGraph.avgDegree.toFixed(1)} />
          <StatPill label="density"  value={density} />
          {parsedGraph.truncated && (
            <StatPill label="cap" value={MAX_DISPLAY_NODES.toLocaleString()} />
          )}
        </div>

        <div className="flex items-center gap-3 flex-wrap">
          <LegendItem color="#0891b2" bg="#0c1a2e" label="leaf"      />
          <LegendItem color="#22d3ee" bg="#0f1f3d" label="mid"       />
          <LegendItem color="#f59e0b" bg="#78350f" label="hub"       />
          <LegendItem color="#f97316" bg="#7c2d12" label="super-hub" />
          {result && !result.error && <>
            <LegendItem color="#4ade80" bg="#14532d" label="start" />
            {algorithm === 'astar' && (
              <LegendItem color="#f87171" bg="#450a0a" label="end" />
            )}
            {algorithm === 'kruskal'
              ? <LegendItem color="#4ade80" bg="#052e16" label="MST"  />
              : <LegendItem color="#22d3ee" bg="#164e63" label="path" />
            }
          </>}
        </div>
      </div>
    </div>
  )
}

// ── Small helpers ───────────────────────────────────────────────────────────────

function StatPill({ label, value }: { label: string; value: string | number }) {
  return (
    <span className="flex items-center gap-1.5">
      <span className="text-zinc-600 text-[10px] font-mono-display uppercase tracking-wider">{label}</span>
      <span className="text-zinc-300 text-xs font-mono-display">{value}</span>
    </span>
  )
}

function LegendItem({ color, bg, label }: { color: string; bg: string; label: string }) {
  return (
    <span className="flex items-center gap-1.5">
      <span
        className="w-3 h-3 rounded-full flex-shrink-0"
        style={{ background: bg, boxShadow: `0 0 0 1.5px ${color}` }}
      />
      <span className="text-[10px] font-mono-display text-zinc-500">{label}</span>
    </span>
  )
}

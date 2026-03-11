import { useRef, useState, useMemo, useCallback, useEffect } from 'react'
import { GraphCanvas } from 'reagraph'
import type { GraphCanvasRef, GraphNode, GraphEdge, InternalGraphNode } from 'reagraph'

import type { ParsedGraph } from '../utils/parseGraph'
import { MAX_DISPLAY_NODES, getWasm } from '../utils/parseGraph'
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
  '2d-force':    'forceDirected2d',
  '3d-force':    'forceDirected3d',
  '2d-circular': 'circular2d',
  '2d-radial':   'radialOut2d',
  '3d-radial':   'radialOut3d',
} as const

const LAYOUT_BUTTONS: { id: LayoutMode; label: string }[] = [
  { id: '2d-force',    label: 'force 2D' },
  { id: '3d-force',    label: 'force 3D' },
  { id: '2d-circular', label: 'circular' },
  { id: '2d-radial',   label: 'radial 2D' },
  { id: '3d-radial',   label: 'radial 3D' },
]

// ── JS fallback color/size helpers (used when WASM not yet loaded) ──────────────

function degreeColor(degree: number, maxDeg: number): string {
  const r = maxDeg > 0 ? degree / maxDeg : 0
  if (r > 0.95) return '#f97316'
  if (r > 0.80) return '#f59e0b'
  if (r > 0.40) return '#22d3ee'
  return '#0891b2'
}

function degreeSize(degree: number, maxDeg: number): number {
  const r = maxDeg > 0 ? degree / maxDeg : 0
  return 2 + r * 8
}

// ── Dark theme ──────────────────────────────────────────────────────────────────

const DARK_THEME = {
  canvas: { background: '#020817', fog: null },
  node: {
    fill: '#0891b2',
    activeFill: '#22d3ee',
    opacity: 1,
    selectedOpacity: 1,
    inactiveOpacity: 0.07,
    label: { color: '#94a3b8', stroke: '#020817', activeColor: '#e2e8f0', fontSize: 6 },
    ring: { fill: '#a78bfa', activeFill: '#7c3aed' },
  },
  edge: {
    fill: '#1e3a5f',
    activeFill: '#22d3ee',
    opacity: 0.55,
    selectedOpacity: 1,
    inactiveOpacity: 0.03,
    label: { color: '#475569', stroke: '#020817', activeColor: '#94a3b8', fontSize: 5 },
  },
  ring:  { fill: '#a78bfa', activeFill: '#7c3aed' },
  arrow: { fill: '#1e3a5f', activeFill: '#22d3ee' },
  lasso: { border: '#a78bfa', background: 'rgba(167,139,250,0.08)' },
  cluster: { stroke: '#1e3a5f', label: { color: '#475569', stroke: '#020817', fontSize: 10 } },
}

const SCC_FILLS = ['#22d3ee','#a855f7','#f59e0b','#4ade80','#f87171','#38bdf8','#fb923c']

// ── Component ───────────────────────────────────────────────────────────────────

export default function GraphView({ parsedGraph, result, algorithm, startNode, endNode }: Props) {
  const graphRef     = useRef<GraphCanvasRef | null>(null)
  const animTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  // ── UI state ───────────────────────────────────────────────────────────────
  const [layoutMode,   setLayoutMode]   = useState<LayoutMode>('2d-force')
  const [showEdges,    setShowEdges]    = useState(true)
  const [selectedNode, setSelectedNode] = useState<SelectedNode | null>(null)

  // ── Animation state ────────────────────────────────────────────────────────
  const [animStep,  setAnimStep]  = useState(-1)
  const [isPlaying, setIsPlaying] = useState(false)
  const [animSpeed, setAnimSpeed] = useState<'fast' | 'medium' | 'slow'>('medium')

  const SPEED_MS = { fast: 40, medium: 100, slow: 280 }

  // ── Search state ───────────────────────────────────────────────────────────
  const [searchInput, setSearchInput] = useState('')
  const [searchError, setSearchError] = useState(false)

  // ── Hover state ────────────────────────────────────────────────────────────
  const [hoverNodeId, setHoverNodeId] = useState<string | null>(null)

  // ── Derived ────────────────────────────────────────────────────────────────
  const maxDeg     = Math.max(parsedGraph.maxDegree, 1)
  const nodeCount  = parsedGraph.nodes.length
  const is3D       = layoutMode.startsWith('3d')
  const showLabels = nodeCount <= 60

  // ── Neighbor map — pre-computed by WASM in worker, no JS work needed ────────
  // parsedGraph.neighborMap is Record<string, string[]> — use directly
  const neighborMap = parsedGraph.neighborMap

  // ── Full path sets (WASM or JS fallback) ──────────────────────────────────
  const { pathNodeSet, pathEdgeSet, mstEdgeSet, pathArray } = useMemo(() => {
    const empty = { pathNodeSet: new Set<string>(), pathEdgeSet: new Set<string>(), mstEdgeSet: new Set<string>(), pathArray: [] as string[] }
    if (!result || result.error) return empty

    const path = result.path ?? []
    if (path.length === 0) return empty

    const wasm = getWasm()
    if (wasm) {
      try {
        const r = JSON.parse(wasm.build_path_sets(JSON.stringify(path), algorithm ?? '')) as {
          path_node_set: string[]
          path_edge_set: string[]
          mst_edge_set:  string[]
          path_array:    string[]
        }
        return {
          pathNodeSet: new Set(r.path_node_set),
          pathEdgeSet: new Set(r.path_edge_set),
          mstEdgeSet:  new Set(r.mst_edge_set),
          pathArray:   r.path_array,
        }
      } catch { /* fall through */ }
    }

    // JS fallback
    const pathNodeSet = new Set<string>()
    const pathEdgeSet = new Set<string>()
    const mstEdgeSet  = new Set<string>()
    if (algorithm === 'kruskal') {
      for (let i = 0; i + 1 < path.length; i += 2) {
        pathNodeSet.add(String(path[i])); pathNodeSet.add(String(path[i + 1]))
        mstEdgeSet.add(`${path[i]}-${path[i + 1]}`); mstEdgeSet.add(`${path[i + 1]}-${path[i]}`)
      }
    } else {
      path.forEach(id => pathNodeSet.add(String(id)))
      for (let i = 0; i + 1 < path.length; i++) {
        pathEdgeSet.add(`${path[i]}-${path[i + 1]}`); pathEdgeSet.add(`${path[i + 1]}-${path[i]}`)
      }
    }
    return { pathNodeSet, pathEdgeSet, mstEdgeSet, pathArray: path.map(String) }
  }, [result, algorithm])

  // ── Reset animation when result changes ───────────────────────────────────
  useEffect(() => {
    setIsPlaying(false)
    setAnimStep(-1)
    if (animTimerRef.current) clearTimeout(animTimerRef.current)
  }, [result])

  // ── Animation tick ────────────────────────────────────────────────────────
  useEffect(() => {
    if (!isPlaying) return
    const total = algorithm === 'kruskal' ? Math.ceil(pathArray.length / 2) : pathArray.length
    if (animStep >= total) { setIsPlaying(false); return }
    animTimerRef.current = setTimeout(() => setAnimStep(s => s + 1), SPEED_MS[animSpeed])
    return () => { if (animTimerRef.current) clearTimeout(animTimerRef.current) }
  }, [isPlaying, animStep, animSpeed, pathArray, algorithm])

  // ── Visible path nodes — respects animation step ──────────────────────────
  const visiblePathNodes = useMemo<Set<string>>(() => {
    if (animStep < 0) return pathNodeSet
    const set = new Set<string>()
    if (algorithm === 'kruskal') {
      const limit = animStep * 2
      for (let i = 0; i < Math.min(limit, pathArray.length); i++) set.add(pathArray[i])
    } else {
      for (let i = 0; i < Math.min(animStep, pathArray.length); i++) set.add(pathArray[i])
    }
    return set
  }, [animStep, pathNodeSet, pathArray, algorithm])

  // ── Animation controls ────────────────────────────────────────────────────
  const startAnimation = useCallback(() => {
    if (pathArray.length === 0) return
    setAnimStep(0); setIsPlaying(true)
  }, [pathArray])

  const togglePlayPause = useCallback(() => {
    if (!isPlaying && animStep < 0) { startAnimation(); return }
    setIsPlaying(p => !p)
  }, [isPlaying, animStep, startAnimation])

  const resetAnimation = useCallback(() => {
    setIsPlaying(false); setAnimStep(-1)
    if (animTimerRef.current) clearTimeout(animTimerRef.current)
  }, [])

  // ── Node search ───────────────────────────────────────────────────────────
  const handleSearch = useCallback((e: React.FormEvent) => {
    e.preventDefault()
    const id = searchInput.trim()
    if (!id) return
    const exists = parsedGraph.nodes.some(n => (n.data.id as string) === id)
    if (!exists) { setSearchError(true); setTimeout(() => setSearchError(false), 1500); return }
    setSearchError(false)
    graphRef.current?.centerGraph([id])
    const degree = parsedGraph.nodes.find(n => n.data.id === id)?.data.degree as number ?? 0
    setSelectedNode({ id, degree, distance: null })
  }, [searchInput, parsedGraph])

  // ── PNG export ────────────────────────────────────────────────────────────
  const exportPNG = useCallback(() => {
    const dataUrl = graphRef.current?.exportCanvas()
    if (!dataUrl) return
    const link = document.createElement('a')
    link.download = 'graph.png'
    link.href = dataUrl
    link.click()
  }, [])

  // PageRank map for WASM / JS fallback
  const pageRankMap = useMemo<Map<string, number>>(() => {
    const map = new Map<string, number>()
    if (algorithm !== 'pagerank' || !result?.scores) return map
    const maxScore = result.scores[0]?.[1] ?? 1
    for (const [id, score] of result.scores) map.set(String(id), score / maxScore)
    return map
  }, [algorithm, result])

  // SCC map for JS fallback
  const sccColorMap = useMemo<Map<string, number>>(() => {
    const map = new Map<string, number>()
    if (algorithm !== 'scc' || !result?.components) return map
    result.components.forEach((comp, idx) => comp.forEach(id => map.set(String(id), idx)))
    return map
  }, [algorithm, result])

  // ── Build nodes — WASM compute_node_styles, JS fallback ───────────────────
  const graphNodes: GraphNode[] = useMemo(() => {
    const wasm = getWasm()
    const algo = algorithm ?? ''
    const visibleArr = Array.from(visiblePathNodes)

    if (wasm) {
      try {
        const nodesJson      = JSON.stringify(parsedGraph.nodes.map(n => ({ id: n.data.id, degree: n.data.degree })))
        const pagerankJson   = algorithm === 'pagerank' && result?.scores ? JSON.stringify(result.scores) : '[]'
        const sccJson        = algorithm === 'scc' && result?.components ? JSON.stringify(result.components) : '[]'
        const pathNodesJson  = JSON.stringify(visibleArr)

        const styles = JSON.parse(wasm.compute_node_styles(
          nodesJson, maxDeg, algo, pagerankJson, sccJson, pathNodesJson, startNode, endNode,
        )) as { id: string; fill: string; size: number }[]

        return styles.map(s => ({
          id: s.id, label: s.id, fill: s.fill, size: s.size,
          labelVisible: showLabels,
          data: { degree: parsedGraph.nodes.find(n => n.data.id === s.id)?.data.degree ?? 0 },
        }))
      } catch { /* fall through */ }
    }

    // JS fallback
    return parsedGraph.nodes.map(n => {
      const id     = n.data.id as string
      const degree = n.data.degree as number
      let fill     = degreeColor(degree, maxDeg)

      if (algorithm === 'pagerank') {
        const rank = pageRankMap.get(id) ?? 0
        if      (rank > 0.8) fill = '#a855f7'
        else if (rank > 0.5) fill = '#7c3aed'
        else if (rank > 0.2) fill = '#22d3ee'
        else                 fill = '#0891b2'
      } else if (algorithm === 'scc' && sccColorMap.size > 0) {
        const idx = sccColorMap.get(id)
        fill = idx !== undefined ? SCC_FILLS[idx % SCC_FILLS.length] : '#334155'
      } else {
        if (visiblePathNodes.has(id) && id !== startNode && !(id === endNode && algorithm === 'astar')) fill = '#22d3ee'
        if (startNode && id === startNode && visiblePathNodes.has(id)) fill = '#4ade80'
        if (endNode   && id === endNode && algorithm === 'astar' && visiblePathNodes.has(id)) fill = '#f87171'
      }

      return { id, label: id, fill, size: degreeSize(degree, maxDeg), labelVisible: showLabels, data: { degree } }
    })
  }, [parsedGraph, maxDeg, visiblePathNodes, startNode, endNode, algorithm, showLabels, pageRankMap, sccColorMap, result])

  // ── Build edges — WASM compute_edge_styles, JS fallback ───────────────────
  const graphEdges: GraphEdge[] = useMemo(() => {
    if (!showEdges) return []

    const wasm = getWasm()
    if (wasm && (pathEdgeSet.size > 0 || mstEdgeSet.size > 0)) {
      try {
        const edgesJson   = JSON.stringify(parsedGraph.edges.map(e => ({ id: e.data.id, source: e.data.source, target: e.data.target })))
        const pathSetJson = JSON.stringify(Array.from(pathEdgeSet))
        const mstSetJson  = JSON.stringify(Array.from(mstEdgeSet))
        const styles = JSON.parse(wasm.compute_edge_styles(edgesJson, pathSetJson, mstSetJson)) as { id: string; fill: string; size: number }[]
        return styles.map(s => ({
          id:     s.id,
          source: parsedGraph.edges.find(e => e.data.id === s.id)?.data.source as string,
          target: parsedGraph.edges.find(e => e.data.id === s.id)?.data.target as string,
          fill:   s.fill,
          size:   s.size,
        }))
      } catch { /* fall through */ }
    }

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

  // ── Active (dimming) set ───────────────────────────────────────────────────
  const actives = useMemo<string[] | undefined>(() => {
    if (hoverNodeId) {
      const neighbors = neighborMap[hoverNodeId] ?? []
      return [hoverNodeId, ...neighbors]
    }
    if (algorithm === 'pagerank' || algorithm === 'scc') return undefined
    if (!result || result.error || visiblePathNodes.size === 0) return undefined
    return Array.from(visiblePathNodes)
  }, [hoverNodeId, neighborMap, result, visiblePathNodes, algorithm])

  // ── Node interaction ───────────────────────────────────────────────────────
  const handleNodeClick = useCallback((node: InternalGraphNode) => {
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

  const handleNodePointerOver = useCallback((node: InternalGraphNode) => setHoverNodeId(node.id), [])
  const handleNodePointerOut  = useCallback((_node: InternalGraphNode) => setHoverNodeId(null), [])

  // ── Stats ──────────────────────────────────────────────────────────────────
  const density = parsedGraph.totalEdges > 0 && parsedGraph.totalNodes > 1
    ? (parsedGraph.totalEdges / (parsedGraph.totalNodes * (parsedGraph.totalNodes - 1))).toFixed(4)
    : '0'

  const resultLabel = !result || result.error ? null
    : algorithm === 'kruskal'                    ? `${(result.path?.length ?? 0) / 2} MST edges`
    : algorithm === 'bfs' || algorithm === 'dfs' ? `${result.path?.length ?? 0} nodes visited`
    : (result.path?.length ?? 0) > 0            ? `Path: ${result.path!.length} nodes`
    : null

  const hasResult    = !!result && !result.error && pathArray.length > 0
  const animTotal    = algorithm === 'kruskal' ? Math.ceil(pathArray.length / 2) : pathArray.length
  const animProgress = animStep >= 0 ? Math.min(animStep, animTotal) : animTotal

  // ── Render ─────────────────────────────────────────────────────────────────
  return (
    <div className="h-full flex flex-col overflow-hidden" style={{ background: '#0a0f1e' }}>

      {/* ── Toolbar row 1: info + layout controls ── */}
      <div className="flex items-center justify-between px-4 py-2 border-b border-zinc-800/80 flex-wrap gap-2"
           style={{ background: '#0d1425' }}>

        {/* Left: graph stats */}
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
            <span className="text-[10px] font-mono-display text-violet-400 bg-violet-950/40 border border-violet-800/50 px-2 py-0.5 rounded-full">
              3D · WebGL
            </span>
          )}
          {parsedGraph.truncated && (
            <span className="text-[10px] font-mono-display text-amber-400 bg-amber-950/40 border border-amber-800/50 px-2 py-0.5 rounded-full">
              showing first {nodeCount.toLocaleString()}
            </span>
          )}
          {resultLabel && (
            <span className="text-[10px] font-mono-display text-cyan-300 bg-cyan-950/40 border border-cyan-800/50 px-2 py-0.5 rounded-full">
              {resultLabel}
            </span>
          )}
        </div>

        {/* Right: layout + zoom + export */}
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
              {showEdges ? 'edges on' : 'edges off'}
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

          <div className="w-px h-4 bg-zinc-800 mx-0.5" />

          <button
            onClick={() => graphRef.current?.fitNodesInView?.()}
            title="Fit all nodes in view"
            className="px-2.5 py-1 rounded text-xs font-mono-display text-zinc-500 border border-zinc-800 hover:text-zinc-200 hover:border-zinc-600"
          >fit</button>
          <button
            onClick={() => graphRef.current?.zoomIn?.()}
            className="w-7 h-7 rounded text-xs font-mono-display text-zinc-400 border border-zinc-800 hover:text-zinc-200 hover:border-zinc-600 flex items-center justify-center"
          >+</button>
          <button
            onClick={() => graphRef.current?.zoomOut?.()}
            className="w-7 h-7 rounded text-xs font-mono-display text-zinc-400 border border-zinc-800 hover:text-zinc-200 hover:border-zinc-600 flex items-center justify-center"
          >−</button>

          <div className="w-px h-4 bg-zinc-800 mx-0.5" />

          <button
            onClick={exportPNG}
            title="Export as PNG"
            className="px-2.5 py-1 rounded text-xs font-mono-display text-zinc-500 border border-zinc-800 hover:text-emerald-300 hover:border-emerald-800 transition-colors"
          >
            ↓ PNG
          </button>
        </div>
      </div>

      {/* ── Toolbar row 2: search + animation ── */}
      <div className="flex items-center gap-3 px-4 py-2 border-b border-zinc-800/50 flex-wrap"
           style={{ background: '#0b1020' }}>

        {/* Node search */}
        <form onSubmit={handleSearch} className="flex items-center gap-2">
          <input
            type="text"
            value={searchInput}
            onChange={e => { setSearchInput(e.target.value); setSearchError(false) }}
            placeholder="Jump to node ID…"
            className={[
              'h-7 px-3 rounded text-xs font-mono-display border bg-zinc-900/80 outline-none transition-colors w-36',
              searchError
                ? 'border-red-600 text-red-400 placeholder-red-800'
                : 'border-zinc-700 text-zinc-200 placeholder-zinc-600 focus:border-cyan-600',
            ].join(' ')}
          />
          <button
            type="submit"
            className="h-7 px-3 rounded text-xs font-mono-display text-zinc-400 border border-zinc-700 hover:text-cyan-300 hover:border-cyan-700 transition-colors"
          >
            find
          </button>
          {searchError && (
            <span className="text-[10px] font-mono-display text-red-500">not found</span>
          )}
        </form>

        <div className="w-px h-4 bg-zinc-800" />

        {/* Animation controls */}
        {hasResult ? (
          <div className="flex items-center gap-2 flex-wrap">
            <span className="text-[10px] font-mono-display text-zinc-600 uppercase tracking-wider">animate</span>

            <button
              onClick={togglePlayPause}
              className={[
                'h-7 px-3 rounded text-xs font-mono-display border transition-colors',
                isPlaying
                  ? 'bg-amber-900/40 text-amber-300 border-amber-700'
                  : 'bg-cyan-900/30 text-cyan-300 border-cyan-800 hover:bg-cyan-900/50',
              ].join(' ')}
            >
              {isPlaying ? '⏸ pause' : animStep >= 0 ? '▶ resume' : '▶ play'}
            </button>

            {animStep >= 0 && (
              <button
                onClick={resetAnimation}
                className="h-7 px-2.5 rounded text-xs font-mono-display text-zinc-500 border border-zinc-800 hover:text-zinc-300 transition-colors"
              >
                ↺ reset
              </button>
            )}

            <div className="flex items-center gap-1">
              {(['fast', 'medium', 'slow'] as const).map(s => (
                <button
                  key={s}
                  onClick={() => setAnimSpeed(s)}
                  className={[
                    'h-6 px-2 rounded text-[10px] font-mono-display border transition-colors',
                    animSpeed === s
                      ? 'bg-zinc-700 text-zinc-200 border-zinc-500'
                      : 'text-zinc-600 border-zinc-800 hover:text-zinc-400',
                  ].join(' ')}
                >{s}</button>
              ))}
            </div>

            {animStep >= 0 && (
              <span className="text-[10px] font-mono-display text-zinc-600">
                {animProgress} / {animTotal}
              </span>
            )}
          </div>
        ) : (
          <span className="text-[10px] font-mono-display text-zinc-700">
            run an algorithm to animate the result
          </span>
        )}
      </div>

      {/* ── WebGL canvas ── */}
      <div
        className="relative flex-1 overflow-hidden"
        style={{ background: '#020817', minHeight: 0 }}
      >
        <div className="absolute inset-0">
          <GraphCanvas
            ref={graphRef}
            nodes={graphNodes}
            edges={graphEdges}
            layoutType={LAYOUT_TYPE[layoutMode] as Parameters<typeof GraphCanvas>[0]['layoutType']}
            actives={actives}
            theme={DARK_THEME as Parameters<typeof GraphCanvas>[0]['theme']}
            onNodeClick={handleNodeClick}
            onNodePointerOver={handleNodePointerOver}
            onNodePointerOut={handleNodePointerOut}
            onCanvasClick={() => setSelectedNode(null)}
          />
        </div>

        {is3D && (
          <div className="absolute top-3 left-3 text-[10px] font-mono-display text-zinc-700 pointer-events-none select-none">
            drag to orbit · scroll to zoom · right-drag to pan
          </div>
        )}

        {hoverNodeId && !selectedNode && (
          <div className="absolute top-3 right-3 text-[10px] font-mono-display text-zinc-600 pointer-events-none select-none">
            node {hoverNodeId} · {(neighborMap[hoverNodeId] ?? []).length} neighbors
          </div>
        )}

        {selectedNode && (
          <div
            className="absolute bottom-4 right-4 rounded-lg border border-zinc-700/80 px-4 py-3 text-xs font-mono-display space-y-1.5 z-10"
            style={{ background: 'rgba(13,20,37,0.96)', backdropFilter: 'blur(8px)' }}
          >
            <div className="flex items-center justify-between gap-6 mb-0.5">
              <span className="text-zinc-400 uppercase tracking-widest text-[10px]">Node</span>
              <button onClick={() => setSelectedNode(null)} className="text-zinc-600 hover:text-zinc-300 leading-none">×</button>
            </div>
            <div className="flex justify-between gap-8">
              <span className="text-zinc-500">ID</span>
              <span className="text-zinc-200">{selectedNode.id}</span>
            </div>
            <div className="flex justify-between gap-8">
              <span className="text-zinc-500">Degree</span>
              <span className="text-cyan-300">{selectedNode.degree}</span>
            </div>
            <div className="flex justify-between gap-8">
              <span className="text-zinc-500">Neighbors</span>
              <span className="text-zinc-400">{(neighborMap[selectedNode.id] ?? []).length}</span>
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
          <StatPill label="renderer" value="WebGL+WASM" />
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
            {algorithm === 'astar' && <LegendItem color="#f87171" bg="#450a0a" label="end" />}
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
      <span className="w-3 h-3 rounded-full flex-shrink-0"
            style={{ background: bg, boxShadow: `0 0 0 1.5px ${color}` }} />
      <span className="text-[10px] font-mono-display text-zinc-500">{label}</span>
    </span>
  )
}

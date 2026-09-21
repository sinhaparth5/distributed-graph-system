import { useRef, useState, useMemo, useCallback, useEffect } from 'react'
import type { GraphCanvasRef, GraphNode, GraphEdge, InternalGraphNode } from 'reagraph'

import type { ParsedGraph } from '../utils/parseGraph'
import { getWasm } from '../utils/parseGraph'
import type { ApiResult, Algorithm } from '../types'
import { SURFACE } from '../theme'

import GraphToolbar from './graph/GraphToolbar'
import GraphSearchAnimBar from './graph/GraphSearchAnimBar'
import GraphCanvasPanel, { type SelectedNode } from './graph/GraphCanvasPanel'
import GraphStatsFooter from './graph/GraphStatsFooter'
import { nextEditId, type PinnedPosition } from './graph/graphEdit'
import {
  degreeColor, degreeSize, SCC_FILLS, SPEED_MS, type LayoutMode,
} from './graph/graphTheme'

interface Props {
  parsedGraph: ParsedGraph
  result: ApiResult | null
  algorithm: Algorithm | null
  startNode: string
  endNode: string
  /** Called whenever the user edits the graph (add/delete/connect) — the current result no longer matches the topology it ran against. */
  onGraphEdited?: () => void
}

// ── Component ───────────────────────────────────────────────────────────────────

export default function GraphView({ parsedGraph, result, algorithm, startNode, endNode, onGraphEdited }: Props) {
  const graphRef     = useRef<GraphCanvasRef | null>(null)
  const animTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  // ── UI state ───────────────────────────────────────────────────────────────
  const [layoutMode,   setLayoutMode]   = useState<LayoutMode>('2d-force')
  const [showEdges,    setShowEdges]    = useState(true)
  const [selectedNode, setSelectedNode] = useState<SelectedNode | null>(null)
  const [multiSelected, setMultiSelected] = useState<string[]>([])

  // ── Editing state ─────────────────────────────────────────────────────────
  const [customNodes,   setCustomNodes]   = useState<GraphNode[]>([])
  const [customEdges,   setCustomEdges]   = useState<GraphEdge[]>([])
  const [removedNodeIds, setRemovedNodeIds] = useState<Set<string>>(new Set())
  const [removedEdgeIds, setRemovedEdgeIds] = useState<Set<string>>(new Set())
  const [pinned, setPinned] = useState<Map<string, PinnedPosition>>(new Map())
  const [editMode, setEditMode] = useState<'view' | 'connect'>('view')
  const [connectFromId, setConnectFromId] = useState<string | null>(null)

  // ── Animation state ────────────────────────────────────────────────────────
  const [animStep,  setAnimStep]  = useState(-1)
  const [isPlaying, setIsPlaying] = useState(false)
  const [animSpeed, setAnimSpeed] = useState<'fast' | 'medium' | 'slow'>('medium')

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

  // ── Reset animation when result changes (adjust state during render, not an effect) ──
  const [prevResult, setPrevResult] = useState(result)
  if (result !== prevResult) {
    setPrevResult(result)
    setIsPlaying(false)
    setAnimStep(-1)
  }

  const animTotal = algorithm === 'kruskal' ? Math.ceil(pathArray.length / 2) : pathArray.length

  // Stop playback once the animation reaches the end (clamp during render, same pattern as above)
  if (isPlaying && animStep >= animTotal) {
    setIsPlaying(false)
  }

  // ── Animation tick ────────────────────────────────────────────────────────
  useEffect(() => {
    if (!isPlaying || animStep >= animTotal) return
    animTimerRef.current = setTimeout(() => setAnimStep(s => s + 1), SPEED_MS[animSpeed])
    return () => { if (animTimerRef.current) clearTimeout(animTimerRef.current) }
  }, [isPlaying, animStep, animSpeed, animTotal])

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

  // PageRank map for JS fallback
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

  // ── Base node styles — WASM compute_node_styles, JS fallback ──────────────
  // WASM returns styles in the same order as the `nodes` array we sent it, so
  // we zip by index instead of `.find()`-ing every node (O(n²) → O(n)).
  // Kept separate from the edit overlay below so dragging/pinning/deleting
  // doesn't re-trigger this WASM/JS pass.
  const baseNodeStyles: GraphNode[] = useMemo(() => {
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

        return styles.map((s, i) => ({
          id: s.id, label: s.id, fill: s.fill, size: s.size,
          labelVisible: showLabels,
          data: { degree: parsedGraph.nodes[i].data.degree as number },
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
        if      (rank > 0.8) fill = '#7c3aed'
        else if (rank > 0.5) fill = '#8b5cf6'
        else if (rank > 0.2) fill = '#06b6d4'
        else                 fill = '#0e7490'
      } else if (algorithm === 'scc' && sccColorMap.size > 0) {
        const idx = sccColorMap.get(id)
        fill = idx !== undefined ? SCC_FILLS[idx % SCC_FILLS.length] : '#94a3b8'
      } else {
        if (visiblePathNodes.has(id) && id !== startNode && !(id === endNode && algorithm === 'astar')) fill = '#0891b2'
        if (startNode && id === startNode && visiblePathNodes.has(id)) fill = '#059669'
        if (endNode   && id === endNode && algorithm === 'astar' && visiblePathNodes.has(id)) fill = '#dc2626'
      }

      return { id, label: id, fill, size: degreeSize(degree, maxDeg), labelVisible: showLabels, data: { degree } }
    })
  }, [parsedGraph, maxDeg, visiblePathNodes, startNode, endNode, algorithm, showLabels, pageRankMap, sccColorMap, result])

  // ── Base edge styles — WASM compute_edge_styles, JS fallback ──────────────
  const baseEdgeStyles: GraphEdge[] = useMemo(() => {
    if (!showEdges) return []

    const wasm = getWasm()
    if (wasm && (pathEdgeSet.size > 0 || mstEdgeSet.size > 0)) {
      try {
        const edgesJson   = JSON.stringify(parsedGraph.edges.map(e => ({ id: e.data.id, source: e.data.source, target: e.data.target })))
        const pathSetJson = JSON.stringify(Array.from(pathEdgeSet))
        const mstSetJson  = JSON.stringify(Array.from(mstEdgeSet))
        const styles = JSON.parse(wasm.compute_edge_styles(edgesJson, pathSetJson, mstSetJson)) as { id: string; fill: string; size: number }[]
        return styles.map((s, i) => ({
          id:     s.id,
          source: parsedGraph.edges[i].data.source as string,
          target: parsedGraph.edges[i].data.target as string,
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
        fill:   isMst ? '#059669' : isPath ? '#0891b2' : '#cbd5e1',
        size:   isMst || isPath ? 2 : 0.7,
      }
    })
  }, [parsedGraph, showEdges, pathEdgeSet, mstEdgeSet])

  // ── Edit overlay — merges in user-added/removed/pinned nodes & edges ──────
  // Cheap pass (bounded by how many edits a person makes by hand), kept apart
  // from the WASM/JS style computation above so edits don't re-run it.
  const graphNodes: GraphNode[] = useMemo(() => {
    const merged: GraphNode[] = []
    for (const n of baseNodeStyles) {
      if (removedNodeIds.has(n.id)) continue
      const pin = pinned.get(n.id)
      merged.push(pin ? { ...n, fx: pin.fx, fy: pin.fy, fz: pin.fz } : n)
    }
    for (const n of customNodes) {
      if (removedNodeIds.has(n.id)) continue
      const pin = pinned.get(n.id)
      merged.push(pin ? { ...n, fx: pin.fx, fy: pin.fy, fz: pin.fz } : n)
    }
    return merged
  }, [baseNodeStyles, customNodes, removedNodeIds, pinned])

  const graphEdges: GraphEdge[] = useMemo(() => {
    const nodeIdSet = new Set(graphNodes.map(n => n.id))
    const merged: GraphEdge[] = []
    for (const e of baseEdgeStyles) {
      if (removedEdgeIds.has(e.id)) continue
      if (!nodeIdSet.has(e.source) || !nodeIdSet.has(e.target)) continue
      merged.push(e)
    }
    for (const e of customEdges) {
      if (removedEdgeIds.has(e.id)) continue
      if (!nodeIdSet.has(e.source) || !nodeIdSet.has(e.target)) continue
      merged.push(e)
    }
    return merged
  }, [baseEdgeStyles, customEdges, removedEdgeIds, graphNodes])

  const allNodeIds = useMemo(() => new Set(graphNodes.map(n => n.id)), [graphNodes])

  // ── Live neighbor map — reflects edits, unlike the WASM-precomputed one ───
  const liveNeighborMap = useMemo<Record<string, string[]>>(() => {
    const map: Record<string, string[]> = {}
    for (const e of graphEdges) {
      (map[e.source] ??= []).push(e.target)
      ;(map[e.target] ??= []).push(e.source)
    }
    return map
  }, [graphEdges])

  // ── Editing handlers ───────────────────────────────────────────────────────
  const handleAddNode = useCallback(() => {
    const id = nextEditId('node')
    setCustomNodes(prev => [...prev, { id, label: id, fill: '#7c3aed', size: 5, data: { degree: 0 } }])
    onGraphEdited?.()
  }, [onGraphEdited])

  const handleDeleteNode = useCallback((id: string) => {
    setRemovedNodeIds(prev => new Set(prev).add(id))
    setCustomNodes(prev => prev.filter(n => n.id !== id))
    setPinned(prev => { if (!prev.has(id)) return prev; const next = new Map(prev); next.delete(id); return next })
    setSelectedNode(sel => sel?.id === id ? null : sel)
    setMultiSelected(prev => prev.filter(x => x !== id))
    onGraphEdited?.()
  }, [onGraphEdited])

  const handleDeleteEdge = useCallback((id: string) => {
    setRemovedEdgeIds(prev => new Set(prev).add(id))
    setCustomEdges(prev => prev.filter(e => e.id !== id))
    setMultiSelected(prev => prev.filter(x => x !== id))
    onGraphEdited?.()
  }, [onGraphEdited])

  const handleDeleteSelected = useCallback(() => {
    const ids = new Set(multiSelected)
    if (selectedNode) ids.add(selectedNode.id)
    for (const id of ids) {
      if (allNodeIds.has(id)) handleDeleteNode(id)
      else handleDeleteEdge(id)
    }
    setMultiSelected([])
    setSelectedNode(null)
  }, [multiSelected, selectedNode, allNodeIds, handleDeleteNode, handleDeleteEdge])

  const isPinned = useCallback((id: string) => pinned.has(id), [pinned])

  const handleTogglePin = useCallback((id: string) => {
    setPinned(prev => {
      if (!prev.has(id)) return prev // nothing to unpin — pinning happens via drag
      const next = new Map(prev)
      next.delete(id)
      return next
    })
  }, [])

  const handleNodeDragged = useCallback((node: InternalGraphNode) => {
    setPinned(prev => new Map(prev).set(node.id, { fx: node.position.x, fy: node.position.y, fz: node.position.z }))
  }, [])

  const handleToggleConnect = useCallback(() => {
    setEditMode(m => m === 'connect' ? 'view' : 'connect')
    setConnectFromId(null)
  }, [])

  const handleStartConnect = useCallback((id: string) => {
    setEditMode('connect')
    setConnectFromId(id)
  }, [])

  const handleConnectClick = useCallback((id: string) => {
    if (!connectFromId) { setConnectFromId(id); return }
    if (id === connectFromId) { setConnectFromId(null); return }
    const edgeId = nextEditId('edge')
    setCustomEdges(prev => [...prev, { id: edgeId, source: connectFromId, target: id, fill: '#7c3aed' }])
    setConnectFromId(null)
    onGraphEdited?.()
  }, [connectFromId, onGraphEdited])

  const handleLassoEnd = useCallback((ids: string[]) => {
    setMultiSelected(ids)
    if (ids.length > 0) setSelectedNode(null)
  }, [])

  // ── Keyboard delete (skips text inputs) ────────────────────────────────────
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key !== 'Delete' && e.key !== 'Backspace') return
      const tag = (document.activeElement as HTMLElement | null)?.tagName
      if (tag === 'INPUT' || tag === 'TEXTAREA') return
      if (multiSelected.length === 0 && !selectedNode) return
      e.preventDefault()
      handleDeleteSelected()
    }
    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [multiSelected, selectedNode, handleDeleteSelected])

  // ── Active (dimming) set ───────────────────────────────────────────────────
  const actives = useMemo<string[] | undefined>(() => {
    if (hoverNodeId) {
      const neighbors = liveNeighborMap[hoverNodeId] ?? []
      return [hoverNodeId, ...neighbors]
    }
    if (algorithm === 'pagerank' || algorithm === 'scc') return undefined
    if (!result || result.error || visiblePathNodes.size === 0) return undefined
    return Array.from(visiblePathNodes)
  }, [hoverNodeId, liveNeighborMap, result, visiblePathNodes, algorithm])

  // ── Node interaction ───────────────────────────────────────────────────────
  const handleNodeClick = useCallback((node: InternalGraphNode) => {
    if (editMode === 'connect') { handleConnectClick(node.id); return }

    const degree = (node.data as { degree: number })?.degree ?? 0
    let distance: string | null = null
    if (result && !result.error && result.distances) {
      const idx = parsedGraph.idToCompact.get(parseInt(node.id, 10))
      if (idx !== undefined && result.distances[idx] !== undefined) {
        const d = result.distances[idx]
        distance = isFinite(d) && d < 1e14 ? d.toFixed(2) : '∞'
      }
    }
    setSelectedNode({ id: node.id, degree, distance })
    setMultiSelected([])
  }, [editMode, handleConnectClick, result, parsedGraph])

  const handleNodePointerOver = useCallback((node: InternalGraphNode) => setHoverNodeId(node.id), [])
  const handleNodePointerOut  = useCallback(() => setHoverNodeId(null), [])

  const handleCanvasClick = useCallback(() => {
    setSelectedNode(null)
    setMultiSelected([])
    if (editMode === 'connect') setConnectFromId(null)
  }, [editMode])

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
  const animProgress = animStep >= 0 ? Math.min(animStep, animTotal) : animTotal

  // ── Render ─────────────────────────────────────────────────────────────────
  return (
    <div className="h-full flex flex-col overflow-hidden" style={{ background: SURFACE.panel }}>
      <GraphToolbar
        totalNodes={graphNodes.length}
        totalEdges={graphEdges.length}
        nodeCount={nodeCount}
        is3D={is3D}
        truncated={parsedGraph.truncated}
        resultLabel={resultLabel}
        showEdges={showEdges}
        onToggleEdges={() => setShowEdges(v => !v)}
        layoutMode={layoutMode}
        onLayoutChange={setLayoutMode}
        onFit={() => graphRef.current?.fitNodesInView?.()}
        onZoomIn={() => graphRef.current?.zoomIn?.()}
        onZoomOut={() => graphRef.current?.zoomOut?.()}
        onExportPNG={exportPNG}
      />

      <GraphSearchAnimBar
        searchInput={searchInput}
        searchError={searchError}
        onSearchInputChange={v => { setSearchInput(v); setSearchError(false) }}
        onSearchSubmit={handleSearch}
        hasResult={hasResult}
        isPlaying={isPlaying}
        animStep={animStep}
        animSpeed={animSpeed}
        animProgress={animProgress}
        animTotal={animTotal}
        onTogglePlayPause={togglePlayPause}
        onReset={resetAnimation}
        onSpeedChange={setAnimSpeed}
      />

      <GraphCanvasPanel
        graphRef={graphRef}
        nodes={graphNodes}
        edges={graphEdges}
        layoutMode={layoutMode}
        actives={actives}
        selections={multiSelected}
        onNodeClick={handleNodeClick}
        onNodePointerOver={handleNodePointerOver}
        onNodePointerOut={handleNodePointerOut}
        onCanvasClick={handleCanvasClick}
        onLasso={handleLassoEnd}
        onLassoEnd={handleLassoEnd}
        onNodeDragged={handleNodeDragged}
        is3D={is3D}
        hoverNodeId={hoverNodeId}
        neighborMap={liveNeighborMap}
        selectedNode={selectedNode}
        onCloseSelected={() => setSelectedNode(null)}
        editMode={editMode}
        connectFromId={connectFromId}
        onToggleConnect={handleToggleConnect}
        onAddNode={handleAddNode}
        onDeleteSelected={handleDeleteSelected}
        isPinned={isPinned}
        onDeleteNode={handleDeleteNode}
        onDeleteEdge={handleDeleteEdge}
        onTogglePin={handleTogglePin}
        onStartConnect={handleStartConnect}
      />

      <GraphStatsFooter
        maxDegree={parsedGraph.maxDegree}
        avgDegree={parsedGraph.avgDegree}
        density={density}
        truncated={parsedGraph.truncated}
        result={result}
        algorithm={algorithm}
      />
    </div>
  )
}

import { useRef, useEffect, useState, useCallback, useMemo } from 'react'
import CytoscapeComponent from 'react-cytoscapejs'
import cytoscape from 'cytoscape'

import type { ParsedGraph } from '../utils/parseGraph'
import type { ApiResult, Algorithm } from '../types'

// ── Types ──────────────────────────────────────────────────────────────────────

type LayoutName = 'cose' | 'circle' | 'grid' | 'concentric'

interface Props {
  parsedGraph: ParsedGraph
  result: ApiResult | null
  algorithm: Algorithm | null
  startNode: string
  endNode: string
}

// ── Stylesheet factory ────────────────────────────────────────────────────────
// font-family with commas is NOT supported in Cytoscape — use a single value.

function makeStylesheet(showLabels: boolean): cytoscape.Stylesheet[] {
  return [
    {
      selector: 'node',
      style: {
        'background-color': '#334155',
        'border-color': '#475569',
        'border-width': 1,
        'width': showLabels ? 28 : 14,
        'height': showLabels ? 28 : 14,
        'label': showLabels ? 'data(label)' : '',
        'color': '#94a3b8',
        'font-size': 9,
        'text-valign': 'center',
        'text-halign': 'center',
        'min-zoomed-font-size': 7,
      } as cytoscape.Css.Node,
    },
    {
      selector: 'edge',
      style: {
        'line-color': '#1e293b',
        'width': showLabels ? 1.5 : 0.8,
        'curve-style': 'haystack',   // faster rendering than bezier for large graphs
        'opacity': 0.8,
      } as cytoscape.Css.Edge,
    },
    // ── Highlight classes ────────────────────────────────────────────────────
    {
      selector: 'node.path-node',
      style: {
        'background-color': '#0e7490',
        'border-color': '#22d3ee',
        'border-width': 2,
        'color': '#cffafe',
        'width': 26,
        'height': 26,
      } as cytoscape.Css.Node,
    },
    {
      selector: 'node.start-node',
      style: {
        'background-color': '#14532d',
        'border-color': '#4ade80',
        'border-width': 2.5,
        'color': '#dcfce7',
        'width': 32,
        'height': 32,
        'font-size': 10,
      } as cytoscape.Css.Node,
    },
    {
      selector: 'node.end-node',
      style: {
        'background-color': '#450a0a',
        'border-color': '#f87171',
        'border-width': 2.5,
        'color': '#fee2e2',
        'width': 32,
        'height': 32,
        'font-size': 10,
      } as cytoscape.Css.Node,
    },
    {
      selector: 'edge.path-edge',
      style: {
        'line-color': '#22d3ee',
        'width': 2.5,
        'opacity': 1,
        'curve-style': 'bezier',
        'target-arrow-shape': 'triangle',
        'target-arrow-color': '#22d3ee',
        'arrow-scale': 0.8,
      } as cytoscape.Css.Edge,
    },
    {
      selector: 'edge.mst-edge',
      style: {
        'line-color': '#4ade80',
        'width': 2.5,
        'opacity': 1,
        'curve-style': 'bezier',
      } as cytoscape.Css.Edge,
    },
    {
      selector: 'node.dimmed',
      style: { 'opacity': 0.15 } as cytoscape.Css.Node,
    },
    {
      selector: 'edge.dimmed',
      style: { 'opacity': 0.05 } as cytoscape.Css.Edge,
    },
  ]
}

// ── Component ──────────────────────────────────────────────────────────────────

export default function GraphView({ parsedGraph, result, algorithm, startNode, endNode }: Props) {
  const cyRef       = useRef<cytoscape.Core | null>(null)
  const mountedRef  = useRef(true)
  const [activeLayout, setActiveLayout] = useState<LayoutName>('cose')
  const [showEdges,    setShowEdges]    = useState(true)

  useEffect(() => {
    mountedRef.current = true
    return () => { mountedRef.current = false }
  }, [])

  const nodeCount  = parsedGraph.nodes.length
  const showLabels = nodeCount <= 50

  const stylesheet = useMemo(() => makeStylesheet(showLabels), [showLabels])

  const elements: cytoscape.ElementDefinition[] = useMemo(() => {
    const nodes = parsedGraph.nodes
    const edges = showEdges ? parsedGraph.edges : []
    return [...nodes, ...edges]
  }, [parsedGraph, showEdges])

  // ── Apply result highlight classes ─────────────────────────────────────────
  const applyResultStyles = useCallback(() => {
    const cy = cyRef.current
    if (!cy || !mountedRef.current) return

    cy.elements().removeClass('start-node end-node path-node mst-edge path-edge dimmed')

    if (!result || result.error) return

    const path    = result.path ?? []
    const pathSet = new Set(path.map(String))

    if (algorithm === 'kruskal') {
      cy.nodes().addClass('dimmed')
      cy.edges().addClass('dimmed')

      for (let i = 0; i + 1 < path.length; i += 2) {
        const u = String(path[i])
        const v = String(path[i + 1])
        cy.$id(u).removeClass('dimmed').addClass('path-node')
        cy.$id(v).removeClass('dimmed').addClass('path-node')
        cy.edges(`[source="${u}"][target="${v}"], [source="${v}"][target="${u}"]`)
          .removeClass('dimmed').addClass('mst-edge')
      }
    } else if (path.length > 0) {
      cy.nodes().forEach(n => { if (!pathSet.has(n.id())) n.addClass('dimmed') })
      cy.edges().addClass('dimmed')

      path.forEach(id => cy.$id(String(id)).removeClass('dimmed').addClass('path-node'))

      for (let i = 0; i + 1 < path.length; i++) {
        cy.edges(`[source="${path[i]}"][target="${path[i + 1]}"]`)
          .removeClass('dimmed').addClass('path-edge')
      }
    }

    if (startNode) cy.$id(startNode).removeClass('dimmed path-node').addClass('start-node')
    if (endNode && algorithm === 'astar') cy.$id(endNode).removeClass('dimmed path-node').addClass('end-node')
  }, [result, algorithm, startNode, endNode])

  useEffect(() => { applyResultStyles() }, [applyResultStyles])

  // ── Layout ─────────────────────────────────────────────────────────────────
  const runLayout = useCallback((name: LayoutName, animate: boolean) => {
    const cy = cyRef.current
    if (!cy || !mountedRef.current) return

    const n = cy.nodes().length

    const opts: Record<string, unknown> = {
      name,
      animate,
      animationDuration: 400,
      fit: true,
      padding: 32,
    }

    if (name === 'cose') {
      opts.nodeRepulsion   = 6000
      opts.idealEdgeLength = n > 100 ? 40 : 60
      opts.nodeOverlap     = 10
      opts.numIter         = n > 100 ? 300 : 800
      opts.randomize       = n > 20
      opts.gravity         = 0.3
    }

    cy.layout(opts as cytoscape.LayoutOptions).run()
  }, [])

  const handleLayoutChange = (name: LayoutName) => {
    setActiveLayout(name)
    runLayout(name, true)
  }

  // Init: no animation to avoid React StrictMode null-notify crash
  const handleCyInit = useCallback((cy: cytoscape.Core) => {
    cyRef.current = cy
    cy.on('layoutstop', () => {
      if (mountedRef.current) applyResultStyles()
    })
    // Run initial layout without animation
    runLayout(activeLayout, false)
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])   // intentionally empty deps — runs once on mount

  // Re-run layout without animation when elements change (new file)
  useEffect(() => {
    const cy = cyRef.current
    if (!cy || !mountedRef.current) return
    runLayout(activeLayout, false)
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [parsedGraph])

  // ── Stats ──────────────────────────────────────────────────────────────────
  const resultLabel = !result || result.error ? null
    : algorithm === 'kruskal'                   ? `${(result.path?.length ?? 0) / 2} MST edges`
    : algorithm === 'bfs' || algorithm === 'dfs' ? `${result.path?.length ?? 0} nodes visited`
    : (result.path?.length ?? 0) > 0            ? `Path: ${result.path!.length} nodes`
    : null

  const isDense = nodeCount > 100

  return (
    <div className="rounded-xl border border-zinc-800 bg-zinc-900/20 overflow-hidden">

      {/* ── Toolbar ── */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-zinc-800/80 flex-wrap gap-2">
        <div className="flex items-center gap-2 flex-wrap">
          <span className="text-xs font-mono-display text-zinc-500 uppercase tracking-widest">Graph</span>

          <span className="text-xs font-mono-display text-zinc-600">
            {parsedGraph.totalNodes.toLocaleString()} nodes · {parsedGraph.totalEdges.toLocaleString()} edges
          </span>

          {parsedGraph.truncated && (
            <span className="text-xs font-mono-display text-amber-400 bg-amber-950/30 border border-amber-800/40 px-2 py-0.5 rounded-full">
              showing first {nodeCount}
            </span>
          )}

          {resultLabel && (
            <span className="text-xs font-mono-display text-cyan-400 bg-cyan-950/30 border border-cyan-800/40 px-2 py-0.5 rounded-full">
              {resultLabel}
            </span>
          )}
        </div>

        {/* Controls */}
        <div className="flex items-center gap-1.5 flex-wrap">
          {/* Edge toggle for dense graphs */}
          {isDense && (
            <button
              onClick={() => setShowEdges(e => !e)}
              className={[
                'px-2.5 py-1 rounded text-xs font-mono-display transition-colors border',
                showEdges
                  ? 'bg-zinc-700/60 text-zinc-300 border-zinc-600'
                  : 'text-zinc-500 border-zinc-800 hover:text-zinc-300',
              ].join(' ')}
            >
              {showEdges ? 'hide edges' : 'show edges'}
            </button>
          )}

          {/* Layout buttons */}
          {(['cose', 'circle', 'grid', 'concentric'] as LayoutName[]).map(name => (
            <button
              key={name}
              onClick={() => handleLayoutChange(name)}
              className={[
                'px-2.5 py-1 rounded text-xs font-mono-display transition-colors border',
                activeLayout === name
                  ? 'bg-zinc-700/80 text-zinc-100 border-zinc-500'
                  : 'text-zinc-500 border-zinc-800 hover:text-zinc-300 hover:border-zinc-700',
              ].join(' ')}
            >
              {name}
            </button>
          ))}

          <button
            onClick={() => cyRef.current?.fit(undefined, 32)}
            className="px-2.5 py-1 rounded text-xs font-mono-display text-zinc-500 border border-zinc-800 hover:text-zinc-300 hover:border-zinc-700 ml-1"
          >
            fit
          </button>
        </div>
      </div>

      {/* ── Dense graph notice ── */}
      {isDense && (
        <div className="flex items-center gap-2 px-4 py-2 bg-amber-950/20 border-b border-amber-900/30">
          <span className="text-amber-400 text-xs font-mono-display">⚠</span>
          <span className="text-xs font-mono-display text-amber-400/80">
            Large graph — labels hidden, edges simplified.
            Use <span className="text-amber-300">hide edges</span> to see node clusters more clearly.
          </span>
        </div>
      )}

      {/* ── Legend ── */}
      <div className="flex items-center gap-5 px-4 py-2 border-b border-zinc-800/40 flex-wrap">
        <LegendItem color="#4ade80" border="#14532d" label="Start node" />
        {algorithm === 'astar' && <LegendItem color="#f87171" border="#450a0a" label="End node" />}
        {algorithm !== 'kruskal'
          ? <LegendItem color="#22d3ee" border="#0e7490" label="Path" />
          : <LegendItem color="#4ade80" border="#14532d" label="MST edge" />}
        <LegendItem color="#475569" border="#334155" label="Node" />
        <LegendItem color="#1e293b" border="#1e293b" label="Edge" />
      </div>

      {/* ── Canvas ── */}
      <CytoscapeComponent
        elements={elements}
        stylesheet={stylesheet}
        style={{ width: '100%', height: '500px', background: '#020817' }}
        cy={handleCyInit}
        layout={{ name: 'preset' } as cytoscape.LayoutOptions}  // layout controlled manually
      />
    </div>
  )
}

function LegendItem({ color, border, label }: { color: string; border: string; label: string }) {
  return (
    <span className="flex items-center gap-1.5">
      <span
        className="w-3 h-3 rounded-full flex-shrink-0"
        style={{ backgroundColor: border, boxShadow: `0 0 0 1.5px ${color}` }}
      />
      <span className="text-xs font-mono-display text-zinc-500">{label}</span>
    </span>
  )
}

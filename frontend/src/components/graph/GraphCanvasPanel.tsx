import { GraphCanvas } from 'reagraph'
import type { GraphCanvasRef, GraphNode, GraphEdge, InternalGraphNode, ContextMenuEvent } from 'reagraph'

import { SURFACE } from '../../theme'
import { LIGHT_THEME, type LayoutMode, LAYOUT_TYPE } from './graphTheme'
import GraphEditBar from './GraphEditBar'
import NodeRadialMenu from './NodeRadialMenu'

export interface SelectedNode { id: string; degree: number; distance: string | null }

interface Props {
  graphRef: React.RefObject<GraphCanvasRef | null>
  nodes: GraphNode[]
  edges: GraphEdge[]
  layoutMode: LayoutMode
  actives: string[] | undefined
  selections: string[]
  onNodeClick: (node: InternalGraphNode) => void
  onNodePointerOver: (node: InternalGraphNode) => void
  onNodePointerOut: () => void
  onCanvasClick: () => void
  onLasso: (ids: string[]) => void
  onLassoEnd: (ids: string[]) => void
  onNodeDragged: (node: InternalGraphNode) => void

  is3D: boolean
  hoverNodeId: string | null
  neighborMap: Record<string, string[]>
  selectedNode: SelectedNode | null
  onCloseSelected: () => void

  // ── Editing ──────────────────────────────────────────────────────────────
  editMode: 'view' | 'connect'
  connectFromId: string | null
  onToggleConnect: () => void
  onAddNode: () => void
  onDeleteSelected: () => void
  isPinned: (id: string) => boolean
  onDeleteNode: (id: string) => void
  onDeleteEdge: (id: string) => void
  onTogglePin: (id: string) => void
  onStartConnect: (id: string) => void
}

export default function GraphCanvasPanel({
  graphRef, nodes, edges, layoutMode, actives, selections,
  onNodeClick, onNodePointerOver, onNodePointerOut, onCanvasClick, onLasso, onLassoEnd, onNodeDragged,
  is3D, hoverNodeId, neighborMap, selectedNode, onCloseSelected,
  editMode, connectFromId, onToggleConnect, onAddNode, onDeleteSelected,
  isPinned, onDeleteNode, onDeleteEdge, onTogglePin, onStartConnect,
}: Props) {
  return (
    <div className="relative flex-1 overflow-hidden" style={{ background: SURFACE.canvas, minHeight: 0 }}>
      <div className="absolute inset-0">
        <GraphCanvas
          ref={graphRef}
          nodes={nodes}
          edges={edges}
          layoutType={LAYOUT_TYPE[layoutMode] as Parameters<typeof GraphCanvas>[0]['layoutType']}
          actives={actives}
          selections={selections}
          theme={LIGHT_THEME as Parameters<typeof GraphCanvas>[0]['theme']}
          draggable={editMode !== 'connect'}
          onNodeDragged={onNodeDragged}
          lassoType="node"
          onLasso={onLasso}
          onLassoEnd={onLassoEnd}
          onNodeClick={onNodeClick}
          onNodePointerOver={onNodePointerOver}
          onNodePointerOut={onNodePointerOut}
          onCanvasClick={onCanvasClick}
          contextMenu={(event: ContextMenuEvent) => (
            <NodeRadialMenu
              event={event}
              isPinned={isPinned}
              onDeleteNode={onDeleteNode}
              onDeleteEdge={onDeleteEdge}
              onTogglePin={onTogglePin}
              onStartConnect={onStartConnect}
            />
          )}
        />
      </div>

      {/* subtle depth vignette over the flat WebGL background */}
      <div
        className="absolute inset-0 pointer-events-none"
        style={{ background: 'radial-gradient(ellipse at center, transparent 45%, rgba(15,23,42,0.05) 100%)' }}
      />

      <GraphEditBar
        editMode={editMode}
        connectFromId={connectFromId}
        onToggleConnect={onToggleConnect}
        onAddNode={onAddNode}
        selectionCount={selections.length}
        onDeleteSelected={onDeleteSelected}
      />

      {is3D && (
        <div className="absolute top-3 right-3 text-[10px] text-slate-500 pointer-events-none select-none">
          drag to orbit · scroll to zoom · right-drag to pan
        </div>
      )}

      {hoverNodeId && !selectedNode && (
        <div className="absolute bottom-4 left-4 text-[10px] text-slate-500 pointer-events-none select-none">
          node {hoverNodeId} · {(neighborMap[hoverNodeId] ?? []).length} neighbors
        </div>
      )}

      {selectedNode && (
        <div
          className="absolute bottom-4 right-4 rounded-lg border border-slate-200 bg-white/97 px-4 py-3 text-xs space-y-1.5 z-10 shadow-lg"
          style={{ backdropFilter: 'blur(8px)' }}
        >
          <div className="flex items-center justify-between gap-6 mb-0.5">
            <span className="text-slate-400 uppercase tracking-wide text-[10px] font-semibold">Node</span>
            <button onClick={onCloseSelected} className="cursor-pointer text-slate-400 hover:text-slate-700 leading-none">×</button>
          </div>
          <div className="flex justify-between gap-8">
            <span className="text-slate-500">ID</span>
            <span className="text-slate-900 font-mono-display">{selectedNode.id}</span>
          </div>
          <div className="flex justify-between gap-8">
            <span className="text-slate-500">Degree</span>
            <span className="text-cyan-700 font-mono-display">{selectedNode.degree}</span>
          </div>
          <div className="flex justify-between gap-8">
            <span className="text-slate-500">Neighbors</span>
            <span className="text-slate-700 font-mono-display">{(neighborMap[selectedNode.id] ?? []).length}</span>
          </div>
          {selectedNode.distance !== null && (
            <div className="flex justify-between gap-8">
              <span className="text-slate-500">Distance</span>
              <span className="text-emerald-600 font-mono-display">{selectedNode.distance}</span>
            </div>
          )}
        </div>
      )}
    </div>
  )
}

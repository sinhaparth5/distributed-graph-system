interface Props {
  editMode: 'view' | 'connect'
  connectFromId: string | null
  onToggleConnect: () => void
  onAddNode: () => void
  selectionCount: number
  onDeleteSelected: () => void
}

export default function GraphEditBar({
  editMode, connectFromId, onToggleConnect, onAddNode, selectionCount, onDeleteSelected,
}: Props) {
  return (
    <div className="absolute top-3 left-1/2 -translate-x-1/2 z-10 flex items-center gap-1.5 px-2 py-1.5 rounded-xl border border-slate-200 bg-white/95 shadow-md backdrop-blur-sm">
      <button
        onClick={onAddNode}
        title="Add a node"
        className="cursor-pointer h-8 px-3 rounded-lg text-xs font-medium text-slate-600 hover:bg-slate-100 hover:text-slate-900 transition-colors flex items-center gap-1.5"
      >
        <span className="text-cyan-600 text-sm leading-none">+</span> node
      </button>

      <div className="w-px h-5 bg-slate-200" />

      <button
        onClick={onToggleConnect}
        title="Click two nodes to connect them"
        className={[
          'cursor-pointer h-8 px-3 rounded-lg text-xs font-medium transition-colors flex items-center gap-1.5',
          editMode === 'connect'
            ? 'bg-cyan-50 text-cyan-700 border border-cyan-200'
            : 'text-slate-600 hover:bg-slate-100 hover:text-slate-900',
        ].join(' ')}
      >
        ⇄ connect
      </button>

      {editMode === 'connect' && (
        <span className="text-[11px] text-cyan-700 font-medium pl-1 pr-1">
          {connectFromId ? `from ${connectFromId} — click a target` : 'click a source node'}
        </span>
      )}

      {selectionCount > 0 && (
        <>
          <div className="w-px h-5 bg-slate-200" />
          <button
            onClick={onDeleteSelected}
            title="Delete selected (Del)"
            className="cursor-pointer h-8 px-3 rounded-lg text-xs font-medium text-red-600 hover:bg-red-50 transition-colors flex items-center gap-1.5"
          >
            delete {selectionCount}
          </button>
        </>
      )}
    </div>
  )
}

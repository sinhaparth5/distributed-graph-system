import type { ApiResult, Algorithm } from '../types'

interface Props {
  result: ApiResult
  algorithm: Algorithm | null
}

export default function Results({ result, algorithm }: Props) {
  const isDistributed = result.mpi_mode?.toLowerCase().includes('distributed')

  return (
    <div className="result-enter rounded-xl border border-zinc-800 bg-zinc-900/30 overflow-hidden">

      {/* Toolbar */}
      <div className="flex items-center justify-between px-5 py-3.5 border-b border-zinc-800/80">
        <span className="text-xs font-mono-display text-zinc-500 uppercase tracking-widest">
          Output
        </span>
        {result.mpi_processes > 0 && (
          <span
            className={[
              'text-xs font-mono-display px-2.5 py-1 rounded-full border',
              isDistributed
                ? 'bg-emerald-950/40 border-emerald-800/60 text-emerald-400'
                : 'bg-zinc-800/60 border-zinc-700 text-zinc-400',
            ].join(' ')}
          >
            {result.mpi_processes} process{result.mpi_processes !== 1 ? 'es' : ''} · {result.mpi_mode}
          </span>
        )}
      </div>

      <div className="p-5 space-y-5">

        {/* Error */}
        {result.error && (
          <div className="flex items-start gap-3 px-4 py-3 rounded-lg bg-red-950/30 border border-red-800/50">
            <span className="text-red-400 font-mono-display mt-0.5 flex-shrink-0">!</span>
            <p className="text-red-300 text-sm">{result.error}</p>
          </div>
        )}

        {/* Negative cycle warning */}
        {result.has_negative_cycle && (
          <div className="flex items-center gap-3 px-4 py-3 rounded-lg bg-orange-950/30 border border-orange-800/50">
            <span className="text-orange-400 font-mono-display flex-shrink-0">⚠</span>
            <p className="text-orange-300 text-sm font-mono-display">Negative cycle detected</p>
          </div>
        )}

        {/* Path / MST edges */}
        {!result.error && result.path && result.path.length > 0 && (
          <div>
            <p className="text-xs font-mono-display text-zinc-500 uppercase tracking-widest mb-3">
              {algorithm === 'kruskal'
                ? 'MST Edges'
                : algorithm === 'bfs' || algorithm === 'dfs'
                  ? 'Traversal Order'
                  : 'Path'}
            </p>
            {algorithm === 'kruskal'
              ? <KruskalEdges path={result.path} />
              : <PathDisplay path={result.path} />}
          </div>
        )}

        {/* Distances */}
        {!result.error && result.distances && result.distances.length > 0 && (
          <div>
            <p className="text-xs font-mono-display text-zinc-500 uppercase tracking-widest mb-3">
              Distances
            </p>
            <DistanceGrid distances={result.distances} />
          </div>
        )}

        {/* Empty */}
        {!result.error &&
          (!result.path || result.path.length === 0) &&
          (!result.distances || result.distances.length === 0) &&
          !result.has_negative_cycle && (
            <p className="text-zinc-500 font-mono-display text-sm">No output returned.</p>
          )}
      </div>
    </div>
  )
}

// ── Path display ───────────────────────────────────────────────────────────────

function PathDisplay({ path }: { path: number[] }) {
  return (
    <div className="overflow-x-auto pb-1">
      <div className="flex items-center gap-1.5 min-w-max">
        {path.map((node, i) => (
          <span key={i} className="flex items-center gap-1.5">
            <span className="px-3 py-1.5 rounded-lg bg-zinc-800/80 border border-cyan-900/60 text-cyan-300 font-mono-display text-sm">
              {node}
            </span>
            {i < path.length - 1 && (
              <span className="text-zinc-600 font-mono-display text-xs select-none">→</span>
            )}
          </span>
        ))}
      </div>
    </div>
  )
}

// ── Kruskal MST edges ──────────────────────────────────────────────────────────

function KruskalEdges({ path }: { path: number[] }) {
  const pairs: [number, number][] = []
  for (let i = 0; i + 1 < path.length; i += 2) {
    pairs.push([path[i], path[i + 1]])
  }

  return (
    <div className="flex flex-wrap gap-2">
      {pairs.map(([a, b], i) => (
        <span
          key={i}
          className="px-3 py-1.5 rounded-lg bg-zinc-800/80 border border-zinc-700/60 text-zinc-300 font-mono-display text-xs"
        >
          {a} — {b}
        </span>
      ))}
    </div>
  )
}

// ── Distance grid ──────────────────────────────────────────────────────────────

function DistanceGrid({ distances }: { distances: number[] }) {
  const entries = distances
    .map((d, i) => ({ node: i, dist: d }))
    .filter(({ dist }) => dist != null && isFinite(dist) && dist < 1e14)

  if (entries.length === 0) {
    return <p className="text-zinc-600 font-mono-display text-sm">All nodes unreachable.</p>
  }

  return (
    <div className="grid grid-cols-2 sm:grid-cols-3 gap-1.5 max-h-52 overflow-y-auto pr-1">
      {entries.map(({ node, dist }) => (
        <div
          key={node}
          className="flex justify-between items-center px-3 py-2 rounded-lg bg-zinc-800/50 border border-zinc-700/40"
        >
          <span className="text-zinc-500 font-mono-display text-xs">n{node}</span>
          <span className="text-white font-mono-display text-xs ml-2">
            {dist === 0 ? '0' : dist.toFixed(2)}
          </span>
        </div>
      ))}
    </div>
  )
}

import type { ApiResult, Algorithm, GraphMetrics } from '../types'

interface Props {
  result: ApiResult
  algorithm: Algorithm | null
  metrics?: GraphMetrics | null
  metricsLoading?: boolean
  onComputeMetrics?: () => void
}

export default function Results({ result, algorithm, metrics, metricsLoading, onComputeMetrics }: Props) {
  const isDistributed = result.mpi_mode?.toLowerCase().includes('distributed')

  return (
    <div className="result-enter rounded-xl border border-zinc-800 bg-zinc-900/30 overflow-hidden">

      {/* Toolbar */}
      <div className="flex items-center justify-between px-5 py-3.5 border-b border-zinc-800/80">
        <span className="text-xs font-mono-display text-zinc-500 uppercase tracking-widest">
          Output
        </span>
        {result.mpi_processes > 0 && (
          <span className={[
            'text-xs font-mono-display px-2.5 py-1 rounded-full border',
            isDistributed
              ? 'bg-emerald-950/40 border-emerald-800/60 text-emerald-400'
              : 'bg-zinc-800/60 border-zinc-700 text-zinc-400',
          ].join(' ')}>
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

        {/* PageRank scores */}
        {!result.error && result.scores && result.scores.length > 0 && (
          <div>
            <p className="text-xs font-mono-display text-zinc-500 uppercase tracking-widest mb-3">
              Top Nodes by PageRank
            </p>
            <PageRankDisplay scores={result.scores} />
          </div>
        )}

        {/* SCC components */}
        {!result.error && result.components && result.components.length > 0 && (
          <div>
            <p className="text-xs font-mono-display text-zinc-500 uppercase tracking-widest mb-3">
              Strongly Connected Components ({result.components.length})
            </p>
            <SCCDisplay components={result.components} />
          </div>
        )}

        {/* Path / MST / Topo sort */}
        {!result.error && result.path && result.path.length > 0 && !result.components && (
          <div>
            <p className="text-xs font-mono-display text-zinc-500 uppercase tracking-widest mb-3">
              {algorithm === 'kruskal'          ? 'MST Edges'
               : algorithm === 'bfs' || algorithm === 'dfs' ? 'Traversal Order'
               : algorithm === 'topological-sort'           ? 'Topological Order'
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

        {/* Empty state */}
        {!result.error &&
          (!result.path || result.path.length === 0) &&
          (!result.distances || result.distances.length === 0) &&
          (!result.scores || result.scores.length === 0) &&
          !result.has_negative_cycle && (
            <p className="text-zinc-500 font-mono-display text-sm">No output returned.</p>
          )}

        {/* Graph Metrics */}
        {onComputeMetrics && (
          <div className="border-t border-zinc-800/60 pt-4">
            <div className="flex items-center justify-between mb-3">
              <p className="text-xs font-mono-display text-zinc-500 uppercase tracking-widest">
                Graph Metrics
              </p>
              <button
                onClick={onComputeMetrics}
                disabled={metricsLoading}
                className="px-2.5 py-1 rounded text-xs font-mono-display border border-zinc-700 text-zinc-400 hover:text-zinc-200 hover:border-zinc-500 disabled:opacity-40 transition-colors"
              >
                {metricsLoading ? 'computing…' : '↻ compute'}
              </button>
            </div>
            {metrics && !metrics.error && <MetricsDisplay metrics={metrics} />}
            {metrics?.error && (
              <p className="text-xs font-mono-display text-red-500">{metrics.error}</p>
            )}
          </div>
        )}
      </div>
    </div>
  )
}

// ── PageRank display ────────────────────────────────────────────────────────────

function PageRankDisplay({ scores }: { scores: [number, number][] }) {
  const top = scores.slice(0, 15)
  const maxScore = top[0]?.[1] ?? 1
  return (
    <div className="space-y-1.5">
      {top.map(([id, score], i) => (
        <div key={id} className="flex items-center gap-3">
          <span className="text-zinc-600 font-mono-display text-[10px] w-4 text-right flex-shrink-0">
            {i + 1}
          </span>
          <span className="text-zinc-400 font-mono-display text-xs w-16 flex-shrink-0">
            {id}
          </span>
          <div className="flex-1 h-1.5 rounded-full bg-zinc-800 overflow-hidden">
            <div
              className="h-full rounded-full bg-cyan-500"
              style={{ width: `${(score / maxScore) * 100}%` }}
            />
          </div>
          <span className="text-zinc-500 font-mono-display text-[10px] w-14 text-right flex-shrink-0">
            {score.toFixed(4)}
          </span>
        </div>
      ))}
    </div>
  )
}

// ── SCC display ─────────────────────────────────────────────────────────────────

const SCC_COLORS = [
  'border-cyan-800/60 bg-cyan-950/20 text-cyan-400',
  'border-violet-800/60 bg-violet-950/20 text-violet-400',
  'border-amber-800/60 bg-amber-950/20 text-amber-400',
  'border-emerald-800/60 bg-emerald-950/20 text-emerald-400',
  'border-rose-800/60 bg-rose-950/20 text-rose-400',
]

function SCCDisplay({ components }: { components: number[][] }) {
  const sorted = [...components].sort((a, b) => b.length - a.length)
  return (
    <div className="space-y-2 max-h-52 overflow-y-auto pr-1">
      {sorted.slice(0, 10).map((comp, i) => (
        <div key={i} className={`px-3 py-2 rounded-lg border text-xs font-mono-display ${SCC_COLORS[i % SCC_COLORS.length]}`}>
          <span className="opacity-60 mr-2">#{i + 1} ({comp.length} nodes)</span>
          <span className="opacity-90">{comp.slice(0, 8).join(', ')}{comp.length > 8 ? ` +${comp.length - 8}` : ''}</span>
        </div>
      ))}
      {sorted.length > 10 && (
        <p className="text-zinc-600 font-mono-display text-xs text-center">
          +{sorted.length - 10} more components
        </p>
      )}
    </div>
  )
}

// ── Path display ────────────────────────────────────────────────────────────────

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

// ── Kruskal MST edges ───────────────────────────────────────────────────────────

function KruskalEdges({ path }: { path: number[] }) {
  const pairs: [number, number][] = []
  for (let i = 0; i + 1 < path.length; i += 2) pairs.push([path[i], path[i + 1]])
  return (
    <div className="flex flex-wrap gap-2">
      {pairs.map(([a, b], i) => (
        <span key={i} className="px-3 py-1.5 rounded-lg bg-zinc-800/80 border border-zinc-700/60 text-zinc-300 font-mono-display text-xs">
          {a} — {b}
        </span>
      ))}
    </div>
  )
}

// ── Distance grid ───────────────────────────────────────────────────────────────

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
        <div key={node} className="flex justify-between items-center px-3 py-2 rounded-lg bg-zinc-800/50 border border-zinc-700/40">
          <span className="text-zinc-500 font-mono-display text-xs">n{node}</span>
          <span className="text-white font-mono-display text-xs ml-2">
            {dist === 0 ? '0' : dist.toFixed(2)}
          </span>
        </div>
      ))}
    </div>
  )
}

// ── Graph metrics panel ─────────────────────────────────────────────────────────

function MetricsDisplay({ metrics }: { metrics: GraphMetrics }) {
  return (
    <div className="space-y-3">
      <div className="grid grid-cols-2 gap-1.5">
        <MetricItem label="components" value={metrics.connected_components} />
        <MetricItem label="is DAG" value={metrics.is_dag ? 'yes' : 'no'} highlight={metrics.is_dag} />
        <MetricItem label="density" value={metrics.density.toFixed(5)} />
        <MetricItem label="avg degree" value={metrics.avg_degree.toFixed(1)} />
      </div>
      {metrics.top_hubs.length > 0 && (
        <div>
          <p className="text-[10px] font-mono-display text-zinc-600 uppercase tracking-wider mb-1.5">
            Top hubs
          </p>
          <div className="flex flex-wrap gap-1.5">
            {metrics.top_hubs.map(h => (
              <span key={h.id} className="px-2 py-1 rounded bg-zinc-800/60 border border-zinc-700/40 text-zinc-400 font-mono-display text-xs">
                {h.id} <span className="text-zinc-600">d={h.degree}</span>
              </span>
            ))}
          </div>
        </div>
      )}
    </div>
  )
}

function MetricItem({ label, value, highlight }: { label: string; value: string | number; highlight?: boolean }) {
  return (
    <div className="flex justify-between items-center px-3 py-2 rounded-lg bg-zinc-800/40 border border-zinc-800/60">
      <span className="text-zinc-600 font-mono-display text-[10px] uppercase tracking-wider">{label}</span>
      <span className={`font-mono-display text-xs ${highlight ? 'text-emerald-400' : 'text-zinc-300'}`}>{value}</span>
    </div>
  )
}

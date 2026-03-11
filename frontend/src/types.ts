export interface MpiStatus {
  mpi_processes: number
  mpi_mode: string
  master_rank: number
  note: string
}

export interface ApiResult {
  path?: number[]
  distances?: number[]
  has_negative_cycle?: boolean
  components?: number[][]
  scores?: [number, number][]   // [nodeId, rankScore] pairs — PageRank
  mpi_processes: number
  mpi_mode: string
  error?: string
}

export interface GraphMetrics {
  node_count: number
  edge_count: number
  density: number
  connected_components: number
  is_dag: boolean
  avg_degree: number
  top_hubs: { id: number; degree: number }[]
  error?: string
}

export type Algorithm =
  | 'bfs' | 'dfs' | 'dijkstra' | 'astar' | 'bellman-ford' | 'kruskal'
  | 'pagerank' | 'scc' | 'topological-sort'

export type FileFormat = 'edgeList' | 'adjacencyList'

export const ALGORITHMS: { id: Algorithm; label: string; group: 'traversal' | 'shortest-path' | 'graph' }[] = [
  { id: 'bfs',              label: 'BFS',           group: 'traversal'     },
  { id: 'dfs',              label: 'DFS',           group: 'traversal'     },
  { id: 'dijkstra',         label: 'Dijkstra',      group: 'shortest-path' },
  { id: 'astar',            label: 'A*',            group: 'shortest-path' },
  { id: 'bellman-ford',     label: 'Bellman-Ford',  group: 'shortest-path' },
  { id: 'kruskal',          label: 'Kruskal MST',   group: 'graph'         },
  { id: 'pagerank',         label: 'PageRank',      group: 'graph'         },
  { id: 'scc',              label: 'SCC',           group: 'graph'         },
  { id: 'topological-sort', label: 'Topo Sort',     group: 'graph'         },
]

export const NEEDS_START = new Set<Algorithm>(['bfs', 'dfs', 'dijkstra', 'astar', 'bellman-ford'])
export const NEEDS_END   = new Set<Algorithm>(['astar'])

export const API_BASE = (import.meta.env.VITE_API_BASE as string | undefined) ?? 'http://localhost:8000'

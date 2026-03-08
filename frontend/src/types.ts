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
  mpi_processes: number
  mpi_mode: string
  error?: string
}

export type Algorithm = 'bfs' | 'dfs' | 'dijkstra' | 'astar' | 'bellman-ford' | 'kruskal'
export type FileFormat = 'edgeList' | 'adjacencyList'

export const ALGORITHMS: { id: Algorithm; label: string }[] = [
  { id: 'bfs',          label: 'BFS'          },
  { id: 'dfs',          label: 'DFS'          },
  { id: 'dijkstra',     label: 'Dijkstra'     },
  { id: 'astar',        label: 'A*'           },
  { id: 'bellman-ford', label: 'Bellman-Ford' },
  { id: 'kruskal',      label: 'Kruskal'      },
]

export const NEEDS_START = new Set<Algorithm>(['bfs', 'dfs', 'dijkstra', 'astar', 'bellman-ford'])
export const NEEDS_END   = new Set<Algorithm>(['astar'])

export const API_BASE = 'http://localhost:8000'

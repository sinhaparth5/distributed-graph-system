export const MAX_DISPLAY_NODES = 3000

/** Minimal graph element — { data: { id, label, degree?, source?, target?, weight? } } */
export interface GraphElement {
  data: Record<string, unknown>
}

export interface ParsedGraph {
  nodes: GraphElement[]
  edges: GraphElement[]
  compactToId: number[]            // compact index → original node ID
  idToCompact: Map<number, number> // original node ID → compact index
  totalNodes: number
  totalEdges: number
  maxDegree: number
  avgDegree: number
  truncated: boolean
}

// ── WASM loader (optional) ─────────────────────────────────────────────────────
// Build WASM with: cd wasm && wasm-pack build --target bundler --out-dir ../frontend/src/wasm-pkg
// Then npm install -D vite-plugin-wasm vite-plugin-top-level-await

type WasmModule = {
  parse_edge_list(s: string): string
  parse_adjacency_list(s: string): string
}
let wasmModule: WasmModule | null = null

const wasmPromise = (async () => {
  try {
    // Dynamic import so the app works even if WASM hasn't been built yet
    const mod = await import('../wasm-pkg/graph_wasm.js' /* @vite-ignore */)
    wasmModule = mod as unknown as WasmModule
  } catch {
    // WASM not built — JS fallback is used automatically
  }
})()

// ── Shared result shape from WASM ──────────────────────────────────────────────
interface WasmResult {
  nodes: { id: string; label: string; degree: number }[]
  edges: { id: string; source: string; target: string; weight: number }[]
  total_nodes: number
  total_edges: number
  max_degree: number
  avg_degree: number
  truncated: boolean
}

function wasmResultToParsed(r: WasmResult): ParsedGraph {
  const compactToId: number[] = r.nodes.map(n => parseInt(n.id, 10))
  const idToCompact = new Map(compactToId.map((id, i) => [id, i]))

  const nodes: ElementDefinition[] = r.nodes.map(n => ({
    data: { id: n.id, label: n.label, degree: n.degree },
  }))
  const edges: ElementDefinition[] = r.edges.map(e => ({
    data: { id: e.id, source: e.source, target: e.target, weight: e.weight },
  }))

  return {
    nodes, edges, compactToId, idToCompact,
    totalNodes: r.total_nodes,
    totalEdges: r.total_edges,
    maxDegree:  r.max_degree,
    avgDegree:  r.avg_degree,
    truncated:  r.truncated,
  }
}

// ── JS fallback parsers ────────────────────────────────────────────────────────

function buildResult(
  order: number[],
  degrees: Map<number, number>,
  rawEdges: { u: number; v: number; w: number }[],
): ParsedGraph {
  const totalNodes = order.length
  const totalEdges = rawEdges.length
  const truncated  = totalNodes > MAX_DISPLAY_NODES

  const displayIds = new Set(order.slice(0, MAX_DISPLAY_NODES))

  const degVals    = Array.from(degrees.values())
  const maxDegree  = degVals.length ? Math.max(...degVals) : 1
  const avgDegree  = degVals.length ? degVals.reduce((a, b) => a + b, 0) / degVals.length : 0

  const compactToId = order.slice(0, MAX_DISPLAY_NODES)
  const idToCompact = new Map(compactToId.map((id, i) => [id, i]))

  const nodes: ElementDefinition[] = compactToId.map(id => ({
    data: { id: String(id), label: String(id), degree: degrees.get(id) ?? 0 },
  }))

  const edges: ElementDefinition[] = rawEdges
    .filter(e => displayIds.has(e.u) && displayIds.has(e.v))
    .map((e, i) => ({
      data: { id: `e_${i}`, source: String(e.u), target: String(e.v), weight: e.w },
    }))

  return { nodes, edges, compactToId, idToCompact, totalNodes, totalEdges, maxDegree, avgDegree, truncated }
}

function parseEdgeListJS(content: string): ParsedGraph {
  const degrees  = new Map<number, number>()
  const order: number[] = []
  const rawEdges: { u: number; v: number; w: number }[] = []

  for (const rawLine of content.split('\n')) {
    const line = rawLine.trim()
    if (!line || line.startsWith('#') || line.startsWith('%')) continue
    const parts = line.split(/\s+/)
    if (parts.length < 2) continue
    const u = parseInt(parts[0], 10)
    const v = parseInt(parts[1], 10)
    const w = parts.length >= 3 ? parseFloat(parts[2]) : 1.0
    if (isNaN(u) || isNaN(v)) continue
    if (!degrees.has(u)) { order.push(u); degrees.set(u, 0) }
    if (!degrees.has(v)) { order.push(v); degrees.set(v, 0) }
    degrees.set(u, (degrees.get(u)! + 1))
    degrees.set(v, (degrees.get(v)! + 1))
    rawEdges.push({ u, v, w })
  }
  return buildResult(order, degrees, rawEdges)
}

function parseAdjacencyListJS(content: string): ParsedGraph {
  const degrees  = new Map<number, number>()
  const order: number[] = []
  const rawEdges: { u: number; v: number; w: number }[] = []

  for (const rawLine of content.split('\n')) {
    const line = rawLine.trim()
    if (!line || line.startsWith('#') || line.startsWith('%')) continue
    const colonIdx = line.indexOf(':')
    if (colonIdx === -1) continue
    const u = parseInt(line.slice(0, colonIdx).trim(), 10)
    if (isNaN(u)) continue
    if (!degrees.has(u)) { order.push(u); degrees.set(u, 0) }
    for (const part of line.slice(colonIdx + 1).trim().split(/\s+/)) {
      if (!part) continue
      const [vStr, wStr] = part.split(',')
      const v = parseInt(vStr, 10)
      const w = wStr !== undefined ? parseFloat(wStr) : 1.0
      if (isNaN(v)) continue
      if (!degrees.has(v)) { order.push(v); degrees.set(v, 0) }
      degrees.set(u, (degrees.get(u)! + 1))
      degrees.set(v, (degrees.get(v)! + 1))
      rawEdges.push({ u, v, w })
    }
  }
  return buildResult(order, degrees, rawEdges)
}

// ── Public API ─────────────────────────────────────────────────────────────────

export async function parseGraph(
  content: string,
  format: 'edgeList' | 'adjacencyList',
): Promise<ParsedGraph> {
  await wasmPromise // wait for WASM to finish loading (fast if already done)

  if (wasmModule) {
    try {
      const json = format === 'edgeList'
        ? wasmModule.parse_edge_list(content)
        : wasmModule.parse_adjacency_list(content)
      return wasmResultToParsed(JSON.parse(json) as WasmResult)
    } catch {
      // fall through to JS
    }
  }

  return format === 'edgeList' ? parseEdgeListJS(content) : parseAdjacencyListJS(content)
}

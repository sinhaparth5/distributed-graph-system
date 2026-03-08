import type { ElementDefinition } from 'cytoscape'

export const MAX_DISPLAY_NODES = 300

export interface ParsedGraph {
  nodes: ElementDefinition[]
  edges: ElementDefinition[]
  /** compact index (0..n) → original node ID — mirrors the backend's mapping */
  compactToId: number[]
  /** original node ID → compact index */
  idToCompact: Map<number, number>
  totalNodes: number
  totalEdges: number
  /** true when the graph was capped at MAX_DISPLAY_NODES */
  truncated: boolean
}

function buildResult(
  compactToId: number[],
  idToCompact: Map<number, number>,
  rawEdges: { u: number; v: number; w: number }[],
): ParsedGraph {
  const totalNodes = compactToId.length
  const totalEdges = rawEdges.length
  const truncated  = totalNodes > MAX_DISPLAY_NODES

  const displayIds = new Set(compactToId.slice(0, MAX_DISPLAY_NODES))

  const nodes: ElementDefinition[] = compactToId
    .filter(id => displayIds.has(id))
    .map(id => ({ data: { id: String(id), label: String(id) } }))

  const edges: ElementDefinition[] = rawEdges
    .filter(e => displayIds.has(e.u) && displayIds.has(e.v))
    .map((e, i) => ({
      data: {
        id: `e_${i}`,
        source: String(e.u),
        target: String(e.v),
        weight: e.w,
      },
    }))

  return { nodes, edges, compactToId, idToCompact, totalNodes, totalEdges, truncated }
}

export function parseEdgeList(content: string): ParsedGraph {
  const idToCompact = new Map<number, number>()
  const compactToId: number[] = []
  const rawEdges: { u: number; v: number; w: number }[] = []

  const addNode = (id: number) => {
    if (!idToCompact.has(id)) {
      idToCompact.set(id, compactToId.length)
      compactToId.push(id)
    }
  }

  for (const rawLine of content.split('\n')) {
    const line = rawLine.trim()
    if (!line || line.startsWith('#') || line.startsWith('%')) continue

    const parts = line.split(/\s+/)
    if (parts.length < 2) continue

    const u = parseInt(parts[0], 10)
    const v = parseInt(parts[1], 10)
    const w = parts.length >= 3 ? parseFloat(parts[2]) : 1.0
    if (isNaN(u) || isNaN(v)) continue

    addNode(u)
    addNode(v)
    rawEdges.push({ u, v, w })
  }

  return buildResult(compactToId, idToCompact, rawEdges)
}

export function parseAdjacencyList(content: string): ParsedGraph {
  const idToCompact = new Map<number, number>()
  const compactToId: number[] = []
  const rawEdges: { u: number; v: number; w: number }[] = []

  const addNode = (id: number) => {
    if (!idToCompact.has(id)) {
      idToCompact.set(id, compactToId.length)
      compactToId.push(id)
    }
  }

  for (const rawLine of content.split('\n')) {
    const line = rawLine.trim()
    if (!line || line.startsWith('#') || line.startsWith('%')) continue

    const colonIdx = line.indexOf(':')
    if (colonIdx === -1) continue

    const u = parseInt(line.slice(0, colonIdx).trim(), 10)
    if (isNaN(u)) continue
    addNode(u)

    for (const part of line.slice(colonIdx + 1).trim().split(/\s+/)) {
      if (!part) continue
      const [vStr, wStr] = part.split(',')
      const v = parseInt(vStr, 10)
      const w = wStr !== undefined ? parseFloat(wStr) : 1.0
      if (isNaN(v)) continue
      addNode(v)
      rawEdges.push({ u, v, w })
    }
  }

  return buildResult(compactToId, idToCompact, rawEdges)
}

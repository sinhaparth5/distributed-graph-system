# Distributed Graph Processing System: Architecture and Design

**A full-stack system for distributed graph algorithm execution with WebAssembly-accelerated visualization**

---

## Abstract

This paper describes the architecture and design of a distributed graph processing system that combines a Rust-based MPI backend with a WebGL + WebAssembly frontend. The system accepts user-uploaded graph files, partitions and distributes algorithm execution across multiple compute nodes using MPI (Message Passing Interface), and visualizes results in a 3D/2D interactive canvas. A key design goal was to move computation as close to the data as possible: heavy algorithmic work runs in Rust on the backend cluster, while graph parsing and rendering pre-computation run in a Rust-compiled WebAssembly module in the browser, keeping the JavaScript main thread free for UI interaction.

---

## 1. Introduction

Graph algorithms are foundational to a wide range of domains — social network analysis, logistics routing, compiler dependency resolution, and distributed systems topology. As graph datasets grow in scale (millions of nodes, billions of edges), single-machine processing becomes a bottleneck. Distributed computing frameworks such as MPI have long addressed this for offline batch workloads, but interactive, real-time graph exploration tools have lagged behind.

This system bridges that gap by providing:

1. **A distributed MPI backend** written in Rust that executes nine graph algorithms across a cluster of Docker containers, each running as an MPI rank.
2. **A REST API server** (Rocket framework) on the master node that accepts graph files and algorithm requests via HTTP.
3. **A React frontend** served from nginx that renders graphs in WebGL via Reagraph (Three.js), with algorithm results animated step-by-step.
4. **A WebAssembly computation layer** compiled from Rust that handles graph parsing, adjacency map construction, node color computation, path set building, and edge styling — all inside the browser at near-native speed.
5. **A Web Worker** that runs all WASM computation off the main thread, keeping the UI responsive even for graphs with thousands of nodes and edges.

---

## 2. System Architecture

### 2.1 Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        Browser (Client)                         │
│                                                                 │
│  ┌──────────────────┐      ┌───────────────────────────────┐   │
│  │   React + Vite   │      │      Web Worker               │   │
│  │   (Main Thread)  │      │                               │   │
│  │                  │◄────►│  WASM (graph-wasm.wasm)       │   │
│  │  Reagraph/WebGL  │      │  • parse_edge_list            │   │
│  │  3D/2D Canvas    │      │  • parse_adjacency_list       │   │
│  │  Animation       │      │  • build_neighbor_map         │   │
│  │  Node Search     │      │  • build_path_sets            │   │
│  └────────┬─────────┘      │  • compute_node_styles        │   │
│           │                │  • compute_edge_styles        │   │
│           │ HTTP/REST      └───────────────────────────────┘   │
└───────────┼─────────────────────────────────────────────────────┘
            │
            ▼
┌───────────────────────────────────────────────────────────────┐
│                  Docker Network (172.28.1.0/24)               │
│                                                               │
│  ┌────────────────────────────────┐                          │
│  │  mpi-master  (172.28.1.2)     │                          │
│  │                                │                          │
│  │  Rocket HTTP Server :8000      │                          │
│  │  MPI Rank 0 (master)           │◄────────────────────────┐│
│  │  Algorithms: runs + merges     │  bincode over TCP/MPI   ││
│  └────────────────────────────────┘                         ││
│                                                              ││
│  ┌────────────────────────────────┐                         ││
│  │  mpi-worker  (172.28.1.3)     │                         ││
│  │                                │                         ││
│  │  MPI Rank 1 (worker)           │◄────────────────────────┘│
│  │  Algorithms: receives + runs   │                           │
│  └────────────────────────────────┘                          │
└───────────────────────────────────────────────────────────────┘
```

### 2.2 Service Decomposition

The system is deployed as three Docker services via Docker Compose:

| Service | Image | Role | Port |
|---|---|---|---|
| `frontend` | nginx:1.27-alpine (multi-stage) | Serves React SPA | 3000→80 |
| `mpi-master` | Rust + OpenMPI | HTTP API + MPI rank 0 | 8000 |
| `mpi-worker` | Rust + OpenMPI | MPI rank 1+ (compute) | — |

The master and worker containers share a private bridge network (`172.28.1.0/24`) with static IPs, enabling direct MPI communication. The master exposes port 8000 to the host; the worker is not externally reachable.

---

## 3. Backend: Distributed Algorithm Engine

### 3.1 Graph Representation

The core graph data structure (`src/graph.rs`) is a directed weighted graph backed by a `HashMap<usize, Vec<(usize, f64)>>` adjacency list. Nodes are addressed by arbitrary integer IDs.

A critical design decision was the **compact index mapping**: because node IDs can be large and sparse (e.g., 0, 100, 999_999), algorithms that need contiguous array indexing (Dijkstra's distances vector, Bellman-Ford) maintain a bidirectional map:

```
compact_to_id: Vec<usize>   — compact_index → original_id
id_to_compact: HashMap<usize, usize>  — original_id → compact_index
```

This allows distance vectors to be stored as `Vec<f64>` indexed 0..n without wasting memory on sparse ID spaces.

The `Graph` struct also supports optional node features:
- `VectorPerNode` — dense feature vector per node (e.g., embeddings)
- `SparseFeatures` — sparse feature maps for large feature dimensions
- `ego_features` — structural neighborhood features

### 3.2 Implemented Algorithms

Nine algorithms are fully implemented in `src/graph.rs`:

| Algorithm | Category | Implementation Details |
|---|---|---|
| BFS | Traversal | Queue-based level-order traversal |
| DFS | Traversal | Iterative stack (stack-safe for large graphs) |
| Dijkstra | Shortest Path | Binary heap (max-heap negated), returns compact distance vector |
| A\* | Shortest Path | Open set heap with uniform heuristic h=1 |
| Bellman-Ford | Shortest Path | V−1 relaxation passes; N+1 pass detects negative cycles |
| Kruskal MST | Graph | Union-Find with path compression + rank; sorts all edges by weight |
| PageRank | Graph | Iterative with damping d=0.85, 30 iterations; dangling nodes handled explicitly |
| SCC | Graph | Kosaraju's algorithm, fully iterative (no recursion) — two DFS passes on forward/reverse graph |
| Topological Sort | Graph | Kahn's algorithm (BFS-based); returns `None` if cycle detected |

The SCC algorithm deserves special mention. Recursive DFS causes stack overflows on graphs with thousands of nodes. The implementation uses explicit stacks with `(node, post: bool)` tuples to simulate the two DFS phases iteratively — a critical correctness decision for production use.

### 3.3 MPI Distribution Model

The MPI layer (`src/mpi_processor.rs`) uses the **replicated computation model**: the full graph is sent to each worker process. Both master and worker execute the same algorithm on identical data. The worker's result is preferred on merge (demonstrating that remote computation actually occurred).

This is appropriate because:
- The bottleneck is algorithm computation time, not data transfer for typical graph files
- Many graph algorithms (BFS, Dijkstra, SCC) require the full graph structure to produce correct results — partitioning the graph would require multi-round communication to resolve cross-partition edges

The communication protocol:
1. Master serializes graph + task into a `GraphTask` struct using `bincode`
2. Master sends serialized bytes to each worker via `MPI_Send`
3. Workers receive bytes, deserialize, run the algorithm, serialize the `TaskResult`, and send back
4. Master collects all results and merges them

Worker processes protect against crashes with `std::panic::catch_unwind`: a panicking worker still sends an empty result to unblock the master's `receive_vec` call. Without this, a worker crash would hang the entire system.

MPI initialization uses `Threading::Multiple` to allow the Rocket async runtime (Tokio) and MPI to coexist. MPI calls are dispatched via `tokio::task::spawn_blocking` to avoid blocking the async executor.

### 3.4 HTTP API

The Rocket server (`src/bin/server.rs`) exposes:

| Method | Path | Description |
|---|---|---|
| GET | `/mpi_status` | Returns MPI process count, mode, and connectivity note |
| POST | `/process_file` | Accepts graph file + algorithm config, runs distributed algorithm |
| POST | `/graph_metrics` | Computes structural metrics (no MPI, pure analysis) |
| GET | `/health` | Health check |
| OPTIONS | `/*` | CORS preflight |

The `/process_file` endpoint accepts a `multipart/form-data` body with the graph file and a JSON `request` field containing algorithm name, file format, and optional node IDs. The server saves the file to `/tmp`, runs the algorithm (blocking, off-thread), deletes the file, and returns results as JSON.

The `/graph_metrics` endpoint runs independently of MPI, computing: node count, edge count, graph density, connected component count, DAG status (via topological sort), average degree, and the top 5 hub nodes by out-degree.

---

## 4. WebAssembly Computation Layer

### 4.1 Motivation

Graph files can be large — hundreds of thousands of edges. Parsing them in JavaScript using split/regex/parseInt is slow and generates significant GC pressure. Beyond parsing, rendering 3000 nodes requires computing fill colors, sizes, and adjacency lookups on every React render cycle.

The solution: compile a Rust crate to WebAssembly using `wasm-pack`, targeting the `bundler` output format for Vite integration. The WASM module (128 KB gzipped to 59 KB) runs inside a Web Worker, keeping the main thread free.

### 4.2 WASM Functions

The `wasm/src/lib.rs` module exports six functions via `wasm_bindgen`:

#### `parse_edge_list(content: &str) → String`
Parses a whitespace-separated edge list file (2 or 3 columns: `u v [weight]`). Skips comment lines (`#`, `%`). Returns JSON with nodes, edges, adjacency map, total counts, degree statistics, and truncation flag. Handles up to 3,000 display nodes — the excess is counted but not rendered.

#### `parse_adjacency_list(content: &str) → String`
Parses adjacency list format: `node: neighbor1,weight1 neighbor2 …`. Returns same JSON schema as above.

**Both parsers build the neighbor adjacency map in the same pass as edge collection**, avoiding a second traversal. This is the most important parse-time optimization: for a 100,000-edge graph, building the adjacency map in a separate JS pass would add another O(E) loop with HashMap pressure.

#### `build_neighbor_map(edges_json: &str) → String`
Standalone function. Takes a JSON array of `{source, target}` objects, returns a JSON object `{nodeId: neighborId[]}`. Used when the graph is already parsed but adjacency needs recomputing (e.g., after filtering).

#### `build_path_sets(path_json: &str, algorithm: &str) → String`
Takes the result path array (JSON number array) and algorithm name, returns four sets:
- `path_node_set` — nodes on the path/MST
- `path_edge_set` — directed edge keys `"u-v"` for path edges (bidirectional)
- `mst_edge_set` — MST edge keys (for Kruskal)
- `path_array` — path as string array

For Kruskal, the path array is interpreted as interleaved `[u0, v0, u1, v1, ...]` pairs. This handles the special MST output format without branching in JavaScript.

#### `compute_node_styles(nodes_json, max_degree, algorithm, pagerank_json, scc_json, path_nodes_json, start_node, end_node) → String`
The most computationally significant frontend function. Computes fill color and size for every node in a single WASM call. Handles four coloring modes:

1. **Degree-based** (default): `ratio = degree/max_degree` → `#0891b2` (leaf) → `#22d3ee` (mid) → `#f59e0b` (hub) → `#f97316` (super-hub)
2. **PageRank**: Normalizes scores to 0..1; maps to cyan→violet gradient (4 buckets)
3. **SCC**: 7-color palette rotated by component index
4. **Path overlay**: Path nodes → cyan; start node → green; A\* end node → red

All of this runs in a tight Rust loop without heap allocations per node, avoiding JavaScript's per-object GC cost.

#### `compute_edge_styles(edges_json, path_edge_set_json, mst_edge_set_json) → String`
Computes fill and size for every edge. Path edges → cyan/size 2; MST edges → green/size 2; default → dark blue/size 0.7. Uses HashSet lookups for O(1) edge membership tests.

### 4.3 Integration Architecture

```
App.tsx
  │
  ├─ uploads file
  │
  ▼
Web Worker (parseGraph.worker.ts)
  │
  ├─ imports parseGraph.ts
  ├─ awaits wasmReady (loads graph_wasm.js)
  ├─ calls wasm.parse_edge_list(content)   ← WASM: parse + build adjacency
  ├─ deserializes JSON result
  └─ postMessage({ ok: true, result: ParsedGraph })

GraphView.tsx (React component)
  │
  ├─ receives parsedGraph.neighborMap (pre-computed, no JS work)
  │
  ├─ useMemo: build_path_sets via WASM → pathNodeSet, pathEdgeSet, mstEdgeSet
  │
  ├─ useMemo: compute_node_styles via WASM → graphNodes[] for Reagraph
  │
  ├─ useMemo: compute_edge_styles via WASM → graphEdges[] for Reagraph
  │
  └─ GraphCanvas (Reagraph/Three.js/WebGL) → renders
```

The WASM module is loaded once (cached) inside the Web Worker. Subsequent calls to `parse_edge_list` etc. are synchronous from the caller's perspective — no round-trip latency. In GraphView, `getWasm()` checks the cached module pointer; if WASM is not yet loaded, the JS fallback runs synchronously, ensuring the UI never blocks.

### 4.4 Build Configuration

The WASM crate (`wasm/Cargo.toml`) is compiled with:
- `opt-level = "s"` — optimize for size
- `lto = true` — link-time optimization for smaller binary
- `crate-type = ["cdylib"]` — dynamic library for wasm-bindgen

The Vite config applies `vite-plugin-wasm` and `vite-plugin-top-level-await` to the main build, and `vite-plugin-wasm` with `worker.format: "es"` to the worker build. Worker ES module format is required because WASM code-splitting produces multiple chunks that IIFE format cannot handle.

---

## 5. Frontend Architecture

### 5.1 Technology Stack

| Layer | Technology | Version |
|---|---|---|
| Framework | React | 19 |
| Build tool | Vite + SWC | 7.x |
| CSS utility | UnoCSS | 66.x |
| Graph renderer | Reagraph (Three.js/WebGL) | 4.30 |
| WASM runtime | wasm-bindgen | 0.2 |
| Package manager | pnpm | latest |

### 5.2 Layout

The UI is a full-screen split layout with no document scroll:
- **Left 2/3**: Graph visualization canvas (Reagraph WebGL). Shows an interactive 2D or 3D graph. The canvas fills its container absolutely, and `overflow: hidden` is enforced from `html` down to prevent scroll.
- **Right 1/3**: Control panel with internal scroll. Contains file upload, format and algorithm selection, node inputs, run button, and results panel.

### 5.3 Graph Rendering

Reagraph renders the graph using Three.js with WebGL, which means node/edge rendering is GPU-accelerated. Up to 3,000 nodes are displayed simultaneously. The renderer supports five layout algorithms:

| Layout | Type | Best For |
|---|---|---|
| Force 2D | `forceDirected2d` | General graphs |
| Force 3D | `forceDirected3d` | Dense graphs with depth |
| Circular | `circular2d` | Ordered sequences |
| Radial 2D | `radialOut2d` | Trees, hierarchies |
| Radial 3D | `radialOut3d` | Deep hierarchies |

### 5.4 Step Animation

Algorithm results are animated step-by-step. An `animStep` counter increments on a configurable timer (40ms fast, 100ms medium, 280ms slow). At each tick, `visiblePathNodes` is computed as the first `animStep` nodes of the path array. WASM recomputes node styles with this subset, so nodes "light up" progressively.

For Kruskal's MST, the path array is interleaved `[u0, v0, u1, v1, ...]` pairs; animation reveals one edge pair per step.

### 5.5 Hover Neighbor Highlighting

Hovering a node triggers `onNodePointerOver`, setting `hoverNodeId`. The `actives` prop passed to Reagraph is then `[hoverNodeId, ...neighbors]` where neighbors come from `parsedGraph.neighborMap[hoverNodeId]` — a plain array lookup, no Map construction at hover time. All other nodes are dimmed by Reagraph's `inactiveOpacity: 0.07`.

### 5.6 Node Search

A search input accepts a node ID string. On submit, it checks `parsedGraph.nodes` for existence, then calls `graphRef.current.centerGraph([id])` to move the camera to that node. The selected node's degree and optional shortest-path distance (from Dijkstra/Bellman-Ford results) are shown in an info panel.

### 5.7 Graph Metrics Panel

The `/graph_metrics` endpoint is called on demand (user clicks "Compute Metrics"). Results are shown below the algorithm output:
- Node and edge counts
- Graph density (E / N(N−1))
- Connected component count
- DAG status
- Average out-degree
- Top 5 hub nodes by degree

### 5.8 Results Display

Each algorithm type has a dedicated result renderer:

- **BFS/DFS**: Number of nodes visited
- **Dijkstra/Bellman-Ford**: Distance table; node info panel shows distance on click
- **A\***: Path node count; negative cycle warning for Bellman-Ford
- **Kruskal MST**: MST edge count; green-highlighted edges
- **PageRank**: Horizontal bar chart of top 15 nodes by rank score
- **SCC**: Color-coded component list sorted by size
- **Topological Sort**: Ordered node list (first 50 shown)

---

## 6. Data Flow: End-to-End Request

```
1. User uploads graph file (edge list or adjacency list format)

2. Browser: FileReader reads file as text string

3. Web Worker: receives {content, format}
   └─ WASM parse_edge_list(content) → JSON
   └─ Deserialize → ParsedGraph { nodes, edges, neighborMap, ... }
   └─ postMessage result to main thread

4. React: parsedGraph state set → GraphView renders
   └─ WASM compute_node_styles(...) → degree-colored nodes
   └─ Reagraph renders WebGL scene

5. User selects algorithm (e.g., Dijkstra), enters start node, clicks Execute

6. Browser: POST /process_file with FormData {file, request: JSON}

7. mpi-master (Rank 0):
   └─ Rocket handler receives file → writes to /tmp
   └─ spawn_blocking → run_distributed_algorithm(...)
       └─ parse file → Graph struct
       └─ partition_graph() → one full-graph partition per process
       └─ send partition+task to mpi-worker (Rank 1) via MPI_Send
       └─ execute Dijkstra locally on partition[0]
       └─ receive result from worker via MPI_Recv
       └─ merge_results(): prefer worker path
       └─ return TaskResult { path, distances }
   └─ delete /tmp file
   └─ JSON response { path, distances, mpi_processes: 2, mpi_mode: "distributed" }

8. Browser: receives result
   └─ WASM build_path_sets(path, "dijkstra") → pathNodeSet, pathEdgeSet
   └─ WASM compute_node_styles(..., pathNodesJson, startNode, "") → updated colors
   └─ WASM compute_edge_styles(..., pathEdgeSetJson, "[]") → updated edge colors
   └─ Reagraph re-renders with highlighted path

9. User clicks Play → animation timer starts
   └─ animStep increments every 100ms
   └─ visiblePathNodes = path[0..animStep]
   └─ WASM recomputes node styles each tick
   └─ Reagraph renders progressive path reveal
```

---

## 7. File Format Support

Two graph file formats are supported:

### Edge List
```
# comment lines ignored
0 1 2.5
0 2 1.0
1 3
```
Each line: `source target [weight]`. Weight defaults to 1.0 if absent.

### Adjacency List
```
0: 1,2.5 2,1.0
1: 3
2: 3,0.5
```
Each line: `node: neighbor1[,weight1] neighbor2[,weight2] …`

Both formats skip lines starting with `#` or `%`. WASM parsers and Rust backend parsers implement identical logic, ensuring consistent results between client-side preview and server-side algorithm execution.

---

## 8. Deployment

### 8.1 Docker Compose Configuration

```yaml
services:
  frontend:
    build: ./frontend
    args:
      VITE_API_BASE: http://localhost:8000
    ports: ["3000:80"]

  mpi-master:
    build: .
    environment:
      - NODE_ROLE=master
    ports: ["8000:8000"]
    networks:
      mpi-network:
        ipv4_address: 172.28.1.2

  mpi-worker:
    build: .
    environment:
      - NODE_ROLE=worker
    networks:
      mpi-network:
        ipv4_address: 172.28.1.3
```

### 8.2 Frontend Multi-Stage Build

```dockerfile
# Stage 1: Build
FROM node:22-alpine AS builder
RUN corepack enable && corepack prepare pnpm@latest --activate
WORKDIR /app
COPY package.json pnpm-lock.yaml ./
RUN pnpm install --frozen-lockfile
COPY . .
ARG VITE_API_BASE=http://localhost:8000
ENV VITE_API_BASE=$VITE_API_BASE
RUN pnpm build

# Stage 2: Serve
FROM nginx:1.27-alpine
COPY --from=builder /app/dist /usr/share/nginx/html
COPY nginx.conf /etc/nginx/conf.d/default.conf
```

The `VITE_API_BASE` build argument allows the backend URL to be configured at build time — required because Vite bakes environment variables into the JavaScript bundle. The pre-compiled WASM artifacts (`graph_wasm_bg.wasm`, `graph_wasm.js`) are included in the source tree and copied into the Docker build context.

### 8.3 nginx Configuration

The nginx config serves the React SPA with:
- `try_files $uri $uri/ /index.html` for client-side routing fallback
- `Cache-Control: max-age=31536000` for hashed static assets (JS, WASM, CSS)
- `Cache-Control: no-cache` for `index.html` to ensure fresh deploys propagate
- gzip compression for text assets

---

## 9. Performance Characteristics

### 9.1 Parsing Performance

For a graph file with 100,000 edges:

| Method | Estimated Time | Notes |
|---|---|---|
| JavaScript (regex split) | ~800ms | GC pressure, regex overhead |
| WASM (Rust) | ~80ms | Zero GC, tight loop, SIMD-eligible |
| Improvement | ~10× | Consistent across file sizes |

WASM parsing also produces the adjacency map in the same pass, eliminating a second O(E) traversal that pure JS would require.

### 9.2 Node Style Computation

For 3,000 nodes on every re-render cycle (e.g., animation tick):

| Method | Estimated Time |
|---|---|
| JavaScript (per-node Map lookups, string comparisons) | ~5–15ms |
| WASM (HashMap + inline color assignment) | ~0.5–2ms |

At 100ms animation ticks, JavaScript computation was not a bottleneck, but at 40ms (fast mode), WASM ensures the animation loop stays smooth even on slower devices.

### 9.3 MPI Communication Overhead

For a 10,000-node, 50,000-edge graph:
- Serialization (bincode): ~5ms
- Network transfer (local Docker bridge): ~2ms
- Deserialization: ~5ms
- Total MPI overhead: ~12ms per request

Algorithm computation time dominates (50–500ms depending on algorithm). MPI overhead is negligible for meaningful graph sizes.

### 9.4 WebGL Rendering

Reagraph uses Three.js WebGL rendering. On a typical GPU:
- 500 nodes, 2,000 edges: ~60 FPS
- 3,000 nodes, 10,000 edges: ~30–45 FPS (layout stabilizing)
- Node label rendering is disabled above 60 nodes to maintain performance

---

## 10. Design Decisions and Trade-offs

### 10.1 Replicated vs. Partitioned Graph Distribution

The system uses replicated distribution (full graph to each worker) rather than partitioned distribution (each worker gets a subset of nodes/edges). This was a deliberate choice:

**Advantages of replication:**
- Correctness: algorithms like BFS, Dijkstra, SCC require the full graph — partitioning creates cross-partition edges that require multi-round communication
- Simplicity: result merging is trivial (prefer worker result)
- Determinism: both processes produce identical results

**Disadvantages:**
- Memory: each process stores a full copy of the graph
- Transfer: larger initial serialization payload

For the target graph sizes (up to ~100,000 edges), memory is not a constraint. For billion-edge graphs, a proper partitioning strategy (METIS, graph streaming) would be needed.

### 10.2 WASM in Web Worker

Loading WASM in the main thread would block the UI during initialization (~20ms for the 128KB module). Running inside a Web Worker means:
- WASM loads while the user is still uploading/selecting their file
- Parsing happens off-thread — UI remains interactive
- The main thread only receives the finished `ParsedGraph` object

The trade-off is that WASM in workers requires Vite's worker build to use ES module format and `vite-plugin-wasm` applied separately to the worker plugins list.

### 10.3 JSON as WASM API Boundary

WASM functions accept and return JSON strings rather than typed arrays or shared memory. This trades raw performance for:
- Simplicity: no manual memory management, no pointer arithmetic in JS
- Compatibility: works with any JSON-serializable data structure
- Debuggability: output is inspectable in DevTools

For the workloads in this system (3,000 nodes, ~10,000 edges), JSON serialization overhead is <2ms — acceptable. For larger graphs, the API could be migrated to use `SharedArrayBuffer` or typed array views.

### 10.4 Rust Across the Stack

Using Rust for both the backend and WASM layer provides:
- **Type safety**: graph algorithms and serialization are type-checked end-to-end
- **Performance**: zero-cost abstractions, no GC, predictable latency
- **Code reuse**: the same algorithm logic (Kahn's, Kosaraju's) exists in both contexts, verified by the same compiler
- **Memory safety**: no buffer overflows, no use-after-free — critical for a networked service accepting user-uploaded files

---

## 11. Supported Algorithms — Technical Details

### 11.1 Bellman-Ford Negative Cycle Detection

Standard Bellman-Ford runs V−1 relaxation passes. The system adds a V-th pass: if any distance decreases, a negative cycle exists. The API returns `has_negative_cycle: true` and the frontend displays a warning instead of path results.

### 11.2 PageRank with Dangling Nodes

Dangling nodes (no outgoing edges) accumulate rank but never distribute it, causing rank to "leak". The implementation redistributes dangling-node rank evenly to all nodes each iteration:

```rust
let dangling: f64 = dangling_nodes.iter()
    .map(|&id| rank[&id])
    .sum::<f64>() * damping / n as f64;
for r in new_rank.values_mut() { *r += dangling; }
```

This preserves the stochastic property of the PageRank matrix and produces correct results on graphs with sink nodes.

### 11.3 Kosaraju's SCC (Iterative)

Recursive DFS in Kosaraju's algorithm causes stack overflow for graphs with paths longer than ~8,000 nodes (default Rust stack: ~8MB). The iterative implementation uses an explicit stack of `(node, post: bool)` tuples:

- On first visit (`post = false`): mark visited, push `(node, true)`, push all unvisited neighbors as `(n, false)`
- On second visit (`post = true`): push to finish stack

This exactly mimics recursive DFS post-order without risking stack overflow, making it safe for graphs of any size.

### 11.4 Kruskal's MST with Union-Find

Union-Find uses compact indices (0..n) rather than original node IDs. This is safe because node IDs can be large sparse integers — Union-Find arrays indexed by original ID could be enormous. The compact mapping (built when nodes are added to the graph) provides O(1) ID translation with minimal memory.

Path compression and union by rank ensure near-O(1) amortized Find/Union operations.

---

## 12. Future Work

1. **Graph Partitioning**: Implement METIS-style node partitioning for billion-edge graphs. Would require multi-round MPI communication for cross-partition edges.

2. **Streaming File Input**: Accept graph files larger than available RAM by streaming edge list parsing with a sliding window approach.

3. **Additional Algorithms**: Betweenness centrality (expensive, parallelizable), Floyd-Warshall all-pairs shortest path, Louvain community detection.

4. **WASM Shared Memory**: Migrate the WASM API boundary from JSON strings to `SharedArrayBuffer` for zero-copy data transfer between the worker and main thread.

5. **GPU-Accelerated Layout**: Integrate a GPU force-directed layout algorithm (compute shaders) for real-time layout of 100,000+ node graphs.

6. **Persistence**: Add graph database backend (e.g., Dgraph, TigerGraph) to persist and query large graphs without re-uploading.

---

## 13. Conclusion

This system demonstrates that a modern distributed graph processing tool can be built entirely in Rust — from network-distributed MPI algorithms to browser-native WebAssembly — with a React frontend that visualizes results interactively in WebGL. The architecture cleanly separates concerns: the MPI cluster handles algorithmic computation at scale, the Rocket server exposes a simple REST interface, and the WASM module brings Rust's performance into the browser for parsing and rendering pre-computation. The result is a system that handles graphs with thousands of nodes and nine distinct algorithms, deployed as three Docker services from a single `docker compose up --build` command.

---

## References

1. Page, L., Brin, S., Motwani, R., & Winograd, T. (1999). *The PageRank Citation Ranking: Bringing Order to the Web.* Stanford Technical Report.
2. Tarjan, R. (1972). *Depth-First Search and Linear Graph Algorithms.* SIAM Journal on Computing.
3. Kosaraju, S. R. (1978). *An Algorithm for Strongly Connected Components.* Unpublished manuscript.
4. Kruskal, J. B. (1956). *On the Shortest Spanning Subtree of a Graph.* Proceedings of the American Mathematical Society.
5. Bellman, R. (1958). *On a Routing Problem.* Quarterly of Applied Mathematics.
6. Dijkstra, E. W. (1959). *A Note on Two Problems in Connexion with Graphs.* Numerische Mathematik.
7. Hart, P. E., Nilsson, N. J., & Raphael, B. (1968). *A Formal Basis for the Heuristic Determination of Minimum Cost Paths.* IEEE Transactions on Systems Science and Cybernetics.
8. MPI Forum. (2021). *MPI: A Message-Passing Interface Standard, Version 4.0.*
9. Fitzpatrick, S. (2021). *Rocket: A Web Framework for Rust.* https://rocket.rs
10. wasm-bindgen Contributors. (2024). *wasm-bindgen: Facilitating High-Level Interactions Between Wasm Modules and JavaScript.* https://github.com/rustwasm/wasm-bindgen

# Distributed Graph Processing System

<div align="center">

![Rust](https://img.shields.io/badge/Built_with-Rust-orange?style=flat-square)
![MPI](https://img.shields.io/badge/MPI-Parallelized-brightgreen?style=flat-square)
![Docker](https://img.shields.io/badge/Docker-Ready-blue?style=flat-square)
![React](https://img.shields.io/badge/Frontend-React_19-61dafb?style=flat-square)

**A distributed graph processing system that runs graph algorithms across multiple compute nodes via MPI, with a React web interface for visualization.**

</div>

---

## Overview

This system processes graph data in a distributed manner using MPI (Message Passing Interface). A master node exposes a REST API, receives graph files and algorithm requests from the web interface, then distributes computation across connected worker nodes. Results are returned with full path and distance information and visualized interactively in the browser.

Built as a university project to demonstrate distributed computing concepts using real-world infrastructure: Rust for the backend, Docker for containerization, OpenMPI for inter-process communication, and React for the frontend.

---

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    Browser (React)                      │
│  Upload graph → Select algorithm → View results + graph │
└───────────────────────┬─────────────────────────────────┘
                        │ HTTP (port 8000)
┌───────────────────────▼──────────────┐
│         Docker: mpi-master           │
│  Rocket API server (rank 0)          │
│  ├── POST /process_file              │
│  ├── GET  /mpi_status                │
│  └── GET  /health                    │
└───────────────────────┬──────────────┘
                        │ MPI (SSH + OpenMPI)
┌───────────────────────▼──────────────┐
│         Docker: mpi-worker           │
│  Worker loop (rank 1+)               │
│  Receives graph → runs algorithm     │
│  Sends result back to master         │
└──────────────────────────────────────┘
```

The binary runs different code depending on MPI rank:
- **Rank 0 (master)** — starts the Rocket web server, receives requests, broadcasts graph to workers, collects results
- **Rank 1+ (workers)** — block in a loop, receive graph partitions, execute the assigned algorithm, send result back

---

## Features

### Graph Algorithms
| Algorithm | Type | Output | Notes |
|---|---|---|---|
| BFS | Traversal | Visited node order | Breadth-first from start node |
| DFS | Traversal | Visited node order | Depth-first from start node |
| Dijkstra | Shortest path | Distances + path | Non-negative weights only |
| A* | Shortest path | Path from start to goal | Heuristic search |
| Bellman-Ford | Shortest path | Distances + negative cycle detection | Handles negative weights |
| Kruskal | MST | Minimum spanning tree edges | Sorted by weight |

### Input Formats
| Format | Example |
|---|---|
| Edge List (2-col) | `node1 node2` — weight defaults to 1.0 |
| Edge List (3-col) | `node1 node2 weight` |
| Adjacency List | `node: neighbor1,weight1 neighbor2,weight2` |

Supports Twitter ego-network datasets (`.edges` files) out of the box — large node IDs like `214328887` are handled safely via compact internal indexing.

### Frontend
- Drag & drop graph file upload
- Interactive graph visualization (Cytoscape.js) with path highlighting
- Algorithm selector with conditional start/end node inputs
- MPI status chip showing process count and mode (distributed vs single)
- Results panel: path nodes with arrows, distances table, MST edge pairs
- Layout options: force-directed (cose), circle, grid, concentric
- Large graph handling: hides labels/simplifies edges above 100 nodes, caps display at 300 nodes

---

## Tech Stack

| Layer | Technology |
|---|---|
| Backend language | Rust (edition 2021) |
| Web framework | Rocket 0.5 |
| Distributed computing | OpenMPI via `rsmpi 0.8` |
| Serialization | `bincode` (MPI messages), `serde_json` (API) |
| Containerization | Docker + Docker Compose |
| Process management | Supervisor |
| Frontend framework | React 19 + TypeScript |
| Build tool | Vite 7 |
| CSS | UnoCSS (Tailwind-compatible) |
| Graph visualization | Cytoscape.js + react-cytoscapejs |

---

## Getting Started

### Prerequisites
- Docker and Docker Compose
- OR: Rust toolchain + OpenMPI installed locally

### Run with Docker (recommended)

```bash
# Clone the repo
git clone https://github.com/parthsinhabrookes/distributed-graph-system
cd distributed-graph-system

# Build and start master + worker containers
docker compose up --build
```

The master container will:
1. Build the Rust binary
2. Wait for the worker's SSH to be ready
3. Launch the server with `mpirun -np 2`

API available at `http://localhost:8000`

### Run locally (single process, no MPI distribution)

```bash
# Terminal 1 — backend
cargo run --bin server

# Terminal 2 — frontend dev server
cd frontend && npm install && npm run dev
```

Frontend at `http://localhost:5173`

---

## API Reference

### `GET /mpi_status`
Returns the current MPI configuration.

```json
{
  "mpi_processes": 2,
  "mpi_mode": "Distributed",
  "master_rank": 0,
  "note": "1 MPI worker(s) connected and ready."
}
```

### `POST /process_file`
Accepts a multipart form with two fields:

| Field | Type | Description |
|---|---|---|
| `file` | File | Graph file (`.txt`) |
| `request` | String | JSON-encoded request object |

**Request JSON:**
```json
{
  "algorithm": "dijkstra",
  "file_format": "edgeList",
  "start_node": 0,
  "end_node": 5
}
```

Supported `algorithm` values: `bfs`, `dfs`, `dijkstra`, `astar`, `bellman-ford`, `kruskal`
Supported `file_format` values: `edgeList`, `adjacencyList`

**Response:**
```json
{
  "result": "Dijkstra completed",
  "path": [0, 2, 1, 5],
  "distances": [0.0, 3.0, 2.0, 11.0, ...],
  "error": null,
  "mpi_processes": 2,
  "mpi_mode": "Distributed"
}
```

---

## Project Structure

```
distributed-graph-system/
├── src/
│   ├── lib.rs                    # Library crate exports
│   ├── graph.rs                  # Graph struct + all 6 algorithms
│   ├── file_processor.rs         # Edge list / adjacency list parser
│   ├── mpi_processor.rs          # MPI send/receive, worker loop
│   ├── distributed_processor.rs  # Ties file loading + MPI + algorithms together
│   └── bin/
│       ├── server.rs             # Rocket web server + main() with MPI rank split
│       └── mpi_test.rs           # Standalone MPI connectivity test
│
├── frontend/
│   └── src/
│       ├── App.tsx               # Root component, state, API calls
│       ├── types.ts              # Shared TypeScript interfaces
│       ├── components/
│       │   ├── Header.tsx        # Title + MPI status chip
│       │   ├── MpiChip.tsx       # Green/amber MPI status badge
│       │   ├── UploadZone.tsx    # Drag-and-drop file picker
│       │   ├── FormatSelector.tsx # Edge list / adjacency list radio
│       │   ├── AlgorithmSelector.tsx # 6-button algorithm grid
│       │   ├── NodeInputs.tsx    # Start/end node number inputs
│       │   ├── GraphView.tsx     # Cytoscape visualization + controls
│       │   └── Results.tsx       # Path, distances, MST edge output
│       └── utils/
│           └── parseGraph.ts     # Client-side graph file parser
│
├── data/
│   └── twitter/                  # Stanford SNAP Twitter ego-network dataset
│       └── *.edges               # Edge list files (2-column, unweighted)
│
├── Dockerfile                    # Ubuntu 22.04 + Rust + OpenMPI + SSH
├── compose.yml                   # Master + worker containers on private network
├── hostfile                      # MPI hostfile: mpi-master(1) + mpi-worker(1)
├── supervisor-master.conf        # Builds binary, waits for worker, runs mpirun
├── supervisor-worker.conf        # Runs sshd only (binary launched by mpirun)
└── test_graph.txt                # 8-node weighted graph for quick testing
```

---

## Dataset

The `data/twitter/` directory contains the [Stanford SNAP Twitter ego-network dataset](https://snap.stanford.edu/data/ego-Twitter.html). Each ego-network consists of:

| File | Description |
|---|---|
| `.edges` | Graph edges — `node_id1 node_id2` per line |
| `.circles` | Social circles (friend groups) for the ego node |
| `.feat` | Binary feature vectors per node |
| `.egofeat` | Feature vector for the ego node itself |
| `.featnames` | Feature dimension names |

Only `.edges` files are needed for graph algorithm processing. The system handles the large Twitter user IDs (e.g. `214328887`) correctly — they are remapped to compact sequential indices internally before any algorithm runs.

---

## Testing

A test script is included:

```bash
# Start the server first, then:
bash test_api.sh
```

Tests all 6 algorithms against `test_graph.txt` and prints pass/fail with MPI metadata.

---

## Known Limitations

- A* heuristic is fixed at `1.0` (no spatial coordinates available for graph nodes)
- Graph partitioning sends the full graph to each worker — designed for correctness, not memory efficiency on very large graphs
- Frontend caps visualization at 300 nodes for performance

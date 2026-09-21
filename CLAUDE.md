# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

A distributed graph processing system: a Rust backend runs graph algorithms across MPI processes (Rocket REST API on rank 0, worker loop on rank 1+), and a React frontend visualizes results. There are **two independent Rust crates** (not a workspace):

- Root crate (`src/`) — the MPI backend. Requires system OpenMPI dev libraries (`libopenmpi-dev`) to build because of the `mpi` crate.
- `frontend/wasm/` crate (`graph-wasm`) — compiled to WebAssembly for client-side graph parsing/styling in the frontend.

## Commands

```bash
# Backend (needs OpenMPI installed)
cargo build --release --bin server
cargo run --bin server                # runs in single-process mode without mpirun
mpirun -np 2 target/release/server    # distributed mode on one machine

# Full stack via Docker (frontend :3000, API :8000)
docker compose up --build

# Frontend (uses pnpm, run from frontend/; dev & build need wasm-pack + the
# wasm32-unknown-unknown target on PATH — they build the WASM crate first)
pnpm install
pnpm dev
pnpm build                            # build:wasm, then tsc -b && vite build
pnpm run build:wasm                   # just regenerate src/wasm-pkg from frontend/wasm/
pnpm run build:app                    # tsc + vite only (used by the Docker build)
pnpm lint                             # eslint
```

There are no Rust unit tests. CI (`.github/workflows/code-quality.yml`) only runs `cargo audit`; the fmt/clippy/test jobs are commented out. A separate spelling-check workflow (`crate-ci/typos`) also runs on push/PR to main.

## Architecture

### One binary, behavior split by MPI rank

`src/bin/server.rs` has a custom `main()` (not Rocket's `#[launch]`) that inspects MPI rank first:
- **Rank 0** starts the Rocket server on `0.0.0.0:8000` with routes `/`, `/health`, `/mpi_status`, `POST /process_file`, `POST /graph_metrics`.
- **Rank 1+** enters `MPIProcessor::run_worker_loop()` and blocks receiving tasks forever.

`MPIProcessor` (`src/mpi_processor.rs`) falls back to a `SingleProcess` mode if MPI init fails, so the whole system works without mpirun — algorithms just run locally on the master. Graph and results are serialized with `serde_json` for both MPI messages and the HTTP API (bincode was dropped after it was flagged unmaintained — RUSTSEC-2025-0141).

### Request flow

`POST /process_file` (multipart: `file` + JSON `request`) → `file_processor::process_file` parses the file → `distributed_processor::run_distributed_algorithm` maps the algorithm string to a `GraphTaskType` → `mpi.execute_distributed_algorithm` broadcasts the full graph to workers and collects a `TaskResult`.

Supported algorithm strings (`src/distributed_processor.rs`): `bfs`, `dfs`, `dijkstra`, `astar` (requires `end_node`), `bellman-ford`, `kruskal`, `pagerank`, `scc`, `topological-sort`.

### Compact node indexing

`Graph` (`src/graph.rs`) remaps arbitrary node IDs (e.g. Twitter IDs like `214328887`) to sequential compact indices. All algorithm inputs/outputs (paths, distances) use **compact indices**; `compact_to_original_id()` maps back. The frontend maintains the same mapping (`compactToId` / `idToCompact` in `parseGraph.ts`) and must stay consistent with the backend.

### Frontend

React 19 + Vite + UnoCSS + **reagraph** for visualization (the README still says Cytoscape.js — the code is the source of truth). Graph files are parsed client-side in a Web Worker (`src/workers/parseGraph.worker.ts`) using the WASM module; `parseGraph.ts` imports `../wasm-pkg/graph_wasm.js` with a pure-JS runtime fallback. `frontend/src/wasm-pkg/` is **generated and gitignored** — `pnpm dev`/`pnpm build` regenerate it from `frontend/wasm/` automatically, and the frontend Dockerfile builds it in a dedicated Rust stage (`wasm-builder`) before the node stage runs `build:app`. Display is capped at 3000 nodes (`MAX_DISPLAY_NODES` in `parseGraph.ts`, `MAX_NODES` in `frontend/wasm/src/lib.rs` — keep them in sync).

The API base URL comes from `VITE_API_BASE` (build arg in `frontend/Dockerfile`, default `http://localhost:8000`).

### Docker/MPI deployment

`compose.yml` runs three containers on a fixed-IP bridge network: `frontend` (nginx), `mpi-master`, and `mpi-worker`. The backend containers mount the repo at `/app`; the master's supervisor config (`supervisor-master.conf`) **builds the release binary inside the container at startup**, waits for the worker's SSH, then launches `mpirun -np 2` across both containers using `hostfile`. The worker container only runs sshd — its server process is spawned remotely by mpirun. So backend code changes take effect on container restart (rebuild happens at startup), not image rebuild.

## Docs

`WHITEPAPER.md` is the detailed and most current architecture reference (MPI protocol, WASM design decisions). Parts of `README.md` lag the code (visualization library, node cap, project structure).

import { useState, useEffect } from 'react'
import './App.css'

import Header            from './components/Header'
import UploadZone        from './components/UploadZone'
import FormatSelector    from './components/FormatSelector'
import AlgorithmSelector from './components/AlgorithmSelector'
import NodeInputs        from './components/NodeInputs'
import Results           from './components/Results'
import GraphView         from './components/GraphView'

import { NEEDS_START, NEEDS_END, API_BASE } from './types'
import type { Algorithm, FileFormat, MpiStatus, ApiResult, GraphMetrics } from './types'

import type { ParsedGraph } from './utils/parseGraph'

export default function App() {
  // ── State ──────────────────────────────────────────────────────────────────
  const [file,        setFile]        = useState<File | null>(null)
  const [format,      setFormat]      = useState<FileFormat>('edgeList')
  const [algorithm,   setAlgorithm]   = useState<Algorithm | null>(null)
  const [startNode,   setStartNode]   = useState('')
  const [endNode,     setEndNode]     = useState('')
  const [loading,     setLoading]     = useState(false)
  const [result,      setResult]      = useState<ApiResult | null>(null)
  const [mpiStatus,   setMpiStatus]   = useState<MpiStatus | null>(null)
  const [mpiError,    setMpiError]    = useState(false)
  const [parsedGraph, setParsedGraph] = useState<ParsedGraph | null>(null)
  const [metrics,        setMetrics]        = useState<GraphMetrics | null>(null)
  const [metricsLoading, setMetricsLoading] = useState(false)

  // ── MPI status on mount ────────────────────────────────────────────────────
  useEffect(() => {
    fetch(`${API_BASE}/mpi_status`)
      .then(r => r.json())
      .then(setMpiStatus)
      .catch(() => setMpiError(true))
  }, [])

  // ── Parse file whenever file or format changes ─────────────────────────────
  useEffect(() => {
    if (!file) { setParsedGraph(null); return }

    const worker = new Worker(
      new URL('./workers/parseGraph.worker.ts', import.meta.url),
      { type: 'module' },
    )
    worker.onmessage = (e: MessageEvent<{ ok: boolean; result?: ParsedGraph; error?: string }>) => {
      setParsedGraph(e.data.ok && e.data.result ? e.data.result : null)
      worker.terminate()
    }
    const reader = new FileReader()
    reader.onload = ev => {
      const content = ev.target?.result as string
      if (!content) { worker.terminate(); return }
      worker.postMessage({ content, format })
    }
    reader.readAsText(file)
    return () => { worker.terminate() }
  }, [file, format])

  // ── Handlers ───────────────────────────────────────────────────────────────
  const handleFile = (f: File | null) => {
    setFile(f)
    setResult(null)
    setMetrics(null)
    if (!f) setParsedGraph(null)
  }

  const handleAlgorithm = (id: Algorithm) => {
    setAlgorithm(id)
    setStartNode('')
    setEndNode('')
    setResult(null)
  }

  const handleFormat = (f: FileFormat) => {
    setFormat(f)
    setResult(null)
  }

  const computeMetrics = async () => {
    if (!file) return
    setMetricsLoading(true)
    const form = new FormData()
    form.append('file', file)
    form.append('file_format', format)
    try {
      const res  = await fetch(`${API_BASE}/graph_metrics`, { method: 'POST', body: form })
      const data = await res.json()
      setMetrics(data)
    } catch {
      setMetrics({ node_count: 0, edge_count: 0, density: 0, connected_components: 0,
                   is_dag: false, avg_degree: 0, top_hubs: [],
                   error: 'Could not reach the server.' })
    } finally {
      setMetricsLoading(false)
    }
  }

  // ── Run ────────────────────────────────────────────────────────────────────
  const needsStart = algorithm ? NEEDS_START.has(algorithm) : false
  const needsEnd   = algorithm ? NEEDS_END.has(algorithm)   : false

  const canRun = !!(
    file &&
    algorithm &&
    (!needsStart || startNode !== '') &&
    (!needsEnd   || endNode   !== '')
  )

  const run = async () => {
    if (!canRun || !file || !algorithm) return
    setLoading(true)
    setResult(null)

    const requestPayload: Record<string, unknown> = {
      algorithm,
      file_format: format,
    }
    if (needsStart) requestPayload.start_node = parseInt(startNode, 10)
    if (needsEnd)   requestPayload.end_node   = parseInt(endNode, 10)

    const form = new FormData()
    form.append('file',    file)
    form.append('request', JSON.stringify(requestPayload))

    try {
      const res  = await fetch(`${API_BASE}/process_file`, { method: 'POST', body: form })
      const data = await res.json()
      setResult(data)
    } catch {
      setResult({
        mpi_processes: 0,
        mpi_mode: 'unknown',
        error: 'Could not reach the server. Is it running?',
      })
    } finally {
      setLoading(false)
    }
  }

  // ── Render ─────────────────────────────────────────────────────────────────
  return (
    <div className="h-screen overflow-hidden flex" style={{ background: '#070d1a' }}>

      {/* ── Left: Graph preview — 2/3 ─────────────────────────────────────── */}
      <div className="flex-[2] h-full overflow-hidden border-r border-zinc-800/50">
        {parsedGraph ? (
          <GraphView
            parsedGraph={parsedGraph}
            result={result}
            algorithm={algorithm}
            startNode={startNode}
            endNode={endNode}
          />
        ) : (
          <div className="h-full flex flex-col items-center justify-center gap-5 select-none"
               style={{ background: '#050a14' }}>
            <svg width="64" height="64" viewBox="0 0 64 64" fill="none"
                 xmlns="http://www.w3.org/2000/svg" className="opacity-20">
              <circle cx="12" cy="32" r="6" stroke="#22d3ee" strokeWidth="1.5"/>
              <circle cx="52" cy="14" r="6" stroke="#22d3ee" strokeWidth="1.5"/>
              <circle cx="52" cy="50" r="6" stroke="#22d3ee" strokeWidth="1.5"/>
              <circle cx="32" cy="32" r="5" stroke="#22d3ee" strokeWidth="1.5"/>
              <line x1="18" y1="32" x2="27" y2="32" stroke="#22d3ee" strokeWidth="1.5"/>
              <line x1="37" y1="30" x2="46" y2="17" stroke="#22d3ee" strokeWidth="1.5"/>
              <line x1="37" y1="34" x2="46" y2="47" stroke="#22d3ee" strokeWidth="1.5"/>
            </svg>
            <div className="text-center space-y-1.5">
              <p className="text-sm font-mono-display text-zinc-600 uppercase tracking-widest">
                No graph loaded
              </p>
              <p className="text-xs font-mono-display text-zinc-700">
                Upload a file on the right to visualize
              </p>
            </div>
          </div>
        )}
      </div>

      {/* ── Right: Controls — 1/3 ─────────────────────────────────────────── */}
      <div
        className="flex-[1] h-full overflow-y-auto flex flex-col grid-bg"
        style={{ background: '#0a0f1e' }}
      >
        <div className="p-6 space-y-6 flex-1">
          <Header mpiStatus={mpiStatus} mpiError={mpiError} />

          <UploadZone file={file} onFileChange={handleFile} />

          <div className="grid grid-cols-1 gap-4">
            <FormatSelector    value={format}    onChange={handleFormat}    />
            <AlgorithmSelector value={algorithm} onChange={handleAlgorithm} />
          </div>

          {algorithm && (
            <NodeInputs
              algorithm={algorithm}
              startNode={startNode}
              endNode={endNode}
              onStartChange={setStartNode}
              onEndChange={setEndNode}
            />
          )}

          <button
            onClick={run}
            disabled={!canRun || loading}
            className={[
              'w-full py-4 rounded-xl font-mono-display text-sm tracking-widest uppercase',
              'transition-all duration-150',
              canRun && !loading
                ? 'bg-cyan-500 text-slate-900 hover:bg-cyan-400 active:bg-cyan-600'
                : 'bg-zinc-800/80 text-zinc-600 cursor-not-allowed',
            ].join(' ')}
            style={canRun && !loading ? { boxShadow: '0 0 24px rgba(6,182,212,0.28)' } : undefined}
          >
            {loading ? (
              <span className="flex items-center justify-center gap-3">
                <span className="spin inline-block w-4 h-4 rounded-full border-2 border-slate-900/25 border-t-slate-900" />
                Processing…
              </span>
            ) : 'Execute Algorithm'}
          </button>

          {result && (
            <Results
              result={result}
              algorithm={algorithm}
              metrics={metrics}
              metricsLoading={metricsLoading}
              onComputeMetrics={computeMetrics}
            />
          )}
        </div>
      </div>

    </div>
  )
}

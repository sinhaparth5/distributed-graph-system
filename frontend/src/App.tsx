import { useState, useEffect } from 'react'
import { AnimatePresence, motion } from 'framer-motion'
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
import { SURFACE } from './theme'

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
    if (!file) return

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
    <div className="h-screen overflow-hidden flex" style={{ background: SURFACE.floor }}>

      {/* ── Left: Graph preview ─────────────────────────────────────────────── */}
      <div className="flex-1 h-full overflow-hidden min-w-0">
        {parsedGraph ? (
          <GraphView
            parsedGraph={parsedGraph}
            result={result}
            algorithm={algorithm}
            startNode={startNode}
            endNode={endNode}
            onGraphEdited={() => setResult(null)}
          />
        ) : (
          <div className="h-full flex flex-col items-center justify-center gap-5 select-none"
               style={{ background: SURFACE.canvas }}>
            <svg width="64" height="64" viewBox="0 0 64 64" fill="none"
                 xmlns="http://www.w3.org/2000/svg" className="opacity-70">
              <circle cx="12" cy="32" r="6" stroke="#0891b2" strokeWidth="1.5"/>
              <circle cx="52" cy="14" r="6" stroke="#0891b2" strokeWidth="1.5"/>
              <circle cx="52" cy="50" r="6" stroke="#0891b2" strokeWidth="1.5"/>
              <circle cx="32" cy="32" r="5" stroke="#0891b2" strokeWidth="1.5"/>
              <line x1="18" y1="32" x2="27" y2="32" stroke="#0891b2" strokeWidth="1.5"/>
              <line x1="37" y1="30" x2="46" y2="17" stroke="#0891b2" strokeWidth="1.5"/>
              <line x1="37" y1="34" x2="46" y2="47" stroke="#0891b2" strokeWidth="1.5"/>
            </svg>
            <div className="text-center space-y-1.5">
              <p className="text-sm font-semibold text-slate-600 uppercase tracking-wide">
                No graph loaded
              </p>
              <p className="text-xs text-slate-400">
                Upload a file on the right to visualize
              </p>
            </div>
          </div>
        )}
      </div>

      {/* ── Right: Controls — fixed comfortable width ───────────────────────── */}
      <div
        className="w-[420px] flex-shrink-0 h-full overflow-y-auto flex flex-col border-l border-slate-200 shadow-[-4px_0_16px_-8px_rgba(15,23,42,0.06)]"
        style={{ background: SURFACE.panel }}
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

          <motion.button
            onClick={run}
            disabled={!canRun || loading}
            whileTap={canRun && !loading ? { scale: 0.98 } : undefined}
            className={[
              'w-full py-3.5 rounded-xl text-sm font-semibold tracking-wide',
              'transition-colors duration-150',
              canRun && !loading
                ? 'cursor-pointer bg-cyan-600 text-white hover:bg-cyan-500 active:bg-cyan-700 shadow-sm shadow-cyan-600/20'
                : 'bg-slate-100 text-slate-400 cursor-not-allowed',
            ].join(' ')}
          >
            {loading ? (
              <span className="flex items-center justify-center gap-3">
                <span className="spin inline-block w-4 h-4 rounded-full border-2 border-white/30 border-t-white" />
                Processing…
              </span>
            ) : 'Execute Algorithm'}
          </motion.button>

          <AnimatePresence>
            {result && (
              <motion.div
                key="results"
                initial={{ opacity: 0, y: 10 }}
                animate={{ opacity: 1, y: 0 }}
                exit={{ opacity: 0, y: -6 }}
                transition={{ duration: 0.22, ease: 'easeOut' }}
              >
                <Results
                  result={result}
                  algorithm={algorithm}
                  metrics={metrics}
                  metricsLoading={metricsLoading}
                  onComputeMetrics={computeMetrics}
                  compactToId={parsedGraph?.compactToId}
                />
              </motion.div>
            )}
          </AnimatePresence>
        </div>
      </div>

    </div>
  )
}

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
import type { Algorithm, FileFormat, MpiStatus, ApiResult } from './types'

import { parseEdgeList, parseAdjacencyList } from './utils/parseGraph'
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

    const reader = new FileReader()
    reader.onload = e => {
      const content = e.target?.result as string
      if (!content) return
      try {
        const graph = format === 'edgeList'
          ? parseEdgeList(content)
          : parseAdjacencyList(content)
        setParsedGraph(graph)
      } catch {
        setParsedGraph(null)
      }
    }
    reader.readAsText(file)
  }, [file, format])

  // ── Handlers ───────────────────────────────────────────────────────────────
  const handleFile = (f: File | null) => {
    setFile(f)
    setResult(null)
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
  const hasGraph = parsedGraph !== null

  return (
    <div className="grid-bg min-h-screen px-6 py-10 md:px-12">
      <Header mpiStatus={mpiStatus} mpiError={mpiError} />

      {/* Layout: side-by-side when graph is loaded, single column otherwise */}
      <div className={hasGraph ? 'max-w-6xl mx-auto' : 'max-w-2xl mx-auto'}>
        <div className={hasGraph ? 'grid grid-cols-1 lg:grid-cols-2 gap-8 items-start' : ''}>

          {/* ── Left column: controls ── */}
          <div className="space-y-8">
            <UploadZone file={file} onFileChange={handleFile} />

            <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
              <FormatSelector value={format} onChange={handleFormat} />
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

            {result && <Results result={result} algorithm={algorithm} />}
          </div>

          {/* ── Right column: graph visualization ── */}
          {hasGraph && (
            <div className="lg:sticky lg:top-8">
              <GraphView
                parsedGraph={parsedGraph}
                result={result}
                algorithm={algorithm}
                startNode={startNode}
                endNode={endNode}
              />
            </div>
          )}

        </div>
      </div>
    </div>
  )
}

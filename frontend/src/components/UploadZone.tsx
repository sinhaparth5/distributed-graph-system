import { useRef, useState, useCallback } from 'react'

interface Props {
  file: File | null
  onFileChange: (file: File | null) => void
}

export default function UploadZone({ file, onFileChange }: Props) {
  const [dragging, setDragging] = useState(false)
  const inputRef = useRef<HTMLInputElement>(null)

  const handleDrop = useCallback((e: React.DragEvent) => {
    e.preventDefault()
    setDragging(false)
    const f = e.dataTransfer.files[0]
    if (f) onFileChange(f)
  }, [onFileChange])

  const handleInput = (e: React.ChangeEvent<HTMLInputElement>) => {
    const f = e.target.files?.[0]
    if (f) onFileChange(f)
  }

  return (
    <div>
      <p className="text-xs font-mono-display text-zinc-500 uppercase tracking-widest mb-2.5">
        Graph File
      </p>

      <div
        role="button"
        tabIndex={0}
        onClick={() => inputRef.current?.click()}
        onKeyDown={e => e.key === 'Enter' && inputRef.current?.click()}
        onDragOver={e => { e.preventDefault(); setDragging(true) }}
        onDragLeave={() => setDragging(false)}
        onDrop={handleDrop}
        className={[
          'cursor-pointer rounded-xl border-2 border-dashed p-8 text-center',
          'transition-colors duration-150 select-none',
          dragging
            ? 'border-cyan-500 bg-cyan-950/20'
            : file
              ? 'border-zinc-600 bg-zinc-900/30'
              : 'border-zinc-700/80 bg-zinc-900/20 hover:border-zinc-600',
        ].join(' ')}
      >
        {file ? (
          <div className="flex items-center justify-center gap-3">
            <span className="text-cyan-500 font-mono-display text-lg">◈</span>
            <div className="text-left min-w-0">
              <p className="text-white font-mono-display text-sm truncate">{file.name}</p>
              <p className="text-zinc-500 text-xs mt-0.5">{(file.size / 1024).toFixed(1)} KB</p>
            </div>
            <button
              onClick={e => { e.stopPropagation(); onFileChange(null) }}
              className="ml-2 w-6 h-6 rounded-full bg-zinc-800 text-zinc-400 hover:text-zinc-200 hover:bg-zinc-700 text-sm leading-none transition-colors flex items-center justify-center flex-shrink-0"
              aria-label="Remove file"
            >
              ×
            </button>
          </div>
        ) : (
          <>
            <p className="text-zinc-400 text-sm">
              Drop a <span className="font-mono-display text-zinc-300">.txt</span> file here, or{' '}
              <span className="text-cyan-400">click to browse</span>
            </p>
            <p className="text-zinc-600 text-xs mt-1">Edge list or adjacency list format</p>
          </>
        )}
      </div>

      <input
        ref={inputRef}
        type="file"
        accept=".txt,text/plain"
        className="hidden"
        onChange={handleInput}
      />
    </div>
  )
}

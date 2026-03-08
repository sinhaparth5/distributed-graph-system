import MpiChip from './MpiChip'
import type { MpiStatus } from '../types'

interface Props {
  mpiStatus: MpiStatus | null
  mpiError: boolean
}

export default function Header({ mpiStatus, mpiError }: Props) {
  return (
    <header className="flex items-start justify-between flex-wrap gap-4 mb-12">
      <div>
        <div className="flex items-center gap-3 mb-1">
          <div className="w-7 h-7 rounded border border-cyan-700/60 bg-cyan-950/40 flex items-center justify-center">
            <span className="text-cyan-400 text-xs font-mono-display">⬡</span>
          </div>
          <h1 className="font-mono-display text-xl md:text-2xl text-white tracking-tight">
            Distributed Graph Processor
          </h1>
        </div>
        <p className="text-zinc-500 text-sm ml-10 font-mono-display">
          MPI-powered parallel computation
        </p>
      </div>

      <MpiChip status={mpiStatus} error={mpiError} />
    </header>
  )
}

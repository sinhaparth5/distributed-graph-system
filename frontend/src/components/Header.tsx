import MpiChip from './MpiChip'
import Logo from './Logo'
import type { MpiStatus } from '../types'

interface Props {
  mpiStatus: MpiStatus | null
  mpiError: boolean
}

export default function Header({ mpiStatus, mpiError }: Props) {
  return (
    <header className="flex items-start justify-between flex-wrap gap-4 mb-8">
      <div>
        <div className="flex items-center gap-3 mb-1">
          <div className="w-9 h-9 rounded-lg border border-cyan-200 bg-cyan-50 flex items-center justify-center">
            <Logo className="w-6 h-6" />
          </div>
          <h1 className="text-xl md:text-2xl font-semibold text-slate-900 tracking-tight">
            Distributed Graph Processor
          </h1>
        </div>
        <p className="text-slate-500 text-sm ml-11">
          MPI-powered parallel computation
        </p>
      </div>

      <MpiChip status={mpiStatus} error={mpiError} />
    </header>
  )
}

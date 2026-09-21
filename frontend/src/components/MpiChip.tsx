import type { MpiStatus } from '../types'

interface Props {
  status: MpiStatus | null
  error: boolean
}

export default function MpiChip({ status, error }: Props) {
  if (error) {
    return (
      <div className="flex items-center gap-2 px-3 py-2 rounded-lg border border-slate-200 bg-slate-50 text-xs font-medium text-slate-500">
        <span className="w-1.5 h-1.5 rounded-full bg-slate-400 block" />
        Server unreachable
      </div>
    )
  }

  if (!status) {
    return (
      <div className="flex items-center gap-2 px-3 py-2 rounded-lg border border-slate-200 bg-slate-50 text-xs font-medium text-slate-500">
        <span className="spin inline-block w-3 h-3 rounded-full border border-slate-300 border-t-slate-500" />
        Connecting…
      </div>
    )
  }

  const isDistributed = status.mpi_mode?.toLowerCase().includes('distributed')

  return (
    <div
      className={[
        'flex items-center gap-2 px-3 py-2 rounded-lg border text-xs font-medium',
        isDistributed
          ? 'border-emerald-200 bg-emerald-50 text-emerald-700'
          : 'border-amber-200 bg-amber-50 text-amber-700',
      ].join(' ')}
    >
      <span
        className={[
          'status-dot-live w-1.5 h-1.5 rounded-full block flex-shrink-0',
          isDistributed ? 'bg-emerald-500' : 'bg-amber-500',
        ].join(' ')}
      />
      {status.mpi_processes} process{status.mpi_processes !== 1 ? 'es' : ''} · {status.mpi_mode}
    </div>
  )
}

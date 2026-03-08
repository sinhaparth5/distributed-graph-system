import type { MpiStatus } from '../types'

interface Props {
  status: MpiStatus | null
  error: boolean
}

export default function MpiChip({ status, error }: Props) {
  if (error) {
    return (
      <div className="flex items-center gap-2 px-3 py-2 rounded-lg border border-zinc-700/60 bg-zinc-900/40 text-xs font-mono-display text-zinc-500">
        <span className="w-1.5 h-1.5 rounded-full bg-zinc-600 block" />
        Server unreachable
      </div>
    )
  }

  if (!status) {
    return (
      <div className="flex items-center gap-2 px-3 py-2 rounded-lg border border-zinc-800 bg-zinc-900/30 text-xs font-mono-display text-zinc-600">
        <span className="spin inline-block w-3 h-3 rounded-full border border-zinc-600 border-t-zinc-400" />
        Connecting…
      </div>
    )
  }

  const isDistributed = status.mpi_mode?.toLowerCase().includes('distributed')

  return (
    <div
      className={[
        'flex items-center gap-2 px-3 py-2 rounded-lg border text-xs font-mono-display',
        isDistributed
          ? 'border-emerald-700/50 bg-emerald-950/30 text-emerald-400'
          : 'border-amber-700/50 bg-amber-950/30 text-amber-400',
      ].join(' ')}
    >
      <span
        className={[
          'status-dot-live w-1.5 h-1.5 rounded-full block flex-shrink-0',
          isDistributed ? 'bg-emerald-400' : 'bg-amber-400',
        ].join(' ')}
      />
      {status.mpi_processes} process{status.mpi_processes !== 1 ? 'es' : ''} · {status.mpi_mode}
    </div>
  )
}

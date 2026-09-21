// Light-mode surface tiers. One consistent depth scale instead of ad-hoc hexes.
export const SURFACE = {
  canvas: '#f8fafc', // graph viewport floor
  floor:  '#f1f5f9', // page background
  panel:  '#ffffff', // control panel / cards
  raised: '#f8fafc', // toolbar bars
} as const

export const TEXT = {
  primary:   '#0f172a', // slate-900
  secondary: '#475569', // slate-600
  muted:     '#64748b', // slate-500 — smallest AA-safe tier, meaningful text only
  faint:     '#94a3b8', // slate-400 — decorative dividers only, not information
} as const

export const BORDER = {
  subtle: '#e2e8f0', // slate-200
  strong: '#cbd5e1', // slate-300
} as const

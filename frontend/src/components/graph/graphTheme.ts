import { SURFACE } from '../../theme'

export type LayoutMode =
  | '2d-force'
  | '3d-force'
  | '2d-circular'
  | '2d-radial'
  | '3d-radial'

export const LAYOUT_TYPE = {
  '2d-force':    'forceDirected2d',
  '3d-force':    'forceDirected3d',
  '2d-circular': 'circular2d',
  '2d-radial':   'radialOut2d',
  '3d-radial':   'radialOut3d',
} as const

export const LAYOUT_BUTTONS: { id: LayoutMode; label: string }[] = [
  { id: '2d-force',    label: 'force 2D' },
  { id: '3d-force',    label: 'force 3D' },
  { id: '2d-circular', label: 'circular' },
  { id: '2d-radial',   label: 'radial 2D' },
  { id: '3d-radial',   label: 'radial 3D' },
]

// ── JS fallback color/size helpers (used when WASM not yet loaded) ──────────────

export function degreeColor(degree: number, maxDeg: number): string {
  const r = maxDeg > 0 ? degree / maxDeg : 0
  if (r > 0.95) return '#ea580c' // super-hub — orange-600
  if (r > 0.80) return '#d97706' // hub — amber-600
  if (r > 0.40) return '#06b6d4' // mid — cyan-500
  return '#0e7490'               // leaf — cyan-700
}

export function degreeSize(degree: number, maxDeg: number): number {
  const r = maxDeg > 0 ? degree / maxDeg : 0
  return 2 + r * 8
}

// ── Light theme (reagraph) ───────────────────────────────────────────────────

export const LIGHT_THEME = {
  canvas: { background: SURFACE.canvas, fog: null },
  node: {
    fill: '#0e7490',
    activeFill: '#0891b2',
    opacity: 1,
    selectedOpacity: 1,
    inactiveOpacity: 0.15,
    label: { color: '#334155', stroke: '#ffffff', activeColor: '#0f172a', fontSize: 6 },
    ring: { fill: '#7c3aed', activeFill: '#6d28d9' },
  },
  edge: {
    fill: '#cbd5e1',
    activeFill: '#0891b2',
    opacity: 0.8,
    selectedOpacity: 1,
    inactiveOpacity: 0.08,
    label: { color: '#64748b', stroke: '#ffffff', activeColor: '#334155', fontSize: 5 },
  },
  ring:  { fill: '#7c3aed', activeFill: '#6d28d9' },
  arrow: { fill: '#cbd5e1', activeFill: '#0891b2' },
  lasso: { border: '#7c3aed', background: 'rgba(124,58,237,0.08)' },
  cluster: { stroke: '#cbd5e1', label: { color: '#64748b', stroke: '#ffffff', fontSize: 10 } },
}

export const SCC_FILLS = ['#0891b2', '#7c3aed', '#d97706', '#059669', '#e11d48', '#0284c7', '#ea580c']

export const SPEED_MS = { fast: 40, medium: 100, slow: 280 }

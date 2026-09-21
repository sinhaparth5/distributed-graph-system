let counter = 0

/** Generates a locally-unique id for a user-added node/edge (never collides with parsed IDs, which are numeric strings). */
export function nextEditId(prefix: 'node' | 'edge'): string {
  counter += 1
  return `${prefix}-${counter}`
}

export interface PinnedPosition {
  fx: number
  fy: number
  fz?: number
}

import { parseGraph } from '../utils/parseGraph'
import type { FileFormat } from '../types'

self.onmessage = async (e: MessageEvent<{ content: string; format: FileFormat }>) => {
  try {
    const result = await parseGraph(e.data.content, e.data.format)
    self.postMessage({ ok: true, result })
  } catch (err) {
    self.postMessage({ ok: false, error: String(err) })
  }
}

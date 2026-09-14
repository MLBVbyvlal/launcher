import type { Instance, MCVersion } from './types'
import type { PinPreview } from '../MigrationScreen'

// Data written before 0.0.5 (marker missing or older) must go through the
// one-time migration screen: shared natives cleanup + latest pinning.
export const isLegacyDataVersion = (v: string): boolean => {
  const p = v.trim().split('.').map(x => { const n = parseInt(x, 10); return isNaN(n) ? 0 : n })
  const target = [0, 0, 5]
  for (let i = 0; i < 3; i++) {
    const a = p[i] ?? 0
    if (a !== target[i]) return a < target[i]
  }
  return false
}

export const buildPinPreviews = (list: Instance[], mf: MCVersion[]): PinPreview[] => {
  const ids = new Set(mf.map(v => v.id))
  return list.filter(i => i.version === 'latest').map(i => {
    if (i.type === 'mc') {
      const hint = i.mcVersion
      const lastKnown = localStorage.getItem('mlbv_last_mc_latest') ?? ''
      const to = (hint && hint !== 'latest' && ids.has(hint)) ? hint
        : (lastKnown && ids.has(lastKnown)) ? lastKnown
        : (mf.find(v => v.type === 'release')?.id ?? '?')
      return { id: i.id, name: i.name, to }
    }
    return { id: i.id, name: i.name, to: localStorage.getItem('mlbv_last_lb_latest') ?? '' }
  })
}

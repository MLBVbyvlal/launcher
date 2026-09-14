import { emit } from '@tauri-apps/api/event'
import { isTauri } from './types'

// ─── Accent helpers ───────────────────────────────────────────────────────────

export const DEFAULT_ACCENT = '#4ade80'

function hexToRgb(hex: string): [number, number, number] | null {
  const m = /^#([0-9a-f]{6})$/i.exec(hex.trim())
  if (!m) return null
  return [parseInt(m[1].slice(0,2),16), parseInt(m[1].slice(2,4),16), parseInt(m[1].slice(4,6),16)]
}

export function applyAccent(hex: string, applyToLb = false) {
  const rgb = hexToRgb(hex)
  if (!rgb) return
  const [r,g,b] = rgb
  const d = (c: number) => Math.round(c * 0.75).toString(16).padStart(2,'0')
  const dark = `#${d(r)}${d(g)}${d(b)}`
  const glow = `rgba(${r},${g},${b},0.38)`
  const lum = (0.299*r + 0.587*g + 0.114*b) / 255
  const onAccent = lum > 0.45 ? '#061206' : '#ffffff'
  const root = document.documentElement.style
  root.setProperty('--accent', hex)
  root.setProperty('--accent-dark', dark)
  root.setProperty('--accent-glow', glow)
  root.setProperty('--on-accent', onAccent)
  root.setProperty('--accent-rgb', `${r} ${g} ${b}`)
  if (applyToLb) {
    root.setProperty('--lb-accent', hex)
    root.setProperty('--lb-accent-dark', dark)
    root.setProperty('--lb-glow', glow)
    root.setProperty('--lb-accent-rgb', `${r} ${g} ${b}`)
  } else {
    root.setProperty('--lb-accent', '#4c8bf5')
    root.setProperty('--lb-accent-dark', '#2563eb')
    root.setProperty('--lb-glow', 'rgba(76,139,245,0.38)')
    root.setProperty('--lb-accent-rgb', '76 139 245')
  }
  // Broadcast to all Tauri windows (console, etc.)
  if (isTauri) {
    const lbHex = applyToLb ? hex : '#4c8bf5'
    emit('accent-updated', { accent: hex, lbAccent: lbHex }).catch(() => {})
  }
}

export const ACCENT_PRESETS = [
  { name: 'Grass',  hex: '#4ade80' },
  { name: 'Lime',   hex: '#a3e635' },
  { name: 'Sky',    hex: '#38bdf8' },
  { name: 'Indigo', hex: '#818cf8' },
  { name: 'Violet', hex: '#c084fc' },
  { name: 'Coral',  hex: '#fb923c' },
  { name: 'Rose',   hex: '#fb7185' },
  { name: 'Gold',   hex: '#fbbf24' },
]

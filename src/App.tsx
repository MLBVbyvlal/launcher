import { useState, useEffect, useCallback, useRef } from 'react'
import { motion, AnimatePresence } from 'framer-motion'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import lbLogo from './assets/lb-logo.svg'
import lbBadgePng from './assets/lb-badge-logo.png'
import SetupWizard from './SetupWizard'
import { getLang, type Lang, useT } from './i18n'
import './App.css'

// ─── Types ────────────────────────────────────────────────────────────────────

type Account   = { type: 'offline' | 'microsoft'; username: string; uuid: string; accessToken?: string }
type MCVersion = { id: string; type: 'release' | 'snapshot' | 'old_alpha' | 'old_beta'; releaseTime: string }
type LBVersion = { tag: string; mcVersion: string; date: string; buildId?: number }
type Instance  = { id: string; name: string; type: 'mc' | 'lb'; version: string; mcVersion: string; buildId?: number; loader?: 'vanilla' | 'fabric' }
type VFilter    = 'release' | 'snapshot' | 'old' | 'all'
type AppState   = 'loading' | 'ready' | 'error'
type Tab        = 'mc' | 'lb'
type UpdateInfo = { version: string; tagName: string; body: string; htmlUrl: string; assetUrl: string }

const spring = { type: 'spring', stiffness: 400, damping: 30 } as const

function LbBadge({ size = 20 }: { size?: number }) {
  return (
    <img src={lbBadgePng} alt="LB" className="lb-badge-img" draggable={false}
      style={{ width: size, height: size }} />
  )
}
const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
const tick = (ms: number) => new Promise<void>(r => setTimeout(r, ms))

// ─── Tooltip ─────────────────────────────────────────────────────────────────

function Tip({ text }: { text: string }) {
  return (
    <span className="tip-wrap">
      <svg className="tip-icon" viewBox="0 0 16 16" fill="none">
        <circle cx="8" cy="8" r="6.5" stroke="currentColor" strokeWidth="1.1"/>
        <path d="M8 7.5v3" stroke="currentColor" strokeWidth="1.3" strokeLinecap="round"/>
        <circle cx="8" cy="5.5" r="0.6" fill="currentColor"/>
      </svg>
      <span className="tip-text">{text}</span>
    </span>
  )
}

// ─── Accent helpers ───────────────────────────────────────────────────────────

const DEFAULT_ACCENT = '#4ade80'

function hexToRgb(hex: string): [number, number, number] | null {
  const m = /^#([0-9a-f]{6})$/i.exec(hex.trim())
  if (!m) return null
  return [parseInt(m[1].slice(0,2),16), parseInt(m[1].slice(2,4),16), parseInt(m[1].slice(4,6),16)]
}

function applyAccent(hex: string, applyToLb = false) {
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
  } else {
    root.setProperty('--lb-accent', '#4c8bf5')
    root.setProperty('--lb-accent-dark', '#2563eb')
    root.setProperty('--lb-glow', 'rgba(76,139,245,0.38)')
  }
}

const ACCENT_PRESETS = [
  { name: 'Grass',  hex: '#4ade80' },
  { name: 'Lime',   hex: '#a3e635' },
  { name: 'Sky',    hex: '#38bdf8' },
  { name: 'Indigo', hex: '#818cf8' },
  { name: 'Violet', hex: '#c084fc' },
  { name: 'Coral',  hex: '#fb923c' },
  { name: 'Rose',   hex: '#fb7185' },
  { name: 'Gold',   hex: '#fbbf24' },
]

function verTag(type?: MCVersion['type']): string {
  switch (type) {
    case 'release':  return 'R'
    case 'snapshot': return 'S'
    case 'old_beta': return 'B'
    default:         return 'A'
  }
}

// ─── Splash ───────────────────────────────────────────────────────────────────

function LoadingScreen({ status, progress, onRetry }: {
  status: string; progress: number; onRetry?: () => void
}) {
  const t = useT(getLang())
  return (
    <motion.div className="splash"
      initial={{ opacity: 1 }} exit={{ opacity: 0, scale: 1.04 }}
      transition={{ duration: 0.5, ease: [0.4, 0, 0.2, 1] }}
    >
      <div className="splash-bg">
        <div className="splash-orb splash-orb-1" /><div className="splash-orb splash-orb-2" /><div className="splash-orb splash-orb-3" />
      </div>
      <div className="splash-content">
        <motion.div className="splash-logo"
          initial={{ opacity: 0, y: 30, scale: 0.9 }} animate={{ opacity: 1, y: 0, scale: 1 }}
          transition={{ ...spring, delay: 0.1 }}
        >
          <div className="splash-logo-mark"><span /><span /><span /><span /></div>
          <div className="splash-title">MLBV</div>
          <div className="splash-subtitle">{t('launcher.subtitle')}</div>
        </motion.div>
        <motion.div className="splash-loader" initial={{ opacity: 0 }} animate={{ opacity: 1 }} transition={{ delay: 0.4 }}>
          {onRetry ? (
            <div className="splash-error">
              <div className="splash-status error">{status}</div>
              <button className="btn-retry" onClick={onRetry}>{t('error.retry')}</button>
            </div>
          ) : (
            <>
              <div className="splash-bar-bg">
                <motion.div className="splash-bar-fill" animate={{ width: `${progress}%` }} transition={{ duration: 0.4, ease: 'easeOut' }} />
                <div className="splash-bar-glow" style={{ left: `${progress}%` }} />
              </div>
              <div className="splash-status">{status}</div>
            </>
          )}
        </motion.div>
      </div>
    </motion.div>
  )
}

// ─── Settings modal ───────────────────────────────────────────────────────────

const RAM_MARKS = [
  { v: 512,   label: '512 MB', pct: 0    },
  { v: 4096,  label: '4 GB',   pct: 22.6 },
  { v: 8192,  label: '8 GB',   pct: 48.4 },
  { v: 16384, label: '16 GB',  pct: 100  },
]

type SettingsTab = 'general' | 'performance' | 'java' | 'about' | 'customize' | 'danger'

function SettingsModal({ onClose, onLangChange }: { onClose: () => void; onLangChange?: (l: Lang) => void }) {
  const [localLang, setLocalLang] = useState<Lang>(getLang)
  const t = useT(localLang)
  const handleLangChange = (l: Lang) => {
    localStorage.setItem('mlbv_lang', l)
    setLocalLang(l)
    onLangChange?.(l)
  }
  const [tab, setTab]     = useState<SettingsTab>('general')
  const [gameDir, setGameDir] = useState('Loading…')
  const [ram, setRam]     = useState(() => { const s = localStorage.getItem('mlbv_ram'); return s ? Number(s) : 2048 })
  const [ramDraft, setRamDraft]             = useState<string | null>(null)
  const [concurrent, setConcurrent]         = useState(() => { const s = localStorage.getItem('mlbv_concurrent'); return s ? Number(s) : 5 })
  const [concurrentDraft, setConcurrentDraft] = useState<string | null>(null)
  const [closeOnLaunch, setCloseOnLaunch]   = useState(() => localStorage.getItem('mlbv_close_on_launch') === '1')
  const [consoleEnabled, setConsoleEnabled] = useState(() => localStorage.getItem('mlbv_console_enabled') === '1')
  const [javaInstalls, setJavaInstalls]     = useState<{ major: number; path: string }[]>([])
  const [dangerOpen, setDangerOpen]         = useState(false)
  const [countdown, setCountdown]           = useState(5)
  const [deleting, setDeleting]             = useState(false)
  const [updateStatus, setUpdateStatus]     = useState<'idle' | 'checking' | 'uptodate' | { version: string; htmlUrl: string }>('idle')

  // Customization
  const [customAccent, setCustomAccent] = useState(() => localStorage.getItem('mlbv_accent') ?? DEFAULT_ACCENT)
  const [hexInput, setHexInput]         = useState(() => localStorage.getItem('mlbv_accent') ?? DEFAULT_ACCENT)
  const [applyToLb, setApplyToLb]       = useState(() => localStorage.getItem('mlbv_lb_accent_same') === '1')

  const handleSetAccent = (hex: string) => {
    const cleaned = hex.startsWith('#') ? hex : `#${hex}`
    if (!/^#[0-9a-f]{6}$/i.test(cleaned)) return
    setCustomAccent(cleaned)
    setHexInput(cleaned)
    localStorage.setItem('mlbv_accent', cleaned)
    applyAccent(cleaned, applyToLb)
  }
  const handleResetAccent = () => {
    setCustomAccent(DEFAULT_ACCENT)
    setHexInput(DEFAULT_ACCENT)
    localStorage.removeItem('mlbv_accent')
    localStorage.removeItem('mlbv_lb_accent_same')
    setApplyToLb(false)
    applyAccent(DEFAULT_ACCENT, false)
  }
  const handleApplyToLbChange = (v: boolean) => {
    setApplyToLb(v)
    localStorage.setItem('mlbv_lb_accent_same', v ? '1' : '0')
    applyAccent(customAccent, v)
  }

  useEffect(() => {
    if (isTauri) {
      invoke<string>('get_game_dir').then(setGameDir).catch(() => setGameDir('Unknown'))
      invoke<{ major: number; path: string }[]>('scan_java').then(setJavaInstalls).catch(() => {})
    } else {
      setGameDir('%APPDATA%\\.mlbv\\shared')
    }
  }, [])

  useEffect(() => { localStorage.setItem('mlbv_ram', String(ram)) }, [ram])
  useEffect(() => { localStorage.setItem('mlbv_concurrent', String(concurrent)) }, [concurrent])
  useEffect(() => { localStorage.setItem('mlbv_close_on_launch', closeOnLaunch ? '1' : '0') }, [closeOnLaunch])
  useEffect(() => { localStorage.setItem('mlbv_console_enabled', consoleEnabled ? '1' : '0') }, [consoleEnabled])

  useEffect(() => {
    if (!dangerOpen || countdown <= 0) return
    const id = setTimeout(() => setCountdown(c => c - 1), 1000)
    return () => clearTimeout(id)
  }, [dangerOpen, countdown])

  const clampRam        = (v: number) => Math.min(16384, Math.max(512, Math.round(v / 512) * 512))
  const clampConcurrent = (v: number) => Math.min(50, Math.max(1, Math.round(v)))
  const commitRam        = (raw: string) => { const n = Number(raw); if (!isNaN(n) && n > 0) setRam(clampRam(n)); setRamDraft(null) }
  const commitConcurrent = (raw: string) => { const n = Number(raw); if (!isNaN(n) && n > 0) setConcurrent(clampConcurrent(n)); setConcurrentDraft(null) }

  const concurrentWarning = concurrent < 5
    ? t('perf.warn.low')
    : concurrent >= 10
    ? t('perf.warn.high')
    : null

  const handleDelete = async () => {
    setDeleting(true)
    localStorage.clear()
    if (isTauri) { try { await invoke('reset_all_data') } catch { /* ignore */ } }
    window.location.reload()
  }

  const handleManualUpdateCheck = async () => {
    if (!isTauri) return
    setUpdateStatus('checking')
    try {
      type RawRelease = { version: string; tag_name: string; body: string; html_url: string; asset_url: string }
      const r = await invoke<RawRelease | null>('check_for_update')
      setUpdateStatus(r ? { version: r.version, htmlUrl: r.html_url } : 'uptodate')
    } catch {
      setUpdateStatus('idle')
    }
  }

  const JAVA_REQS = [
    { major: 8,  label: 'Java 8',  mc: '≤ 1.16.5' },
    { major: 17, label: 'Java 17', mc: '1.17 – 1.20.4' },
    { major: 21, label: 'Java 21', mc: '1.20.5 – 1.21.x' },
    { major: 25, label: 'Java 25', mc: '26.1+' },
  ]

  type NavItem = { id: SettingsTab; label: string; danger?: boolean; icon: React.ReactNode }
  const NAV: NavItem[] = [
    { id: 'general', label: t('settings.tab.general'), icon: (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
        <path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"/><circle cx="12" cy="12" r="3"/>
      </svg>
    )},
    { id: 'performance', label: t('settings.tab.performance'), icon: (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
        <path d="M13 2L3 14h9l-1 8 10-12h-9l1-8z"/>
      </svg>
    )},
    { id: 'java', label: 'Java', icon: (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
        <path d="M8 17c0 1.1.9 2 2 2h4c1.1 0 2-.9 2-2v-1H8v1z"/><path d="M7 6s1-2.5 5-3 5 3 5 3-1 2-5 2-5-2-5-2z"/><path d="M12 14V8"/>
      </svg>
    )},
    { id: 'about', label: t('settings.tab.about'), icon: (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
        <circle cx="12" cy="12" r="10"/><path d="M12 16v-4M12 8h.01"/>
      </svg>
    )},
    { id: 'customize', label: t('settings.tab.customize'), icon: (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
        <circle cx="13.5" cy="6.5" r="1" fill="currentColor" stroke="none"/><circle cx="17.5" cy="10.5" r="1" fill="currentColor" stroke="none"/><circle cx="8.5" cy="7.5" r="1" fill="currentColor" stroke="none"/><circle cx="6.5" cy="12.5" r="1" fill="currentColor" stroke="none"/>
        <path d="M12 2C6.5 2 2 6.5 2 12c0 5.52 4.5 10 10 10 .83 0 1.5-.67 1.5-1.5 0-.39-.15-.74-.39-1-.23-.27-.38-.62-.38-1 0-.83.67-1.5 1.5-1.5H16c2.76 0 5-2.24 5-5 0-4.42-4.03-8-9-8z"/>
      </svg>
    )},
    { id: 'danger', label: t('settings.tab.danger'), danger: true, icon: (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
        <polyline points="3 6 5 6 21 6"/><path d="M19 6l-1 14H6L5 6"/><path d="M10 11v6M14 11v6"/><path d="M9 6V4h6v2"/>
      </svg>
    )},
  ]

  return (
    <motion.div className="overlay" initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} onClick={onClose}>
      <motion.div className="modal glass settings-modal"
        initial={{ opacity: 0, scale: 0.9, y: 24 }} animate={{ opacity: 1, scale: 1, y: 0 }}
        exit={{ opacity: 0, scale: 0.9, y: 16 }} transition={spring}
        onClick={e => e.stopPropagation()}
      >
        <div className="modal-head">
          <span className="modal-title">{t('settings.title')}</span>
          <button className="modal-close" onClick={onClose}>×</button>
        </div>

        <div className="settings-layout">
          {/* Left nav */}
          <nav className="settings-nav">
            {NAV.map(n => (
              <button key={n.id}
                className={['snav-btn', tab === n.id ? 'active' : '', n.danger ? 'snav-danger' : ''].filter(Boolean).join(' ')}
                onClick={() => { setTab(n.id); if (n.id !== 'danger') { setDangerOpen(false); setCountdown(5) } }}
                title={n.label}
              >
                <span className="snav-icon">{n.icon}</span>
                <span className="snav-label">{n.label}</span>
              </button>
            ))}
          </nav>

          {/* Right panel */}
          <div className="settings-panel">
            <AnimatePresence mode="wait">
              <motion.div key={tab} className="settings-content"
                initial={{ opacity: 0, x: 10 }} animate={{ opacity: 1, x: 0 }}
                exit={{ opacity: 0, x: -10 }} transition={{ duration: 0.13 }}
              >

                {/* ── GENERAL ── */}
                {tab === 'general' && <>
                  <div className="setting-section-title">{t('settings.tab.general')}</div>
                  <div className="setting-group">
                    <div className="setting-label">{t('settings.game_dir')}</div>
                    <div className="setting-path">{gameDir}</div>
                    <div className="setting-hint">{t('settings.game_dir_hint')}</div>
                  </div>
                  <div className="setting-group">
                    <div className="setting-label-row">
                      <label className="setting-toggle" style={{ flex: 1 }}>
                        <input type="checkbox" checked={closeOnLaunch} onChange={e => setCloseOnLaunch(e.target.checked)} />
                        <span className="toggle-track"><span className="toggle-thumb" /></span>
                        <span className="toggle-label">{t('settings.close_on_launch')}</span>
                      </label>
                      <Tip text={t('settings.tip.close_on_launch')} />
                    </div>
                  </div>
                  <div className="setting-group">
                    <div className="setting-label-row">
                      <label className="setting-toggle" style={{ flex: 1 }}>
                        <input type="checkbox" checked={consoleEnabled} onChange={e => setConsoleEnabled(e.target.checked)} />
                        <span className="toggle-track"><span className="toggle-thumb" /></span>
                        <span className="toggle-label">{t('settings.console')}</span>
                      </label>
                      <Tip text={t('settings.console_hint')} />
                    </div>
                  </div>
                  <div className="setting-group">
                    <div className="setting-label">{t('settings.reset_setup')}</div>
                    <button className="btn-secondary" onClick={() => {
                      localStorage.removeItem('mlbv_setup_done')
                      onClose()
                      window.location.reload()
                    }}>{t('settings.reset_setup')}</button>
                    <div className="setting-hint">{t('settings.reset_setup_hint')}</div>
                  </div>
                  <div className="setting-group">
                    <div className="setting-label">{t('settings.language')}</div>
                    <div className="sw-lang-row" style={{ marginTop: 6, justifyContent: 'flex-start' }}>
                      <button className={`sw-lang-btn${localLang === 'en' ? ' sw-lang-active' : ''}`} onClick={() => handleLangChange('en')}>
                        <span className="sw-lang-badge">🇬🇧</span>
                        <span>English</span>
                      </button>
                      <button className={`sw-lang-btn${localLang === 'ru' ? ' sw-lang-active' : ''}`} onClick={() => handleLangChange('ru')}>
                        <span className="sw-lang-badge">🇷🇺</span>
                        <span>Русский</span>
                      </button>
                    </div>
                  </div>
                </>}

                {/* ── PERFORMANCE ── */}
                {tab === 'performance' && <>
                  <div className="setting-section-title">{t('settings.tab.performance')}</div>
                  <div className="setting-group">
                    <div className="setting-label-row">
                      <div className="setting-label">{t('settings.ram')} — {ram >= 1024 ? `${(ram/1024).toFixed(1)} GB` : `${ram} MB`}</div>
                      <Tip text={t('settings.tip.ram')} />
                    </div>
                    <div className="ram-row">
                      <div className="ram-slider-wrap">
                        <input type="range" className="glass-range" min={512} max={16384} step={512}
                          value={ram} onChange={e => { setRam(Number(e.target.value)); setRamDraft(null) }} />
                        <div className="ram-marks-abs">
                          {RAM_MARKS.map(m => (
                            <span key={m.v} className="ram-mark"
                              style={{ left: `calc(8px + ${m.pct / 100} * (100% - 16px))` }}>{m.label}</span>
                          ))}
                        </div>
                      </div>
                      <input type="number" className="ram-input" min={512} max={16384}
                        value={ramDraft ?? ram}
                        onChange={e => setRamDraft(e.target.value)}
                        onBlur={e => commitRam(e.target.value)}
                        onKeyDown={e => { if (e.key === 'Enter') e.currentTarget.blur() }} />
                    </div>
                  </div>
                  <div className="setting-group">
                    <div className="setting-label-row">
                      <div className="setting-label">{t('settings.concurrent')} — {concurrent}</div>
                      <Tip text={t('settings.tip.concurrent')} />
                    </div>
                    <div className="concurrent-row">
                      <input type="range" className="glass-range" min={1} max={50} step={1}
                        value={concurrent} onChange={e => { setConcurrent(Number(e.target.value)); setConcurrentDraft(null) }} />
                      <input type="number" className="ram-input" min={1} max={50}
                        value={concurrentDraft ?? concurrent}
                        onChange={e => setConcurrentDraft(e.target.value)}
                        onBlur={e => commitConcurrent(e.target.value)}
                        onKeyDown={e => { if (e.key === 'Enter') e.currentTarget.blur() }} />
                    </div>
                    <div className="concurrent-marks">
                      <span>1</span><span>5</span><span>10</span><span>25</span><span>50</span>
                    </div>
                    {concurrentWarning && (
                      <div className="concurrent-warn concurrent-warn-warn">⚠ {concurrentWarning}</div>
                    )}
                  </div>
                </>}

                {/* ── JAVA ── */}
                {tab === 'java' && <>
                  <div className="setting-section-title">{t('settings.java')}</div>
                  <div className="setting-group">
                    <div className="java-grid">
                      {JAVA_REQS.map(({ major, label, mc }) => {
                        const exact  = javaInstalls.find(j => j.major === major)
                        const status = !isTauri ? null
                          : exact ? { cls: 'java-ok', text: t('settings.java_found') }
                          :         { cls: 'java-dl', text: t('settings.java_auto') }
                        return (
                          <div key={major} className="java-row">
                            <span className="java-ver">{label}</span>
                            <span className="java-mc">MC {mc}</span>
                            {status && <span className={`java-status ${status.cls}`}>{status.text}</span>}
                          </div>
                        )
                      })}
                    </div>
                    <div className="setting-hint">{t('settings.java_hint')}</div>
                  </div>
                </>}

                {/* ── ABOUT ── */}
                {tab === 'about' && (
                  <div className="about-panel">
                    <div className="about-logos">
                      <div className="about-logo-item">
                        <div className="logo-mark" style={{ transform: 'scale(1.4)', margin: '4px 0' }}><span /><span /><span /><span /></div>
                        <span className="about-logo-name">MLBV</span>
                      </div>
                      <span className="about-x">×</span>
                      <div className="about-logo-item">
                        <img src={lbLogo} className="about-lb-logo" alt="LiquidBounce" draggable={false} />
                        <span className="about-logo-name">LiquidBounce</span>
                      </div>
                    </div>
                    <p className="about-disclaimer">{t('settings.disclaimer')}</p>
                    <div className="about-info">
                      <span className="about-ver">MLBV v{__APP_VERSION__}</span>
                      <span className="about-stack">{t('settings.stack')}</span>
                    </div>
                    <div className="about-update-row">
                      <button className="btn-secondary" onClick={handleManualUpdateCheck}
                        disabled={updateStatus === 'checking'}>
                        {updateStatus === 'checking' ? t('settings.checking') : t('settings.check_updates')}
                      </button>
                      {updateStatus === 'uptodate' && (
                        <span className="about-update-ok">{t('settings.up_to_date')}</span>
                      )}
                      {typeof updateStatus === 'object' && (
                        <span className="about-update-avail">
                          v{updateStatus.version} {t('settings.update_available')} —{' '}
                          <button className="about-update-link"
                            onClick={() => isTauri && invoke('open_url', { url: updateStatus.htmlUrl }).catch(() => {})}>
                            {t('update.download')}
                          </button>
                        </span>
                      )}
                    </div>
                    <DebugInfoBlock />
                    <div className="about-by">{t('settings.by')}</div>
                  </div>
                )}

                {/* ── CUSTOMIZE ── */}
                {tab === 'customize' && (
                  <div className="customize-panel">
                    <div className="setting-section-title">{t('settings.tab.customize')}</div>

                    <div className="setting-group">
                      <div className="setting-label-row">
                        <div className="setting-label">{t('customize.accent')}</div>
                        <Tip text={t('customize.accent_tip')} />
                      </div>
                      <div className="color-palette">
                        {ACCENT_PRESETS.map(p => (
                          <button key={p.hex}
                            className={`color-swatch${customAccent.toLowerCase() === p.hex ? ' active' : ''}`}
                            style={{ background: p.hex }} title={p.name}
                            onClick={() => handleSetAccent(p.hex)} />
                        ))}
                      </div>
                      <div className="color-hex-row">
                        <div className="color-preview" style={{ background: customAccent }} />
                        <input className="glass-input hex-input"
                          value={hexInput}
                          onChange={e => setHexInput(e.target.value)}
                          onBlur={() => handleSetAccent(hexInput)}
                          onKeyDown={e => e.key === 'Enter' && handleSetAccent(hexInput)}
                          placeholder="#4ade80"
                          maxLength={7}
                          spellCheck={false}
                        />
                        <button className="btn-cancel" onClick={handleResetAccent}>{t('customize.reset')}</button>
                      </div>
                    </div>

                    <div className="setting-group">
                      <div className="setting-label-row">
                        <label className="setting-toggle" style={{ flex: 1 }}>
                          <input type="checkbox" checked={applyToLb} onChange={e => handleApplyToLbChange(e.target.checked)} />
                          <span className="toggle-track"><span className="toggle-thumb"/></span>
                          <span className="toggle-label">{t('customize.apply_lb')}</span>
                        </label>
                        <Tip text={t('customize.apply_lb_tip')} />
                      </div>
                    </div>
                  </div>
                )}

                {/* ── DANGER ── */}
                {tab === 'danger' && (
                  <div className="danger-panel">
                    <div className="danger-title">{t('settings.danger_title')}</div>
                    <div className="danger-hint">{t('settings.danger_hint')}</div>
                    {!dangerOpen ? (
                      <button className="btn-danger-trigger"
                        onClick={() => { setDangerOpen(true); setCountdown(5) }}>
                        {t('settings.danger_btn')}
                      </button>
                    ) : (
                      <div className="danger-confirm-box">
                        <div className="danger-warn-title">{t('settings.danger_warn')}</div>
                        <ul className="danger-list">
                          <li>{t('settings.danger_item1')}</li>
                          <li>{t('settings.danger_item2')}</li>
                          <li>{t('settings.danger_item3')}</li>
                          <li>{t('settings.danger_item4')}</li>
                          <li>{t('settings.danger_item5')}</li>
                        </ul>
                        <div className="danger-countdown">
                          {countdown > 0
                            ? t('settings.danger_wait').replace('{0}', String(countdown))
                            : t('settings.danger_confirm_hint')}
                        </div>
                        <div className="danger-actions">
                          <button className="btn-cancel"
                            onClick={() => { setDangerOpen(false); setCountdown(5) }}>
                            {t('settings.cancel')}
                          </button>
                          <button
                            className={`btn-delete-confirm${countdown <= 0 ? ' enabled' : ''}`}
                            disabled={countdown > 0 || deleting}
                            onClick={handleDelete}
                          >
                            {deleting ? t('settings.deleting') : t('settings.confirm_delete')}
                          </button>
                        </div>
                      </div>
                    )}
                  </div>
                )}

              </motion.div>
            </AnimatePresence>
          </div>
        </div>

      </motion.div>
    </motion.div>
  )
}

// ─── Create Instance Modal ────────────────────────────────────────────────────

function CreateInstanceModal({ defaultTab, mcVersions, existingNames, onAdd, onClose }: {
  defaultTab: Tab
  mcVersions: MCVersion[]
  existingNames: string[]
  onAdd: (inst: Instance) => void
  onClose: () => void
}) {
  const [instType, setInstType]       = useState<Tab>(defaultTab)
  const [step, setStep]               = useState<1 | 2>(1)
  const [selectedLoader, setSelectedLoader] = useState<'vanilla' | 'fabric'>('vanilla')
  const [vFilter, setVFilter]         = useState<VFilter>('release')
  const [selVer, setSelVer]           = useState<string>('')
  const [name, setName]               = useState('')
  const [nameEdited, setNameEdited]   = useState(false)
  const [shake, setShake]             = useState(false)
  const [error, setError]             = useState('')

  // LB branch state
  const t = useT(getLang())

  const [lbBranches, setLbBranches]       = useState<string[]>(['nextgen', 'legacy'])
  const [lbBranch, setLbBranch]           = useState('nextgen')
  const [lbVersionsMap, setLbVersionsMap] = useState<Record<string, LBVersion[]>>({})
  const [lbLoading, setLbLoading]         = useState(false)
  const [lbLoadError, setLbLoadError]     = useState<string | null>(null)
  const loadedBranches = useRef(new Set<string>())

  const currentLbVersions = lbVersionsMap[lbBranch] ?? []

  const loadBranchVersions = useCallback(async (branch: string) => {
    if (loadedBranches.current.has(branch)) return
    loadedBranches.current.add(branch)
    setLbLoading(true)
    setLbLoadError(null)
    try {
      type RawBuild = { build_id: number; lb_version: string; mc_version: string; date: string }
      const builds: RawBuild[] = isTauri
        ? await invoke<RawBuild[]>('get_lb_versions', { branch })
        : await fetch(`https://api.liquidbounce.net/api/v1/version/builds/${branch}/release`).then(r => r.json())
      const mapped: LBVersion[] = builds.map(r => ({
        buildId: r.build_id, tag: r.lb_version, mcVersion: r.mc_version, date: r.date?.slice(0, 10) ?? ''
      }))
      setLbVersionsMap(prev => ({ ...prev, [branch]: mapped }))
      setSelVer(v => (!v ? (mapped[0]?.tag ?? '') : v))
    } catch (e) {
      loadedBranches.current.delete(branch) // allow retry
      setLbLoadError(String(e))
    }
    setLbLoading(false)
  }, [])

  // Fetch branches list once
  useEffect(() => {
    if (!isTauri) return
    invoke<string[]>('get_lb_branches')
      .then(b => setLbBranches(b))
      .catch(() => {})
  }, [])

  // Load versions when switching to LB tab or changing branch
  useEffect(() => {
    if (instType === 'lb') loadBranchVersions(lbBranch)
  }, [instType, lbBranch, loadBranchVersions])

  // Reset step when switching to LB type
  useEffect(() => { if (instType === 'lb') setStep(1) }, [instType])

  // Auto-pick first version when type/branch changes
  useEffect(() => {
    if (instType === 'mc') {
      const filtered = mcVersions.filter(v => v.type === 'release')
      setSelVer(filtered[0]?.id ?? '')
    } else {
      setSelVer(lbVersionsMap[lbBranch]?.[0]?.tag ?? '')
    }
    setNameEdited(false)
    setError('')
  }, [instType, lbBranch]) // eslint-disable-line react-hooks/exhaustive-deps

  // When LB versions finish loading, auto-select first if nothing selected
  useEffect(() => {
    if (instType === 'lb' && !selVer && currentLbVersions.length > 0) {
      setSelVer(currentLbVersions[0].tag)
    }
  }, [currentLbVersions.length, instType]) // eslint-disable-line react-hooks/exhaustive-deps

  const autoName = instType === 'mc'
    ? (selectedLoader === 'fabric' ? `Minecraft ${selVer} (Fabric)` : `Minecraft ${selVer}`)
    : `LiquidBounce ${selVer}`

  const displayName = nameEdited ? name : autoName

  const filteredMc = mcVersions.filter(v => {
    if (vFilter === 'all')      return true
    if (vFilter === 'release')  return v.type === 'release'
    if (vFilter === 'snapshot') return v.type === 'snapshot'
    return v.type === 'old_beta' || v.type === 'old_alpha'
  })

  const handleCreate = () => {
    const finalName = displayName.trim()
    if (!finalName) return
    if (existingNames.includes(finalName)) {
      setError(t('inst.name_taken'))
      setShake(true)
      setTimeout(() => setShake(false), 500)
      return
    }
    const lbBuild = currentLbVersions.find(v => v.tag === selVer)
    const mcVer = instType === 'mc' ? selVer : (lbBuild?.mcVersion ?? selVer)
    onAdd({
      id: crypto.randomUUID(),
      name: finalName,
      type: instType,
      version: selVer,
      mcVersion: mcVer,
      buildId: lbBuild?.buildId,
      loader: instType === 'mc' ? selectedLoader : undefined,
    })
    onClose()
  }

  return (
    <motion.div className="overlay" initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }}
      onClick={onClose}
    >
      <motion.div className={`modal glass create-inst-modal${shake ? ' shake' : ''}`}
        initial={{ opacity: 0, scale: 0.9, y: 24 }} animate={{ opacity: 1, scale: 1, y: 0 }}
        exit={{ opacity: 0, scale: 0.9, y: 16 }} transition={spring}
        onClick={e => e.stopPropagation()}
      >
        <div className="modal-head">
          <span className="modal-title">{instType === 'lb' ? t('inst.modal.title_lb') : t('inst.modal.title_mc')}</span>
          <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
            <div className="vtabs">
              <button className={`vtab${instType === 'mc' ? ' on' : ''}`} onClick={() => setInstType('mc')}>{t('tab.mc')}</button>
              <button className={`vtab${instType === 'lb' ? ' on' : ''}`} onClick={() => setInstType('lb')}>{t('tab.lb')}</button>
            </div>
            <button className="modal-close" onClick={onClose}>×</button>
          </div>
        </div>

        {instType === 'mc' ? (
          step === 1 ? (
          <>
            <div className="vtabs" style={{ padding: '8px 16px 0', gap: 4, display: 'flex' }}>
              {(['release','snapshot','old','all'] as VFilter[]).map(f => (
                <button key={f} className={`vtab${vFilter === f ? ' on' : ''}`} onClick={() => setVFilter(f)}>
                  {f === 'release' ? t('inst.modal.filter.release') : f === 'snapshot' ? t('inst.modal.filter.snapshot') : f === 'old' ? t('inst.modal.filter.old') : t('inst.modal.filter.all')}
                </button>
              ))}
            </div>
            <div className="vlist">
              {filteredMc.map(v => (
                <motion.button key={v.id}
                  className={`vitem${v.id === selVer ? ' picked' : ''}`}
                  onClick={() => { setSelVer(v.id); setNameEdited(false) }}
                  whileHover={{ x: 3 }} transition={spring}
                >
                  <span className={`vbadge ${v.type}`}>{verTag(v.type)}</span>
                  <span className="vid">{v.id}</span>
                  <span className="vyr">{new Date(v.releaseTime).getFullYear()}</span>
                  {v.id === selVer && <span className="vcheck">✓</span>}
                </motion.button>
              ))}
            </div>
          </>
          ) : (
          <>
            <div className="loader-grid">
              {([
                { id: 'vanilla', icon: '🌿', label: t('inst.loader.vanilla'), desc: t('inst.loader.vanilla_desc'), enabled: true },
                { id: 'fabric',  icon: '🧵', label: t('inst.loader.fabric'),  desc: t('inst.loader.fabric_desc'),  enabled: true },
                { id: 'forge',   icon: '⚒️', label: 'Forge',   desc: t('inst.loader.soon'), enabled: false },
                { id: 'neoforge',icon: '🔥', label: 'NeoForge',desc: t('inst.loader.soon'), enabled: false },
              ] as const).map(opt => (
                <div key={opt.id}
                  className={[
                    'loader-opt',
                    !opt.enabled ? 'loader-disabled' : '',
                    opt.enabled && selectedLoader === opt.id ? 'loader-selected' : '',
                  ].filter(Boolean).join(' ')}
                  onClick={() => opt.enabled && setSelectedLoader(opt.id as 'vanilla' | 'fabric')}
                >
                  <div className="loader-icon">{opt.icon}</div>
                  <div className="loader-info">
                    <div className="loader-name">
                      {opt.label}
                      {!opt.enabled && <span className="loader-soon-tag">{t('inst.loader.soon')}</span>}
                    </div>
                    <div className="loader-desc">{opt.desc}</div>
                  </div>
                  {opt.enabled && <div className="loader-radio" />}
                </div>
              ))}
            </div>
            <div className="loader-step-hint">
              {selectedLoader === 'fabric' ? t('inst.loader.fabric_desc') : t('inst.loader.vanilla_desc')}
            </div>
          </>
          )
        ) : (
          <>
            <div className="vtabs" style={{ padding: '8px 16px 0', gap: 4, display: 'flex' }}>
              {lbBranches.map(b => (
                <button key={b}
                  className={`vtab${lbBranch === b ? ' on' : ''}`}
                  onClick={() => setLbBranch(b)}
                >
                  {b === 'nextgen' ? 'Nextgen' : b === 'legacy' ? 'Legacy' : b.charAt(0).toUpperCase() + b.slice(1)}
                </button>
              ))}
            </div>
            <div className="vlist">
              {lbLoading ? (
                <div style={{ padding: 20, color: 'var(--text-muted)', fontSize: 13 }}>{t('loading')}</div>
              ) : lbLoadError ? (
                <div style={{ padding: 16, display: 'flex', flexDirection: 'column', gap: 8 }}>
                  <div style={{ color: '#f87171', fontSize: 12 }}>{t('error.prefix')} {lbLoadError}</div>
                  <button className="btn-retry" style={{ alignSelf: 'flex-start' }}
                    onClick={() => { loadedBranches.current.delete(lbBranch); loadBranchVersions(lbBranch) }}>
                    {t('error.retry')}
                  </button>
                </div>
              ) : currentLbVersions.length === 0 ? (
                <div style={{ padding: 20, color: 'var(--text-muted)', fontSize: 13 }}>{t('inst.no_versions')}</div>
              ) : currentLbVersions.map(v => (
                <motion.button key={v.tag}
                  className={`vitem${v.tag === selVer ? ' picked lb-picked' : ''}`}
                  onClick={() => { setSelVer(v.tag); setNameEdited(false) }}
                  whileHover={{ x: 3 }} transition={spring}
                >
                  <LbBadge />
                  <span className="vid">{v.tag}</span>
                  <span className="vyr">MC {v.mcVersion}</span>
                  {v.tag === selVer && <span className="vcheck" style={{ color: 'var(--lb-accent)' }}>✓</span>}
                </motion.button>
              ))}
            </div>
          </>
        )}

        <div className="inst-name-row">
          <input
            className={`glass-input${error ? ' input-error' : ''}`}
            value={displayName}
            onChange={e => { setName(e.target.value); setNameEdited(true); setError('') }}
            onKeyDown={e => e.key === 'Enter' && handleCreate()}
            placeholder={t('inst.modal.name_ph')}
            maxLength={64}
          />
          {error && <div className="inst-error">{error}</div>}
        </div>

        <div className="inst-modal-footer">
          {instType === 'mc' && step === 2 ? (
            <button className="btn-cancel" onClick={() => setStep(1)}>{t('btn.back')}</button>
          ) : (
            <button className="btn-cancel" onClick={onClose}>{t('inst.modal.cancel')}</button>
          )}
          {instType === 'mc' && step === 1 ? (
            <button className="btn-ok" onClick={() => setStep(2)} disabled={!selVer}>
              {t('inst.loader.next')}
            </button>
          ) : (
            <button
              className={`btn-ok${instType === 'lb' ? ' lb-btn' : ''}`}
              onClick={handleCreate}
              disabled={!selVer}
            >
              {t('inst.modal.create')}
            </button>
          )}
        </div>
      </motion.div>
    </motion.div>
  )
}

// ─── Crash Dialog ────────────────────────────────────────────────────────────

type CrashInfo = { exitCode: number; log: string; logPath: string; instanceName?: string }

function CrashDialog({ info, onClose }: { info: CrashInfo; onClose: () => void }) {
  return (
    <motion.div className="overlay" initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }}>
      <motion.div className="modal glass crash-modal"
        initial={{ opacity: 0, scale: 0.9, y: 24 }} animate={{ opacity: 1, scale: 1, y: 0 }}
        exit={{ opacity: 0, scale: 0.9, y: 16 }} transition={spring}
      >
        <div className="crash-header">
          <div className="crash-header-left">
            <span className="crash-icon-wrap">💥</span>
            <div>
              <div className="crash-title">Game crashed</div>
              <div className="crash-subtitle">Exit code: {info.exitCode}</div>
            </div>
          </div>
          <button className="modal-close" onClick={onClose}>×</button>
        </div>
        <pre className="crash-log">{info.log || 'No log output available.'}</pre>
        <div className="crash-actions">
          <button className="btn-secondary" onClick={() => navigator.clipboard.writeText(info.log)}>
            Copy Log
          </button>
          {isTauri && (
            <button className="btn-secondary" onClick={() =>
              invoke('open_instance_logs_folder', { instanceName: info.instanceName ?? '' }).catch(() => {})
            }>
              Open Log Folder
            </button>
          )}
          <button className="btn-ok" onClick={onClose}>Close</button>
        </div>
      </motion.div>
    </motion.div>
  )
}

// ─── Instance Context Menu ────────────────────────────────────────────────────

type CtxTarget = { inst: Instance; x: number; y: number }
type CtxAction = 'rename' | 'settings' | 'reinstall' | 'delete'

function InstanceCtxMenu({ target, isLb, onAction, onClose }: {
  target: CtxTarget; isLb: boolean
  onAction: (a: CtxAction, inst: Instance) => void
  onClose: () => void
}) {
  const t = useT(getLang())
  const W = 176, H = 196
  const x = Math.min(target.x, window.innerWidth  - W - 8)
  const y = Math.min(target.y, window.innerHeight - H - 8)
  return (
    <>
      <div className="ctx-backdrop" onClick={onClose} onContextMenu={e => { e.preventDefault(); onClose() }} />
      <motion.div className={`ctx-menu glass ${isLb ? 'ctx-lb' : 'ctx-mc'}`}
        style={{ left: x, top: y }}
        initial={{ opacity: 0, scale: 0.92, y: -8 }} animate={{ opacity: 1, scale: 1, y: 0 }}
        exit={{ opacity: 0, scale: 0.92, y: -8 }} transition={{ duration: 0.12 }}
      >
        <button className="ctx-item" onClick={() => { onAction('rename', target.inst); onClose() }}>
          <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round"><path d="M13 3l4 4-9 9H4v-4L13 3z"/></svg>
          {t('ctx.rename')}
        </button>
        <button className="ctx-item" onClick={() => { onAction('settings', target.inst); onClose() }}>
          <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round"><path d="M10.27 2h-.54a1.5 1.5 0 0 0-1.5 1.5v.14a1.5 1.5 0 0 1-.75 1.3l-.32.18a1.5 1.5 0 0 1-1.5 0l-.12-.06a1.5 1.5 0 0 0-2.05.55l-.27.47a1.5 1.5 0 0 0 .55 2.05l.11.07a1.5 1.5 0 0 1 .75 1.3v.38a1.5 1.5 0 0 1-.75 1.3l-.11.07a1.5 1.5 0 0 0-.55 2.05l.27.47a1.5 1.5 0 0 0 2.05.55l.12-.06a1.5 1.5 0 0 1 1.5 0l.32.18a1.5 1.5 0 0 1 .75 1.3v.14A1.5 1.5 0 0 0 9.73 18h.54a1.5 1.5 0 0 0 1.5-1.5v-.14a1.5 1.5 0 0 1 .75-1.3l.32-.18a1.5 1.5 0 0 1 1.5 0l.12.06a1.5 1.5 0 0 0 2.05-.55l.27-.47a1.5 1.5 0 0 0-.55-2.05l-.11-.07A1.5 1.5 0 0 1 15.37 11v-.38a1.5 1.5 0 0 1 .75-1.3l.11-.07a1.5 1.5 0 0 0 .55-2.05l-.27-.47a1.5 1.5 0 0 0-2.05-.55l-.12.06a1.5 1.5 0 0 1-1.5 0l-.32-.18a1.5 1.5 0 0 1-.75-1.3V3.5A1.5 1.5 0 0 0 10.27 2z"/><circle cx="10" cy="10" r="2.25"/></svg>
          {t('ctx.settings')}
        </button>
        <div className="ctx-sep" />
        <button className="ctx-item ctx-warn" onClick={() => { onAction('reinstall', target.inst); onClose() }}>
          <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round"><path d="M4 4v5h5"/><path d="M16 16v-5h-5"/><path d="M4.93 9A8 8 0 1 1 4 13.42"/></svg>
          {t('ctx.reinstall')}
        </button>
        <button className="ctx-item ctx-danger" onClick={() => { onAction('delete', target.inst); onClose() }}>
          <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round"><polyline points="4 5 5 5 16 5"/><path d="M15 5l-.9 11H5.9L5 5"/><path d="M8 9v5M12 9v5"/><path d="M7 5V3.5A.5.5 0 0 1 7.5 3h5a.5.5 0 0 1 .5.5V5"/></svg>
          {t('ctx.delete')}
        </button>
      </motion.div>
    </>
  )
}

// ─── Reinstall Modal ──────────────────────────────────────────────────────────

function ReinstallModal({ inst, isLb, onClose }: { inst: Instance; isLb: boolean; onClose: () => void }) {
  const t = useT(getLang())
  const [mode, setMode] = useState<'keep' | 'wipe'>('keep')
  const [busy, setBusy] = useState(false)

  const handleReinstall = async () => {
    setBusy(true)
    if (isTauri) {
      try { await invoke('reinstall_instance', { instanceName: inst.name, fullWipe: mode === 'wipe' }) } catch { /* */ }
    }
    onClose()
  }

  return (
    <motion.div className="overlay" initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} onClick={onClose}>
      <motion.div className={`modal glass reinstall-modal${isLb ? ' lb-theme-modal' : ''}`}
        initial={{ opacity: 0, scale: 0.9, y: 20 }} animate={{ opacity: 1, scale: 1, y: 0 }}
        exit={{ opacity: 0, scale: 0.9, y: 16 }} transition={spring}
        onClick={e => e.stopPropagation()}
      >
        <div className="modal-head">
          <span className="modal-title">{t('reinstall.title')} — {inst.name}</span>
          <button className="modal-close" onClick={onClose}>×</button>
        </div>
        <div className="reinstall-body">
          <p className="reinstall-desc">{t('reinstall.choose')}</p>
          <label className={`reinstall-opt${mode === 'keep' ? ' active' : ''}`} onClick={() => setMode('keep')}>
            <span className="reinstall-radio" />
            <div>
              <div className="reinstall-opt-title">{t('reinstall.keep')}</div>
              <div className="reinstall-opt-sub">{t('reinstall.keep_desc')}</div>
            </div>
          </label>
          <label className={`reinstall-opt${mode === 'wipe' ? ' active danger-opt' : ''}`} onClick={() => setMode('wipe')}>
            <span className="reinstall-radio" />
            <div>
              <div className="reinstall-opt-title">{t('reinstall.wipe')}</div>
              <div className="reinstall-opt-sub">{t('reinstall.wipe_desc')}</div>
            </div>
          </label>
        </div>
        <div className="inst-modal-footer">
          <button className="btn-cancel" onClick={onClose}>{t('reinstall.cancel')}</button>
          <button className={`btn-ok${mode === 'wipe' ? ' btn-delete-confirm enabled' : ''}`}
            onClick={handleReinstall} disabled={busy}>
            {busy ? t('reinstall.doing') : t('reinstall.btn')}
          </button>
        </div>
      </motion.div>
    </motion.div>
  )
}

// ─── Instance Settings Modal ──────────────────────────────────────────────────

function InstanceSettingsModal({ inst, isLb, onClose }: { inst: Instance; isLb: boolean; onClose: () => void }) {
  const t = useT(getLang())
  const ramKey = `mlbv_inst_ram_${inst.id}`
  const globalRam = Number(localStorage.getItem('mlbv_ram') ?? '2048') || 2048
  const [useCustomRam, setUseCustomRam] = useState(() => !!localStorage.getItem(ramKey))
  const [ram, setRam] = useState(() => Number(localStorage.getItem(ramKey) ?? globalRam))
  const [logText, setLogText]   = useState('')
  const [logBusy, setLogBusy]   = useState(false)

  useEffect(() => {
    if (!isTauri) return
    invoke<string>('read_instance_log', { instanceName: inst.name })
      .then(s => setLogText(s))
      .catch(() => {})
  }, [inst.name])

  useEffect(() => {
    if (useCustomRam) localStorage.setItem(ramKey, String(ram))
    else localStorage.removeItem(ramKey)
  }, [useCustomRam, ram, ramKey])

  const clampRam = (v: number) => Math.min(16384, Math.max(512, Math.round(v / 512) * 512))
  const logLines = logText ? logText.split('\n').slice(-30).join('\n') : ''
  const accentVar = isLb ? 'var(--lb-accent)' : 'var(--accent)'

  return (
    <motion.div className="overlay" initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} onClick={onClose}>
      <motion.div className="modal glass inst-settings-modal"
        style={{ '--inst-accent': accentVar } as React.CSSProperties}
        initial={{ opacity: 0, scale: 0.9, y: 20 }} animate={{ opacity: 1, scale: 1, y: 0 }}
        exit={{ opacity: 0, scale: 0.9, y: 16 }} transition={spring}
        onClick={e => e.stopPropagation()}
      >
        <div className="modal-head">
          <span className="modal-title">{inst.name}</span>
          <button className="modal-close" onClick={onClose}>×</button>
        </div>
        <div className="inst-settings-body">
          <div className="setting-group">
            <div className="setting-label">{t('isettings.ram')}</div>
            <label className="setting-toggle">
              <input type="checkbox" checked={useCustomRam} onChange={e => setUseCustomRam(e.target.checked)} />
              <span className="toggle-track ist-toggle-track"><span className="toggle-thumb" /></span>
              <span className="toggle-label">{useCustomRam ? `${ram >= 1024 ? `${(ram/1024).toFixed(1)} GB` : `${ram} MB`}` : `Global (${globalRam >= 1024 ? `${(globalRam/1024).toFixed(1)} GB` : `${globalRam} MB`})`}</span>
            </label>
            {useCustomRam && (
              <input type="range" className="glass-range" min={512} max={16384} step={512}
                value={ram} onChange={e => setRam(clampRam(Number(e.target.value)))} />
            )}
          </div>
          <div className="setting-group">
            <div className="setting-label">{t('isettings.logs')}</div>
            <div className="inst-log-actions">
              <button className="btn-secondary" onClick={() => { navigator.clipboard.writeText(logText); setLogBusy(true); setTimeout(() => setLogBusy(false), 1200) }}>
                {logBusy ? t('isettings.copied') : t('isettings.copy_log')}
              </button>
              {isTauri && (
                <button className="btn-secondary" onClick={() =>
                  invoke('open_instance_logs_folder', { instanceName: inst.name }).catch(() => {})
                }>{t('isettings.open_logs')}</button>
              )}
            </div>
            {logLines
              ? <pre className="inst-log-preview">{logLines}</pre>
              : <div className="setting-hint">{t('isettings.no_log')}</div>
            }
          </div>
          <div className="setting-group">
            <div className="setting-label">{t('isettings.info')}</div>
            <div className="setting-hint">
              Version: {inst.version} · MC {inst.mcVersion}<br/>
              Type: {inst.type === 'lb' ? t('isettings.type_lb') : t('isettings.type_mc')}
            </div>
          </div>
        </div>
      </motion.div>
    </motion.div>
  )
}

// ─── Update Modal ─────────────────────────────────────────────────────────────

function UpdateModal({ info, onClose }: { info: UpdateInfo; onClose: () => void }) {
  const t = useT(getLang())
  const [phase, setPhase] = useState<'ask' | 'downloading' | 'installing'>('ask')
  const [percent, setPercent] = useState(0)
  const [dlError, setDlError] = useState('')

  const handleDownload = async () => {
    if (!isTauri || !info.assetUrl) {
      invoke('open_url', { url: info.htmlUrl }).catch(() => {})
      onClose()
      return
    }
    setPhase('downloading')
    setDlError('')
    const unlisten = await listen<{ percent: number }>('update-progress', e => {
      setPercent(Math.round(e.payload.percent))
    })
    try {
      await invoke('download_update', { url: info.assetUrl })
      unlisten()
      setPhase('installing')
      await new Promise(r => setTimeout(r, 700))
      await invoke('apply_update', { newVersion: info.version })
      // app.exit(0) is called in Rust; this line is a safety fallback
    } catch (e) {
      unlisten()
      setDlError(String(e))
      setPhase('ask')
    }
  }

  return (
    <motion.div className="overlay" initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }}
      onClick={phase === 'ask' ? onClose : undefined}
    >
      <motion.div className="modal glass update-modal"
        initial={{ opacity: 0, scale: 0.9, y: 24 }} animate={{ opacity: 1, scale: 1, y: 0 }}
        exit={{ opacity: 0, scale: 0.9, y: 16 }} transition={spring}
        onClick={e => e.stopPropagation()}
      >
        <div className="modal-head">
          <div>
            <span className="modal-title">{t('update.title')}</span>
            <span className="update-tag-badge">v{info.version}</span>
          </div>
          {phase === 'ask' && <button className="modal-close" onClick={onClose}>×</button>}
        </div>

        {phase === 'ask' && <>
          {info.body
            ? <pre className="update-changelog">{info.body.trim()}</pre>
            : <p className="update-nobody">{t('update.no_notes')}</p>
          }
          {dlError && <div className="inst-error" style={{ margin: '0 18px 10px' }}>{dlError}</div>}
          <div className="inst-modal-footer">
            <button className="btn-cancel" onClick={onClose}>{t('update.later')}</button>
            <button className="btn-ok" onClick={handleDownload}>{t('update.download')}</button>
          </div>
        </>}

        {(phase === 'downloading' || phase === 'installing') && (
          <div className="update-dl-wrap">
            <div className="update-dl-label">
              {phase === 'downloading' ? t('update.downloading') : t('update.installing')}
            </div>
            <div className="update-dl-bar-bg">
              <motion.div className="update-dl-bar-fill"
                animate={{ width: phase === 'installing' ? '100%' : `${percent}%` }}
                transition={{ duration: 0.25, ease: 'easeOut' }}
              />
            </div>
            {phase === 'downloading' && (
              <div className="update-dl-pct">{percent}%</div>
            )}
            {phase === 'installing' && (
              <div className="update-dl-pct">{t('update.installing')}</div>
            )}
          </div>
        )}
      </motion.div>
    </motion.div>
  )
}

// ─── Debug Info Block ─────────────────────────────────────────────────────────

function DebugInfoBlock() {
  const [info, setInfo] = useState<Record<string, unknown> | null>(null)
  const [open, setOpen] = useState(false)
  const [copied, setCopied] = useState(false)

  const load = async () => {
    if (!isTauri) return
    try {
      const d = await invoke<Record<string, unknown>>('get_debug_info')
      setInfo(d)
    } catch (e) {
      setInfo({ error: String(e) })
    }
  }

  const toggle = () => {
    if (!open && !info) load()
    setOpen(o => !o)
  }

  const copy = () => {
    if (!info) return
    navigator.clipboard.writeText(JSON.stringify(info, null, 2)).then(() => {
      setCopied(true)
      setTimeout(() => setCopied(false), 2000)
    })
  }

  return (
    <div className="debug-block">
      <button className="debug-toggle" onClick={toggle}>
        {open ? '▾' : '▸'} Debug info
      </button>
      {open && (
        <div className="debug-body">
          {info ? (
            <>
              <pre className="debug-pre">{JSON.stringify(info, null, 2)}</pre>
              <button className="debug-copy" onClick={copy}>{copied ? '✓ Copied' : 'Copy'}</button>
            </>
          ) : (
            <span className="debug-loading">Loading…</span>
          )}
        </div>
      )}
    </div>
  )
}

// ─── Main App ─────────────────────────────────────────────────────────────────

export default function App() {
  // First-run wizard
  const [setupDone, setSetupDone] = useState(() => !!localStorage.getItem('mlbv_setup_done'))
  const [lang, setLang] = useState<Lang>(() => getLang())
  const t = useT(lang)

  // Restore saved accent color on mount
  useEffect(() => {
    const saved = localStorage.getItem('mlbv_accent')
    const lbSame = localStorage.getItem('mlbv_lb_accent_same') === '1'
    if (saved) applyAccent(saved, lbSame)
  }, [])

  // App state
  const [appState, setAppState]     = useState<AppState>('loading')
  const [loadStatus, setLoadStatus] = useState('Connecting to Mojang…')
  const [loadProgress, setLoadProg] = useState(0)

  // Accounts — persisted in localStorage
  const [accounts, setAccounts] = useState<Account[]>(() => {
    try { return JSON.parse(localStorage.getItem('mlbv_accounts') ?? '[]') } catch { return [] }
  })
  const [selected, setSelected] = useState<Account | null>(() => {
    try {
      const uuid = localStorage.getItem('mlbv_selected_uuid')
      if (!uuid) return null
      const saved: Account[] = JSON.parse(localStorage.getItem('mlbv_accounts') ?? '[]')
      return saved.find(a => a.uuid === uuid) ?? null
    } catch { return null }
  })
  const [username, setUsername]       = useState('')
  const [showAddAcct, setShowAddAcct] = useState(false)
  const [msLoading, setMsLoading]     = useState(false)
  const [msError, setMsError]         = useState('')

  // Versions (MC manifest — for picker)
  const [versions, setVersions]     = useState<MCVersion[]>([])

  // (LiquidBounce versions are loaded inside CreateInstanceModal per-branch)

  // Instances
  const [instances, setInstances] = useState<Instance[]>(() => {
    try { return JSON.parse(localStorage.getItem('mlbv_instances') ?? '[]') } catch { return [] }
  })
  const [activeMcInstId, setActiveMcInstId] = useState<string | null>(() =>
    localStorage.getItem('mlbv_active_mc')
  )
  const [activeLbInstId, setActiveLbInstId] = useState<string | null>(() =>
    localStorage.getItem('mlbv_active_lb')
  )
  const [showCreateInst, setShowCreateInst] = useState(false)

  // Persist accounts
  useEffect(() => { localStorage.setItem('mlbv_accounts', JSON.stringify(accounts)) }, [accounts])
  useEffect(() => {
    if (selected) localStorage.setItem('mlbv_selected_uuid', selected.uuid)
    else localStorage.removeItem('mlbv_selected_uuid')
    setSkinError(false) // reset skin when account changes
  }, [selected])

  // Persist instances
  useEffect(() => {
    localStorage.setItem('mlbv_instances', JSON.stringify(instances))
  }, [instances])
  useEffect(() => {
    if (activeMcInstId) localStorage.setItem('mlbv_active_mc', activeMcInstId)
  }, [activeMcInstId])
  useEffect(() => {
    if (activeLbInstId) localStorage.setItem('mlbv_active_lb', activeLbInstId)
  }, [activeLbInstId])

  // UI
  const [activeTab, setActiveTab]             = useState<Tab>('mc')
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false)
  const [showSettings, setShowSettings]       = useState(false)
  const [launching, setLaunching]             = useState(false)
  const [launchingTab, setLaunchingTab]       = useState<Tab | null>(null)
  const [gameRunning, setGameRunning]         = useState<Tab | null>(null)
  const [busyFlash, setBusyFlash]             = useState<Tab | null>(null)
  const [progress, setProgress]               = useState(0)
  const [status, setStatus]                   = useState('')
  const [launchError, setLaunchError]         = useState(false)
  const [skinError, setSkinError]             = useState(false)
  const swipeStartX = useRef<number | null>(null)

  // Download controls
  const [dlPaused, setDlPaused]               = useState(false)
  const [dlSpeedBps, setDlSpeedBps]           = useState(0)

  // Update check
  const [updateInfo, setUpdateInfo]           = useState<UpdateInfo | null>(null)
  const [justUpdated, setJustUpdated]         = useState<string | null>(null)

  // Crash dialog
  const [crashInfo, setCrashInfo]             = useState<CrashInfo | null>(null)
  const lastLaunchedInst = useRef<Instance | null>(null)

  // Context menu
  const [ctxMenu, setCtxMenu]                 = useState<CtxTarget | null>(null)

  // Rename
  const [renamingId, setRenamingId]           = useState<string | null>(null)
  const [renameText, setRenameText]           = useState('')

  // Instance modals
  const [instSettingsOf, setInstSettingsOf]   = useState<Instance | null>(null)
  const [reinstallOf, setReinstallOf]         = useState<Instance | null>(null)

  // ── Computed ─────────────────────────────────────────────────────────────
  const fmtSpeed = (bps: number) => {
    if (bps <= 0) return ''
    if (bps < 1024) return `${bps} B/s`
    if (bps < 1024 * 1024) return `${(bps / 1024).toFixed(0)} KB/s`
    return `${(bps / (1024 * 1024)).toFixed(1)} MB/s`
  }
  const mcInstances = instances.filter(i => i.type === 'mc')
  const lbInstances = instances.filter(i => i.type === 'lb')
  const activeMcInst = mcInstances.find(i => i.id === activeMcInstId) ?? mcInstances[0] ?? null
  const activeLbInst = lbInstances.find(i => i.id === activeLbInstId) ?? lbInstances[0] ?? null
  const activeInstance = activeTab === 'mc' ? activeMcInst : activeLbInst
  const tabInstances = activeTab === 'mc' ? mcInstances : lbInstances
  const otherInstances = tabInstances.filter(i => i.id !== activeInstance?.id)

  // ── Fetch MC versions ────────────────────────────────────────────────────
  const fetchVersions = useCallback(async () => {
    setAppState('loading'); setLoadProg(0); setLoadStatus('Connecting to Mojang…')
    try {
      await tick(300); setLoadProg(30); setLoadStatus('Fetching version manifest…')
      const res = await fetch('https://launchermeta.mojang.com/mc/game/version_manifest_v2.json')
      if (!res.ok) throw new Error(`HTTP ${res.status}`)
      setLoadProg(70); setLoadStatus('Parsing versions…')
      await tick(150)
      const data = await res.json()
      setVersions(data.versions as MCVersion[])
      setLoadProg(100); setLoadStatus('Ready!')
      await tick(400); setAppState('ready')
    } catch {
      setLoadStatus('Failed to connect to Mojang servers'); setAppState('error')
    }
  }, [])

  useEffect(() => { fetchVersions() }, [fetchVersions])

  // Check for updates once the app is ready; also check if we just updated
  useEffect(() => {
    if (appState !== 'ready' || !isTauri) return
    // Was the app just updated?
    invoke<string>('get_just_updated')
      .then(ver => { if (ver) { setJustUpdated(ver); setTimeout(() => setJustUpdated(null), 5000) } })
      .catch(() => {})
    type RawRelease = { version: string; tag_name: string; body: string; html_url: string; asset_url: string }
    invoke<RawRelease | null>('check_for_update')
      .then(r => { if (r) setUpdateInfo({ version: r.version, tagName: r.tag_name, body: r.body, htmlUrl: r.html_url, assetUrl: r.asset_url }) })
      .catch(() => {})
  }, [appState])

  // ── Accounts ─────────────────────────────────────────────────────────────
  const addOffline = () => {
    const name = username.trim(); if (!name) return
    const acct: Account = { type: 'offline', username: name, uuid: crypto.randomUUID() }
    setAccounts(prev => [...prev, acct]); setSelected(acct); setUsername(''); setShowAddAcct(false)
  }

  const handleMsLogin = async () => {
    if (!isTauri) return
    setMsLoading(true); setMsError('')
    try {
      type Raw = { username: string; uuid: string; access_token: string; refresh_token: string }
      const raw = await invoke<Raw>('microsoft_login')
      const acct: Account = { type: 'microsoft', username: raw.username, uuid: raw.uuid, accessToken: raw.access_token }
      setAccounts(prev => [...prev.filter(a => a.uuid !== acct.uuid), acct])
      setSelected(acct); setShowAddAcct(false)
    } catch (err) { setMsError(String(err)) }
    setMsLoading(false)
  }

  // ── Instance management ───────────────────────────────────────────────────
  const addInstance = (inst: Instance) => {
    setInstances(prev => [...prev, inst])
    if (inst.type === 'mc') setActiveMcInstId(inst.id)
    else setActiveLbInstId(inst.id)
  }

  const removeInstance = (id: string) => {
    setInstances(prev => prev.filter(i => i.id !== id))
    if (activeMcInstId === id) setActiveMcInstId(null)
    if (activeLbInstId === id) setActiveLbInstId(null)
  }

  const renameInstance = (id: string, newName: string) => {
    const trimmed = newName.trim()
    if (!trimmed) return
    if (instances.some(i => i.id !== id && i.name === trimmed)) return
    setInstances(prev => prev.map(i => i.id === id ? { ...i, name: trimmed } : i))
  }

  const handleCtxAction = (action: CtxAction, inst: Instance) => {
    if (action === 'delete')    { removeInstance(inst.id) }
    if (action === 'rename')    { setRenamingId(inst.id); setRenameText(inst.name) }
    if (action === 'settings')  { setInstSettingsOf(inst) }
    if (action === 'reinstall') { setReinstallOf(inst) }
  }

  const handleCtxMenu = (e: React.MouseEvent, inst: Instance) => {
    e.preventDefault(); e.stopPropagation()
    setCtxMenu({ inst, x: e.clientX, y: e.clientY })
  }

  // ── Game-running event from backend ──────────────────────────────────────
  useEffect(() => {
    if (!isTauri) return
    let unlistenRunning: (() => void) | null = null
    let unlistenCrash:   (() => void) | null = null
    let unlistenSpeed:   (() => void) | null = null
    listen<boolean>('game-running', e => {
      if (!e.payload) {
        setGameRunning(null)
        getCurrentWindow().show().catch(() => {})
      }
    }).then(fn => { unlistenRunning = fn })
    listen<CrashInfo>('game-crashed', e => {
      setCrashInfo({ ...e.payload, instanceName: lastLaunchedInst.current?.name })
    }).then(fn => { unlistenCrash = fn })
    listen<{ bps: number }>('download-speed', e => {
      setDlSpeedBps(e.payload.bps)
    }).then(fn => { unlistenSpeed = fn })
    return () => { unlistenRunning?.(); unlistenCrash?.(); unlistenSpeed?.() }
  }, [])

  // Block native browser context menu everywhere
  useEffect(() => {
    const block = (e: MouseEvent) => e.preventDefault()
    document.addEventListener('contextmenu', block)
    return () => document.removeEventListener('contextmenu', block)
  }, [])

  // ── Stop running game ─────────────────────────────────────────────────────
  const handleStop = async () => {
    if (!isTauri) return
    try { await invoke('stop_game') } catch { /* already stopped */ }
  }

  // ── Download controls ─────────────────────────────────────────────────────
  const handleCancelDownload = () => {
    if (isTauri) invoke('cancel_download').catch(() => {})
  }
  const handlePauseDownload = async () => {
    if (!isTauri) return
    try {
      const nowPaused = await invoke<boolean>('pause_download')
      setDlPaused(nowPaused)
    } catch { /* */ }
  }

  // ── Launch ────────────────────────────────────────────────────────────────
  const handlePlay = async () => {
    if (!selected || !activeInstance) return
    if (launching) {
      setBusyFlash(activeTab)
      setTimeout(() => setBusyFlash(null), 900)
      return
    }
    if (gameRunning !== null) return
    const instTab: Tab = activeInstance.type === 'lb' ? 'lb' : 'mc'
    setLaunching(true); setLaunchingTab(instTab)
    setLaunchError(false); setProgress(0); setStatus('Preparing…')
    setDlPaused(false); setDlSpeedBps(0)
    lastLaunchedInst.current = activeInstance

    if (isTauri) {
      const unlisten = await listen<{ stage: string; progress: number; message: string }>(
        'launch-progress', evt => {
          setProgress(Math.round(evt.payload.progress))
          setStatus(evt.payload.message)
        }
      )
      try {
        const concurrentDl = Number(localStorage.getItem('mlbv_concurrent') ?? '5') || 5
        const globalRam = Number(localStorage.getItem('mlbv_ram') ?? '2048') || 2048
        const ramMb = Number(localStorage.getItem(`mlbv_inst_ram_${activeInstance.id}`) || globalRam)
        const showConsole = localStorage.getItem('mlbv_console_enabled') === '1'
        const baseArgs = {
          instanceName: activeInstance.name,
          username: selected.username,
          uuid: selected.uuid,
          offline: selected.type === 'offline',
          accessToken: selected.type === 'offline' ? '0' : (selected.accessToken ?? ''),
          concurrentDownloads: concurrentDl,
          maxRamMb: ramMb,
        }
        if (showConsole) {
          invoke('open_console_window', { instanceName: activeInstance.name }).catch(() => {})
        }
        if (activeInstance.type === 'lb' && activeInstance.buildId) {
          await invoke('launch_lb_game', {
            buildId: activeInstance.buildId,
            mcVersion: activeInstance.mcVersion,
            ...baseArgs,
          })
        } else if (activeInstance.loader === 'fabric') {
          await invoke('launch_fabric_game', {
            versionId: activeInstance.mcVersion,
            ...baseArgs,
          })
        } else {
          await invoke('launch_game', {
            versionId: activeInstance.mcVersion,
            ...baseArgs,
          })
        }
        setProgress(100); setStatus('Launched!')
        setGameRunning(instTab)
        if (localStorage.getItem('mlbv_close_on_launch') === '1') getCurrentWindow().hide().catch(() => {})
        await tick(1500)
      } catch (err) {
        setLaunchError(true); setStatus(String(err)); await tick(4000)
      } finally {
        unlisten()
        setLaunching(false); setLaunchingTab(null); setProgress(0); setStatus(''); setLaunchError(false)
        setDlPaused(false); setDlSpeedBps(0)
      }
    } else {
      const steps: [number, string][] = [[15,'Checking files…'],[35,'Downloading assets…'],[60,'Preparing Java…'],[85,'Verifying libs…'],[100,'Launched!']]
      for (const [p, m] of steps) { setProgress(p); setStatus(m); await tick(500) }
      await tick(1200); setLaunching(false); setProgress(0); setStatus('')
    }
  }

  // ── Setup wizard completion ───────────────────────────────────────────────
  const handleSetupDone = useCallback((newLang: Lang, account: Account | null) => {
    setLang(newLang)
    if (account) {
      setAccounts(prev => [...prev.filter(a => a.uuid !== account.uuid), account])
      setSelected(account)
    }
    setSetupDone(true)
  }, [])

  const winControls = {
    minimize: () => { if (isTauri) getCurrentWindow().minimize() },
    maximize: () => { if (isTauri) getCurrentWindow().toggleMaximize() },
    close:    () => { if (isTauri) getCurrentWindow().close() },
  }

  const onHeroPointerDown = (e: React.PointerEvent<HTMLDivElement>) => {
    if ((e.target as HTMLElement).closest('button, input, a, [role="button"]')) return
    e.currentTarget.setPointerCapture(e.pointerId)
    swipeStartX.current = e.clientX
  }
  const onHeroPointerUp = (e: React.PointerEvent<HTMLDivElement>) => {
    if (swipeStartX.current === null) return
    e.currentTarget.releasePointerCapture(e.pointerId)
    const delta = e.clientX - swipeStartX.current
    swipeStartX.current = null
    if (Math.abs(delta) < 60) return
    if (delta < 0 && activeTab === 'mc') setActiveTab('lb')
    if (delta > 0 && activeTab === 'lb') setActiveTab('mc')
  }
  const onHeroPointerCancel = () => { swipeStartX.current = null }

  // ─────────────────────────────────────────────────────────────────────────

  return (
    <div className={`app${activeTab === 'lb' ? ' lb-active' : ''}`}>
      <AnimatePresence>
        {(appState === 'loading' || appState === 'error') && (
          <LoadingScreen key="splash" status={loadStatus} progress={loadProgress}
            onRetry={appState === 'error' ? fetchVersions : undefined} />
        )}
      </AnimatePresence>

      <div className="bg-canvas">
        <div className="bg-grid" />
        <div className="orb orb-1" /><div className="orb orb-2" />
        <div className="orb orb-3" /><div className="orb orb-4" />
      </div>

      <AnimatePresence>
      {appState === 'ready' && (
      <motion.div key="main-ui" className="main-ui"
        initial={{ opacity: 0 }} animate={{ opacity: 1 }} transition={{ duration: 0.4, ease: 'easeOut' }}
      >
        {/* ── TITLEBAR ── */}
        <div className="titlebar" onMouseDown={e => {
          if (!(e.target as HTMLElement).closest('button,input,a,[role="button"]'))
            getCurrentWindow().startDragging().catch(() => {})
        }}>
          <div className="titlebar-left">
            <div className="logo-mark"><span /><span /><span /><span /></div>
            <span className="titlebar-name">MLBV</span>
          </div>
          <div className="titlebar-drag" />
          <div className="win-controls">
            <button className="wc min" onClick={winControls.minimize}>─</button>
            <button className="wc max" onClick={winControls.maximize}>□</button>
            <button className="wc cls" onClick={winControls.close}>✕</button>
          </div>
        </div>

        {/* ── CONTENT ── */}
        <div className="content">

          {/* ── SIDEBAR ── */}
          <motion.div
            className={`sidebar glass${sidebarCollapsed ? ' sidebar-collapsed' : ''}`}
            animate={{ width: sidebarCollapsed ? 62 : 228 }}
            transition={{ type: 'spring', stiffness: 380, damping: 36 }}
          >
            {/* Account section */}
            <div className="s-section">
              <AnimatePresence>
                {!sidebarCollapsed && (
                  <motion.div className="s-label"
                    initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} transition={{ duration: 0.18 }}
                  >{t('sidebar.account')}</motion.div>
                )}
              </AnimatePresence>
              <AnimatePresence mode="wait">
                {selected ? (
                  <motion.div key={`card-${selected.uuid}`}
                    className={`acct-card${sidebarCollapsed ? ' collapsed' : ''}`}
                    initial={{ opacity: 0, y: 8 }} animate={{ opacity: 1, y: 0 }}
                    exit={{ opacity: 0, y: -6 }} transition={spring}
                  >
                    {selected.type === 'microsoft' && !skinError
                      ? <img src={`https://mc-heads.net/avatar/${selected.uuid}/32`}
                             className="acct-skin" alt={selected.username[0]}
                             onError={() => setSkinError(true)} />
                      : <div className="acct-avatar">{selected.username[0].toUpperCase()}</div>
                    }
                    <AnimatePresence>
                      {!sidebarCollapsed && (
                        <motion.div className="acct-card-info" initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} transition={{ duration: 0.15 }}>
                          <div className="acct-card-text">
                            <div className="acct-name">{selected.username}</div>
                            <div className="acct-badge">{selected.type === 'offline' ? t('sw.acct.offline.label') : t('sw.acct.ms.label')}</div>
                          </div>
                          <motion.button className="acct-settings-btn" title={t('settings.title')}
                            onClick={e => { e.stopPropagation(); setShowSettings(true) }}
                            whileHover={{ scale: 1.12 }} whileTap={{ scale: 0.9 }}
                          >
                            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
                              <path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"/><circle cx="12" cy="12" r="3"/>
                            </svg>
                          </motion.button>
                        </motion.div>
                      )}
                    </AnimatePresence>
                  </motion.div>
                ) : (
                  <motion.div key="no-acct" className="no-acct"
                    initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }}
                  >{sidebarCollapsed ? '?' : t('no_account')}</motion.div>
                )}
              </AnimatePresence>
              <AnimatePresence>
                {!sidebarCollapsed && accounts.filter(a => a.uuid !== selected?.uuid).map(a => (
                  <motion.div key={a.uuid} className="acct-mini"
                    initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }}
                    onClick={() => setSelected(a)} whileHover={{ x: 3 }} transition={spring}
                  >
                    <span className="acct-mini-av">{a.username[0].toUpperCase()}</span>
                    <span>{a.username}</span>
                  </motion.div>
                ))}
              </AnimatePresence>
              <motion.button
                className={`btn-add-acct${sidebarCollapsed ? ' icon-only' : ''}`}
                onClick={() => setShowAddAcct(true)}
                whileHover={{ scale: 1.02 }} whileTap={{ scale: 0.97 }}
              >{sidebarCollapsed ? '+' : t('add_account')}</motion.button>
            </div>

            {/* Instance section */}
            <div className="s-section">
              <AnimatePresence>
                {!sidebarCollapsed && (
                  <motion.div className="s-label"
                    initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} transition={{ duration: 0.18 }}
                  >{activeTab === 'mc' ? t('sidebar.instances') : t('tab.lb')}</motion.div>
                )}
              </AnimatePresence>

              {/* Active instance card */}
              <AnimatePresence mode="wait">
                {activeInstance ? (
                  <motion.div key={activeInstance.id}
                    className={`ver-card${activeInstance.type === 'lb' ? ' lb-ver-card' : ''}${sidebarCollapsed ? ' collapsed' : ''}`}
                    initial={{ opacity: 0, y: 8 }} animate={{ opacity: 1, y: 0 }}
                    exit={{ opacity: 0, y: -6 }} transition={spring}
                    onContextMenu={e => handleCtxMenu(e, activeInstance)}
                  >
                    {activeInstance.type === 'lb'
                      ? <LbBadge size={24} />
                      : <span className="ver-tag release">MC</span>
                    }
                    <AnimatePresence>
                      {!sidebarCollapsed && (
                        <motion.div className="ver-card-info"
                          initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} transition={{ duration: 0.15 }}
                        >
                          {renamingId === activeInstance.id
                            ? <input autoFocus className="inst-rename-input"
                                value={renameText} onChange={e => setRenameText(e.target.value)}
                                onBlur={() => { renameInstance(activeInstance.id, renameText); setRenamingId(null) }}
                                onKeyDown={e => {
                                  if (e.key === 'Enter') { renameInstance(activeInstance.id, renameText); setRenamingId(null) }
                                  if (e.key === 'Escape') setRenamingId(null)
                                }}
                                onClick={e => e.stopPropagation()}
                              />
                            : <span className="ver-card-id">{activeInstance.name}</span>
                          }
                          <span className="ver-mc-hint">{activeInstance.mcVersion}</span>
                        </motion.div>
                      )}
                    </AnimatePresence>
                  </motion.div>
                ) : (
                  <motion.div key="no-inst" className="no-acct"
                    initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }}
                  >{sidebarCollapsed ? '?' : t('no_instance')}</motion.div>
                )}
              </AnimatePresence>

              {/* Other instances */}
              <AnimatePresence>
                {!sidebarCollapsed && otherInstances.map(inst => (
                  <motion.div key={inst.id} className={`ver-mini${inst.type === 'lb' ? ' lb-ver-mini' : ''}`}
                    initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }}
                    onClick={() => inst.type === 'mc' ? setActiveMcInstId(inst.id) : setActiveLbInstId(inst.id)}
                    onContextMenu={e => handleCtxMenu(e, inst)}
                    whileHover={{ x: 3 }} transition={spring}
                  >
                    {inst.type === 'lb'
                      ? <LbBadge size={18} />
                      : <span className="vbadge release">MC</span>
                    }
                    {renamingId === inst.id
                      ? <input autoFocus className="inst-rename-input"
                          value={renameText} onChange={e => setRenameText(e.target.value)}
                          onBlur={() => { renameInstance(inst.id, renameText); setRenamingId(null) }}
                          onKeyDown={e => {
                            if (e.key === 'Enter') { renameInstance(inst.id, renameText); setRenamingId(null) }
                            if (e.key === 'Escape') setRenamingId(null)
                          }}
                          onClick={e => e.stopPropagation()}
                        />
                      : <span className="ver-mini-id">{inst.name}</span>
                    }
                    <button className="inst-del" onClick={e => { e.stopPropagation(); removeInstance(inst.id) }}>×</button>
                  </motion.div>
                ))}
              </AnimatePresence>

              {/* Add instance button */}
              <AnimatePresence>
                {!sidebarCollapsed && (
                  <motion.button className="btn-browse-ver"
                    initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }}
                    onClick={() => setShowCreateInst(true)}
                  >{t('new_instance')}</motion.button>
                )}
              </AnimatePresence>
              {sidebarCollapsed && (
                <motion.button className="btn-add-acct icon-only" onClick={() => setShowCreateInst(true)}
                  whileHover={{ scale: 1.05 }} whileTap={{ scale: 0.95 }}
                >+</motion.button>
              )}
            </div>

            {/* Bottom bar */}
            <div className="s-bottom">
              {sidebarCollapsed && (
                <motion.button className="btn-icon-sm" title={t('settings.title')}
                  onClick={() => setShowSettings(true)}
                  whileHover={{ scale: 1.08 }} whileTap={{ scale: 0.93 }}
                >
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
                    <path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"/><circle cx="12" cy="12" r="3"/>
                  </svg>
                </motion.button>
              )}
              <motion.button className="btn-icon-sm collapse-toggle"
                title={sidebarCollapsed ? t('sidebar.expand') : t('sidebar.collapse')}
                onClick={() => setSidebarCollapsed(c => !c)}
                whileHover={{ scale: 1.08 }} whileTap={{ scale: 0.93 }}
                animate={{ rotate: sidebarCollapsed ? 180 : 0 }} transition={spring}
              >
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
                  <path d="M15 6l-6 6 6 6"/>
                </svg>
              </motion.button>
            </div>
          </motion.div>

          {/* ── MAIN AREA ── */}
          <div className="main" onPointerDown={onHeroPointerDown} onPointerUp={onHeroPointerUp} onPointerCancel={onHeroPointerCancel}>
            <div className="tab-switcher">
              {(['mc', 'lb'] as Tab[]).map(tab => {
                const isActive      = activeTab === tab
                const isOtherBusy   = (launching && launchingTab === tab && !isActive)
                const isBusy        = busyFlash === tab
                const label         = tab === 'mc' ? 'Minecraft' : 'LiquidBounce'
                return (
                  <button key={tab}
                    className={[
                      'tab-pill',
                      isActive    ? `tab-pill-active tab-pill-${tab}` : '',
                      isOtherBusy ? `tab-pill-loading-${tab}` : '',
                      isBusy      ? 'tab-pill-busy' : '',
                    ].filter(Boolean).join(' ')}
                    onClick={() => setActiveTab(tab)}
                  >
                    {isOtherBusy && <div className="tab-pill-fill" style={{ width: `${progress}%` }} />}
                    <span className="tab-pill-label">
                      {isOtherBusy ? `${progress}%` : label}
                    </span>
                  </button>
                )
              })}
            </div>

            <AnimatePresence mode="wait">
              {activeTab === 'mc' ? (

                /* ─── Minecraft tab ─── */
                <motion.div key="mc-tab" className="tab-content"
                  initial={{ opacity: 0, x: -24 }} animate={{ opacity: 1, x: 0 }}
                  exit={{ opacity: 0, x: -24 }} transition={{ duration: 0.22, ease: [0.4,0,0.2,1] }}
                >
                  <div className="hero">
                    <motion.div className="mc-title noselect"
                      initial={{ opacity: 0, y: -24, scale: 0.95 }} animate={{ opacity: 1, y: 0, scale: 1 }}
                      transition={{ ...spring, delay: 0.05 }}
                    >Minecraft</motion.div>
                    <motion.div className="mc-ver-pill noselect"
                      initial={{ opacity: 0, scale: 0.9 }} animate={{ opacity: 1, scale: 1 }}
                      transition={{ ...spring, delay: 0.12 }}
                    >
                      <span className="dot" />
                      {activeInstance ? activeInstance.name : t('no_instance')}
                    </motion.div>
                  </div>
                  <div className="launch-zone">
                    <AnimatePresence mode="wait">
                      {launching && launchingTab === 'mc' ? (
                        <motion.div key="prog" className={`progress-wrap glass${launchError ? ' error' : ''}`}
                          initial={{ opacity: 0, scale: 0.92 }} animate={{ opacity: 1, scale: 1 }}
                          exit={{ opacity: 0, scale: 0.92 }} transition={spring}
                        >
                          <div className="progress-label">{dlPaused ? '⏸ Paused' : status}</div>
                          <div className="progress-track">
                            <motion.div className={`progress-fill${launchError ? ' error' : ''}`}
                              animate={{ width: `${progress}%` }} transition={{ duration: 0.3, ease: 'easeOut' }} />
                          </div>
                          <div className="dl-controls">
                            <span className="dl-speed">{fmtSpeed(dlSpeedBps)}</span>
                            <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                              <span className="progress-pct" style={{ margin: 0 }}>{progress}%</span>
                              {!launchError && (
                                <>
                                  <button className="dl-btn" onClick={handlePauseDownload}>
                                    {dlPaused ? '▶ Resume' : '⏸ Pause'}
                                  </button>
                                  <button className="dl-btn dl-cancel" onClick={handleCancelDownload}>✕ Cancel</button>
                                </>
                              )}
                            </div>
                          </div>
                        </motion.div>
                      ) : busyFlash === 'mc' ? (
                        <motion.div key="busy-mc" className="progress-wrap glass error"
                          initial={{ opacity: 0, scale: 0.92 }} animate={{ opacity: 1, scale: 1 }}
                          exit={{ opacity: 0, scale: 0.92 }} transition={spring}
                        >
                          <div className="progress-label">{t('busy')}</div>
                        </motion.div>
                      ) : gameRunning === 'mc' ? (
                        <motion.button key="stop-mc" className="stop-btn"
                          onClick={handleStop}
                          initial={{ opacity: 0, y: 20, scale: 0.95 }} animate={{ opacity: 1, y: 0, scale: 1 }}
                          exit={{ opacity: 0, scale: 0.95 }}
                          whileHover={{ scale: 1.035 }} whileTap={{ scale: 0.97 }}
                          transition={{ ...spring, delay: 0.18 }}
                        >
                          <span className="stop-icon">■</span> {t('stop')}
                        </motion.button>
                      ) : gameRunning === 'lb' ? (
                        <motion.div key="lb-running-mc" className="progress-wrap glass other-running-wrap"
                          initial={{ opacity: 0, scale: 0.92 }} animate={{ opacity: 1, scale: 1 }}
                          exit={{ opacity: 0, scale: 0.92 }} transition={spring}
                        >
                          <div className="progress-label other-running-label">
                            <span className="other-running-icon">■</span>
                            {t('running.lb')}
                          </div>
                        </motion.div>
                      ) : (
                        <motion.button key="play"
                          className={`play-btn${!selected || !activeInstance ? ' off' : ''}`}
                          onClick={handlePlay} disabled={!selected || !activeInstance}
                          initial={{ opacity: 0, y: 20, scale: 0.95 }} animate={{ opacity: 1, y: 0, scale: 1 }}
                          exit={{ opacity: 0, scale: 0.95 }}
                          whileHover={selected && activeInstance ? { scale: 1.035 } : {}}
                          whileTap={selected && activeInstance ? { scale: 0.97 } : {}}
                          transition={{ ...spring, delay: 0.18 }}
                        >
                          <span className="play-arrow">▶</span>
                          {!selected ? t('no_account') : !activeInstance ? t('no_instance') : t('play')}
                        </motion.button>
                      )}
                    </AnimatePresence>
                    {!launching && selected && activeInstance && (
                      <motion.div className="hint-text" initial={{ opacity: 0 }} animate={{ opacity: 1 }} transition={{ delay: 0.35 }}>
                        {gameRunning === 'mc'
                          ? <>{t('status.running')} · <strong style={{ color: 'var(--accent)' }}>{selected.username}</strong></>
                          : <>{t('status.playing_as')} <strong style={{ color: 'var(--accent)' }}>{selected.username}</strong> · {activeInstance.mcVersion}</>
                        }
                      </motion.div>
                    )}
                  </div>
                </motion.div>

              ) : (

                /* ─── LiquidBounce tab ─── */
                <motion.div key="lb-tab" className="tab-content lb-tab"
                  initial={{ opacity: 0, x: 24 }} animate={{ opacity: 1, x: 0 }}
                  exit={{ opacity: 0, x: 24 }} transition={{ duration: 0.22, ease: [0.4,0,0.2,1] }}
                >
                  <div className="hero lb-hero">
                    <motion.div className="lb-title-wrap noselect"
                      initial={{ opacity: 0, y: -20, scale: 0.95 }} animate={{ opacity: 1, y: 0, scale: 1 }}
                      transition={{ ...spring, delay: 0.05 }}
                    >
                      <img src={lbLogo} alt="LiquidBounce" className="lb-logo-img" draggable={false} />
                    </motion.div>
                    <motion.div className="mc-ver-pill lb-pill noselect"
                      initial={{ opacity: 0, scale: 0.9 }} animate={{ opacity: 1, scale: 1 }}
                      transition={{ ...spring, delay: 0.12 }}
                    >
                      <span className="dot lb-dot-inline" />
                      {activeInstance ? activeInstance.name : '—'}
                      {activeInstance && (
                        <span className="lb-mc-badge">MC {activeInstance.mcVersion}</span>
                      )}
                    </motion.div>
                  </div>
                  <div className="launch-zone">
                    <AnimatePresence mode="wait">
                      {launching && launchingTab === 'lb' ? (
                        <motion.div key="prog-lb" className={`progress-wrap glass lb-progress${launchError ? ' error' : ''}`}
                          initial={{ opacity: 0, scale: 0.92 }} animate={{ opacity: 1, scale: 1 }}
                          exit={{ opacity: 0, scale: 0.92 }} transition={spring}
                        >
                          <div className="progress-label">{dlPaused ? '⏸ Paused' : status}</div>
                          <div className="progress-track">
                            <motion.div className={`progress-fill lb-fill${launchError ? ' error' : ''}`}
                              animate={{ width: `${progress}%` }} transition={{ duration: 0.3, ease: 'easeOut' }} />
                          </div>
                          <div className="dl-controls">
                            <span className="dl-speed">{fmtSpeed(dlSpeedBps)}</span>
                            <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                              <span className="progress-pct" style={{ margin: 0 }}>{progress}%</span>
                              {!launchError && (
                                <>
                                  <button className="dl-btn" onClick={handlePauseDownload}>
                                    {dlPaused ? '▶ Resume' : '⏸ Pause'}
                                  </button>
                                  <button className="dl-btn dl-cancel" onClick={handleCancelDownload}>✕ Cancel</button>
                                </>
                              )}
                            </div>
                          </div>
                        </motion.div>
                      ) : busyFlash === 'lb' ? (
                        <motion.div key="busy-lb" className="progress-wrap glass error"
                          initial={{ opacity: 0, scale: 0.92 }} animate={{ opacity: 1, scale: 1 }}
                          exit={{ opacity: 0, scale: 0.92 }} transition={spring}
                        >
                          <div className="progress-label">{t('busy')}</div>
                        </motion.div>
                      ) : gameRunning === 'lb' ? (
                        <motion.button key="stop-lb" className="stop-btn lb-stop"
                          onClick={handleStop}
                          initial={{ opacity: 0, y: 20, scale: 0.95 }} animate={{ opacity: 1, y: 0, scale: 1 }}
                          exit={{ opacity: 0, scale: 0.95 }}
                          whileHover={{ scale: 1.035 }} whileTap={{ scale: 0.97 }}
                          transition={{ ...spring, delay: 0.18 }}
                        >
                          <span className="stop-icon">■</span> {t('stop')}
                        </motion.button>
                      ) : gameRunning === 'mc' ? (
                        <motion.div key="mc-running-lb" className="progress-wrap glass other-running-wrap"
                          initial={{ opacity: 0, scale: 0.92 }} animate={{ opacity: 1, scale: 1 }}
                          exit={{ opacity: 0, scale: 0.92 }} transition={spring}
                        >
                          <div className="progress-label other-running-label">
                            <span className="other-running-icon">■</span>
                            {t('running.mc')}
                          </div>
                        </motion.div>
                      ) : (
                        <motion.button key="play-lb"
                          className={`play-btn lb-play${!selected || !activeInstance ? ' off' : ''}`}
                          onClick={handlePlay} disabled={!selected || !activeInstance}
                          initial={{ opacity: 0, y: 20, scale: 0.95 }} animate={{ opacity: 1, y: 0, scale: 1 }}
                          exit={{ opacity: 0, scale: 0.95 }}
                          whileHover={selected && activeInstance ? { scale: 1.035 } : {}}
                          whileTap={selected && activeInstance ? { scale: 0.97 } : {}}
                          transition={{ ...spring, delay: 0.18 }}
                        >
                          <span className="play-arrow">▶</span>
                          {!selected ? t('no_account') : !activeInstance ? t('no_instance') : t('play.lb')}
                        </motion.button>
                      )}
                    </AnimatePresence>
                    {!launching && selected && activeInstance && (
                      <motion.div className="hint-text lb-hint" initial={{ opacity: 0 }} animate={{ opacity: 1 }} transition={{ delay: 0.35 }}>
                        {gameRunning === 'lb'
                          ? <>{t('status.running')} · <strong style={{ color: 'var(--lb-accent)' }}>{selected.username}</strong></>
                          : <><strong style={{ color: 'var(--lb-accent)' }}>{selected.username}</strong> · {activeInstance.name} (MC {activeInstance.mcVersion})</>
                        }
                      </motion.div>
                    )}
                  </div>
                </motion.div>

              )}
            </AnimatePresence>
          </div>
        </div>

        {/* ── CREATE INSTANCE MODAL ── */}
        <AnimatePresence>
          {showCreateInst && (
            <CreateInstanceModal
              defaultTab={activeTab}
              mcVersions={versions}
              existingNames={instances.map(i => i.name)}
              onAdd={addInstance}
              onClose={() => setShowCreateInst(false)}
            />
          )}
        </AnimatePresence>

        {/* ── ADD ACCOUNT MODAL ── */}
        <AnimatePresence>
          {showAddAcct && (
            <motion.div className="overlay" initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }}
              onClick={() => { setShowAddAcct(false); setMsError('') }}
            >
              <motion.div className="modal glass modal-sm"
                initial={{ opacity: 0, scale: 0.9, y: 24 }} animate={{ opacity: 1, scale: 1, y: 0 }}
                exit={{ opacity: 0, scale: 0.9, y: 16 }} transition={spring}
                onClick={e => e.stopPropagation()}
              >
                <div className="modal-head">
                  <span className="modal-title">{t('acct.title')}</span>
                  <button className="modal-close" onClick={() => { setShowAddAcct(false); setMsError('') }}>×</button>
                </div>
                <div className="acct-modal-body">
                  <div>
                    <div className="field-label">{t('acct.offline_label')}</div>
                    <div className="input-row">
                      <input className="glass-input" placeholder="Enter username…" value={username}
                        onChange={e => setUsername(e.target.value)}
                        onKeyDown={e => e.key === 'Enter' && addOffline()}
                        maxLength={16} autoFocus />
                      <button className="btn-ok" onClick={addOffline} disabled={!username.trim()}>{t('acct.add')}</button>
                    </div>
                  </div>
                  <div className="or-divider">{t('acct.or')}</div>
                  <div>
                    <div className="field-label">{t('acct.ms_label')}</div>
                    <button className="btn-ms" onClick={handleMsLogin} disabled={msLoading}>
                      {msLoading ? (
                        <span className="ms-spinner" />
                      ) : (
                        <svg viewBox="0 0 21 21" width="17" height="17" fill="currentColor">
                          <rect x="0"  y="0"  width="9" height="9" fill="#f25022"/>
                          <rect x="11" y="0"  width="9" height="9" fill="#7fba00"/>
                          <rect x="0"  y="11" width="9" height="9" fill="#00a4ef"/>
                          <rect x="11" y="11" width="9" height="9" fill="#ffb900"/>
                        </svg>
                      )}
                      {msLoading ? t('acct.ms_loading') : t('acct.ms_btn')}
                    </button>
                    {msError && <div className="ms-error">{msError}</div>}
                    {!msError && <div className="ms-note">{msLoading ? t('acct.ms_note_loading') : t('acct.ms_note')}</div>}
                  </div>
                </div>
              </motion.div>
            </motion.div>
          )}
        </AnimatePresence>

        {/* ── SETTINGS MODAL ── */}
        <AnimatePresence>
          {showSettings && <SettingsModal onClose={() => setShowSettings(false)} onLangChange={l => setLang(l)} />}
        </AnimatePresence>

        {/* ── INSTANCE CONTEXT MENU ── */}
        <AnimatePresence>
          {ctxMenu && (
            <InstanceCtxMenu
              target={ctxMenu}
              isLb={ctxMenu.inst.type === 'lb'}
              onAction={handleCtxAction}
              onClose={() => setCtxMenu(null)}
            />
          )}
        </AnimatePresence>

        {/* ── INSTANCE SETTINGS ── */}
        <AnimatePresence>
          {instSettingsOf && (
            <InstanceSettingsModal
              inst={instSettingsOf}
              isLb={instSettingsOf.type === 'lb'}
              onClose={() => setInstSettingsOf(null)}
            />
          )}
        </AnimatePresence>

        {/* ── REINSTALL ── */}
        <AnimatePresence>
          {reinstallOf && (
            <ReinstallModal
              inst={reinstallOf}
              isLb={reinstallOf.type === 'lb'}
              onClose={() => setReinstallOf(null)}
            />
          )}
        </AnimatePresence>

        {/* ── CRASH DIALOG ── */}
        <AnimatePresence>
          {crashInfo && (
            <CrashDialog info={crashInfo} onClose={() => setCrashInfo(null)} />
          )}
        </AnimatePresence>

        {/* ── UPDATE MODAL ── */}
        <AnimatePresence>
          {updateInfo && (
            <UpdateModal info={updateInfo} onClose={() => setUpdateInfo(null)} />
          )}
        </AnimatePresence>

        {/* ── JUST UPDATED TOAST ── */}
        <AnimatePresence>
          {justUpdated && (
            <motion.div className="just-updated-toast"
              initial={{ opacity: 0, y: 16, scale: 0.95 }}
              animate={{ opacity: 1, y: 0, scale: 1 }}
              exit={{ opacity: 0, y: 10, scale: 0.95 }}
              transition={spring}
            >
              <svg viewBox="0 0 16 16" fill="none" style={{ width: 15, height: 15, flexShrink: 0 }}>
                <circle cx="8" cy="8" r="7" fill="rgba(74,222,128,0.2)" stroke="rgba(74,222,128,0.5)" strokeWidth="1"/>
                <path d="M5 8l2 2 4-4" stroke="#4ade80" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round"/>
              </svg>
              {t('update.done')} v{justUpdated}
            </motion.div>
          )}
        </AnimatePresence>

      </motion.div>
      )}
      </AnimatePresence>

      {/* ── SETUP WIZARD (overlays everything) ── */}
      <AnimatePresence>
        {!setupDone && (
          <motion.div key="wizard"
            style={{ position: 'fixed', inset: 0, zIndex: 9999 }}
            initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }}
            transition={{ duration: 0.3 }}
          >
            <SetupWizard onDone={handleSetupDone} />
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  )
}

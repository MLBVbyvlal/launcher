import { useState, useEffect } from 'react'
import { motion, AnimatePresence } from 'framer-motion'
import { invoke } from '@tauri-apps/api/core'
import { getLang, type Lang, useT } from '../i18n'
import { isTauri, spring } from '../lib/types'
import { DEFAULT_ACCENT, ACCENT_PRESETS, applyAccent } from '../lib/accent'
import { Tip } from './ui'
import { AboutPanel, DangerPanel } from './SettingsPanels'

// ─── Settings modal ───────────────────────────────────────────────────────────

const RAM_MARKS = [
  { v: 512,   label: '512 MB', pct: 0    },
  { v: 4096,  label: '4 GB',   pct: 22.6 },
  { v: 8192,  label: '8 GB',   pct: 48.4 },
  { v: 16384, label: '16 GB',  pct: 100  },
]

type SettingsTab = 'general' | 'performance' | 'java' | 'about' | 'customize' | 'danger'

export default function SettingsModal({ onClose, onLangChange, updateCheckError }: { onClose: () => void; onLangChange?: (l: Lang) => void; updateCheckError?: string | null }) {
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
  const [minRam, setMinRam]       = useState(() => { const s = localStorage.getItem('mlbv_min_ram'); return s ? Number(s) : 512 })
  const [minRamDraft, setMinRamDraft] = useState<string | null>(null)
  const [javaPath, setJavaPath]   = useState(() => localStorage.getItem('mlbv_java_path') ?? '')
  const [jvmArgs, setJvmArgs]     = useState(() => localStorage.getItem('mlbv_jvm_args') ?? '')
  const [closeOnLaunch, setCloseOnLaunch]   = useState(() => localStorage.getItem('mlbv_close_on_launch') === '1')
  const [consoleEnabled, setConsoleEnabled] = useState(() => localStorage.getItem('mlbv_console_enabled') === '1')
  const [javaInstalls, setJavaInstalls]     = useState<{ major: number; path: string }[]>([])
  const [dangerOpen, setDangerOpen]         = useState(false)
  const [cfKey, setCfKey]                     = useState(() => localStorage.getItem('mlbv_cf_key') ?? '')
  const [countdown, setCountdown]           = useState(5)

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
  useEffect(() => { localStorage.setItem('mlbv_min_ram', String(minRam)) }, [minRam])
  useEffect(() => { localStorage.setItem('mlbv_java_path', javaPath) }, [javaPath])
  useEffect(() => { localStorage.setItem('mlbv_jvm_args', jvmArgs) }, [jvmArgs])
  useEffect(() => { localStorage.setItem('mlbv_concurrent', String(concurrent)) }, [concurrent])
  useEffect(() => { localStorage.setItem('mlbv_close_on_launch', closeOnLaunch ? '1' : '0') }, [closeOnLaunch])
  useEffect(() => { localStorage.setItem('mlbv_console_enabled', consoleEnabled ? '1' : '0') }, [consoleEnabled])

  useEffect(() => {
    if (!dangerOpen || countdown <= 0) return
    const id = setTimeout(() => setCountdown(c => c - 1), 1000)
    return () => clearTimeout(id)
  }, [dangerOpen, countdown])

  const clampRam        = (v: number) => Math.min(16384, Math.max(512, Math.round(v / 512) * 512))
  const clampMinRam     = (v: number) => Math.min(8192, Math.max(256, Math.round(v / 256) * 256))
  const clampConcurrent = (v: number) => Math.min(50, Math.max(1, Math.round(v)))
  const commitRam        = (raw: string) => { const n = Number(raw); if (!isNaN(n) && n > 0) setRam(clampRam(n)); setRamDraft(null) }
  const commitMinRam     = (raw: string) => { const n = Number(raw); if (!isNaN(n) && n > 0) setMinRam(clampMinRam(n)); setMinRamDraft(null) }
  const commitConcurrent = (raw: string) => { const n = Number(raw); if (!isNaN(n) && n > 0) setConcurrent(clampConcurrent(n)); setConcurrentDraft(null) }

  const concurrentWarning = concurrent < 5
    ? t('perf.warn.low')
    : concurrent >= 10
    ? t('perf.warn.high')
    : null

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
                  <div className="setting-group">
                    <div className="setting-label">{t('settings.cf_key')}</div>
                    <input className="glass-input" type="password" autoComplete="off" spellCheck={false}
                      placeholder={t('settings.cf_key_ph')} value={cfKey}
                      onChange={e => { setCfKey(e.target.value); localStorage.setItem('mlbv_cf_key', e.target.value) }} />
                    <div className="setting-hint">{t('settings.cf_key_hint')}</div>
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
                      <div className="setting-label">{t('settings.min_ram')} — {minRam >= 1024 ? `${(minRam/1024).toFixed(1)} GB` : `${minRam} MB`}</div>
                      <Tip text={t('settings.tip.ram')} />
                    </div>
                    <div className="ram-row">
                      <div className="ram-slider-wrap">
                        <input type="range" className="glass-range" min={256} max={8192} step={256}
                          value={Math.min(minRam, 8192)} onChange={e => { setMinRam(Number(e.target.value)); setMinRamDraft(null) }} />
                      </div>
                      <input type="number" className="ram-input" min={256} max={8192}
                        value={minRamDraft ?? minRam}
                        onChange={e => setMinRamDraft(e.target.value)}
                        onBlur={e => commitMinRam(e.target.value)}
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
                  <div className="setting-group">
                    <div className="setting-label-row">
                      <div className="setting-label">{t('settings.java_path')}</div>
                      <Tip text={t('settings.java_path_hint')} />
                    </div>
                    <input type="text" className="lb-input" placeholder={t('settings.java_path_ph')}
                      value={javaPath} onChange={e => setJavaPath(e.target.value)} spellCheck={false} />
                  </div>
                  <div className="setting-group">
                    <div className="setting-label-row">
                      <div className="setting-label">{t('settings.jvm_args')}</div>
                      <Tip text={t('settings.jvm_args_hint')} />
                    </div>
                    <input type="text" className="lb-input" placeholder={t('settings.jvm_args_ph')}
                      value={jvmArgs} onChange={e => setJvmArgs(e.target.value)} spellCheck={false} />
                  </div>
                </>}

                {/* ── ABOUT ── */}
                {tab === 'about' && <AboutPanel updateCheckError={updateCheckError} />}

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
                {tab === 'danger' && <DangerPanel dangerOpen={dangerOpen} setDangerOpen={setDangerOpen} countdown={countdown} setCountdown={setCountdown} />}

              </motion.div>
            </AnimatePresence>
          </div>
        </div>

      </motion.div>
    </motion.div>
  )
}

import { useState, useRef, useCallback } from 'react'
import { motion, AnimatePresence } from 'framer-motion'
import { invoke } from '@tauri-apps/api/core'
import { type Lang, useT } from './i18n'
import msLogo from './assets/ms-logo.png'
import SetupJavaShowcase from './components/SetupJavaShowcase'

const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
const spring   = { type: 'spring', stiffness: 340, damping: 28 } as const

type Account    = { type: 'offline' | 'microsoft'; username: string; uuid: string }
type StepId     = 'welcome' | 'prefs' | 'account' | 'offline-warn' | 'nick' | 'nick-warn' | 'ms-loading' | 'java'

const RAM_MARKS = [
  { v: 512,   label: '512 MB', pct: 0    },
  { v: 4096,  label: '4 GB',   pct: 22.6 },
  { v: 8192,  label: '8 GB',   pct: 48.4 },
  { v: 16384, label: '16 GB',  pct: 100  },
]

function validateNick(s: string): 'short' | 'long' | 'chars' | null {
  if (s.length < 3)  return 'short'
  if (s.length > 16) return 'long'
  if (!/^[a-zA-Z0-9_]+$/.test(s)) return 'chars'
  return null
}

const slideIn  = (dir: number) => ({ opacity: 0, x: dir * 48 })
const slideOut = (dir: number) => ({ opacity: 0, x: -dir * 48 })

const STEPS_MAIN: StepId[] = ['welcome', 'prefs', 'account', 'java']

function StepDots({ step }: { step: StepId }) {
  const idx = STEPS_MAIN.indexOf(step)
  const cur  = idx >= 0 ? idx : STEPS_MAIN.indexOf('account')
  return (
    <div className="sw-dots">
      {STEPS_MAIN.map((_, i) => (
        <div key={i} className={`sw-dot${i === cur ? ' sw-dot-active' : i < cur ? ' sw-dot-done' : ''}`} />
      ))}
    </div>
  )
}

function PersonIcon({ size = 48 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 48 48" fill="none">
      <circle cx="24" cy="16" r="10" fill="currentColor" opacity="0.9" />
      <path d="M6 44c0-9.941 8.059-18 18-18s18 8.059 18 18"
        stroke="currentColor" strokeWidth="3.5" strokeLinecap="round" opacity="0.9" />
    </svg>
  )
}

export default function SetupWizard({ onDone }: { onDone: (lang: Lang, account: Account | null) => void }) {
  const [lang, setLang] = useState<Lang>('en')
  const t = useT(lang)

  const [step, setStep] = useState<StepId>('welcome')
  const [dir,  setDir]  = useState(1)
  const [isLbTheme, setIsLbTheme] = useState(false)

  const [ram,           setRam]           = useState(2048)
  const [concurrent,    setConcurrent]    = useState(5)
  const [closeOnLaunch, setCloseOnLaunch] = useState(false)

  const [msError,      setMsError]      = useState('')
  const pendingAccount                  = useRef<Account | null>(null)

  const [nick,        setNick]        = useState('')
  const [nickError,   setNickError]   = useState('')
  const [nickForWarn, setNickForWarn] = useState('')

  const goTo = useCallback((next: StepId, forward = true) => {
    setDir(forward ? 1 : -1)
    setStep(next)
  }, [])

  // ── MS auth ───────────────────────────────────────────────────────────────
  const handleMsLogin = useCallback(async () => {
    goTo('ms-loading')
    setMsError('')
    try {
      type Raw = { username: string; uuid: string }
      // Tokens stay in the Rust vault; only the profile comes back.
      const raw = await invoke<Raw>('microsoft_login')
      pendingAccount.current = { type: 'microsoft', username: raw.username, uuid: raw.uuid }
      goTo('java')
    } catch (e) {
      setMsError(String(e))
      goTo('account')
    }
  }, [goTo])

  // ── Nick ──────────────────────────────────────────────────────────────────
  const handleNickSubmit = () => {
    const err = validateNick(nick)
    if (err) { setNickError(t(`sw.nick.err.${err}`)); return }
    if (!/^[a-zA-Z][a-zA-Z0-9_]{2,15}$/.test(nick)) { setNickForWarn(nick); goTo('nick-warn'); return }
    pendingAccount.current = { type: 'offline', username: nick, uuid: crypto.randomUUID() }
    goTo('java')
  }

  // ── Render ────────────────────────────────────────────────────────────────
  return (
    <div className={`sw-root${isLbTheme ? ' sw-lb-theme' : ''}`}>
      <div className="bg-canvas">
        <div className="bg-grid" />
        <div className="orb orb-1" />
        <div className="orb orb-2" />
        <div className="orb orb-3" />
        <div className="orb orb-4" />
      </div>

      <div className="sw-corner-logo">
        <div className="logo-mark"><span /><span /><span /><span /></div>
        <span className="sw-corner-name">MLBV</span>
      </div>

      {step !== 'java' ? (

        /* ── Card-based wizard steps ── */
        <div className="sw-center">
          <StepDots step={step} />
          <div className="sw-card-wrap">
            <AnimatePresence mode="wait">

              {step === 'welcome' && (
                <motion.div key="welcome" className="sw-card glass"
                  initial={slideIn(dir)} animate={{ opacity: 1, x: 0 }} exit={slideOut(dir)} transition={spring}>
                  <div className="sw-card-inner">
                    <div className="sw-logo-big">
                      <div className="splash-logo-mark" style={{ width: 56, height: 56 }}><span /><span /><span /><span /></div>
                      <div className="sw-title-big">{t('sw.welcome.title')}</div>
                      <div className="sw-subtitle">{t('sw.welcome.sub')}</div>
                    </div>
                    <div className="sw-lang-hint">{t('sw.welcome.choose')}</div>
                    <div className="sw-lang-row">
                      <button className={`sw-lang-btn${lang === 'en' ? ' sw-lang-active' : ''}`} onClick={() => setLang('en')}>
                        <span className="sw-lang-badge sw-lang-en">🇬🇧</span>
                        <span>English</span>
                      </button>
                      <button className={`sw-lang-btn${lang === 'ru' ? ' sw-lang-active' : ''}`} onClick={() => setLang('ru')}>
                        <span className="sw-lang-badge sw-lang-ru">🇷🇺</span>
                        <span>Русский</span>
                      </button>
                    </div>
                  </div>
                  <div className="sw-footer">
                    <span />
                    <button className="sw-btn-primary" onClick={() => goTo('prefs')}>{t('btn.next')} →</button>
                  </div>
                </motion.div>
              )}

              {step === 'prefs' && (
                <motion.div key="prefs" className="sw-card glass"
                  initial={slideIn(dir)} animate={{ opacity: 1, x: 0 }} exit={slideOut(dir)} transition={spring}>
                  <div className="sw-card-inner">
                    <div className="sw-step-header">
                      <div className="sw-step-title">{t('sw.prefs.title')}</div>
                      <div className="sw-step-sub">{t('sw.prefs.sub')}</div>
                    </div>
                    <div className="sw-setting-group">
                      <div className="sw-setting-label">{t('sw.prefs.ram')} — {ram >= 1024 ? `${(ram/1024).toFixed(1)} GB` : `${ram} MB`}</div>
                      <div className="ram-slider-wrap">
                        <input type="range" className="glass-range sw-range" min={512} max={16384} step={512}
                          value={ram} onChange={e => setRam(Number(e.target.value))} />
                        <div className="ram-marks-abs">
                          {RAM_MARKS.map(m => (
                            <span key={m.v} className="ram-mark"
                              style={{ left: `calc(8px + ${m.pct/100} * (100% - 16px))` }}>{m.label}</span>
                          ))}
                        </div>
                      </div>
                    </div>
                    <div className="sw-setting-group">
                      <div className="sw-setting-label">{t('sw.prefs.dl')} — {concurrent}</div>
                      <div className="concurrent-row">
                        <input type="range" className="glass-range sw-range" min={1} max={20} step={1}
                          value={concurrent} onChange={e => setConcurrent(Number(e.target.value))} />
                      </div>
                      <div className="sw-range-marks"><span>1</span><span>5</span><span>10</span><span>20</span></div>
                    </div>
                    <label className="sw-checkbox-row">
                      <div className={`sw-checkbox${closeOnLaunch ? ' sw-checkbox-on' : ''}`}
                        onClick={() => setCloseOnLaunch(c => !c)}>
                        {closeOnLaunch && <span className="sw-checkbox-check">✓</span>}
                      </div>
                      <span className="sw-checkbox-label">{t('sw.prefs.close')}</span>
                    </label>
                  </div>
                  <div className="sw-footer">
                    <button className="sw-btn-ghost" onClick={() => goTo('welcome', false)}>← {t('btn.back')}</button>
                    <button className="sw-btn-primary" onClick={() => goTo('account')}>{t('btn.next')} →</button>
                  </div>
                </motion.div>
              )}

              {step === 'account' && (
                <motion.div key="account" className="sw-card glass"
                  initial={slideIn(dir)} animate={{ opacity: 1, x: 0 }} exit={slideOut(dir)} transition={spring}>
                  <div className="sw-card-inner">
                    <div className="sw-step-header">
                      <div className="sw-step-title">{t('sw.acct.title')}</div>
                      <div className="sw-step-sub">{t('sw.acct.sub')}</div>
                    </div>
                    {msError && <div className="sw-ms-error">{msError}</div>}
                    <div className="sw-acct-cards">
                      <button className="sw-acct-card sw-acct-ms" onClick={handleMsLogin} disabled={!isTauri}>
                        <img src={msLogo} alt="Microsoft" className="sw-acct-ms-logo" draggable={false} />
                        <div className="sw-acct-card-label">{t('sw.acct.ms.label')}</div>
                        <div className="sw-acct-card-sub">{t('sw.acct.ms.sub')}</div>
                      </button>
                      <button className="sw-acct-card sw-acct-offline" onClick={() => goTo('offline-warn')}>
                        <div className="sw-acct-person"><PersonIcon size={44} /></div>
                        <div className="sw-acct-card-label">{t('sw.acct.offline.label')}</div>
                        <div className="sw-acct-card-sub">{t('sw.acct.offline.sub')}</div>
                      </button>
                    </div>
                  </div>
                  <div className="sw-footer">
                    <button className="sw-btn-ghost" onClick={() => goTo('prefs', false)}>← {t('btn.back')}</button>
                    <button className="sw-btn-link" onClick={() => { pendingAccount.current = null; goTo('java') }}>{t('sw.acct.skip')}</button>
                  </div>
                </motion.div>
              )}

              {step === 'ms-loading' && (
                <motion.div key="ms-loading" className="sw-card glass"
                  initial={slideIn(dir)} animate={{ opacity: 1, x: 0 }} exit={slideOut(dir)} transition={spring}>
                  <div className="sw-card-inner sw-card-center">
                    <div className="sw-ms-loading-logo">
                      <img src={msLogo} alt="Microsoft" style={{ width: 56, height: 56, borderRadius: 12, objectFit: 'contain' }} />
                    </div>
                    <div className="sw-step-title" style={{ marginTop: 20 }}>Microsoft</div>
                    <div className="sw-step-sub" style={{ marginTop: 8 }}>
                      {t('acct.ms_loading')}
                    </div>
                    <div className="sw-ms-spinner-wrap">
                      <span className="ms-spinner" style={{ width: 24, height: 24, borderWidth: 3 }} />
                    </div>
                  </div>
                </motion.div>
              )}

              {step === 'offline-warn' && (
                <motion.div key="offline-warn" className="sw-card glass"
                  initial={slideIn(dir)} animate={{ opacity: 1, x: 0 }} exit={slideOut(dir)} transition={spring}>
                  <div className="sw-card-inner">
                    <div className="sw-warn-icon">⚠️</div>
                    <div className="sw-step-title" style={{ textAlign: 'center' }}>{t('sw.warn.title')}</div>
                    <div className="sw-warn-body">
                      {t('sw.warn.body').split('\n').map((line, i) => (
                        <p key={i} style={{ margin: i > 0 ? '10px 0 0' : '0' }}>{line}</p>
                      ))}
                    </div>
                  </div>
                  <div className="sw-footer sw-footer-col">
                    <button className="sw-btn-primary" onClick={() => goTo('nick')}>{t('sw.warn.skip')}</button>
                    <div className="sw-footer-row2">
                      <button className="sw-btn-ghost" onClick={() => goTo('account', false)}>← {t('btn.back')}</button>
                      <a href="https://minecraft.net" target="_blank" rel="noreferrer" className="sw-btn-link">{t('sw.warn.buy')} ↗</a>
                    </div>
                  </div>
                </motion.div>
              )}

              {step === 'nick' && (
                <motion.div key="nick" className="sw-card glass"
                  initial={slideIn(dir)} animate={{ opacity: 1, x: 0 }} exit={slideOut(dir)} transition={spring}>
                  <div className="sw-card-inner">
                    <div className="sw-step-header">
                      <div className="sw-step-title">{t('sw.nick.title')}</div>
                      <div className="sw-step-sub">{t('sw.nick.hint')}</div>
                    </div>
                    <div className="sw-nick-wrap">
                      <input
                        className={`glass-input sw-nick-input${nickError ? ' input-error' : ''}`}
                        placeholder={t('sw.nick.ph')}
                        value={nick}
                        onChange={e => { setNick(e.target.value); setNickError('') }}
                        onKeyDown={e => e.key === 'Enter' && handleNickSubmit()}
                        maxLength={20}
                        autoFocus
                      />
                      {nickError && <div className="sw-nick-error">{nickError}</div>}
                      <div className="sw-nick-chars" style={{ color: nick.length > 16 ? '#f87171' : 'var(--text-muted)' }}>
                        {nick.length}/16
                      </div>
                    </div>
                  </div>
                  <div className="sw-footer">
                    <button className="sw-btn-ghost" onClick={() => { setNick(''); setNickError(''); goTo('offline-warn', false) }}>
                      ← {t('btn.back')}
                    </button>
                    <button className="sw-btn-primary" onClick={handleNickSubmit} disabled={!nick.trim()}>
                      {t('sw.nick.add')} →
                    </button>
                  </div>
                </motion.div>
              )}

              {step === 'nick-warn' && (
                <motion.div key="nick-warn" className="sw-card glass"
                  initial={slideIn(dir)} animate={{ opacity: 1, x: 0 }} exit={slideOut(dir)} transition={spring}>
                  <div className="sw-card-inner">
                    <div className="sw-warn-icon" style={{ fontSize: 32 }}>⚠️</div>
                    <div className="sw-step-title" style={{ textAlign: 'center' }}>{t('sw.nick.warn.title')}</div>
                    <div className="sw-warn-body">
                      <p>{t('sw.nick.warn.body')}</p>
                      <div className="sw-nick-example-box">
                        <div className="sw-nick-example-chosen">
                          <span className="sw-nick-example-lbl">{t('sw.nick.warn.your_nick')}</span>
                          <span className="sw-nick-example-val">{nickForWarn}</span>
                        </div>
                        <div className="sw-nick-tips">{t('sw.nick.warn.tips')}</div>
                      </div>
                    </div>
                  </div>
                  <div className="sw-footer sw-footer-col">
                    <button className="sw-btn-primary" onClick={() => {
                      pendingAccount.current = { type: 'offline', username: nickForWarn, uuid: crypto.randomUUID() }
                      goTo('java')
                    }}>{t('sw.nick.warn.ok')}</button>
                    <button className="sw-btn-ghost" onClick={() => goTo('nick', false)}>← {t('sw.nick.warn.back')}</button>
                  </div>
                </motion.div>
              )}

            </AnimatePresence>
          </div>
        </div>

      ) : (
        <SetupJavaShowcase lang={lang} onThemeChange={setIsLbTheme}
          onStart={() => {
            localStorage.setItem('mlbv_ram',             String(ram))
            localStorage.setItem('mlbv_concurrent',      String(concurrent))
            localStorage.setItem('mlbv_close_on_launch', closeOnLaunch ? '1' : '')
          }}
          onFinish={() => {
            localStorage.setItem('mlbv_setup_done', '1')
            localStorage.setItem('mlbv_lang', lang)
            onDone(lang, pendingAccount.current)
          }} />
      )}
    </div>
  )
}

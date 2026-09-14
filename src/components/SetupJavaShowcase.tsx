import { useState, useEffect, useRef, useCallback, useMemo } from 'react'
import { motion, AnimatePresence } from 'framer-motion'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import lbLogo from '../assets/lb-logo.svg'
import { type Lang, useT } from '../i18n'
import { isTauri } from '../lib/types'

// ─── Setup wizard: final step — feature showcase while Java downloads ─────────

const spring = { type: 'spring', stiffness: 340, damping: 28 } as const
const JAVA_MAJORS = [8, 17, 21, 25] as const

type JavaStatus = 'pending' | 'already' | 'downloading' | 'installing' | 'done' | 'error'
interface JavaDl { major: number; status: JavaStatus; progress: number; message: string }

const SHOWCASE_FEATS = [
  { id: 'lb',        icon: 'lb-logo', theme: 'lb'      as const },
  { id: 'dl',        icon: '⚡',       theme: 'default' as const },
  { id: 'java',      icon: '☕',       theme: 'default' as const },
  { id: 'offline',   icon: '🌐',       theme: 'default' as const },
  { id: 'instances', icon: '📦',       theme: 'default' as const },
  { id: 'custom',    icon: '🎛',       theme: 'default' as const },
] as const


export default function SetupJavaShowcase({ lang, onStart, onFinish, onThemeChange }: {
  lang: Lang
  /// Called once on mount, before the Java scan (the wizard persists prefs here).
  onStart: () => void
  onFinish: () => void
  /// The LB slide tints the whole wizard; the parent owns the root class.
  onThemeChange: (lb: boolean) => void
}) {
  const t = useT(lang)
  const [javaDl, setJavaDl] = useState<JavaDl[]>(
    JAVA_MAJORS.map(m => ({ major: m, status: 'pending' as JavaStatus, progress: 0, message: '' }))
  )
  const [javaAllDone, setJavaAllDone] = useState(false)
  const [canFinish,   setCanFinish]   = useState(false)
  const finishTimerRef                = useRef<ReturnType<typeof setTimeout> | null>(null)
  const unlistenJavaRef               = useRef<(() => void) | null>(null)

  const [featIdx,      setFeatIdx]      = useState(0)
  const [autoProgress, setAutoProgress] = useState(0)


  useEffect(() => { onThemeChange(SHOWCASE_FEATS[featIdx].theme === 'lb') }, [featIdx, onThemeChange])

  const overallJavaProgress = useMemo(() => {
    if (!javaDl.length) return 0
    const sum = javaDl.reduce((acc, d) =>
      acc + (d.status === 'already' || d.status === 'done' ? 100
           : d.status === 'downloading' || d.status === 'installing' ? d.progress
           : 0), 0)
    return Math.round(sum / javaDl.length)
  }, [javaDl])

  // ── Auto-advance showcase every 10 s ─────────────────────────────────────
  useEffect(() => {
    const start = Date.now()
    setAutoProgress(0)
    const tickId = setInterval(() => {
      setAutoProgress(Math.min(100, ((Date.now() - start) / 10_000) * 100))
    }, 80)
    const advId = setTimeout(() => {
      setFeatIdx(i => (i + 1) % SHOWCASE_FEATS.length)
    }, 10_000)
    return () => { clearInterval(tickId); clearTimeout(advId) }
  }, [featIdx])

  // ── Java downloads ────────────────────────────────────────────────────────
  const startJavaSetup = useCallback(async () => {
    if (!isTauri) {
      setJavaDl(JAVA_MAJORS.map(m => ({ major: m, status: 'already' as JavaStatus, progress: 100, message: '' })))
      setJavaAllDone(true)
      finishTimerRef.current = setTimeout(() => setCanFinish(true), 5000)
      return
    }

    unlistenJavaRef.current?.()
    const unlisten = await listen<{ major: number; progress: number; message: string; status: JavaStatus }>(
      'java-progress',
      e => {
        const { major, progress, message, status } = e.payload
        setJavaDl(prev => {
          const next = prev.map(d => d.major === major ? { ...d, status, progress, message } : d)
          if (next.every(d => d.status === 'done' || d.status === 'already' || d.status === 'error')) {
            setJavaAllDone(true)
            finishTimerRef.current = setTimeout(() => setCanFinish(true), 5000)
          }
          return next
        })
      }
    )
    unlistenJavaRef.current = unlisten

    let installed: number[] = []
    try {
      const scanned = await invoke<{ major: number; path: string }[]>('scan_java')
      installed = scanned.map(j => j.major)
    } catch { /* ignore */ }

    // Exact major match only — Java 25 on PATH does NOT substitute for Java 8/17/21
    setJavaDl(prev =>
      prev.map(d =>
        installed.some(m => m === d.major)
          ? { ...d, status: 'already' as JavaStatus, progress: 100 }
          : d
      )
    )

    const missing = JAVA_MAJORS.filter(m => !installed.some(i => i === m))
    if (missing.length === 0) {
      setJavaAllDone(true)
      finishTimerRef.current = setTimeout(() => setCanFinish(true), 5000)
      return
    }
    for (const major of missing) {
      invoke('download_java', { major }).catch(() => {
        setJavaDl(prev => prev.map(d => d.major === major ? { ...d, status: 'error' as JavaStatus, progress: 0 } : d))
      })
    }
  }, [])

  useEffect(() => {
    onStart()
    startJavaSetup()
    return () => {
      unlistenJavaRef.current?.()
      if (finishTimerRef.current) clearTimeout(finishTimerRef.current)
    }
  }, []) // eslint-disable-line react-hooks/exhaustive-deps

  const handleFinish = onFinish

  return (
    <div className="sw-java-showcase">

      {/* Thin auto-advance timer bar at top */}
      <div className="sw-timer-bar">
        <div className="sw-timer-fill" style={{ width: `${autoProgress}%` }} />
      </div>

      {/* Feature stage — fills available space */}
      <div className="sw-showcase-stage">
        <AnimatePresence mode="wait">
          <motion.div key={featIdx} className="sw-showcase-feat"
            initial={{ opacity: 0, y: 32 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -32 }}
            transition={{ duration: 0.38, ease: [0.4, 0, 0.2, 1] }}
          >
            <div className="sw-feat-big-icon">
              {SHOWCASE_FEATS[featIdx].icon === 'lb-logo'
                ? <img src={lbLogo} alt="LiquidBounce" className="sw-feat-lb-big" draggable={false} />
                : <span className="sw-feat-emoji">{SHOWCASE_FEATS[featIdx].icon}</span>
              }
            </div>
            <div className="sw-feat-name">{t(`feat.${SHOWCASE_FEATS[featIdx].id}`)}</div>
            <div className="sw-feat-desc">{t(`feat.${SHOWCASE_FEATS[featIdx].id}_d`)}</div>
          </motion.div>
        </AnimatePresence>
      </div>

      {/* Bottom — fixed min-height prevents layout shift when button appears */}
      <div className="sw-showcase-bottom">

        {/* Navigation dots */}
        <div className="sw-feat-nav-dots">
          {SHOWCASE_FEATS.map((_, i) => (
            <button key={i}
              className={`sw-feat-nav-dot${i === featIdx ? ' active' : ''}`}
              onClick={() => setFeatIdx(i)}
            />
          ))}
        </div>

        {/* Java overall progress strip */}
        <div className="sw-java-dl-strip">
          <div className="sw-java-dl-label">
            {javaAllDone
              ? t('sw.java.all_ready')
              : t('sw.java.dl_progress').replace('{0}', String(overallJavaProgress))
            }
          </div>
          <div className="sw-java-dl-track">
            <motion.div className="sw-java-dl-fill"
              animate={{ width: `${overallJavaProgress}%` }}
              transition={{ duration: 0.4, ease: 'easeOut' }}
            />
          </div>
        </div>

        {/* Launch button slot — min-height pre-reserves space */}
        <div className="sw-launch-wrap">
          <AnimatePresence mode="wait">
            {canFinish ? (
              <motion.button key="btn-launch"
                className="sw-btn-primary sw-btn-launch"
                onClick={handleFinish}
                initial={{ opacity: 0, scale: 0.92, y: 8 }}
                animate={{ opacity: 1, scale: 1, y: 0 }}
                exit={{ opacity: 0 }}
                transition={spring}
              >
                {t('sw.java.finish')} →
              </motion.button>
            ) : (
              <motion.div key="lbl-wait" className="sw-java-wait"
                initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }}
              >
                <span className="ms-spinner" style={{ width: 12, height: 12, borderWidth: 2 }} />
                <span>{t('sw.java.wait')}</span>
              </motion.div>
            )}
          </AnimatePresence>
        </div>

      </div>
    </div>

  )
}

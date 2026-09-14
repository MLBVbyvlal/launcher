import { useState } from 'react'
import { motion } from 'framer-motion'
import lbBadgePng from '../assets/lb-badge-logo.png'
import { getLang, useT } from '../i18n'
import { spring, type MCVersion } from '../lib/types'

// ─── Small shared widgets ─────────────────────────────────────────────────────

export function LbBadge({ size = 20 }: { size?: number }) {
  return (
    <img src={lbBadgePng} alt="LB" className="lb-badge-img" draggable={false}
      style={{ width: size, height: size }} />
  )
}
// ─── Tooltip ─────────────────────────────────────────────────────────────────

export function Tip({ text }: { text: string }) {
  const [pos, setPos] = useState<'right' | 'left' | 'top'>('right')
  return (
    <span className={`tip-wrap tip-pos-${pos}`}
      onMouseEnter={e => {
        const r = (e.currentTarget as HTMLElement).getBoundingClientRect()
        const spaceRight = window.innerWidth - r.right
        if (spaceRight < 240 && r.left > 240) setPos('left')
        else if (spaceRight < 240) setPos('top')
        else setPos('right')
      }}
    >
      <svg className="tip-icon" viewBox="0 0 16 16" fill="none">
        <circle cx="8" cy="8" r="6.5" stroke="currentColor" strokeWidth="1.1"/>
        <path d="M8 7.5v3" stroke="currentColor" strokeWidth="1.3" strokeLinecap="round"/>
        <circle cx="8" cy="5.5" r="0.6" fill="currentColor"/>
      </svg>
      <span className="tip-text">{text}</span>
    </span>
  )
}

export function verTag(type?: MCVersion['type']): string {
  switch (type) {
    case 'release':  return 'R'
    case 'snapshot': return 'S'
    case 'old_beta': return 'B'
    default:         return 'A'
  }
}

// ─── Splash ───────────────────────────────────────────────────────────────────

export function LoadingScreen({ status, progress, onRetry }: {
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

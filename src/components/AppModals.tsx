import { motion } from 'framer-motion'
import { getLang, useT } from '../i18n'
import { spring } from '../lib/types'

// ─── Add-account modal ────────────────────────────────────────────────────────

export function AddAccountModal(p: {
  username: string; onUsername: (v: string) => void; onAddOffline: () => void
  msLoading: boolean; msError: string; onMsLogin: () => void; onClose: () => void
}) {
  const t = useT(getLang())
  const { username, msLoading, msError } = p
  const setUsername = p.onUsername
  const addOffline = p.onAddOffline
  const handleMsLogin = p.onMsLogin
  const close = p.onClose
  return (
      <motion.div className="overlay" initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }}
      onClick={close}
    >
      <motion.div className="modal glass modal-sm"
        initial={{ opacity: 0, scale: 0.9, y: 24 }} animate={{ opacity: 1, scale: 1, y: 0 }}
        exit={{ opacity: 0, scale: 0.9, y: 16 }} transition={spring}
        onClick={e => e.stopPropagation()}
      >
        <div className="modal-head">
          <span className="modal-title">{t('acct.title')}</span>
          <button className="modal-close" onClick={close}>×</button>
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
  )
}

// ─── Stop-game confirmation ───────────────────────────────────────────────────

export function StopGameModal(p: { countdown: number; onConfirm: () => void; onClose: () => void }) {
  const t = useT(getLang())
  const stopCd = p.countdown
  const confirmStop = p.onConfirm
  return (
      <motion.div className="overlay" initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }}>
      <motion.div className="modal glass"
        initial={{ opacity: 0, scale: 0.9, y: 20 }} animate={{ opacity: 1, scale: 1, y: 0 }}
        exit={{ opacity: 0, scale: 0.9, y: 16 }} transition={spring}
        onClick={e => e.stopPropagation()}
        style={{ maxWidth: 320, padding: '24px 28px', textAlign: 'center' }}
      >
        <div style={{ fontSize: 28, marginBottom: 8 }}>⚠️</div>
        <div style={{ fontSize: 15, fontWeight: 700, color: 'var(--text-primary)', marginBottom: 8 }}>{t('stop.warn.title')}</div>
        <div style={{ fontSize: 12, color: 'var(--text-muted)', marginBottom: 20, lineHeight: 1.5 }}>
          {t('stop.warn.body').split('\n').map((l, i) => <span key={i}>{i > 0 && <br/>}{l}</span>)}
        </div>
        <div style={{ display: 'flex', gap: 10 }}>
          <button
            style={{
              flex: 1, padding: '9px 0', borderRadius: 9, border: 'none', cursor: stopCd > 0 ? 'not-allowed' : 'pointer',
              background: stopCd > 0 ? 'rgba(248,113,113,0.3)' : '#f87171',
              color: '#fff', fontWeight: 700, fontSize: 13, opacity: stopCd > 0 ? 0.7 : 1,
              transition: 'all 0.2s',
            }}
            onClick={confirmStop}
            disabled={stopCd > 0}
          >
            {stopCd > 0 ? t('stop.warn.yes_cd').replace('{0}', String(stopCd)) : t('stop.warn.yes')}
          </button>
          <button
            style={{
              flex: 1, padding: '9px 0', borderRadius: 9, border: '1.5px solid var(--accent)',
              cursor: 'pointer', background: 'transparent', color: 'var(--accent)', fontWeight: 700, fontSize: 13,
            }}
            onClick={p.onClose}
          >{t('stop.warn.no')}</button>
        </div>
      </motion.div>
    </motion.div>
  )
}

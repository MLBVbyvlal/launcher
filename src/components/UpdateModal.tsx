import { useState } from 'react'
import { motion } from 'framer-motion'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { getLang, useT } from '../i18n'
import { isTauri, spring, type UpdateInfo } from '../lib/types'

// ─── Update Modal ─────────────────────────────────────────────────────────────

export default function UpdateModal({ info, onClose }: { info: UpdateInfo; onClose: () => void }) {
  const t = useT(getLang())
  const [phase, setPhase] = useState<'ask' | 'downloading' | 'installing'>('ask')
  const [percent, setPercent] = useState(0)
  const [dlError, setDlError] = useState('')
  const [format, setFormat] = useState<'exe' | 'msi'>('exe')
  // Old hand-assembled releases may carry only one installer — offer what exists.
  const hasExe = !!info.assetUrl
  const hasMsi = !!info.msiUrl
  const showChoice = hasExe && hasMsi
  const effectiveFormat = showChoice ? format : hasExe ? 'exe' : 'msi'

  const handleDownload = async () => {
    const url = effectiveFormat === 'msi' ? info.msiUrl : info.assetUrl
    const sha256 = effectiveFormat === 'msi' ? info.msiSha256 : info.assetSha256
    if (!isTauri || !url) {
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
      await invoke('download_update', { url, sha256 })
      unlisten()
      setPhase('installing')
      await new Promise(r => setTimeout(r, 700))
      await invoke('apply_update', { newVersion: info.version, kind: effectiveFormat })
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

        {phase === 'ask' && info.unstableWarning && (
          <div style={{
            background: 'rgba(251,191,36,0.1)', border: '1px solid rgba(251,191,36,0.3)',
            borderRadius: 8, padding: '10px 14px', marginBottom: 12, fontSize: 12,
            color: 'rgba(255,255,255,0.8)', lineHeight: 1.55
          }}>
            <strong style={{ color: '#fbbf24' }}>{t('update.unstable_warn')}</strong>{' '}
            {t('update.unstable_body')}
          </div>
        )}
        {phase === 'ask' && <>
          {info.body
            ? <pre className="update-changelog">{info.body.trim()}</pre>
            : <p className="update-nobody">{t('update.no_notes')}</p>
          }
          {dlError && <div className="inst-error" style={{ margin: '0 18px 10px' }}>{dlError}</div>}
          {showChoice && (
            <div style={{ padding: '0 18px 12px', display: 'flex', flexDirection: 'column', gap: 8 }}>
              <div className="field-label">{t('update.choose_format')}</div>
              <label className={`reinstall-opt${format === 'exe' ? ' active' : ''}`} onClick={() => setFormat('exe')}>
                <span className="reinstall-radio" />
                <div>
                  <div className="reinstall-opt-title">{t('update.format_exe')}</div>
                  <div className="reinstall-opt-sub">{t('update.format_exe_desc')}</div>
                </div>
              </label>
              <label className={`reinstall-opt${format === 'msi' ? ' active' : ''}`} onClick={() => setFormat('msi')}>
                <span className="reinstall-radio" />
                <div>
                  <div className="reinstall-opt-title">{t('update.format_msi')}</div>
                  <div className="reinstall-opt-sub">{t('update.format_msi_desc')}</div>
                </div>
              </label>
            </div>
          )}
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

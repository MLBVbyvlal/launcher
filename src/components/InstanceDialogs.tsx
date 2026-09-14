import { useState } from 'react'
import { motion } from 'framer-motion'
import { invoke } from '@tauri-apps/api/core'
import { getLang, useT } from '../i18n'
import { isTauri, spring, type Instance } from '../lib/types'

// ─── Crash Dialog ────────────────────────────────────────────────────────────

export type CrashInfo = { exitCode: number; log: string; logPath: string; instanceName?: string }

export function CrashDialog({ info, onClose }: { info: CrashInfo; onClose: () => void }) {
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

export type CtxTarget = { inst: Instance; x: number; y: number }
export type CtxAction = 'rename' | 'settings' | 'reinstall' | 'delete' | 'open_folder'

export function InstanceCtxMenu({ target, isLb, onAction, onClose }: {
  target: CtxTarget; isLb: boolean
  onAction: (a: CtxAction, inst: Instance) => void
  onClose: () => void
}) {
  const t = useT(getLang())
  const W = 176, H = 228
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
        <button className="ctx-item" onClick={() => { onAction('open_folder', target.inst); onClose() }}>
          <svg viewBox="0 0 20 20" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round"><path d="M2 6.5A1.5 1.5 0 0 1 3.5 5h4l2 2h7A1.5 1.5 0 0 1 18 8.5v7A1.5 1.5 0 0 1 16.5 17h-13A1.5 1.5 0 0 1 2 15.5v-9z"/></svg>
          Открыть папку
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

export function ReinstallModal({ inst, isLb, onClose }: { inst: Instance; isLb: boolean; onClose: () => void }) {
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

// ─── Delete Instance Modal ────────────────────────────────────────────────────

export function DeleteInstanceModal({ inst, onRemoveList, onDeleteDisk, onClose }: {
  inst: Instance
  onRemoveList: () => void
  onDeleteDisk: () => void
  onClose: () => void
}) {
  const t = useT(getLang())
  const [mode, setMode] = useState<'list' | 'disk'>('list')

  return (
    <motion.div className="overlay" initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} onClick={onClose}>
      <motion.div className="modal glass delete-inst-modal"
        initial={{ opacity: 0, scale: 0.9, y: 20 }} animate={{ opacity: 1, scale: 1, y: 0 }}
        exit={{ opacity: 0, scale: 0.9, y: 16 }} transition={spring}
        onClick={e => e.stopPropagation()}
      >
        <div className="modal-head">
          <span className="modal-title">{t('delete_inst.title')} — {inst.name}</span>
          <button className="modal-close" onClick={onClose}>×</button>
        </div>
        <div className="delete-inst-body">
          <label className={`reinstall-opt${mode === 'list' ? ' active' : ''}`} onClick={() => setMode('list')}>
            <span className="reinstall-radio" />
            <div>
              <div className="reinstall-opt-title">{t('delete_inst.from_list')}</div>
              <div className="reinstall-opt-sub">{t('delete_inst.from_list_desc')}</div>
            </div>
          </label>
          <label className={`reinstall-opt${mode === 'disk' ? ' active danger-opt' : ''}`} onClick={() => setMode('disk')}>
            <span className="reinstall-radio" />
            <div>
              <div className="reinstall-opt-title">{t('delete_inst.from_disk')}</div>
              <div className="reinstall-opt-sub">{t('delete_inst.from_disk_desc')}</div>
            </div>
          </label>
        </div>
        <div className="inst-modal-footer">
          <button className="btn-cancel" onClick={onClose}>{t('settings.cancel')}</button>
          <button
            className={`btn-ok${mode === 'disk' ? ' btn-delete-confirm enabled' : ''}`}
            onClick={() => { if (mode === 'disk') onDeleteDisk(); else onRemoveList(); onClose() }}
          >
            {t('delete_inst.confirm')}
          </button>
        </div>
      </motion.div>
    </motion.div>
  )
}

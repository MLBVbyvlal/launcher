import { motion, AnimatePresence } from 'framer-motion'
import { getLang, useT } from '../i18n'
import { spring, type Account, type Instance, type Tab } from '../lib/types'
import type { LaunchJob } from '../lib/useLaunchQueue'
import { LbBadge } from './ui'
import { ProgressRing } from './LaunchCard'

// ─── Sidebar: account + instance lists ────────────────────────────────────────

export type SidebarProps = {
  collapsed: boolean; onToggleCollapsed: () => void
  activeTab: Tab
  accounts: Account[]; selected: Account | null; onSelectAccount: (a: Account) => void
  skinError: boolean; onSkinError: () => void
  onAddAccount: () => void; onOpenSettings: () => void
  activeInstance: Instance | null; otherInstances: Instance[]
  onSelectInstance: (inst: Instance) => void; onCreateInstance: () => void
  onRemoveInstance: (id: string) => void
  onCtxMenu: (e: React.MouseEvent, inst: Instance) => void
  renamingId: string | null; renameText: string
  onRenameText: (v: string) => void; onRenameCommit: (id: string, v: string) => void; onRenameEnd: () => void
  jobs: Record<string, LaunchJob>; running: string[]
}

export default function Sidebar(p: SidebarProps) {
  const t = useT(getLang())
  const {
    collapsed: sidebarCollapsed, activeTab, accounts, selected, skinError,
    activeInstance, otherInstances, renamingId, renameText, jobs, running,
  } = p
  const setSelected = p.onSelectAccount
  const setSkinError = () => p.onSkinError()
  const setShowSettings = () => p.onOpenSettings()
  const setShowAddAcct = () => p.onAddAccount()
  const setShowCreateInst = () => p.onCreateInstance()
  const setSidebarCollapsed = () => p.onToggleCollapsed()
  const handleCtxMenu = p.onCtxMenu
  const setRenameText = p.onRenameText
  const renameInstance = p.onRenameCommit
  const setRenamingId = () => p.onRenameEnd()
  const removeInstance = p.onRemoveInstance
  const launchQ = { jobs }
  return (
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
                       onError={setSkinError} />
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
                      onClick={e => { e.stopPropagation(); setShowSettings() }}
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
          onClick={setShowAddAcct}
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
              {launchQ.jobs[activeInstance.name]
                ? <ProgressRing size={24} pct={launchQ.jobs[activeInstance.name].progress}
                    queued={launchQ.jobs[activeInstance.name].phase === 'queued'}
                    error={launchQ.jobs[activeInstance.name].phase === 'error'} />
                : activeInstance.type === 'lb'
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
                          onBlur={() => { renameInstance(activeInstance.id, renameText); setRenamingId() }}
                          onKeyDown={e => {
                            if (e.key === 'Enter') { renameInstance(activeInstance.id, renameText); setRenamingId() }
                            if (e.key === 'Escape') setRenamingId()
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
              onClick={() => p.onSelectInstance(inst)}
              onContextMenu={e => handleCtxMenu(e, inst)}
              whileHover={{ x: 3 }} transition={spring}
            >
              {launchQ.jobs[inst.name]
                ? <ProgressRing size={18} pct={launchQ.jobs[inst.name].progress}
                    queued={launchQ.jobs[inst.name].phase === 'queued'}
                    error={launchQ.jobs[inst.name].phase === 'error'} />
                : inst.type === 'lb'
                ? <LbBadge size={18} />
                : <span className="vbadge release">MC</span>
              }
              {renamingId === inst.id
                ? <input autoFocus className="inst-rename-input"
                    value={renameText} onChange={e => setRenameText(e.target.value)}
                    onBlur={() => { renameInstance(inst.id, renameText); setRenamingId() }}
                    onKeyDown={e => {
                      if (e.key === 'Enter') { renameInstance(inst.id, renameText); setRenamingId() }
                      if (e.key === 'Escape') setRenamingId()
                    }}
                    onClick={e => e.stopPropagation()}
                  />
                : <span className="ver-mini-id">{inst.name}</span>
              }
              {!launchQ.jobs[inst.name] && !running.includes(inst.name) && (
                <button className="inst-del" onClick={e => { e.stopPropagation(); removeInstance(inst.id) }}>×</button>
              )}
            </motion.div>
          ))}
        </AnimatePresence>

        {/* Add instance button */}
        <AnimatePresence>
          {!sidebarCollapsed && (
            <motion.button className="btn-browse-ver"
              initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }}
              onClick={setShowCreateInst}
            >{t('new_instance')}</motion.button>
          )}
        </AnimatePresence>
        {sidebarCollapsed && (
          <motion.button className="btn-add-acct icon-only" onClick={setShowCreateInst}
            whileHover={{ scale: 1.05 }} whileTap={{ scale: 0.95 }}
          >+</motion.button>
        )}
      </div>

      {/* Bottom bar */}
      <div className="s-bottom">
        {sidebarCollapsed && (
          <motion.button className="btn-icon-sm" title={t('settings.title')}
            onClick={setShowSettings}
            whileHover={{ scale: 1.08 }} whileTap={{ scale: 0.93 }}
          >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
              <path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"/><circle cx="12" cy="12" r="3"/>
            </svg>
          </motion.button>
        )}
        <motion.button className="btn-icon-sm collapse-toggle"
          title={sidebarCollapsed ? t('sidebar.expand') : t('sidebar.collapse')}
          onClick={setSidebarCollapsed}
          whileHover={{ scale: 1.08 }} whileTap={{ scale: 0.93 }}
          animate={{ rotate: sidebarCollapsed ? 180 : 0 }} transition={spring}
        >
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
            <path d="M15 6l-6 6 6 6"/>
          </svg>
        </motion.button>
      </div>
    </motion.div>
  )
}

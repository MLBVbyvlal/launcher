import { useState, useEffect, useCallback, useRef } from 'react'
import { motion, AnimatePresence } from 'framer-motion'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import SetupWizard from './SetupWizard'
import LbConfigsPanel from './LbConfigsPanel'
import MigrationScreen from './MigrationScreen'
import { getLang, type Lang, useT } from './i18n'
import { isTauri, spring, type Account, type Instance, type Tab } from './lib/types'
import { useLaunchQueue } from './lib/useLaunchQueue'
import { useInstances } from './lib/useInstances'
import { useAccounts } from './lib/useAccounts'
import { useUpdateCheck } from './lib/useUpdateCheck'
import { useBoot } from './lib/useBoot'
import Sidebar from './components/Sidebar'
import MainArea from './components/MainArea'
import { AddAccountModal, RemoveAccountModal, StopGameModal } from './components/AppModals'
import { applyAccent } from './lib/accent'
import { LoadingScreen } from './components/ui'
import SettingsModal from './components/SettingsModal'
import CreateInstanceModal from './components/CreateInstanceModal'
import InstanceSettingsModal from './components/InstanceSettingsModal'
import UpdateModal from './components/UpdateModal'
import { CrashDialog, InstanceCtxMenu, ReinstallModal, DeleteInstanceModal, type CrashInfo, type CtxTarget, type CtxAction } from './components/InstanceDialogs'
import './App.css'

export type { MCVersion, Instance } from './lib/types'

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

  // App state — manifest fetch, migration gate
  const { appState, setAppState, migrateScan, migratePins, loadStatus, loadProgress, versions, fetchVersions } = useBoot()

  // Accounts — the frontend copy is {type, username, uuid}; tokens stay in the
  // Rust vault. See src/lib/useAccounts.ts for what that implies.
  const {
    accounts, selected, setSelected, adopt,
    msLoading, msError, setMsError,
    needsRelogin, addOffline, loginMicrosoft, removeAccount,
  } = useAccounts()
  const [username, setUsername]       = useState('')
  const [showAddAcct, setShowAddAcct] = useState(false)
  // Account whose removal is being confirmed (it also deletes the vault entry),
  // the busy flag, and the failure text the confirm dialog shows.
  const [removeAcctOf, setRemoveAcctOf] = useState<Account | null>(null)
  const [removeBusy, setRemoveBusy]     = useState(false)
  const [removeError, setRemoveError]   = useState('')


  // (LiquidBounce versions are loaded inside CreateInstanceModal per-branch)

  const {
    instances, activeMcInstId, activeLbInstId, setActiveMcInstId, setActiveLbInstId,
    addInstance, removeInstance, renameInstance, handleDeleteDisk, applyMigrationPins,
  } = useInstances(appState)

  useEffect(() => {
    setSkinError(false) // reset skin when account changes
  }, [selected])

  // UI
  const [showCreateInst, setShowCreateInst] = useState(false)
  const [activeTab, setActiveTab]             = useState<Tab>('mc')
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false)
  const [showSettings, setShowSettings]       = useState(false)
  // Instances with a live game process. Several may run at the same time.
  const [running, setRunning]                 = useState<string[]>([])
  // Console tab overlays whichever mc/lb tab is underneath; the game keeps running.
  const [consoleOpen, setConsoleOpen]         = useState(false)
  const [consoleInst, setConsoleInst]         = useState<string | null>(null)
  const [stopWarn, setStopWarn]               = useState(false)
  const [stopCd, setStopCd]                   = useState(5)
  const stopCdRef = useRef<ReturnType<typeof setInterval> | null>(null)
  const [skinError, setSkinError]             = useState(false)

  const [showLbConfigs, setShowLbConfigs]     = useState(false)

  // Update check
  const { updateInfo, setUpdateInfo, updateCheckError, justUpdated } = useUpdateCheck(appState)

  // Crash dialog
  const [crashInfo, setCrashInfo]             = useState<CrashInfo | null>(null)

  // Launch queue — one job per instance, progress attributed by name.
  const launchQ = useLaunchQueue({
    onLaunched: name => setRunning(prev => prev.includes(name) ? prev : [...prev, name]),
    onConsole:  name => { setConsoleInst(name); setConsoleOpen(true) },
  })

  // Context menu
  const [ctxMenu, setCtxMenu]                 = useState<CtxTarget | null>(null)

  // Rename
  const [renamingId, setRenamingId]           = useState<string | null>(null)
  const [renameText, setRenameText]           = useState('')

  // Instance modals
  const [instSettingsOf, setInstSettingsOf]   = useState<Instance | null>(null)
  const [reinstallOf, setReinstallOf]         = useState<Instance | null>(null)
  const [deleteOf, setDeleteOf]               = useState<Instance | null>(null)

  // ── Computed ─────────────────────────────────────────────────────────────
  const mcInstances = instances.filter(i => i.type === 'mc')
  const lbInstances = instances.filter(i => i.type === 'lb')
  const activeMcInst = mcInstances.find(i => i.id === activeMcInstId) ?? mcInstances[0] ?? null
  const activeLbInst = lbInstances.find(i => i.id === activeLbInstId) ?? lbInstances[0] ?? null
  const activeInstance = activeTab === 'mc' ? activeMcInst : activeLbInst
  // The button belongs to the selected instance: it stops that one only.
  const runningActive = !!activeInstance && running.includes(activeInstance.name)
  const runningOther  = running.find(name => name !== activeInstance?.name)
  const tabInstances = activeTab === 'mc' ? mcInstances : lbInstances
  const otherInstances = tabInstances.filter(i => i.id !== activeInstance?.id)

  // ── Accounts ─────────────────────────────────────────────────────────────
  // State and the vault handshake live in useAccounts; these wrappers only add
  // what the modal owns — closing it and clearing the draft name.
  const submitOffline = () => {
    if (!username.trim()) return
    addOffline(username)
    setUsername(''); setShowAddAcct(false)
  }

  const submitMsLogin = async () => {
    // Only close on success: the error text stays in the modal so the reason is
    // readable instead of vanishing behind a closed overlay.
    if (await loginMicrosoft()) return
    setShowAddAcct(false)
  }

  const confirmRemoveAccount = async () => {
    if (!removeAcctOf) return
    setRemoveBusy(true)
    const err = await removeAccount(removeAcctOf)
    setRemoveBusy(false)
    // A failed vault delete keeps the dialog open with the reason: the account
    // is still listed, so closing the modal would hide an unreferenced token.
    if (err) { setRemoveError(err); return }
    setRemoveAcctOf(null); setRemoveError('')
  }

  const handleCtxAction = (action: CtxAction, inst: Instance) => {
    if (action === 'delete')      { setDeleteOf(inst) }
    if (action === 'rename')      { setRenamingId(inst.id); setRenameText(inst.name) }
    if (action === 'settings')    { setInstSettingsOf(inst) }
    if (action === 'reinstall')   { setReinstallOf(inst) }
    if (action === 'open_folder') { if (isTauri) invoke('open_game_dir', { instanceName: inst.name }).catch(() => {}) }
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
    listen<{ instance: string; running: boolean }>('game-running', e => {
      const { instance, running: isRunning } = e.payload
      setRunning(prev => isRunning
        ? (prev.includes(instance) ? prev : [...prev, instance])
        : prev.filter(n => n !== instance))
      if (!isRunning) getCurrentWindow().show().catch(() => {})
    }).then(fn => { unlistenRunning = fn })
    listen<CrashInfo & { instance?: string }>('game-crashed', e => {
      setCrashInfo({ ...e.payload, instanceName: e.payload.instance })
    }).then(fn => { unlistenCrash = fn })
    return () => { unlistenRunning?.(); unlistenCrash?.() }
  }, [])

  // Block native browser context menu everywhere
  useEffect(() => {
    const block = (e: MouseEvent) => e.preventDefault()
    document.addEventListener('contextmenu', block)
    return () => document.removeEventListener('contextmenu', block)
  }, [])

  // ── Stop running game ─────────────────────────────────────────────────────
  const handleStop = () => {
    setStopWarn(true)
    setStopCd(5)
    if (stopCdRef.current) clearInterval(stopCdRef.current)
    stopCdRef.current = setInterval(() => {
      setStopCd(prev => {
        if (prev <= 1) { clearInterval(stopCdRef.current!); stopCdRef.current = null; return 0 }
        return prev - 1
      })
    }, 1000)
  }
  const confirmStop = async () => {
    if (stopCdRef.current) { clearInterval(stopCdRef.current); stopCdRef.current = null }
    setStopWarn(false)
    if (!isTauri || !activeInstance) return
    try { await invoke('stop_game', { instanceName: activeInstance.name }) } catch { /* process already gone */ }
  }

  // ── Launch ────────────────────────────────────────────────────────────────
  // Press → the instance joins the queue (starts at once when nothing else
  // is launching). Tokens for Microsoft accounts are resolved in Rust.
  const handlePlay = () => {
    if (!selected || !activeInstance) return
    // A Microsoft account with no vault entry cannot authenticate. Take the
    // user to sign-in here rather than failing after the whole download — the
    // launch path would refuse with the same message (lib.rs, `launch_game`),
    // only minutes later.
    if (needsRelogin.includes(selected.uuid)) { setMsError(''); setShowAddAcct(true); return }
    if (running.includes(activeInstance.name)) return
    if (launchQ.jobs[activeInstance.name]) return
    launchQ.enqueue(activeInstance, selected)
  }

  // ── Setup wizard completion ───────────────────────────────────────────────
  const handleSetupDone = useCallback((newLang: Lang, account: Account | null) => {
    setLang(newLang)
    if (account) adopt(account)
    setSetupDone(true)
  }, [adopt])

  const winControls = {
    minimize: () => { if (isTauri) getCurrentWindow().minimize() },
    maximize: () => { if (isTauri) getCurrentWindow().toggleMaximize() },
    close:    () => { if (isTauri) getCurrentWindow().close() },
  }


  // ─────────────────────────────────────────────────────────────────────────

  return (
    <div className={`app${activeTab === 'lb' ? ' lb-active' : ''}`}>
      <AnimatePresence>
        {(appState === 'loading' || appState === 'error') && (
          <LoadingScreen key="splash" status={loadStatus} progress={loadProgress}
            onRetry={appState === 'error' ? fetchVersions : undefined} />
        )}
      </AnimatePresence>

      {appState === 'migrate' && migrateScan && (
        <MigrationScreen key="migrate" pins={migratePins} scan={migrateScan}
          versions={versions} instances={instances}
          onApplyPins={applyMigrationPins} onDone={() => setAppState('ready')} />
      )}

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

          <Sidebar
            collapsed={sidebarCollapsed} onToggleCollapsed={() => setSidebarCollapsed(c => !c)}
            activeTab={activeTab}
            accounts={accounts} selected={selected} onSelectAccount={setSelected}
            needsRelogin={needsRelogin} onRemoveAccount={acct => { setRemoveError(''); setRemoveAcctOf(acct) }}
            skinError={skinError} onSkinError={() => setSkinError(true)}
            onAddAccount={() => setShowAddAcct(true)} onOpenSettings={() => setShowSettings(true)}
            activeInstance={activeInstance} otherInstances={otherInstances}
            onSelectInstance={inst => inst.type === 'mc' ? setActiveMcInstId(inst.id) : setActiveLbInstId(inst.id)}
            onCreateInstance={() => setShowCreateInst(true)} onRemoveInstance={removeInstance}
            onCtxMenu={handleCtxMenu}
            renamingId={renamingId} renameText={renameText}
            onRenameText={setRenameText} onRenameCommit={renameInstance} onRenameEnd={() => setRenamingId(null)}
            jobs={launchQ.jobs} running={running}
          />

          <MainArea
            activeTab={activeTab} onTab={setActiveTab}
            consoleOpen={consoleOpen} onConsoleOpen={setConsoleOpen}
            consoleInst={consoleInst} onConsoleInst={setConsoleInst}
            running={running} runningActive={runningActive} runningOther={runningOther}
            selected={selected} activeInstance={activeInstance}
            launchQ={launchQ}
            onPlay={handlePlay} onStop={handleStop} onLbConfigs={() => setShowLbConfigs(true)}
          />
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
            <AddAccountModal username={username} onUsername={setUsername} onAddOffline={submitOffline}
              msLoading={msLoading} msError={msError} onMsLogin={submitMsLogin}
              onClose={() => { setShowAddAcct(false); setMsError('') }} />
          )}
        </AnimatePresence>

        {/* ── REMOVE ACCOUNT MODAL ── */}
        <AnimatePresence>
          {removeAcctOf && (
            <RemoveAccountModal acct={removeAcctOf} busy={removeBusy} error={removeError}
              onConfirm={confirmRemoveAccount}
              onClose={() => { setRemoveAcctOf(null); setRemoveError('') }} />
          )}
        </AnimatePresence>

        {/* ── SETTINGS MODAL ── */}
        <AnimatePresence>
          {showSettings && <SettingsModal onClose={() => setShowSettings(false)} onLangChange={l => setLang(l)} updateCheckError={updateCheckError} />}
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

        {/* ── DELETE INSTANCE ── */}
        <AnimatePresence>
          {deleteOf && (
            <DeleteInstanceModal
              inst={deleteOf}
              onRemoveList={() => removeInstance(deleteOf.id)}
              onDeleteDisk={() => handleDeleteDisk(deleteOf)}
              onClose={() => setDeleteOf(null)}
            />
          )}
        </AnimatePresence>

        {/* ── STOP GAME WARNING ── */}
        <AnimatePresence>
          {stopWarn && (
            <StopGameModal countdown={stopCd} onConfirm={confirmStop}
              onClose={() => {
                if (stopCdRef.current) { clearInterval(stopCdRef.current); stopCdRef.current = null }
                setStopWarn(false)
              }} />
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

      {/* ── LB CONFIGS CATALOG ── */}
      <AnimatePresence>
        {showLbConfigs && (
          <LbConfigsPanel
            onClose={() => setShowLbConfigs(false)}
            lbInstances={lbInstances.map(i => ({ name: i.name, version: i.version }))}
          />
        )}
      </AnimatePresence>

      {/* ── SETUP WIZARD (overlays everything except migration) ── */}
      <AnimatePresence>
        {!setupDone && appState === 'ready' && (
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

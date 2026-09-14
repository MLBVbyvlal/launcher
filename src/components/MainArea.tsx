import { useRef } from 'react'
import { motion, AnimatePresence } from 'framer-motion'
import lbLogo from '../assets/lb-logo.svg'
import ConsolePanel from '../ConsolePanel'
import { getLang, useT } from '../i18n'
import { spring, type Account, type Instance, type Tab } from '../lib/types'
import type { useLaunchQueue } from '../lib/useLaunchQueue'
import { LaunchCard, QueueStrip } from './LaunchCard'

// ─── Main area: tab switcher, hero, launch zone, console ─────────────────────

export type MainAreaProps = {
  activeTab: Tab; onTab: (t: Tab) => void
  consoleOpen: boolean; onConsoleOpen: (open: boolean) => void
  consoleInst: string | null; onConsoleInst: (name: string | null) => void
  running: string[]; runningActive: boolean; runningOther: string | undefined
  selected: Account | null; activeInstance: Instance | null
  launchQ: ReturnType<typeof useLaunchQueue>
  onPlay: () => void; onStop: () => void; onLbConfigs: () => void
}

export default function MainArea(p: MainAreaProps) {
  const t = useT(getLang())
  const { activeTab, consoleOpen, consoleInst, running, runningActive, runningOther, selected, activeInstance, launchQ } = p
  const setActiveTab = p.onTab
  const setConsoleOpen = p.onConsoleOpen
  const setConsoleInst = p.onConsoleInst
  const handlePlay = p.onPlay
  const handleStop = p.onStop
  const setShowLbConfigs = () => p.onLbConfigs()
  // Horizontal swipe on the hero switches mc ↔ lb.
  const swipeStartX = useRef<number | null>(null)
  const onHeroPointerDown = (e: React.PointerEvent<HTMLDivElement>) => {
    if (consoleOpen) return
    if ((e.target as HTMLElement).closest('button, input, a, [role="button"]')) return
    e.currentTarget.setPointerCapture(e.pointerId)
    swipeStartX.current = e.clientX
  }
  const onHeroPointerUp = (e: React.PointerEvent<HTMLDivElement>) => {
    if (swipeStartX.current === null) return
    e.currentTarget.releasePointerCapture(e.pointerId)
    if (consoleOpen) { swipeStartX.current = null; return }
    const delta = e.clientX - swipeStartX.current
    swipeStartX.current = null
    if (Math.abs(delta) < 60) return
    if (delta < 0 && activeTab === 'mc') setActiveTab('lb')
    if (delta > 0 && activeTab === 'lb') setActiveTab('mc')
  }
  const onHeroPointerCancel = () => { swipeStartX.current = null }
  const activeJob = activeInstance ? launchQ.jobs[activeInstance.name] ?? null : null
  const tabQueued = launchQ.queued.filter(j => j.instance.type === (activeTab === 'lb' ? 'lb' : 'mc'))
  const anyLaunching = launchQ.active !== null
  return (
    <div className="main" onPointerDown={onHeroPointerDown} onPointerUp={onHeroPointerUp} onPointerCancel={onHeroPointerCancel}>
      <div className="tab-switcher">
        {(['mc', 'lb'] as Tab[]).map(tab => {
          const isActive      = activeTab === tab
          const tabJob        = launchQ.active && launchQ.active.instance.type === (tab === 'lb' ? 'lb' : 'mc') ? launchQ.active : null
          const isOtherBusy   = !!tabJob && !isActive
          const progress      = tabJob?.progress ?? 0
          const label         = tab === 'mc' ? 'Minecraft' : 'LiquidBounce'
          return (
            <button key={tab}
              className={[
                'tab-pill',
                isActive    ? `tab-pill-active tab-pill-${tab}` : '',
                isOtherBusy ? `tab-pill-loading-${tab}` : '',
              ].filter(Boolean).join(' ')}
              onClick={() => { setActiveTab(tab); setConsoleOpen(false) }}
            >
              {isOtherBusy && <div className="tab-pill-fill" style={{ width: `${progress}%` }} />}
              <AnimatePresence mode="wait" initial={false}>
                <motion.span key={isOtherBusy ? 'progress' : label}
                  className="tab-pill-label"
                  initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }}
                  transition={{ duration: 0.18 }}
                >
                  {isOtherBusy ? `${tabJob?.instance.name} · ${progress}%` : label}
                </motion.span>
              </AnimatePresence>
            </button>
          )
        })}
        <button
          className={['tab-pill', consoleOpen ? 'tab-pill-active tab-pill-console' : ''].filter(Boolean).join(' ')}
          onClick={() => {
            if (!consoleInst) {
              const d = activeInstance && running.includes(activeInstance.name)
                ? activeInstance.name
                : running[0] ?? null
              if (d) setConsoleInst(d)
            }
            setConsoleOpen(true)
          }}
        >
          <span className="tab-pill-label">
            {running.length > 0 && <span className="dot" style={{ background: '#4ade80', boxShadow: '0 0 6px #4ade80' }} />}
            {t('tab.console')}
          </span>
        </button>
        <AnimatePresence>
          {activeTab === 'lb' && (
            <motion.button
              key="lb-configs-btn"
              className="tab-pill tab-pill-lb-plus"
              initial={{ opacity: 0, width: 0, marginLeft: 0 }}
              animate={{ opacity: 1, width: 36, marginLeft: 6 }}
              exit={{ opacity: 0, width: 0, marginLeft: 0 }}
              transition={{ duration: 0.25, ease: [0.4, 0, 0.2, 1] }}
              onClick={setShowLbConfigs}
              title="LB Config Catalog"
            >+</motion.button>
          )}
        </AnimatePresence>
      </div>

      <AnimatePresence mode="wait">
        {consoleOpen ? (

          /* ─── Console tab ─── */
          <motion.div key="console-tab" className="tab-content console-tab-wrap"
            initial={{ opacity: 0, y: 16 }} animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: 16 }} transition={{ duration: 0.22, ease: [0.4,0,0.2,1] }}
          >
            <ConsolePanel instanceName={consoleInst} running={running} onSelect={setConsoleInst} />
          </motion.div>

        ) : activeTab === 'mc' ? (

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
                {activeJob ? (
                  <LaunchCard key={`job-${activeJob.instance.name}`} job={activeJob} lb={false}
                    onPause={() => launchQ.togglePause(activeJob.instance.name)}
                    onCancel={() => launchQ.cancel(activeJob.instance.name)} />
                ) : runningActive ? (
                  <motion.button key="stop-mc" className="stop-btn"
                    onClick={handleStop}
                    initial={{ opacity: 0, y: 20, scale: 0.95 }} animate={{ opacity: 1, y: 0, scale: 1 }}
                    exit={{ opacity: 0, scale: 0.95 }}
                    whileHover={{ scale: 1.035 }} whileTap={{ scale: 0.97 }}
                    transition={{ ...spring, delay: 0.18 }}
                  >
                    <span className="stop-icon">■</span> {t('stop')}
                  </motion.button>
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
                    {!selected ? t('no_account') : !activeInstance ? t('no_instance') : anyLaunching ? t('play.queue') : t('play')}
                  </motion.button>
                )}
              </AnimatePresence>
              <AnimatePresence>
                <QueueStrip key="q-mc" queued={tabQueued} lb={false} onRemove={launchQ.cancel} />
              </AnimatePresence>
              {!activeJob && selected && activeInstance && (
                <motion.div className="hint-text" initial={{ opacity: 0 }} animate={{ opacity: 1 }} transition={{ delay: 0.35 }}>
                  {runningActive
                    ? <>{t('status.running')} · <strong style={{ color: 'var(--accent)' }}>{selected.username}</strong></>
                    : <>
                        {t('status.playing_as')} <strong style={{ color: 'var(--accent)' }}>{selected.username}</strong> · {activeInstance.mcVersion}
                        {runningOther && <> · <span className="other-running-label">{t('running.other').replace('{0}', runningOther)}</span></>}
                      </>
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
                {activeJob ? (
                  <LaunchCard key={`job-${activeJob.instance.name}`} job={activeJob} lb
                    onPause={() => launchQ.togglePause(activeJob.instance.name)}
                    onCancel={() => launchQ.cancel(activeJob.instance.name)} />
                ) : runningActive ? (
                  <motion.button key="stop-lb" className="stop-btn lb-stop"
                    onClick={handleStop}
                    initial={{ opacity: 0, y: 20, scale: 0.95 }} animate={{ opacity: 1, y: 0, scale: 1 }}
                    exit={{ opacity: 0, scale: 0.95 }}
                    whileHover={{ scale: 1.035 }} whileTap={{ scale: 0.97 }}
                    transition={{ ...spring, delay: 0.18 }}
                  >
                    <span className="stop-icon">■</span> {t('stop')}
                  </motion.button>
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
                    {!selected ? t('no_account') : !activeInstance ? t('no_instance') : anyLaunching ? t('play.queue') : t('play.lb')}
                  </motion.button>
                )}
              </AnimatePresence>
              <AnimatePresence>
                <QueueStrip key="q-lb" queued={tabQueued} lb onRemove={launchQ.cancel} />
              </AnimatePresence>
              {!activeJob && selected && activeInstance && (
                <motion.div className="hint-text lb-hint" initial={{ opacity: 0 }} animate={{ opacity: 1 }} transition={{ delay: 0.35 }}>
                  {runningActive
                    ? <>{t('status.running')} · <strong style={{ color: 'var(--lb-accent)' }}>{selected.username}</strong></>
                    : <>
                        <strong style={{ color: 'var(--lb-accent)' }}>{selected.username}</strong> · {activeInstance.name} (MC {activeInstance.mcVersion})
                        {runningOther && <> · <span className="other-running-label">{t('running.other').replace('{0}', runningOther)}</span></>}
                      </>
                  }
                </motion.div>
              )}
            </div>
          </motion.div>

        )}
      </AnimatePresence>
    </div>
  )
}

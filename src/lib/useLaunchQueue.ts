import { useCallback, useEffect, useRef, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { isTauri, tick, type Account, type Instance } from './types'

// ─── Per-instance launch state ────────────────────────────────────────────────
//
// Every launch is tracked by instance name. Several may be in flight: the
// backend serialises downloads of the *same* Minecraft version (version
// lock), everything else runs concurrently. Instances pressed while another
// launch is active go into a FIFO queue and start one after another; the
// user can pull one out of the queue again.

export type LaunchPhase = 'queued' | 'running' | 'error' | 'done'

export type LaunchJob = {
  instance: Instance
  phase: LaunchPhase
  progress: number
  status: string
  stage: string
  paused: boolean
  bps: number
  error?: string
}

type Jobs = Record<string, LaunchJob>

/// Settings a launch needs, read at press time so Settings edits apply
/// to the next launch without a reload.
function launchArgs(inst: Instance, acct: Account) {
  const concurrentDl = Number(localStorage.getItem('mlbv_concurrent') ?? '5') || 5
  const globalRam = Number(localStorage.getItem('mlbv_ram') ?? '2048') || 2048
  const ramMb = Number(localStorage.getItem(`mlbv_inst_ram_${inst.id}`) || globalRam)
  const globalMinRam = Number(localStorage.getItem('mlbv_min_ram') ?? '512') || 512
  const minRamMb = Number(localStorage.getItem(`mlbv_inst_min_ram_${inst.id}`) || globalMinRam)
  return {
    loader: inst.type === 'lb' ? 'liquidbounce' : (inst.loader ?? 'vanilla'),
    mcVersion: inst.mcVersion,
    loaderVersion: inst.loaderVersion ?? '',
    lbBuildId: inst.buildId ?? 0,
    instanceName: inst.name,
    username: acct.username,
    uuid: acct.uuid,
    offline: acct.type === 'offline',
    concurrentDownloads: concurrentDl,
    maxRamMb: ramMb,
    javaPath: localStorage.getItem('mlbv_java_path') ?? '',
    jvmArgs: localStorage.getItem('mlbv_jvm_args') ?? '',
    minRamMb,
  }
}

export function useLaunchQueue(opts: {
  onLaunched: (instanceName: string) => void
  onConsole: (instanceName: string) => void
}) {
  const [jobs, setJobs] = useState<Jobs>({})
  const queueRef = useRef<Instance[]>([])
  const accountRef = useRef<Account | null>(null)
  const optsRef = useRef(opts)
  optsRef.current = opts

  const patch = useCallback((name: string, p: Partial<LaunchJob>) => {
    setJobs(prev => prev[name] ? { ...prev, [name]: { ...prev[name], ...p } } : prev)
  }, [])
  const remove = useCallback((name: string) => {
    setJobs(prev => { if (!prev[name]) return prev; const n = { ...prev }; delete n[name]; return n })
  }, [])

  // Backend events are attributed by instance name.
  useEffect(() => {
    if (!isTauri) return
    const offs: (() => void)[] = []
    listen<{ instance: string; stage: string; progress: number; message: string }>('launch-progress', e => {
      const { instance, stage, progress, message } = e.payload
      patch(instance, { progress: Math.round(progress), status: message, stage })
    }).then(fn => offs.push(fn))
    listen<{ instance: string; bps: number }>('download-speed', e => {
      patch(e.payload.instance, { bps: e.payload.bps })
    }).then(fn => offs.push(fn))
    return () => offs.forEach(fn => fn())
  }, [patch])

  const runOne = useCallback(async (inst: Instance, acct: Account) => {
    const name = inst.name
    patch(name, { phase: 'running', progress: 0, status: 'Preparing…', paused: false, bps: 0, error: undefined })
    if (localStorage.getItem('mlbv_console_enabled') === '1') optsRef.current.onConsole(name)
    if (isTauri) {
      try {
        await invoke('launch_game', launchArgs(inst, acct))
        patch(name, { phase: 'done', progress: 100, status: 'Launched!' })
        optsRef.current.onLaunched(name)
        if (localStorage.getItem('mlbv_close_on_launch') === '1') getCurrentWindow().hide().catch(() => {})
        await tick(1500)
      } catch (err) {
        patch(name, { phase: 'error', error: String(err), status: String(err) })
        await tick(4000)
      }
    } else {
      const steps: [number, string][] = [[15,'Checking files…'],[35,'Downloading assets…'],[60,'Preparing Java…'],[85,'Verifying libs…'],[100,'Launched!']]
      for (const [p, m] of steps) { patch(name, { progress: p, status: m }); await tick(500) }
      await tick(1200)
    }
    remove(name)
  }, [patch, remove])

  // Queue pump: starts the next queued instance once a slot is free. One
  // launch at a time keeps the UI readable; the backend would allow more.
  const pumping = useRef(false)
  const pump = useCallback(async () => {
    if (pumping.current) return
    pumping.current = true
    try {
      while (queueRef.current.length > 0) {
        const next = queueRef.current.shift()!
        const acct = accountRef.current
        if (!acct) { remove(next.name); continue }
        await runOne(next, acct)
      }
    } finally { pumping.current = false }
  }, [runOne, remove])

  const enqueue = useCallback((inst: Instance, acct: Account) => {
    accountRef.current = acct
    setJobs(prev => {
      if (prev[inst.name]) return prev
      return { ...prev, [inst.name]: { instance: inst, phase: 'queued', progress: 0, status: '', stage: '', paused: false, bps: 0 } }
    })
    if (!queueRef.current.some(i => i.name === inst.name)) queueRef.current.push(inst)
    void pump()
  }, [pump])

  const dequeue = useCallback((name: string) => {
    queueRef.current = queueRef.current.filter(i => i.name !== name)
    remove(name)
  }, [remove])

  const cancel = useCallback((name: string) => {
    const job = jobs[name]
    if (!job) return
    if (job.phase === 'queued') { dequeue(name); return }
    if (isTauri) invoke('cancel_download', { instanceName: name }).catch(() => {})
  }, [jobs, dequeue])

  const togglePause = useCallback(async (name: string) => {
    if (!isTauri) return
    try {
      const paused = await invoke<boolean>('pause_download', { instanceName: name })
      patch(name, { paused })
    } catch { /* the launch may have just finished */ }
  }, [patch])

  const active = Object.values(jobs).find(j => j.phase === 'running' || j.phase === 'error') ?? null
  const queued = Object.values(jobs).filter(j => j.phase === 'queued')

  return { jobs, active, queued, enqueue, cancel, togglePause }
}

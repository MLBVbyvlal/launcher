import { useCallback, useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import type { MigrationScanT, PinPreview } from '../MigrationScreen'
import { isTauri, tick, type AppState, type Instance, type MCVersion } from './types'
import { isLegacyDataVersion, buildPinPreviews } from './migration'

// ─── Boot: version manifest + 0.0.5 migration gate ───────────────────────────

export function useBoot() {
  const [appState, setAppState]     = useState<AppState>('loading')
  const [migrateScan, setMigrateScan] = useState<MigrationScanT | null>(null)
  const [migratePins, setMigratePins] = useState<PinPreview[]>([])
  const [loadStatus, setLoadStatus] = useState('Connecting to Mojang…')
  const [loadProgress, setLoadProg] = useState(0)
  const [versions, setVersions]     = useState<MCVersion[]>([])

  // ── Fetch MC versions ────────────────────────────────────────────────────
  const fetchVersions = useCallback(async () => {
    setAppState('loading'); setLoadProg(0); setLoadStatus('Connecting to Mojang…')
    try {
      await tick(300); setLoadProg(30); setLoadStatus('Fetching version manifest…')
      const res = await fetch('https://launchermeta.mojang.com/mc/game/version_manifest_v2.json')
      if (!res.ok) throw new Error(`HTTP ${res.status}`)
      setLoadProg(70); setLoadStatus('Parsing versions…')
      await tick(150)
      const data = await res.json()
      const mfVersions = data.versions as MCVersion[]
      setVersions(mfVersions)
      // ── 0.0.5 data migration gate ──
      // Legacy data (marker missing/older) with anything on disk or in the
      // instance list must convert before entering. Fresh installs and
      // post-reset states have nothing to convert: stamp the marker silently.
      if (isTauri) {
        try {
          const dv = await invoke<string>('get_data_version')
          if (isLegacyDataVersion(dv)) {
            setLoadProg(85); setLoadStatus('Checking data version…')
            const scan = await invoke<MigrationScanT>('migration_scan')
            let bootInst: Instance[] = []
            try { bootInst = JSON.parse(localStorage.getItem('mlbv_instances') ?? '[]') } catch { bootInst = [] }
            if (bootInst.length > 0 || scan.has_instances || scan.has_versions || scan.garbage.length > 0) {
              setMigrateScan(scan)
              setMigratePins(buildPinPreviews(bootInst, mfVersions))
              setLoadProg(100)
              setAppState('migrate')
              return
            }
          }
          if (!dv) { try { await invoke('set_data_version') } catch { /* next boot retries */ } }
        } catch { /* transport failure → fall through to ready */ }
      }
      setLoadProg(100); setLoadStatus('Ready!')
      await tick(400); setAppState('ready')
    } catch {
      setLoadStatus('Failed to connect to Mojang servers'); setAppState('error')
    }
  }, [])

  useEffect(() => { fetchVersions() }, [fetchVersions])

  return { appState, setAppState, migrateScan, migratePins, loadStatus, loadProgress, versions, fetchVersions }
}

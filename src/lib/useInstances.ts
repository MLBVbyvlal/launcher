import { useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { isTauri, type AppState, type Instance } from './types'

// ─── Instance list: localStorage + on-disk metadata mirror ────────────────────

export function useInstances(appState: AppState) {
  const [instances, setInstances] = useState<Instance[]>(() => {
    try { return JSON.parse(localStorage.getItem('mlbv_instances') ?? '[]') } catch { return [] }
  })
  const [activeMcInstId, setActiveMcInstId] = useState<string | null>(() =>
    localStorage.getItem('mlbv_active_mc')
  )
  const [activeLbInstId, setActiveLbInstId] = useState<string | null>(() =>
    localStorage.getItem('mlbv_active_lb')
  )

  // Persist instances
  useEffect(() => {
    localStorage.setItem('mlbv_instances', JSON.stringify(instances))
  }, [instances])
  useEffect(() => {
    if (activeMcInstId) localStorage.setItem('mlbv_active_mc', activeMcInstId)
  }, [activeMcInstId])
  useEffect(() => {
    if (activeLbInstId) localStorage.setItem('mlbv_active_lb', activeLbInstId)
  }, [activeLbInstId])

  // ── Instance persistence (disk metadata) ──────────────────────────────────
  // localStorage alone loses the instance list when WebView data is cleared
  // while the game dirs stay on disk. Mirror each instance into its dir as
  // .mlbv-instance.json and recover missing ones on startup.
  const persistInstance = (inst: Instance) => {
    if (!isTauri) return
    invoke('save_instance_metadata', {
      instanceName: inst.name,
      instanceType: inst.type,
      mcVersion: inst.mcVersion,
      loader: inst.loader ?? 'vanilla',
      loaderVersion: inst.loaderVersion ?? '',
      buildId: inst.buildId ?? null,
    }).catch(() => {})
  }

  // Migration result: replace the instance list, re-save every metadata file.
  const applyMigrationPins = (updated: Instance[]) => {
    setInstances(updated)
    updated.forEach(persistInstance)
  }

  useEffect(() => {
    if (!isTauri || appState !== 'ready') return
    type Found = { name: string; instance_type: string; mc_version: string | null; loader: string | null; loader_version: string | null; build_id: number | null }
    invoke<Found[]>('scan_instances')
      .then(found => {
        setInstances(prev => {
          const known = new Set(prev.map(i => i.name))
          const recovered: Instance[] = found
            .filter(f => !known.has(f.name))
            .map(f => ({
              id: crypto.randomUUID(),
              name: f.name,
              type: (f.instance_type === 'lb' ? 'lb' : 'mc') as 'mc' | 'lb',
              // Rolling versions no longer exist; a stale 'latest' in old
              // metadata surfaces as Unknown instead of breaking the launch.
              version: (f.mc_version && f.mc_version !== 'latest') ? f.mc_version : 'Unknown',
              mcVersion: (f.mc_version && f.mc_version !== 'latest') ? f.mc_version : '',
              loader: (f.loader && f.loader !== 'vanilla' ? f.loader : 'vanilla') as Instance['loader'],
              loaderVersion: f.loader_version ?? undefined,
              buildId: f.build_id ?? undefined,
            }))
          return recovered.length ? [...prev, ...recovered] : prev
        })
      })
      .catch(() => {})
  }, [appState])

  // ── Instance management ───────────────────────────────────────────────────
  const addInstance = (inst: Instance) => {
    setInstances(prev => [...prev, inst])
    if (inst.type === 'mc') setActiveMcInstId(inst.id)
    else setActiveLbInstId(inst.id)
    persistInstance(inst)
  }

  const removeInstance = (id: string) => {
    setInstances(prev => prev.filter(i => i.id !== id))
    if (activeMcInstId === id) setActiveMcInstId(null)
    if (activeLbInstId === id) setActiveLbInstId(null)
  }

  const renameInstance = (id: string, newName: string) => {
    const trimmed = newName.trim()
    if (!trimmed) return
    if (instances.some(i => i.id !== id && i.name === trimmed)) return
    const old = instances.find(i => i.id === id)
    if (!old || old.name === trimmed) return
    setInstances(prev => prev.map(i => i.id === id ? { ...i, name: trimmed } : i))
    // Move the on-disk dir too — otherwise saves/mods stay in the old dir and
    // the next launch starts from a fresh empty instance.
    const renamed: Instance = { ...old, name: trimmed }
    if (isTauri) invoke('rename_instance_data', { oldName: old.name, newName: trimmed }).catch(() => {})
    persistInstance(renamed)
  }

  const handleDeleteDisk = (inst: Instance) => {
    removeInstance(inst.id)
    if (isTauri) invoke('delete_instance_data', { instanceName: inst.name }).catch(() => {})
  }

  return {
    instances, setInstances, activeMcInstId, activeLbInstId, setActiveMcInstId, setActiveLbInstId,
    addInstance, removeInstance, renameInstance, handleDeleteDisk, applyMigrationPins,
  }
}

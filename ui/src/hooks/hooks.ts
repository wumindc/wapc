/**
 * Real Tauri hooks — calls Rust backend via invoke.
 * @author codex
 */

import { useState, useCallback, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'
import type { DesktopSnapshot } from '../types'

export function useSnapshot() {
  const [snapshot, setSnapshot] = useState<DesktopSnapshot | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [lastRefresh, setLastRefresh] = useState<Date | null>(null)

  const refresh = useCallback(async () => {
    setLoading(true)
    try {
      const data = await invoke<DesktopSnapshot>('get_snapshot')
      setSnapshot(data)
      setLastRefresh(new Date())
      setError(null)
    } catch (err) {
      setError(String(err))
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    void Promise.resolve().then(refresh)
  }, [refresh])

  return { snapshot, loading, error, lastRefresh, refresh }
}

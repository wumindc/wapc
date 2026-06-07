/**
 * Auto-scan controller + context.
 *
 * Replaces the old macOS LaunchAgent background service: while the app is in
 * the foreground, it periodically calls the `scan_now` command. Preferences
 * (on/off + interval) are persisted to localStorage.
 *
 * @author Claude Opus 4.8
 */

import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useRef,
  useState,
} from 'react'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

export interface AutoScanValue {
  enabled: boolean
  setEnabled: (v: boolean) => void
  intervalMinutes: number
  setIntervalMinutes: (v: number) => void
  lastScan: Date | null
  scanning: boolean
  /** Trigger a scan immediately; resolves to the indexed record count. */
  scanNow: () => Promise<number>
}

export const SCAN_INTERVAL_OPTIONS = [5, 15, 30, 60]

export const AutoScanContext = createContext<AutoScanValue | null>(null)

export function useAutoScanController(scan: () => Promise<number>, refresh: () => void): AutoScanValue {
  const [enabled, setEnabledState] = useState<boolean>(false)
  const [intervalMinutes, setIntervalState] = useState<number>(60)
  const [lastScan, setLastScan] = useState<Date | null>(null)
  const [scanning, setScanning] = useState(false)

  // Load initial config from backend
  useEffect(() => {
    invoke<{ enabled: boolean; interval_minutes: number }>('get_auto_scan_config')
      .then((config) => {
        setEnabledState(config.enabled)
        setIntervalState(config.interval_minutes)
      })
      .catch((e) => console.error('Failed to load auto-scan config:', e))
  }, [])

  const setEnabled = useCallback((v: boolean) => {
    setEnabledState(v)
    invoke('set_auto_scan_config', { config: { enabled: v, interval_minutes: intervalMinutes } }).catch(console.error)
  }, [intervalMinutes])

  const setIntervalMinutes = useCallback((v: number) => {
    setIntervalState(v)
    invoke('set_auto_scan_config', { config: { enabled, interval_minutes: v } }).catch(console.error)
  }, [enabled])

  const scanRef = useRef(scan)
  const refreshRef = useRef(refresh)
  useEffect(() => {
    scanRef.current = scan
    refreshRef.current = refresh
  }, [scan, refresh])

  const scanNow = useCallback(async () => {
    try {
      return await scanRef.current()
    } catch (e) {
      console.error(e)
      return 0
    }
  }, [])

  // Listen to backend scan events
  useEffect(() => {
    const unlistenStart = listen('scan-started', () => {
      setScanning(true)
    })
    const unlistenFinish = listen('scan-finished', () => {
      setScanning(false)
      setLastScan(new Date())
      // Refresh dashboard snapshot data from backend
      refreshRef.current()
    })

    return () => {
      unlistenStart.then((f) => f())
      unlistenFinish.then((f) => f())
    }
  }, [])

  return {
    enabled,
    setEnabled,
    intervalMinutes,
    setIntervalMinutes,
    lastScan,
    scanning,
    scanNow,
  }
}

export function useAutoScan(): AutoScanValue {
  const ctx = useContext(AutoScanContext)
  if (!ctx) throw new Error('useAutoScan must be used within an AutoScanContext provider')
  return ctx
}

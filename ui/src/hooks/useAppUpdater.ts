import { useState, useEffect, useCallback } from 'react'
import { listen } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/core'
import { message } from '@tauri-apps/plugin-dialog'

export interface UpdateInfo {
  version: string
  notes?: string
  pub_date?: string
}

export interface UpdateProgress {
  percent: number
  downloaded: number
  total?: number
}

export function useAppUpdater() {
  const [available, setAvailable] = useState(false)
  const [updateInfo, setUpdateInfo] = useState<UpdateInfo | null>(null)
  const [progress, setProgress] = useState<UpdateProgress | null>(null)
  const [isInstalling, setIsInstalling] = useState(false)

  // 检查是否有可用更新
  const checkUpdate = useCallback(async () => {
    try {
      const info = await invoke<UpdateInfo | null>('check_update')
      if (info) {
        setAvailable(true)
        setUpdateInfo(info)
      }
    } catch (e) {
      console.error('Failed to check update:', e)
    }
  }, [])

  // 触发安装
  const installUpdate = useCallback(async () => {
    if (!available || isInstalling) return
    setIsInstalling(true)
    try {
      // install_update 会在 Rust 层阻塞直到下载完成，并触发应用重启
      await invoke('install_update')
    } catch (e) {
      console.error('Failed to install update:', e)
      await message(String(e), { title: '更新失败', kind: 'error' })
      setIsInstalling(false)
      setProgress(null)
    }
  }, [available, isInstalling])

  // 监听后端发来的事件
  useEffect(() => {
    // 自动扫描触发的可用事件
    const unlistenAvailable = listen<UpdateInfo>('update-available', (event) => {
      setAvailable(true)
      setUpdateInfo(event.payload)
    })

    // 下载进度事件
    const unlistenProgress = listen<UpdateProgress>('update-progress', (event) => {
      setProgress(event.payload)
    })

    // 初次加载时主动查一次
    // eslint-disable-next-line
    void checkUpdate()

    return () => {
      unlistenAvailable.then(f => f())
      unlistenProgress.then(f => f())
    }
  }, [checkUpdate])

  return {
    available,
    updateInfo,
    progress,
    isInstalling,
    installUpdate,
    checkUpdate
  }
}

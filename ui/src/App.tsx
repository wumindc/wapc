/**
 * App.tsx — root shell with Sidebar + GlobalTopbar layout.
 * @author WAPC
 */

import { useState, useCallback, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { getCurrent, onOpenUrl } from '@tauri-apps/plugin-deep-link'
import { Sidebar, type ViewId } from './components/layout/Sidebar'
import { OverviewPage } from './pages/OverviewPage'
import { ToolsStatusPage } from './pages/ToolsStatusPage'
import { TokensPage } from './pages/TokensPage'
import { AnalyticsPage } from './pages/AnalyticsPage'
import { PricingPage } from './pages/PricingPage'
import { SyncPage } from './pages/SyncPage'
import { SessionsPage } from './pages/SessionsPage'
import { McpPage } from './pages/McpPage'
import { SkillsPage } from './pages/SkillsPage'
import { AgentsPage } from './pages/AgentsPage'
import { PluginsPage } from './pages/PluginsPage'
import { AboutPage } from './pages/OtherPages'
import { useSnapshot } from './hooks/hooks'
import { AutoScanContext, useAutoScanController } from './hooks/auto-scan'
import { attachWapcDeepLinkHandler } from './hooks/deep-link'
import { AlertCircle } from 'lucide-react'
import { ThemeProvider } from './components/ThemeProvider'

export default function App() {
  const [isSidebarOpen, setIsSidebarOpen] = useState(false)
  const [activeMenu, setActiveMenu] = useState<ViewId>('overview')
  const [globalSelectedTool, setGlobalSelectedTool] = useState('all')
  const [notification, setNotification] = useState<string | null>(null)
  const [pendingDeepLinkUrl, setPendingDeepLinkUrl] = useState<string | null>(null)

  const { snapshot, loading, error, refresh } = useSnapshot()

  // Foreground auto-scan: scan the local sources, then refresh the dashboard.
  const rawScan = useCallback(async () => {
    const count = await invoke<number>('scan_now')
    // Note: We don't need to call refresh() here manually because scan_now
    // emits 'scan-finished' on the backend, which our controller will catch
    // and then invoke the refresh() callback.
    return count
  }, [])
  const autoScan = useAutoScanController(rawScan, refresh)

  const handleDeepLinkImport = useCallback((url: string) => {
    setActiveMenu('sync')
    setPendingDeepLinkUrl(url)
    setNotification('已接收 wapc://import 深链，请在数据导入预览内容后再选择目标')
    setTimeout(() => setNotification(null), 5000)
  }, [])

  useEffect(() => {
    let disposed = false
    let cleanup: (() => void) | null = null
    attachWapcDeepLinkHandler({ getCurrent, onOpenUrl }, handleDeepLinkImport)
      .then(unlisten => {
        if (disposed) {
          unlisten()
          return
        }
        cleanup = unlisten
      })
      .catch(err => {
        console.warn('WAPC deep-link listener unavailable', err)
      })
    return () => {
      disposed = true
      cleanup?.()
    }
  }, [handleDeepLinkImport])

  if (loading && !snapshot) {
    return (
      <ThemeProvider defaultTheme="system">
        <div className="flex items-center justify-center h-screen bg-page text-muted">
          加载中…
        </div>
      </ThemeProvider>
    )
  }

  if (!snapshot) {
    return (
      <ThemeProvider defaultTheme="system">
        <div className="flex h-screen items-center justify-center bg-page px-6 text-text">
          <div className="max-w-md rounded-xl card p-6">
            <h1 className="text-lg font-semibold text-heading">本机快照暂不可用</h1>
            <p className="mt-2 text-sm text-muted">
              {error ?? '未能从本机数据源加载 WAPC 快照。'}
            </p>
            <button
              type="button"
              onClick={refresh}
              className="mt-4 rounded-md bg-brand-blue px-4 py-2 text-sm font-medium text-white transition-colors hover:opacity-90"
            >
              重新加载
            </button>
          </div>
        </div>
      </ThemeProvider>
    )
  }

  // Render view based on active menu
  const renderView = () => {
    switch (activeMenu) {
      case 'overview': return <OverviewPage snapshot={snapshot} selectedTool={globalSelectedTool} setSelectedTool={setGlobalSelectedTool} setIsSidebarOpen={setIsSidebarOpen} />
      case 'tokens': return <TokensPage snapshot={snapshot} selectedTool={globalSelectedTool} setSelectedTool={setGlobalSelectedTool} setIsSidebarOpen={setIsSidebarOpen} />
      case 'analytics': return <AnalyticsPage snapshot={snapshot} selectedTool={globalSelectedTool} setSelectedTool={setGlobalSelectedTool} setIsSidebarOpen={setIsSidebarOpen} />
      case 'mcp': return <McpPage snapshot={snapshot} setIsSidebarOpen={setIsSidebarOpen} />
      case 'skills': return <SkillsPage snapshot={snapshot} setIsSidebarOpen={setIsSidebarOpen} />
      case 'agents': return <AgentsPage snapshot={snapshot} setIsSidebarOpen={setIsSidebarOpen} />
      case 'plugins': return <PluginsPage setIsSidebarOpen={setIsSidebarOpen} />
      case 'tools': return <ToolsStatusPage snapshot={snapshot} setIsSidebarOpen={setIsSidebarOpen} />
      case 'pricing': return <PricingPage setIsSidebarOpen={setIsSidebarOpen} />
      case 'sync': return <SyncPage initialDeepLinkUrl={pendingDeepLinkUrl} setIsSidebarOpen={setIsSidebarOpen} />
      case 'sessions': return <SessionsPage snapshot={snapshot} selectedTool={globalSelectedTool} setSelectedTool={setGlobalSelectedTool} setIsSidebarOpen={setIsSidebarOpen} />
      case 'about': return <AboutPage snapshot={snapshot} setIsSidebarOpen={setIsSidebarOpen} />
      default:
        return (
          <div className="flex flex-col items-center justify-center h-full text-slate-500 fade-in">
             <AlertCircle size={48} className="mb-4 opacity-20" />
             <p className="text-lg font-medium">模块 [{activeMenu}] 正在开发中</p>
             <p className="text-sm mt-2">请点击侧边栏的其他模块</p>
          </div>
        )
    }
  }

  return (
    <ThemeProvider defaultTheme="system">
      <AutoScanContext.Provider value={autoScan}>
        <div className="flex h-screen bg-page text-text font-sans overflow-hidden">
          
          {/* Mobile Sidebar Overlay */}
        {isSidebarOpen && (
          <div 
            className="fixed inset-0 bg-black/60 z-40 md:hidden backdrop-blur-sm"
            onClick={() => setIsSidebarOpen(false)}
          />
        )}

        {/* Sidebar Component */}
        <Sidebar 
          isOpen={isSidebarOpen} 
          setIsOpen={setIsSidebarOpen}
          activeMenu={activeMenu}
          setActiveMenu={(id) => { setActiveMenu(id); setIsSidebarOpen(false); }}
        />

        {/* Main Content Area */}
        <div className="flex-1 flex flex-col min-w-0 overflow-hidden relative z-0">
          {/* Global top drag region */}
          <div data-tauri-drag-region className="absolute top-0 left-0 w-full h-[24px] z-50 bg-transparent" />

          {/* Error & Notification Banner */}
          <div className="px-6 py-2 shrink-0">
            {error && (
              <div className="px-4 py-2.5 bg-red-950/30 border border-red-900 rounded-lg text-sm text-red-400">
                {error}
              </div>
            )}
            {notification && (
              <div className="mt-2 px-4 py-2.5 bg-blue-950/30 border border-blue-900 rounded-lg text-sm text-brand-blue font-medium">
                {notification}
              </div>
            )}
          </div>

          {/* View Container */}
          <main className="flex-1 overflow-hidden bg-page">
             {renderView()}
          </main>
        </div>
      </div>
    </AutoScanContext.Provider>
    </ThemeProvider>
  )
}

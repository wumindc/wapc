import { useState, type ReactNode } from 'react'
import {
  Activity,
  Box,
  Monitor,
  Server,
  Code,
  Upload,
  RefreshCw,
  ChevronDown,
  ChevronRight,
  X,
  Moon,
  Sun
} from 'lucide-react'
import { useTheme } from '../useTheme'
import { useAutoScan } from '../../hooks/auto-scan'
import { UpdateBadge } from './UpdateBadge'

export type ViewId =
  | 'overview'
  | 'tokens'
  | 'analytics'
  | 'mcp'
  | 'skills'
  | 'agents'
  | 'plugins'
  | 'tools'
  | 'pricing'
  | 'sync'
  | 'sessions'
  | 'service'
  | 'about'

interface SidebarProps {
  isOpen: boolean
  setIsOpen: (isOpen: boolean) => void
  activeMenu: ViewId
  setActiveMenu: (id: ViewId) => void
}

export function Sidebar({ isOpen, setIsOpen, activeMenu, setActiveMenu }: SidebarProps) {
  const [expandedGroups, setExpandedGroups] = useState<Record<string, boolean>>({ 
    usage: true, 
    core: true, 
    tools: true, 
    data: false 
  })

  const toggleGroup = (group: string) => {
    setExpandedGroups(prev => ({ ...prev, [group]: !prev[group] }))
  }

  const { theme, setTheme } = useTheme()
  const { enabled: autoScanEnabled, setEnabled: setAutoScanEnabled, scanning, scanNow } = useAutoScan()

  const toggleTheme = () => {
    if (theme === 'system') setTheme('dark')
    else if (theme === 'dark') setTheme('light')
    else setTheme('system')
  }

  const renderThemeIcon = () => {
    if (theme === 'system') return <Monitor size={15} />
    if (theme === 'dark') return <Moon size={15} />
    return <Sun size={15} />
  }

  const themeLabel = theme === 'system' ? '跟随系统' : theme === 'dark' ? '暗色模式' : '亮色模式'

  type MenuItem = {
    id: ViewId
    label: string
    icon?: ReactNode
  }

  type MenuGroup = {
    id: string
    label: string
    icon: ReactNode
    items: MenuItem[]
  }

  const menuGroups: MenuGroup[] = [
    {
      id: 'usage', label: 'Usage', icon: <Activity size={18} />,
      items: [
        { id: 'overview' as ViewId, label: '概览 (Overview)' },
        { id: 'tokens' as ViewId, label: '使用明细 (Tokens)' },
        { id: 'analytics' as ViewId, label: '分析视图 (Analytics)' },
        { id: 'sessions' as ViewId, label: '会话浏览 (Sessions)' },
      ]
    },
    {
      id: 'core', label: '核心能力库', icon: <Box size={18} />,
      items: [
        { id: 'mcp' as ViewId, label: 'MCP 管理', icon: <Server size={16} /> },
        { id: 'skills' as ViewId, label: 'Skills 管理', icon: <Code size={16} /> },
        { id: 'agents' as ViewId, label: 'Agents 规范', icon: <Code size={16} /> },
        { id: 'plugins' as ViewId, label: 'Plugin 包', icon: <Box size={16} /> },
      ]
    },
    {
      id: 'tools', label: '支持工具与计费', icon: <Monitor size={18} />,
      items: [
        { id: 'tools' as ViewId, label: '工具状态矩阵' },
        { id: 'pricing' as ViewId, label: '模型价格' },
      ]
    },
    {
      id: 'data', label: '数据同步', icon: <Activity size={18} />,
      items: [
        { id: 'sync' as ViewId, label: '导入与导出', icon: <Upload size={16} /> },
      ]
    }
  ]

  return (
    <div className={`fixed inset-y-0 left-0 z-50 w-64 bg-sidebar-bg border-r border-border-soft transform transition-transform duration-300 ease-in-out md:translate-x-0 ${isOpen ? 'translate-x-0' : '-translate-x-full'} md:static md:flex-shrink-0 flex flex-col`}>
      <div data-tauri-drag-region className="flex items-center justify-between h-[68px] pl-[76px] pr-4 pt-[12px] bg-page border-b border-border">
        <div className="flex items-center space-x-2 text-brand-blue font-bold text-xl pointer-events-none">
          <Activity size={24} />
          <span>WAPC</span>
        </div>
        <div className="flex items-center gap-2">
          <UpdateBadge />
          <button className="md:hidden text-muted hover:text-text z-10" onClick={() => setIsOpen(false)}>
            <X size={20} />
          </button>
        </div>
      </div>

      <div className="flex-1 overflow-y-auto py-4 px-3 space-y-1 custom-scrollbar">
        {menuGroups.map((group) => (
          <div key={group.id} className="mb-2">
            <button
              onClick={() => toggleGroup(group.id)}
              className="w-full flex items-center justify-between px-3 py-2 text-sm font-medium text-text hover:bg-surface-hover rounded-md transition-colors"
            >
              <div className="flex items-center space-x-3 text-muted">
                <span>{group.icon}</span>
                <span className="text-text">{group.label}</span>
              </div>
              <span className="text-muted">
                {expandedGroups[group.id] ? <ChevronDown size={16} /> : <ChevronRight size={16} />}
              </span>
            </button>
            
            {expandedGroups[group.id] && (
              <div className="mt-1 space-y-1 pl-10">
                {group.items.map((item) => (
                  <button
                    key={item.id}
                    onClick={() => setActiveMenu(item.id)}
                    className={`w-full flex items-center space-x-2 px-3 py-2 text-sm rounded-md transition-colors ${
                      activeMenu === item.id 
                        ? 'bg-brand-blue/10 text-brand-blue font-medium' 
                        : 'text-muted hover:bg-surface-hover hover:text-text'
                    }`}
                  >
                    {item.icon && <span className="opacity-70">{item.icon}</span>}
                    <span>{item.label}</span>
                  </button>
                ))}
              </div>
            )}
          </div>
        ))}
      </div>

      <div className="p-3 border-t border-border-soft bg-surface-soft shrink-0">
        <div 
          onClick={() => {
            if (autoScanEnabled && !scanning) {
              void scanNow(); // Trigger a scan immediately if enabled
            } else {
              setAutoScanEnabled(!autoScanEnabled)
            }
          }} 
          className={`bg-surface border border-border rounded-lg p-2.5 mb-2.5 cursor-pointer hover:border-brand-blue/30 transition-all shadow-sm flex items-center justify-between ${scanning ? 'opacity-80' : ''}`}
        >
          <div className="flex items-center gap-2">
             <span className={`w-2.5 h-2.5 rounded-full shrink-0 shadow-[0_0_8px_rgba(0,0,0,0.2)] transition-colors ${autoScanEnabled ? 'bg-brand-green' : 'bg-muted'}`} />
             <span className="text-[13px] font-semibold text-text truncate">
               {scanning ? '正在扫描...' : (autoScanEnabled ? '自动扫描运行中' : '自动扫描已关闭')}
             </span>
          </div>
          <button 
            className={`p-1.5 rounded-md hover:bg-surface-hover text-muted transition-colors ${!autoScanEnabled ? 'opacity-0' : ''}`}
            onClick={(e) => { e.stopPropagation(); if (!scanning) void scanNow(); }}
            title="手动刷新"
          >
            <RefreshCw size={13} className={scanning ? 'animate-spin' : ''} />
          </button>
        </div>
        
        <button 
          onClick={toggleTheme} 
          className="w-full h-9 flex items-center px-3 gap-3 rounded-lg text-sm text-text hover:bg-surface-hover border border-transparent transition-colors"
        >
          <div className="text-muted shrink-0">
            {renderThemeIcon()}
          </div>
          <span className="flex-1 text-left truncate">{themeLabel}</span>
        </button>
      </div>
    </div>
  )
}

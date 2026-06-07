/**
 * McpPage — MCP 服务器管理页面
 * 支持浏览、搜索，以及通过 SyncDialog 真实下发到其他工具
 * @author Claude Sonnet 4.6 (Thinking)
 */
import { useState } from 'react'
import { Server, Search, CheckCircle, Monitor, Box, Terminal, Plus, Send } from 'lucide-react'
import type { DesktopSnapshot, CanonicalResource } from '../types'
import { getToolMeta } from '../types'
import { PageHeader } from '../components/layout/PageHeader'
import { SyncDialog } from '../components/sync/SyncDialog'
import { AddMcpDialog } from '../components/sync/AddMcpDialog'

export function McpPage({ 
  snapshot,
  setIsSidebarOpen
}: { 
  snapshot: DesktopSnapshot;
  setIsSidebarOpen?: (o: boolean) => void;
}) {
  // Filter for MCP resources
  const rawMcpResources = snapshot.resources.filter(r => r.kind.includes('mcp'))
  
  // Deduplicate by name and merge enabled_in
  const mcpResources = Array.from(
    rawMcpResources.reduce((map, mcp) => {
      if (!map.has(mcp.name)) {
        map.set(mcp.name, { ...mcp, enabled_in: [...mcp.enabled_in] })
      } else {
        const existing = map.get(mcp.name)!;
        existing.enabled_in = Array.from(new Set([...existing.enabled_in, ...mcp.enabled_in]))
      }
      return map;
    }, new Map<string, CanonicalResource>()).values()
  )
  
  const [selectedMCP, setSelectedMCP] = useState<CanonicalResource | null>(mcpResources[0] || null)
  const [searchQuery, setSearchQuery] = useState('')
  const [syncTarget, setSyncTarget] = useState<CanonicalResource | null>(null)
  const [showAddDialog, setShowAddDialog] = useState(false)

  const filteredMCPs = mcpResources.filter(mcp => 
    mcp.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
    mcp.kind.toLowerCase().includes(searchQuery.toLowerCase())
  )

  // Status mapping based on confidence
  const getStatus = (confidence: number) => {
    if (confidence >= 0.9) return { id: 'normal', label: '运行正常', color: 'emerald' }
    if (confidence >= 0.5) return { id: 'partial', label: '部分挂载', color: 'amber' }
    return { id: 'error', label: '解析异常', color: 'red' }
  }

  // ALL tools detected vs supported by this MCP
  const allTools = snapshot.detected_tools

  return (
    <div className="flex flex-col h-full fade-in">
      <div className="px-6 pt-6 pb-2 border-b border-border bg-page">
        <div className="flex justify-between items-end">
          <PageHeader 
            title="MCP 服务器"
            subtitle="统一模型上下文协议能力库"
            setIsSidebarOpen={setIsSidebarOpen}
          />
          <div className="pb-4">
            <button 
              onClick={() => setShowAddDialog(true)}
              className="px-4 h-9 flex items-center justify-center bg-brand-blue hover:bg-brand-blue/90 text-white rounded-lg text-[13px] font-medium shadow-sm transition-colors"
            >
              <Plus size={16} className="mr-1" /> 添加 MCP
            </button>
          </div>
        </div>
      </div>
      
      <div className="flex flex-1 overflow-hidden">
        {/* Master List */}
        <div className="w-1/3 min-w-[280px] max-w-[380px] border-r border-border-soft bg-surface flex flex-col h-full">
          <div className="p-4 border-b border-border-soft">
            <div className="relative group">
              <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 text-muted group-focus-within:text-brand-blue transition-colors" size={16} />
              <input 
                type="text" 
                placeholder="搜索 MCP 实例或协议类型..." 
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="w-full bg-page border border-border rounded-lg py-2 pl-9 pr-3 text-[13px] text-text focus:outline-none focus:border-brand-blue/50 focus:ring-1 focus:ring-brand-blue/30 transition-all shadow-sm" 
              />
            </div>
          </div>
          <div className="flex-1 overflow-y-auto custom-scrollbar p-2">
          {filteredMCPs.map(mcp => {
            const status = getStatus(mcp.confidence)
            return (
              <button
                key={mcp.id}
                onClick={() => setSelectedMCP(mcp)}
                className={`w-full text-left p-3.5 mb-1 rounded-xl transition-all border ${
                  selectedMCP?.id === mcp.id 
                    ? 'bg-brand-blue/5 border-brand-blue/30 shadow-sm' 
                    : 'bg-transparent border-transparent hover:bg-surface-hover hover:border-border-soft'
                }`}
              >
                <div className="flex items-center justify-between mb-1.5">
                  <div className="flex items-center space-x-2">
                    <Box size={14} className={selectedMCP?.id === mcp.id ? 'text-brand-blue' : 'text-muted'} />
                    <span className={`font-semibold text-[14px] ${selectedMCP?.id === mcp.id ? 'text-brand-blue' : 'text-text'}`}>{mcp.name}</span>
                  </div>
                  <span className={`w-2 h-2 rounded-full bg-${status.color}-500 shadow-[0_0_8px_rgba(0,0,0,0.2)]`}></span>
                </div>
                <div className="flex justify-between items-center text-[12px] text-muted mt-2">
                  <span className="line-clamp-1 flex-1 pr-2 opacity-80">{mcp.scope} · {mcp.kind}</span>
                  <div className="flex items-center gap-1">
                    {mcp.enabled_in.slice(0, 2).map(t => (
                      <span key={t} className="shrink-0 bg-page px-1.5 py-0.5 rounded text-[11px] border border-border-soft">{t}</span>
                    ))}
                    {mcp.enabled_in.length > 2 && (
                      <span className="shrink-0 bg-page px-1.5 py-0.5 rounded text-[11px] border border-border-soft">+{mcp.enabled_in.length - 2}</span>
                    )}
                  </div>
                </div>
              </button>
            )
          })}
          {filteredMCPs.length === 0 && (
             <div className="p-8 text-center text-muted text-sm">
               {searchQuery ? '没有找到匹配的 MCP 服务器' : (
                 <div className="flex flex-col items-center gap-2">
                   <Server size={32} className="opacity-20" />
                   <p>尚未扫描到 MCP 服务器</p>
                   <p className="text-[11px]">请先运行一次扫描</p>
                 </div>
               )}
             </div>
          )}
        </div>
      </div>

      {/* Detail Area */}
      <div className="flex-1 bg-page p-6 overflow-y-auto custom-scrollbar">
        {selectedMCP ? (
          <div className="max-w-3xl mx-auto">
            <div className="flex items-center justify-between mb-8 p-6 bg-surface border border-border rounded-2xl shadow-sm">
              <div className="flex items-center space-x-4">
                <div className="p-4 bg-brand-blue/10 text-brand-blue rounded-xl shadow-inner">
                  <Server size={28} />
                </div>
                <div>
                  <h2 className="text-2xl font-bold text-heading tracking-tight">{selectedMCP.name}</h2>
                  <div className="flex items-center space-x-3 text-[13px] mt-2">
                     <span className={`flex items-center px-2 py-1 rounded-md bg-${getStatus(selectedMCP.confidence).color}-900/20 text-${getStatus(selectedMCP.confidence).color}-400 border border-${getStatus(selectedMCP.confidence).color}-900/30 font-medium`}>
                       <span className={`w-1.5 h-1.5 rounded-full bg-${getStatus(selectedMCP.confidence).color}-400 mr-2`}></span>
                       {getStatus(selectedMCP.confidence).label}
                     </span>
                     <span className="text-muted font-mono bg-page px-2 py-1 rounded-md border border-border-soft text-[11px]">ID: {selectedMCP.id}</span>
                  </div>
                </div>
              </div>
              {/* 下发按钮 */}
              <button
                onClick={() => setSyncTarget(selectedMCP)}
                className="flex items-center gap-2 px-4 h-9 rounded-lg bg-brand-blue text-white text-[13px] font-medium hover:bg-brand-blue/90 transition-colors shadow-sm shrink-0"
              >
                <Send size={14} />
                同步下发
              </button>
            </div>

            <div className="grid grid-cols-1 xl:grid-cols-2 gap-6 mb-6">
              <div className="card p-5 border-border-soft/50 shadow-sm">
                <div className="flex items-center space-x-2 mb-4">
                  <Monitor size={16} className="text-muted" />
                  <h3 className="text-[13px] font-bold text-heading">分发矩阵 (已挂载工具)</h3>
                </div>
                <div className="flex flex-wrap gap-2">
                  {allTools.map(tool => {
                    const isSupported = selectedMCP.enabled_in.includes(tool.id)
                    const meta = getToolMeta(tool.id)
                    return (
                      <div key={tool.id} className={`flex items-center space-x-2 px-3 py-1.5 rounded-lg border text-[13px] transition-all ${isSupported ? 'bg-surface border-brand-green/30 text-text shadow-sm' : 'bg-page border-border text-muted/50'}`}>
                        {isSupported ? <CheckCircle size={14} className="text-brand-green" /> : <div className="w-[14px] h-[14px] rounded-full border border-muted/30" />}
                        <span className="font-medium">{meta.displayName}</span>
                      </div>
                    )
                  })}
                </div>
                {allTools.length === 0 && (
                  <p className="text-[12px] text-muted">尚未检测到工具，请先扫描</p>
                )}
              </div>
            </div>

            <div className="card border-border-soft/50 shadow-sm overflow-hidden">
               <div className="flex justify-between items-center px-5 py-4 border-b border-border bg-surface">
                 <div className="flex items-center space-x-2">
                   <Terminal size={16} className="text-muted" />
                   <h3 className="text-[13px] font-bold text-heading">配置文件 Payload</h3>
                 </div>
                 <span className="text-[11px] font-mono text-muted bg-page px-2 py-1 rounded border border-border shadow-inner max-w-[300px] truncate">
                   {selectedMCP.origin_path}
                 </span>
               </div>
               <div className="p-5 bg-[#0d1117]">
                 <pre className="text-[13px] text-[#e6edf3] font-mono overflow-x-auto custom-scrollbar">
                   {(() => {
                     try {
                       return JSON.stringify(JSON.parse(selectedMCP.payload_json), null, 2)
                     } catch {
                       return selectedMCP.payload_json
                     }
                   })()}
                 </pre>
               </div>
            </div>

          </div>
        ) : (
          <div className="h-full flex flex-col items-center justify-center text-muted fade-in">
            <Server size={48} className="mb-4 opacity-20" />
            <p className="text-[15px] font-medium">请在左侧选择一个 MCP 服务器查看详情</p>
            <p className="text-[12px] mt-2 opacity-70">或先运行扫描来发现本地工具中的 MCP 配置</p>
          </div>
        )}
      </div>
    </div>

    {/* 同步下发弹窗 */}
    {syncTarget && (
      <SyncDialog
        resource={syncTarget}
        homeDir={snapshot.home_path}
        onClose={() => setSyncTarget(null)}
      />
    )}

    {/* 添加 MCP 弹窗 */}
    {showAddDialog && (
      <AddMcpDialog
        homeDir={snapshot.home_path}
        onClose={() => setShowAddDialog(false)}
      />
    )}
  </div>
  )
}

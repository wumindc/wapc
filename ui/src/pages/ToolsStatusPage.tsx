import { Monitor } from 'lucide-react'
import type { DesktopSnapshot } from '../types'
import { getToolMeta } from '../types'
import { PageHeader } from '../components/layout/PageHeader'

interface MixedTool {
  id: string
  name: string
  status: 'connected' | 'manual' | 'planning' | 'error'
  type: string
  dataSource: string
}

export function ToolsStatusPage({ 
  snapshot,
  setIsSidebarOpen 
}: { 
  snapshot: DesktopSnapshot;
  setIsSidebarOpen?: (o: boolean) => void;
}) {
  // Build the tool matrix
  const matrix: MixedTool[] = []

  // 1. Add detected tools
  snapshot.detected_tools.forEach(tool => {
    const meta = getToolMeta(tool.id)
    matrix.push({
      id: tool.id,
      name: meta.displayName,
      status: tool.data_dir_exists ? 'connected' : (tool.installed ? 'manual' : 'error'),
      type: 'Local AI Tool',
      dataSource: tool.data_dir || tool.config_dir || '需手动指定路径'
    })
  })

  // 2. Add roadmap tools (if not already detected)
  const roadmapTools = [
    { id: 'claude', type: 'CLI' },
    { id: 'gemini', type: 'CLI' },
    { id: 'codex', type: 'CLI' },
    { id: 'opencode', type: 'CLI' },
    { id: 'cursor', type: 'Editor' },
    { id: 'trae', type: 'Editor' },
    { id: 'qoder', type: 'Editor' },
    { id: 'kiro', type: 'Editor' },
    { id: 'antigravity ide', type: 'IDE' }
  ]

  roadmapTools.forEach(rt => {
    if (!matrix.find(m => m.id === rt.id)) {
      matrix.push({
        id: rt.id,
        name: getToolMeta(rt.id).displayName,
        status: 'planning',
        type: rt.type,
        dataSource: '—'
      })
    }
  })

  return (
    <div className="flex flex-col h-full fade-in">
      <div className="px-6 pt-6 pb-2 border-b border-border bg-page shrink-0">
        <PageHeader 
          title="支持工具矩阵与健康度"
          subtitle="监控本地 AI 编程工具的连接状态与数据流"
          setIsSidebarOpen={setIsSidebarOpen}
        />
      </div>
      
      <div className="flex-1 p-6 overflow-y-auto custom-scrollbar bg-page">
        <div className="card overflow-hidden max-w-6xl mx-auto">
        <div className="overflow-x-auto">
          <table className="w-full text-left border-collapse min-w-[600px]">
            <thead>
              <tr className="bg-surface-soft border-b border-border text-muted text-sm uppercase tracking-wider">
                <th className="p-4 font-semibold">工具名</th>
                <th className="p-4 font-semibold">类型</th>
                <th className="p-4 font-semibold">状态</th>
                <th className="p-4 font-semibold">数据源示例</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-border-soft">
              {matrix.map((tool) => (
                <tr key={tool.id} className="hover:bg-surface-hover transition-colors">
                  <td className="p-4 font-medium text-text">
                    <div className="flex items-center space-x-2">
                      <Monitor size={16} className="text-muted" />
                      <span>{tool.name}</span>
                    </div>
                  </td>
                  <td className="p-4 text-muted text-sm">{tool.type}</td>
                  <td className="p-4">
                    <div className="flex items-center space-x-2">
                      {tool.status === 'connected' && (
                        <><span className="w-2 h-2 rounded-full bg-brand-green shadow-[0_0_8px_rgba(38,151,91,0.5)]"></span><span className="text-brand-green text-sm font-medium">已连接</span></>
                      )}
                      {tool.status === 'manual' && (
                        <><span className="w-2 h-2 rounded-full bg-brand-blue"></span><span className="text-brand-blue text-sm font-medium">需手动配置</span></>
                      )}
                      {tool.status === 'error' && (
                        <><span className="w-2 h-2 rounded-full bg-brand-red"></span><span className="text-brand-red text-sm font-medium">检测异常</span></>
                      )}
                      {tool.status === 'planning' && (
                        <><span className="w-2 h-2 rounded-full bg-slate-500"></span><span className="text-slate-500 text-sm font-medium">规划支持中</span></>
                      )}
                    </div>
                  </td>
                  <td className="p-4 text-muted font-mono text-[13px] break-all max-w-[300px]">
                    {tool.dataSource}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        </div>
      </div>
    </div>
  )
}

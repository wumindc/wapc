/**
 * AgentsPage — AGENTS.md 等开发规范资产管理
 * 使用 SyncDialog 实现真实的跨工具下发
 * @author Claude Sonnet 4.6 (Thinking)
 */
import { useState } from 'react'
import { PageHeader } from '../components/layout/PageHeader'
import type { DesktopSnapshot, CanonicalResource } from '../types'
import { FileText, Plus, Send } from 'lucide-react'
import { message } from '@tauri-apps/plugin-dialog'
import { SyncDialog } from '../components/sync/SyncDialog'

export function AgentsPage({ 
  snapshot,
  setIsSidebarOpen
}: { 
  snapshot: DesktopSnapshot;
  setIsSidebarOpen?: (o: boolean) => void;
}) {
  const agentsResources = snapshot.resources.filter(
    r => r.kind === 'instruction' || r.name.toLowerCase().includes('agents.md') || r.name.toLowerCase().includes('claude.md') || r.name.toLowerCase().includes('gemini.md') || r.name.toLowerCase().includes('cursorrules')
  )

  const [syncTarget, setSyncTarget] = useState<CanonicalResource | null>(null)

  return (
    <div className="flex flex-col h-full fade-in bg-page">
      <div className="px-6 pt-6 pb-2 border-b border-border bg-page">
        <div className="flex justify-between items-end">
          <PageHeader 
            title="Agents 规范管理"
            subtitle="统一开发规范沉淀与工程下发"
            setIsSidebarOpen={setIsSidebarOpen}
          />
          <div className="pb-4">
            <button 
              onClick={() => void message('新建 Agents 规范功能即将上线', { title: '提示', kind: 'info' })}
              className="px-4 h-9 flex items-center justify-center bg-brand-blue hover:bg-brand-blue/90 text-white rounded-lg text-[13px] font-medium shadow-sm transition-colors"
            >
              <Plus size={16} className="mr-1" /> 新建规范
            </button>
          </div>
        </div>
      </div>

      <div className="flex-1 p-6 overflow-y-auto custom-scrollbar">
        {agentsResources.length === 0 ? (
          <div className="h-full flex flex-col items-center justify-center text-muted">
            <FileText size={48} className="mb-4 opacity-20" />
            <p className="text-[15px] font-medium">尚未扫描到任何开发规范文件</p>
            <p className="text-[12px] mt-2 opacity-70">AGENTS.md、CLAUDE.md、.cursorrules 等文件扫描后将在此展示</p>
          </div>
        ) : (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
            {agentsResources.map(agent => (
              <div key={agent.id} className="card p-5 border border-border-soft hover:border-brand-blue/50 transition-colors shadow-sm group">
                <div className="flex items-start gap-3 mb-4">
                  <div className="p-2.5 bg-brand-blue/10 text-brand-blue rounded-xl shrink-0">
                    <FileText size={20} />
                  </div>
                  <div className="min-w-0 flex-1">
                    <h3 className="font-bold text-heading group-hover:text-brand-blue transition-colors truncate">
                      {agent.name}
                    </h3>
                    <p className="text-[11px] text-muted mt-0.5 truncate" title={agent.origin_path}>{agent.origin_path}</p>
                  </div>
                </div>

                {/* 已挂载工具标签 */}
                {agent.enabled_in.length > 0 && (
                  <div className="flex flex-wrap gap-1 mb-4">
                    {agent.enabled_in.map(t => (
                      <span key={t} className="text-[11px] px-2 py-0.5 rounded-full bg-brand-green/10 text-brand-green border border-brand-green/20">{t}</span>
                    ))}
                  </div>
                )}

                <div className="flex justify-between items-center pt-4 border-t border-border-soft">
                  <span className="text-[11px] text-muted">{agent.scope} · {agent.kind}</span>
                  <button
                    onClick={() => setSyncTarget(agent)}
                    className="flex items-center gap-1.5 text-[12px] text-brand-blue font-medium hover:text-brand-blue/80 transition-colors"
                  >
                    <Send size={13} />
                    <span>同步下发</span>
                  </button>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* 同步下发弹窗 */}
      {syncTarget && (
        <SyncDialog
          resource={syncTarget}
          homeDir={snapshot.home_path}
          onClose={() => setSyncTarget(null)}
        />
      )}
    </div>
  )
}

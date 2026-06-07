/**
 * SkillsPage — Skills/Prompt 技能资产管理
 * 支持查看详情并通过 SyncDialog 真实下发到其他工具
 * @author Claude Sonnet 4.6 (Thinking)
 */
import { useState } from 'react'
import { Filter, Code, Monitor, X, Send, Wand2 } from 'lucide-react'
import { message } from '@tauri-apps/plugin-dialog'
import type { DesktopSnapshot, CanonicalResource } from '../types'
import { getToolMeta } from '../types'
import { PageHeader } from '../components/layout/PageHeader'
import { SyncDialog } from '../components/sync/SyncDialog'

export function SkillsPage({ 
  snapshot,
  setIsSidebarOpen
}: { 
  snapshot: DesktopSnapshot;
  setIsSidebarOpen?: (o: boolean) => void;
}) {
  const rawSkillResources = snapshot.resources.filter(r => r.kind.includes('skill') || r.kind.includes('prompt'))
  
  // Deduplicate by name and merge enabled_in
  const skillResources = Array.from(
    rawSkillResources.reduce((map, skill) => {
      if (!map.has(skill.name)) {
        map.set(skill.name, { ...skill, enabled_in: [...skill.enabled_in] })
      } else {
        const existing = map.get(skill.name)!;
        existing.enabled_in = Array.from(new Set([...existing.enabled_in, ...skill.enabled_in]))
      }
      return map;
    }, new Map<string, CanonicalResource>()).values()
  )
  const [selectedSkill, setSelectedSkill] = useState<CanonicalResource | null>(null)
  const [syncTarget, setSyncTarget] = useState<CanonicalResource | null>(null)

  return (
    <>
      <div className="relative h-full flex flex-col fade-in">
        <div className="px-6 pt-6 pb-2 border-b border-border bg-page z-0">
          <div className="flex justify-between items-end mb-4">
            <div className="flex-1">
              <PageHeader 
                title="Skills 技能管理库"
                subtitle="Prompt 与 Agent 技能资产流转"
                setIsSidebarOpen={setIsSidebarOpen}
              />
            </div>
            <div className="flex space-x-2 pb-6">
              <button className="p-2 h-9 bg-surface hover:bg-surface-hover border border-border rounded-lg text-text transition-colors shadow-sm">
                <Filter size={16} />
              </button>
              <button 
                onClick={() => void message('导入 Skill 功能即将开发', { title: '提示', kind: 'info' })}
                className="px-4 h-9 flex items-center justify-center bg-brand-blue hover:bg-brand-blue/90 text-white rounded-lg text-[13px] font-medium shadow-sm transition-colors"
              >
                导入 Skill
              </button>
            </div>
          </div>
        </div>

        <div className="flex-1 p-6 overflow-y-auto custom-scrollbar bg-page">
          {skillResources.length === 0 ? (
            <div className="h-full flex flex-col items-center justify-center text-muted">
              <Code size={48} className="mb-4 opacity-20" />
              <p className="text-[15px] font-medium">尚未扫描到任何 Skill 或 Prompt 资源</p>
              <p className="text-[12px] mt-2 opacity-70">请先运行扫描，或导入 Skill 文件</p>
            </div>
          ) : (
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
              {skillResources.map(skill => {
                return (
                  <div 
                    key={skill.id} 
                    onClick={() => setSelectedSkill(skill)}
                    className="group relative bg-surface border border-border rounded-2xl p-5 hover:border-brand-blue/50 hover:shadow-card-hover cursor-pointer transition-all overflow-hidden"
                  >
                    <div className="absolute top-0 left-0 w-full h-1 bg-gradient-to-r from-brand-blue/0 via-brand-blue/0 to-brand-blue/0 group-hover:from-brand-blue/20 group-hover:via-brand-blue/50 group-hover:to-brand-purple/50 transition-all duration-500"></div>
                    <div className="flex justify-between items-start mb-4">
                      <div className="p-2.5 bg-brand-purple/10 text-brand-purple rounded-xl group-hover:bg-brand-purple/20 transition-colors shadow-inner">
                        <Code size={20} />
                      </div>
                      <span className={`text-[11px] font-medium px-2 py-0.5 rounded-md border ${skill.scope === 'project' ? 'bg-brand-blue/10 text-brand-blue border-brand-blue/20' : 'bg-page text-muted border-border'}`}>
                        {skill.scope}
                      </span>
                    </div>
                    <h3 className="text-[15px] font-bold text-heading mb-1.5 line-clamp-1 group-hover:text-brand-blue transition-colors" title={skill.name}>{skill.name}</h3>
                    <div className="flex items-center justify-between mt-5 pt-4 border-t border-border-soft">
                      <div className="flex items-center space-x-1.5 text-[12px] text-muted font-medium">
                        <Monitor size={14} className="opacity-70"/> 
                        <div className="flex gap-1 items-center">
                          {skill.enabled_in.slice(0, 2).map(t => (
                             <span key={t}>{getToolMeta(t).displayName}</span>
                          ))}
                          {skill.enabled_in.length > 2 && (
                            <span>等 {skill.enabled_in.length} 项</span>
                          )}
                        </div>
                      </div>
                      <div className="text-[11px] text-muted bg-page px-2 py-1 rounded-md border border-border-soft font-mono">
                         {skill.kind}
                      </div>
                    </div>
                  </div>
                )
              })}
            </div>
          )}
        </div>

        {/* Right Drawer Overlay */}
        {selectedSkill && (
          <div 
            className="absolute inset-0 bg-black/20 backdrop-blur-sm z-20 transition-opacity fade-in"
            onClick={() => setSelectedSkill(null)}
          />
        )}

        {/* Right Drawer for Detail */}
        <div className={`absolute top-0 right-0 h-full w-[400px] max-w-full bg-surface border-l border-border shadow-2xl transform transition-transform duration-300 ease-in-out flex flex-col z-30 ${selectedSkill ? 'translate-x-0' : 'translate-x-full'}`}>
           {selectedSkill && (
             <>
              <div className="flex items-center justify-between p-5 border-b border-border bg-page">
                <div className="flex items-center gap-3 overflow-hidden">
                  <div className="p-2 bg-brand-purple/10 text-brand-purple rounded-lg shrink-0">
                    <Wand2 size={18} />
                  </div>
                  <h3 className="text-[15px] font-bold text-heading truncate">{selectedSkill.name}</h3>
                </div>
                <button onClick={() => setSelectedSkill(null)} className="p-1.5 text-muted hover:text-text hover:bg-surface-hover rounded-md transition-colors shrink-0">
                  <X size={18} />
                </button>
              </div>
              
              <div className="p-6 overflow-y-auto flex-1 custom-scrollbar">
                <div className="space-y-6">
                  <div>
                    <div className="text-[12px] font-semibold text-muted mb-2 uppercase tracking-wider">来源工具</div>
                    <div className="text-[13px] text-text bg-page px-3 py-2.5 rounded-lg border border-border flex items-center shadow-sm">
                      {getToolMeta(selectedSkill.origin_tool).displayName}
                    </div>
                  </div>
                  <div>
                    <div className="text-[12px] font-semibold text-muted mb-2 uppercase tracking-wider">文件路径</div>
                    <div className="text-[12px] text-muted bg-page px-3 py-2.5 rounded-lg border border-border break-all font-mono shadow-sm">
                      {selectedSkill.origin_path}
                    </div>
                  </div>
                  <div className="flex-1 flex flex-col">
                    <div className="text-[12px] font-semibold text-muted mb-2 uppercase tracking-wider">Content Payload</div>
                    <div className="bg-[#0d1117] border border-border rounded-xl p-4 font-mono text-[13px] text-[#e6edf3] h-72 overflow-y-auto custom-scrollbar whitespace-pre-wrap break-all shadow-inner">
                      {(() => {
                        try {
                          return JSON.stringify(JSON.parse(selectedSkill.payload_json), null, 2)
                        } catch {
                          return selectedSkill.payload_json
                        }
                      })()}
                    </div>
                  </div>
                </div>
              </div>
              
              <div className="p-5 border-t border-border bg-page">
                 <button
                   onClick={() => { setSyncTarget(selectedSkill); setSelectedSkill(null); }}
                   className="w-full h-10 bg-brand-blue hover:bg-brand-blue/90 text-white text-[14px] font-medium rounded-lg transition-colors flex items-center justify-center shadow-sm"
                 >
                   <Send size={16} className="mr-2" /> 同步下发到其他工具
                 </button>
              </div>
             </>
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
    </>
  )
}

/**
 * SyncDialog — 跨工具下发弹窗组件
 * 流程: 选择目标工具 → plan_sync 获取 diff 预览 → apply_sync 执行写入
 * @author Claude Sonnet 4.6 (Thinking)
 */
import { useState, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { X, ChevronRight, AlertTriangle, CheckCircle2, Loader2, FolderOpen, Diff } from 'lucide-react'
import type {
  CanonicalResource,
  PlanSyncRequest,
  PlanSyncResult,
  SyncTargetPlan,
  WritePlan,
  ApplySyncRequest,
  ApplySyncResult,
  SyncTarget,
} from '../../types'

// 每个工具的默认配置文件格式和路径规则
const TOOL_CONFIG: Record<string, { label: string; format: string; userScopePath: (home: string) => string; projectScopePath: (dir: string) => string; supportsProject: boolean; supportsUser: boolean }> = {
  claude: {
    label: 'Claude Code',
    format: 'json',
    userScopePath: (home) => `${home}/.claude.json`,
    projectScopePath: (dir) => `${dir}/.mcp.json`,
    supportsUser: true,
    supportsProject: true,
  },
  cursor: {
    label: 'Cursor',
    format: 'json',
    userScopePath: (home) => `${home}/.cursor/mcp.json`,
    projectScopePath: (dir) => `${dir}/.cursor/mcp.json`,
    supportsUser: true,
    supportsProject: true,
  },
  gemini: {
    label: 'Gemini CLI',
    format: 'json',
    userScopePath: (home) => `${home}/.gemini/settings.json`,
    projectScopePath: (dir) => `${dir}/.gemini/settings.json`,
    supportsUser: true,
    supportsProject: false,
  },
  codex: {
    label: 'Codex',
    format: 'toml',
    userScopePath: (home) => `${home}/.codex/config.toml`,
    projectScopePath: (dir) => `${dir}/.codex/config.toml`,
    supportsUser: true,
    supportsProject: true,
  },
  opencode: {
    label: 'OpenCode',
    format: 'json',
    userScopePath: (home) => `${home}/.config/opencode/opencode.json`,
    projectScopePath: (dir) => `${dir}/opencode.json`,
    supportsUser: true,
    supportsProject: true,
  },
}

export type SyncResourceKind = 'mcp' | 'instruction' | 'skill'

interface TargetSelection {
  tool: string
  scope: 'user' | 'project'
  projectPath?: string
}

interface PlanResult {
  selection: TargetSelection
  targetPlan: SyncTargetPlan
}

type Step = 'select' | 'preview' | 'applying' | 'done' | 'error'

interface SyncDialogProps {
  resource: CanonicalResource
  onClose: () => void
  homeDir: string
}

export function SyncDialog({ resource, onClose, homeDir }: SyncDialogProps) {
  const [step, setStep] = useState<Step>('select')
  const [selectedTargets, setSelectedTargets] = useState<TargetSelection[]>([])
  const [planResults, setPlanResults] = useState<PlanResult[]>([])
  const [applyResult, setApplyResult] = useState<ApplySyncResult | null>(null)
  const [errorMessage, setErrorMessage] = useState<string>('')
  const [planning, setPlanning] = useState(false)

  // 已挂载到哪些工具（用来标记"已同步"状态）
  const enabledTools = new Set(resource.enabled_in)

  const supportedTools = Object.entries(TOOL_CONFIG).filter(([, cfg]) =>
    resource.kind === 'mcp' || resource.kind === 'instruction'
      ? true
      : cfg.supportsUser
  )

  const toggleTarget = useCallback((tool: string, scope: 'user' | 'project') => {
    setSelectedTargets(prev => {
      const key = `${tool}:${scope}`
      const exists = prev.find(t => `${t.tool}:${t.scope}` === key)
      if (exists) return prev.filter(t => `${t.tool}:${t.scope}` !== key)
      return [...prev, { tool, scope }]
    })
  }, [])

  const pickProjectDir = useCallback(async (tool: string) => {
    const dir = await open({ directory: true, multiple: false, title: `选择 ${TOOL_CONFIG[tool]?.label ?? tool} 项目目录` })
    if (typeof dir !== 'string') return
    setSelectedTargets(prev => {
      const filtered = prev.filter(t => !(t.tool === tool && t.scope === 'project'))
      return [...filtered, { tool, scope: 'project', projectPath: dir }]
    })
  }, [])

  const buildSyncTargets = useCallback((): SyncTarget[] => {
    return selectedTargets.map(sel => {
      const cfg = TOOL_CONFIG[sel.tool]
      const targetPath = sel.scope === 'user'
        ? cfg.userScopePath(homeDir)
        : cfg.projectScopePath(sel.projectPath ?? '')
      return {
        tool: sel.tool,
        scope: sel.scope,
        project_path: sel.scope === 'project' ? (sel.projectPath ?? null) : null,
        target_path: targetPath,
        format: cfg.format,
      }
    })
  }, [selectedTargets, homeDir])

  const handlePlan = useCallback(async () => {
    if (selectedTargets.length === 0) return
    setPlanning(true)
    try {
      const targets = buildSyncTargets()
      const req: PlanSyncRequest = {
        resource_id: resource.id,
        targets,
        allow_cross_scope: true,
        env_strategy: 'skip',
      }
      const result = await invoke<PlanSyncResult>('plan_sync', { request: req })
      const results: PlanResult[] = result.targets.map((tp, i) => ({
        selection: selectedTargets[i],
        targetPlan: tp,
      }))
      setPlanResults(results)
      setStep('preview')
    } catch (err) {
      setErrorMessage(err instanceof Error ? err.message : String(err))
      setStep('error')
    } finally {
      setPlanning(false)
    }
  }, [selectedTargets, buildSyncTargets, resource.id])

  const handleApply = useCallback(async () => {
    const plans: WritePlan[] = planResults
      .filter(r => r.targetPlan.status === 'planned' && r.targetPlan.plan != null)
      .map(r => r.targetPlan.plan!)
    if (plans.length === 0) return

    setStep('applying')
    try {
      const req: ApplySyncRequest = {
        plans,
        confirm_drift: true,
        allow_cross_scope: true,
        env_strategy: 'skip',
      }
      const result = await invoke<ApplySyncResult>('apply_sync', { request: req })
      setApplyResult(result)
      setStep('done')
    } catch (err) {
      setErrorMessage(err instanceof Error ? err.message : String(err))
      setStep('error')
    }
  }, [planResults])

  // 点击遮罩关闭
  const handleBackdropClick = (e: React.MouseEvent<HTMLDivElement>) => {
    if (e.target === e.currentTarget) onClose()
  }

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm fade-in"
      onClick={handleBackdropClick}
    >
      <div className="relative w-full max-w-2xl max-h-[85vh] flex flex-col bg-surface border border-border rounded-2xl shadow-2xl overflow-hidden">
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-border bg-page shrink-0">
          <div>
            <h2 className="text-[16px] font-bold text-heading">同步下发</h2>
            <p className="text-[12px] text-muted mt-0.5">
              将 <span className="font-mono text-text">{resource.name}</span> 分发到其他工具
            </p>
          </div>
          <button onClick={onClose} className="p-1.5 rounded-lg text-muted hover:text-text hover:bg-surface-hover transition-colors">
            <X size={18} />
          </button>
        </div>

        {/* Step Indicator */}
        <div className="flex items-center px-6 py-3 border-b border-border-soft bg-surface shrink-0">
          {(['select', 'preview', 'done'] as const).map((s, i) => {
            const labels = ['选择目标', '预览变更', '完成']
            const isActive = step === s || (step === 'applying' && s === 'preview') || (step === 'error' && i <= ['select','preview','done'].indexOf(step))
            const isDone = ['select', 'preview', 'applying', 'done'].indexOf(step) > i
            return (
              <div key={s} className="flex items-center">
                <div className={`flex items-center gap-1.5 text-[12px] font-medium ${isDone ? 'text-brand-green' : isActive ? 'text-brand-blue' : 'text-muted'}`}>
                  <span className={`w-5 h-5 rounded-full flex items-center justify-center text-[11px] border ${isDone ? 'bg-brand-green border-brand-green text-white' : isActive ? 'bg-brand-blue/10 border-brand-blue text-brand-blue' : 'border-border text-muted'}`}>
                    {isDone ? <CheckCircle2 size={12} /> : i + 1}
                  </span>
                  {labels[i]}
                </div>
                {i < 2 && <ChevronRight size={14} className="mx-2 text-muted" />}
              </div>
            )
          })}
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto custom-scrollbar p-6">

          {/* STEP: select */}
          {step === 'select' && (
            <div className="space-y-3">
              <p className="text-[13px] text-muted mb-4">选择要将此资源下发的目标工具和作用域。已检测到的工具会标记来源。</p>
              {supportedTools.map(([toolId, cfg]) => {
                const isEnabled = enabledTools.has(toolId)
                const userSel = selectedTargets.find(t => t.tool === toolId && t.scope === 'user')
                const projSel = selectedTargets.find(t => t.tool === toolId && t.scope === 'project')
                return (
                  <div key={toolId} className={`border rounded-xl p-4 transition-colors ${isEnabled ? 'border-brand-green/30 bg-brand-green/5' : 'border-border bg-surface'}`}>
                    <div className="flex items-center justify-between mb-3">
                      <div className="flex items-center gap-2">
                        <span className="text-[14px] font-semibold text-text">{cfg.label}</span>
                        {isEnabled && (
                          <span className="text-[11px] px-2 py-0.5 rounded-full bg-brand-green/10 text-brand-green border border-brand-green/20 font-medium">已挂载</span>
                        )}
                      </div>
                    </div>
                    <div className="flex flex-wrap gap-2">
                      {cfg.supportsUser && (
                        <button
                          onClick={() => toggleTarget(toolId, 'user')}
                          className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg border text-[12px] font-medium transition-colors ${userSel ? 'bg-brand-blue/10 border-brand-blue/50 text-brand-blue' : 'border-border text-muted hover:border-brand-blue/30 hover:text-text'}`}
                        >
                          {userSel ? <CheckCircle2 size={13} /> : <span className="w-3.5 h-3.5 rounded border border-current" />}
                          用户级 (User)
                        </button>
                      )}
                      {cfg.supportsProject && (
                        <button
                          onClick={() => { void pickProjectDir(toolId) }}
                          className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg border text-[12px] font-medium transition-colors ${projSel ? 'bg-brand-purple/10 border-brand-purple/50 text-brand-purple' : 'border-border text-muted hover:border-brand-purple/30 hover:text-text'}`}
                        >
                          {projSel ? <CheckCircle2 size={13} /> : <FolderOpen size={13} />}
                          项目级 (Project){projSel?.projectPath ? ` — ${projSel.projectPath.split('/').slice(-1)[0]}` : ''}
                        </button>
                      )}
                    </div>
                  </div>
                )
              })}
            </div>
          )}

          {/* STEP: preview */}
          {(step === 'preview' || step === 'applying') && (
            <div className="space-y-4">
              {planResults.map((r, i) => (
                <PlanPreviewCard key={i} result={r} />
              ))}
            </div>
          )}

          {/* STEP: done */}
          {step === 'done' && applyResult && (
            <div className="space-y-3">
              <div className="flex items-center gap-3 p-4 rounded-xl bg-brand-green/10 border border-brand-green/30">
                <CheckCircle2 size={20} className="text-brand-green shrink-0" />
                <div>
                  <p className="text-[14px] font-semibold text-heading">下发成功</p>
                  <p className="text-[12px] text-muted">同步 ID: {applyResult.sync_id}</p>
                </div>
              </div>
              {applyResult.changes.map(c => (
                <div key={c.plan_id} className={`p-3 rounded-lg border text-[13px] flex items-center gap-2 ${c.status === 'applied' ? 'border-brand-green/20 text-text' : 'border-red-900/20 text-red-400'}`}>
                  {c.status === 'applied' ? <CheckCircle2 size={15} className="text-brand-green shrink-0" /> : <AlertTriangle size={15} className="shrink-0" />}
                  <span className="font-mono break-all text-[12px] text-muted flex-1">{c.target_path}</span>
                  <span className={`shrink-0 font-medium ${c.status === 'applied' ? 'text-brand-green' : 'text-red-400'}`}>{c.status}</span>
                </div>
              ))}
            </div>
          )}

          {/* STEP: applying indicator */}
          {step === 'applying' && (
            <div className="flex items-center justify-center gap-3 py-4 text-brand-blue">
              <Loader2 size={18} className="animate-spin" />
              <span className="text-[13px] font-medium">正在写入配置文件...</span>
            </div>
          )}

          {/* STEP: error */}
          {step === 'error' && (
            <div className="flex items-start gap-3 p-4 rounded-xl bg-red-900/10 border border-red-900/30">
              <AlertTriangle size={18} className="text-red-400 shrink-0 mt-0.5" />
              <div>
                <p className="text-[14px] font-semibold text-red-400">操作失败</p>
                <p className="text-[12px] text-muted mt-1 break-all">{errorMessage}</p>
              </div>
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="flex items-center justify-end gap-3 px-6 py-4 border-t border-border bg-page shrink-0">
          {step === 'select' && (
            <>
              <button onClick={onClose} className="px-4 h-9 rounded-lg border border-border text-[13px] text-muted hover:text-text hover:bg-surface-hover transition-colors">取消</button>
              <button
                onClick={() => void handlePlan()}
                disabled={selectedTargets.length === 0 || planning}
                className="px-5 h-9 rounded-lg bg-brand-blue text-white text-[13px] font-medium hover:bg-brand-blue/90 disabled:opacity-50 disabled:cursor-not-allowed transition-colors flex items-center gap-2"
              >
                {planning && <Loader2 size={14} className="animate-spin" />}
                {planning ? '规划中...' : `预览变更 (${selectedTargets.length} 个目标)`}
              </button>
            </>
          )}
          {step === 'preview' && (
            <>
              <button onClick={() => setStep('select')} className="px-4 h-9 rounded-lg border border-border text-[13px] text-muted hover:text-text hover:bg-surface-hover transition-colors">返回</button>
              <button
                onClick={() => void handleApply()}
                disabled={!planResults.some(r => r.targetPlan.status === 'planned')}
                className="px-5 h-9 rounded-lg bg-brand-blue text-white text-[13px] font-medium hover:bg-brand-blue/90 disabled:opacity-50 transition-colors"
              >
                确认写入
              </button>
            </>
          )}
          {(step === 'done' || step === 'error') && (
            <button onClick={onClose} className="px-5 h-9 rounded-lg bg-surface border border-border text-[13px] text-text hover:bg-surface-hover transition-colors">关闭</button>
          )}
        </div>
      </div>
    </div>
  )
}

// ── Plan Preview Card ─────────────────────────────────────────────────────────
function PlanPreviewCard({ result }: { result: PlanResult }) {
  const { selection, targetPlan } = result
  const [showDiff, setShowDiff] = useState(false)
  const cfg = TOOL_CONFIG[selection.tool]

  const statusColor = targetPlan.status === 'planned' ? 'text-brand-blue' : 'text-muted'
  const statusBg = targetPlan.status === 'planned' ? 'bg-brand-blue/10 border-brand-blue/30' : 'bg-surface border-border'

  return (
    <div className={`border rounded-xl overflow-hidden ${statusBg}`}>
      <div className="flex items-center justify-between px-4 py-3">
        <div className="flex items-center gap-2">
          <span className="text-[13px] font-semibold text-text">{cfg?.label ?? selection.tool}</span>
          <span className="text-[11px] px-1.5 py-0.5 rounded bg-surface border border-border text-muted">{selection.scope}</span>
        </div>
        <div className="flex items-center gap-2">
          <span className={`text-[12px] font-medium ${statusColor}`}>{targetPlan.status}</span>
          {targetPlan.status === 'planned' && targetPlan.plan && (
            <button
              onClick={() => setShowDiff(v => !v)}
              className="flex items-center gap-1 text-[11px] text-muted hover:text-brand-blue transition-colors"
            >
              <Diff size={12} />
              {showDiff ? '收起' : '查看变更'}
            </button>
          )}
        </div>
      </div>
      {targetPlan.reason && (
        <div className="px-4 pb-3 text-[12px] text-muted">{targetPlan.reason}</div>
      )}
      {targetPlan.plan && (
        <div className="px-4 pb-1 text-[11px] font-mono text-muted break-all">{targetPlan.plan.target_path}</div>
      )}
      {showDiff && targetPlan.plan && (
        <div className="border-t border-border mx-0">
          <DiffViewer diff={targetPlan.plan.diff} />
        </div>
      )}
    </div>
  )
}

// ── Diff Viewer ───────────────────────────────────────────────────────────────
function DiffViewer({ diff }: { diff: string }) {
  const lines = diff.split('\n')
  return (
    <div className="bg-[#0d1117] p-4 overflow-x-auto custom-scrollbar max-h-64">
      <pre className="text-[12px] font-mono">
        {lines.map((line, i) => {
          let color = 'text-[#8b949e]'
          if (line.startsWith('+')) color = 'text-green-400'
          else if (line.startsWith('-')) color = 'text-red-400'
          else if (line.startsWith('@')) color = 'text-blue-400'
          return (
            <span key={i} className={`block ${color}`}>{line || '\u00a0'}</span>
          )
        })}
      </pre>
    </div>
  )
}

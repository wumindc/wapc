/**
 * AddMcpDialog — 手动添加 MCP 服务器弹窗
 * 支持填写 MCP 配置并选择写入目标工具的配置文件
 * @author Claude Sonnet 4.6 (Thinking)
 */
import { useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { X, Plus, Loader2, CheckCircle2, AlertTriangle } from 'lucide-react'
import type { WritePlan, ApplySyncRequest, ApplySyncResult } from '../../types'

interface McpEntry {
  name: string
  transport: 'stdio' | 'http' | 'sse'
  command: string        // for stdio
  args: string           // for stdio, space-separated
  url: string            // for http/sse
  env: string            // JSON string of env vars
}

// 目标工具与其配置文件格式（只列支持 json 写入的主流工具）
const TARGET_TOOLS = [
  { tool: 'claude', label: 'Claude Code', format: 'json', scope: 'user' as const },
  { tool: 'cursor', label: 'Cursor', format: 'json', scope: 'user' as const },
  { tool: 'gemini', label: 'Gemini CLI', format: 'json', scope: 'user' as const },
]

type Step = 'form' | 'applying' | 'done' | 'error'

interface AddMcpDialogProps {
  homeDir: string
  onClose: () => void
}

export function AddMcpDialog({ homeDir, onClose }: AddMcpDialogProps) {
  const [step, setStep] = useState<Step>('form')
  const [entry, setEntry] = useState<McpEntry>({
    name: '',
    transport: 'stdio',
    command: '',
    args: '',
    url: '',
    env: '',
  })
  const [selectedTools, setSelectedTools] = useState<string[]>([])
  const [applyResult, setApplyResult] = useState<ApplySyncResult | null>(null)
  const [errorMessage, setErrorMessage] = useState('')

  const targetPathFor = (tool: string): string => {
    switch (tool) {
      case 'claude': return `${homeDir}/.claude.json`
      case 'cursor': return `${homeDir}/.cursor/mcp.json`
      case 'gemini': return `${homeDir}/.gemini/settings.json`
      default: return ''
    }
  }

  const buildPayload = () => {
    if (entry.transport === 'stdio') {
      const args = entry.args.trim() ? entry.args.trim().split(/\s+/) : []
      let env: Record<string, string> = {}
      try { env = JSON.parse(entry.env || '{}') } catch { /* ignore */ }
      return { command: entry.command, ...(args.length ? { args } : {}), ...(Object.keys(env).length ? { env } : {}) }
    } else {
      return { url: entry.url, type: entry.transport }
    }
  }

  const validate = (): string | null => {
    if (!entry.name.trim()) return 'MCP 名称不能为空'
    if (entry.transport === 'stdio' && !entry.command.trim()) return '命令行 (command) 不能为空'
    if ((entry.transport === 'http' || entry.transport === 'sse') && !entry.url.trim()) return 'URL 不能为空'
    if (selectedTools.length === 0) return '请至少选择一个目标工具'
    return null
  }

  const handleApply = async () => {
    const err = validate()
    if (err) { setErrorMessage(err); return }
    setErrorMessage('')
    setStep('applying')

    try {
      const payload = buildPayload()

      // 构造 WritePlan 列表（每个目标工具一个 plan）
      const plans: WritePlan[] = await Promise.all(
        selectedTools.map(async (tool) => {
          const targetPath = targetPathFor(tool)
          const req = {
            tool,
            kind: 'mcp',
            op: 'add',
            resource_id: null,
            resource_name: entry.name.trim(),
            target_path: targetPath,
            scope: 'user',
            payload_json: JSON.stringify({ [entry.name.trim()]: payload }),
          }
          // Use plan_resource_change to get a proper WritePlan
          return await invoke<WritePlan>('plan_resource_change', { request: req })
        })
      )

      const applyReq: ApplySyncRequest = {
        plans,
        confirm_drift: true,
        allow_cross_scope: true,
        env_strategy: 'skip',
      }
      const result = await invoke<ApplySyncResult>('apply_sync', { request: applyReq })
      setApplyResult(result)
      setStep('done')
    } catch (err) {
      setErrorMessage(err instanceof Error ? err.message : String(err))
      setStep('error')
    }
  }

  const toggleTool = (tool: string) => {
    setSelectedTools(prev => prev.includes(tool) ? prev.filter(t => t !== tool) : [...prev, tool])
  }

  const handleBackdropClick = (e: React.MouseEvent<HTMLDivElement>) => {
    if (e.target === e.currentTarget) onClose()
  }

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm fade-in"
      onClick={handleBackdropClick}
    >
      <div className="relative w-full max-w-lg flex flex-col bg-surface border border-border rounded-2xl shadow-2xl overflow-hidden">
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-border bg-page shrink-0">
          <div>
            <h2 className="text-[16px] font-bold text-heading">添加 MCP 服务器</h2>
            <p className="text-[12px] text-muted mt-0.5">手动配置并注入到目标工具</p>
          </div>
          <button onClick={onClose} className="p-1.5 rounded-lg text-muted hover:text-text hover:bg-surface-hover transition-colors">
            <X size={18} />
          </button>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto custom-scrollbar p-6 space-y-5">
          {step === 'form' && (
            <>
              {errorMessage && (
                <div className="flex items-center gap-2 p-3 rounded-lg bg-red-900/10 border border-red-900/30 text-[12px] text-red-400">
                  <AlertTriangle size={14} />
                  {errorMessage}
                </div>
              )}

              {/* Name */}
              <div>
                <label className="block text-[12px] font-semibold text-muted mb-1.5">MCP 名称 *</label>
                <input
                  value={entry.name}
                  onChange={e => setEntry(p => ({ ...p, name: e.target.value }))}
                  placeholder="例如: my-docs-mcp"
                  className="w-full h-9 px-3 rounded-lg border border-border bg-page text-[13px] text-text focus:outline-none focus:border-brand-blue/50 transition-colors"
                />
              </div>

              {/* Transport */}
              <div>
                <label className="block text-[12px] font-semibold text-muted mb-1.5">传输协议 *</label>
                <div className="flex gap-2">
                  {(['stdio', 'http', 'sse'] as const).map(t => (
                    <button
                      key={t}
                      onClick={() => setEntry(p => ({ ...p, transport: t }))}
                      className={`flex-1 h-9 rounded-lg border text-[12px] font-medium transition-colors ${entry.transport === t ? 'bg-brand-blue/10 border-brand-blue/50 text-brand-blue' : 'border-border text-muted hover:border-brand-blue/30'}`}
                    >
                      {t}
                    </button>
                  ))}
                </div>
              </div>

              {/* stdio fields */}
              {entry.transport === 'stdio' && (
                <>
                  <div>
                    <label className="block text-[12px] font-semibold text-muted mb-1.5">命令行 (command) *</label>
                    <input
                      value={entry.command}
                      onChange={e => setEntry(p => ({ ...p, command: e.target.value }))}
                      placeholder="例如: npx 或 /usr/bin/python3"
                      className="w-full h-9 px-3 rounded-lg border border-border bg-page text-[13px] font-mono text-text focus:outline-none focus:border-brand-blue/50 transition-colors"
                    />
                  </div>
                  <div>
                    <label className="block text-[12px] font-semibold text-muted mb-1.5">参数 (args，空格分隔)</label>
                    <input
                      value={entry.args}
                      onChange={e => setEntry(p => ({ ...p, args: e.target.value }))}
                      placeholder="例如: -y @modelcontextprotocol/server-everything"
                      className="w-full h-9 px-3 rounded-lg border border-border bg-page text-[13px] font-mono text-text focus:outline-none focus:border-brand-blue/50 transition-colors"
                    />
                  </div>
                  <div>
                    <label className="block text-[12px] font-semibold text-muted mb-1.5">环境变量 (JSON，可选)</label>
                    <textarea
                      value={entry.env}
                      onChange={e => setEntry(p => ({ ...p, env: e.target.value }))}
                      placeholder='{"API_KEY": "your-key"}'
                      rows={2}
                      className="w-full px-3 py-2 rounded-lg border border-border bg-page text-[12px] font-mono text-text focus:outline-none focus:border-brand-blue/50 transition-colors resize-none"
                    />
                  </div>
                </>
              )}

              {/* http/sse fields */}
              {(entry.transport === 'http' || entry.transport === 'sse') && (
                <div>
                  <label className="block text-[12px] font-semibold text-muted mb-1.5">URL *</label>
                  <input
                    value={entry.url}
                    onChange={e => setEntry(p => ({ ...p, url: e.target.value }))}
                    placeholder="https://your-mcp-server.com/mcp"
                    className="w-full h-9 px-3 rounded-lg border border-border bg-page text-[13px] font-mono text-text focus:outline-none focus:border-brand-blue/50 transition-colors"
                  />
                </div>
              )}

              {/* Target tools */}
              <div>
                <label className="block text-[12px] font-semibold text-muted mb-2">注入目标工具 *</label>
                <div className="space-y-2">
                  {TARGET_TOOLS.map(({ tool, label }) => (
                    <button
                      key={tool}
                      onClick={() => toggleTool(tool)}
                      className={`w-full flex items-center justify-between px-4 py-2.5 rounded-lg border text-[13px] transition-colors ${selectedTools.includes(tool) ? 'bg-brand-blue/10 border-brand-blue/40 text-text' : 'border-border text-muted hover:border-brand-blue/30 hover:text-text'}`}
                    >
                      <span className="font-medium">{label}</span>
                      <div className="flex items-center gap-2">
                        <span className="text-[11px] font-mono text-muted">{targetPathFor(tool)}</span>
                        {selectedTools.includes(tool) && <CheckCircle2 size={14} className="text-brand-blue" />}
                      </div>
                    </button>
                  ))}
                </div>
              </div>
            </>
          )}

          {step === 'applying' && (
            <div className="flex flex-col items-center justify-center py-12 gap-3 text-brand-blue">
              <Loader2 size={28} className="animate-spin" />
              <p className="text-[14px] font-medium">正在写入配置...</p>
            </div>
          )}

          {step === 'done' && applyResult && (
            <div className="space-y-3">
              <div className="flex items-center gap-3 p-4 rounded-xl bg-brand-green/10 border border-brand-green/30">
                <CheckCircle2 size={20} className="text-brand-green" />
                <p className="text-[14px] font-semibold text-heading">MCP 添加成功，需重启工具生效</p>
              </div>
              {applyResult.changes.map(c => (
                <div key={c.plan_id} className={`p-3 rounded-lg border text-[12px] flex items-center gap-2 ${c.status === 'applied' ? 'border-border text-muted' : 'border-red-900/20 text-red-400'}`}>
                  {c.status === 'applied' ? <CheckCircle2 size={13} className="text-brand-green" /> : <AlertTriangle size={13} />}
                  <span className="font-mono break-all flex-1">{c.target_path}</span>
                  <span className="font-medium">{c.status}</span>
                </div>
              ))}
            </div>
          )}

          {step === 'error' && (
            <div className="flex items-start gap-3 p-4 rounded-xl bg-red-900/10 border border-red-900/30">
              <AlertTriangle size={18} className="text-red-400 shrink-0 mt-0.5" />
              <div>
                <p className="text-[14px] font-semibold text-red-400">添加失败</p>
                <p className="text-[12px] text-muted mt-1 break-all">{errorMessage}</p>
              </div>
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="flex items-center justify-end gap-3 px-6 py-4 border-t border-border bg-page shrink-0">
          {step === 'form' && (
            <>
              <button onClick={onClose} className="px-4 h-9 rounded-lg border border-border text-[13px] text-muted hover:text-text hover:bg-surface-hover transition-colors">取消</button>
              <button
                onClick={() => void handleApply()}
                className="px-5 h-9 rounded-lg bg-brand-blue text-white text-[13px] font-medium hover:bg-brand-blue/90 transition-colors flex items-center gap-2"
              >
                <Plus size={14} /> 添加并注入
              </button>
            </>
          )}
          {(step === 'done' || step === 'error') && (
            <button onClick={onClose} className="px-5 h-9 rounded-lg border border-border text-[13px] text-text hover:bg-surface-hover transition-colors">关闭</button>
          )}
        </div>
      </div>
    </div>
  )
}

/**
 * SessionsPage — metadata-only local session browser.
 * @author codex
 */

import { useCallback, useEffect, useMemo, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { History, RefreshCw, Search } from 'lucide-react'
import { ToolBadge } from '../components/ui/ToolBadge'
import { PageHeader } from '../components/layout/PageHeader'
import type { DesktopSnapshot, SessionMeta } from '../types'

function formatDateTime(value: string | null): string {
  if (!value) return '未知'
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return '未知'
  return date.toLocaleString('zh-CN', {
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  })
}

function formatCurrency(value: number): string {
  return `$${value.toFixed(2)}`
}

export function SessionsPage({ 
  snapshot,
  selectedTool,
  setSelectedTool,
  setIsSidebarOpen
}: { 
  snapshot: DesktopSnapshot;
  selectedTool?: string;
  setSelectedTool?: (t: string) => void;
  setIsSidebarOpen?: (o: boolean) => void;
}) {
  const [sessions, setSessions] = useState<SessionMeta[]>([])
  const [tool, setTool] = useState(selectedTool === 'all' ? 'all' : selectedTool || 'all')
  const [project, setProject] = useState('all')
  const [fromDate, setFromDate] = useState('')
  const [toDate, setToDate] = useState('')
  const [query, setQuery] = useState('')
  const [submittedQuery, setSubmittedQuery] = useState('')
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const tools = useMemo(() => snapshot.detected_tools.map(item => item.id).sort(), [snapshot.detected_tools])
  const projects = useMemo(
    () => Array.from(new Set(sessions.map(session => session.project_path).filter(Boolean) as string[])).sort(),
    [sessions],
  )

  const loadSessions = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      const rows = await invoke<SessionMeta[]>('list_sessions', {
        tool: tool === 'all' ? null : tool,
        project: project === 'all' ? null : project,
        from: fromDate ? `${fromDate}T00:00:00+00:00` : null,
        to: toDate ? `${toDate}T23:59:59+00:00` : null,
        query: submittedQuery.trim() || null,
      })
      setSessions(rows)
    } catch (e) {
      setError(String(e))
    } finally {
      setLoading(false)
    }
  }, [fromDate, project, submittedQuery, toDate, tool])

  useEffect(() => {
    void Promise.resolve().then(loadSessions)
  }, [loadSessions])

  return (
    <div className="p-6 h-full flex flex-col fade-in">
      <PageHeader 
        title="会话记录"
        subtitle="浏览所有拦截到的本地会话元数据"
        showToolSelector={true}
        selectedTool={selectedTool}
        setSelectedTool={(t) => {
          setSelectedTool?.(t)
          setTool(t)
        }}
        setIsSidebarOpen={setIsSidebarOpen}
      />

      <div className="card p-4 sm:p-[18px] flex-1 overflow-y-auto custom-scrollbar">
        <div className="flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between mb-4">
          <div>
            <div className="text-[15px] font-bold text-heading">筛选与检索</div>
          </div>
          <button
            onClick={loadSessions}
            disabled={loading}
            className={`h-9 px-3 inline-flex items-center justify-center gap-2 rounded-lg border border-border bg-surface text-[13px] text-text transition-colors ${
              loading ? 'opacity-70 cursor-not-allowed' : 'hover:bg-brand-blue/5 hover:border-brand-blue/30 hover:text-brand-blue'
            }`}
          >
            <RefreshCw size={14} className={loading ? 'animate-spin text-brand-blue' : ''} />
            <span>{loading ? '加载中' : '刷新'}</span>
          </button>
        </div>

        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-[1fr_140px_190px_150px_150px_90px] gap-2 mb-4">
          <label className="relative block">
            <Search size={15} className="absolute left-3 top-1/2 -translate-y-1/2 text-muted" />
            <input
              value={query}
              onChange={event => setQuery(event.target.value)}
              onKeyDown={event => {
                if (event.key === 'Enter') setSubmittedQuery(query)
              }}
              className="w-full h-9 rounded-lg border border-border bg-page pl-9 pr-3 text-[13px] text-text outline-none focus:border-brand-blue/50"
              placeholder="搜索 session_id 或项目路径"
            />
          </label>
          <select
            value={tool}
            onChange={event => setTool(event.target.value)}
            className="h-9 rounded-lg border border-border bg-page px-3 text-[13px] text-text outline-none focus:border-brand-blue/50"
          >
            <option value="all">全部工具</option>
            {tools.map(value => <option key={value} value={value}>{value}</option>)}
          </select>
          <select
            value={project}
            onChange={event => setProject(event.target.value)}
            className="h-9 rounded-lg border border-border bg-page px-3 text-[13px] text-text outline-none focus:border-brand-blue/50"
          >
            <option value="all">全部项目</option>
            {projects.map(value => <option key={value} value={value}>{value}</option>)}
          </select>
          <input
            type="date"
            value={fromDate}
            onChange={event => setFromDate(event.target.value)}
            className="h-9 rounded-lg border border-border bg-page px-3 text-[13px] text-text outline-none focus:border-brand-blue/50"
            aria-label="起始日期"
          />
          <input
            type="date"
            value={toDate}
            onChange={event => setToDate(event.target.value)}
            className="h-9 rounded-lg border border-border bg-page px-3 text-[13px] text-text outline-none focus:border-brand-blue/50"
            aria-label="结束日期"
          />
          <button
            onClick={() => setSubmittedQuery(query)}
            className="h-9 rounded-lg border border-border bg-surface text-[13px] text-text hover:bg-brand-blue/5 hover:border-brand-blue/30 hover:text-brand-blue transition-colors"
          >
            查询
          </button>
        </div>

        {error && (
          <div className="mb-3 rounded-lg border border-red-200 bg-red-50 px-3 py-2 text-[13px] text-red-700 dark:border-red-900 dark:bg-red-950/30 dark:text-red-300">
            {error}
          </div>
        )}

        <div className="mb-3 rounded-lg border border-blue-200 bg-blue-50 px-3 py-2.5 text-[13px] text-brand-blue dark:border-blue-900 dark:bg-blue-950/30">
          本页仅展示已落库会话元数据：session_id、工具、项目、时间范围、记录数、Token、费用与源文件路径。不会读取、返回或渲染 prompt、response、message body、源码或工具输出正文。
        </div>

        {sessions.length === 0 ? (
          <div className="flex min-h-[220px] flex-col items-center justify-center rounded-lg border border-dashed border-border-soft text-center">
            <History size={28} className="text-muted mb-2" />
            <div className="text-sm font-semibold text-text">暂无会话元数据</div>
            <div className="mt-1 text-xs text-muted">先运行扫描，或调整筛选条件</div>
          </div>
        ) : (
          <div className="overflow-x-auto">
            <table className="w-full border-collapse text-[13px]">
              <thead>
                <tr>
                  <th className="text-left pb-2.5 text-muted font-semibold text-[12px] pr-4 whitespace-nowrap">Session</th>
                  <th className="text-left pb-2.5 text-muted font-semibold text-[12px] pr-4 whitespace-nowrap">工具</th>
                  <th className="text-left pb-2.5 text-muted font-semibold text-[12px] pr-4 whitespace-nowrap">项目</th>
                  <th className="text-left pb-2.5 text-muted font-semibold text-[12px] pr-4 whitespace-nowrap">时间范围</th>
                  <th className="text-left pb-2.5 text-muted font-semibold text-[12px] pr-4 whitespace-nowrap">记录</th>
                  <th className="text-left pb-2.5 text-muted font-semibold text-[12px] pr-4 whitespace-nowrap">Token</th>
                  <th className="text-left pb-2.5 text-muted font-semibold text-[12px] pr-4 whitespace-nowrap">费用</th>
                  <th className="text-left pb-2.5 text-muted font-semibold text-[12px] pr-4 whitespace-nowrap">源文件</th>
                </tr>
              </thead>
              <tbody>
                {sessions.map(session => (
                  <tr key={`${session.tool}-${session.session_id}-${session.project_path ?? ''}`} className="border-t border-border-soft hover:bg-surface-hover">
                    <td className="py-2.5 pr-4 font-mono text-[12px] text-text min-w-[180px] break-all">{session.session_id}</td>
                    <td className="py-2.5 pr-4">
                      <div className="flex items-center gap-2">
                        <ToolBadge tool={session.tool} size={20} />
                        <span className="text-text whitespace-nowrap">{session.tool}</span>
                      </div>
                    </td>
                    <td className="py-2.5 pr-4 text-muted min-w-[220px] break-all">{session.project_path ?? '(unknown)'}</td>
                    <td className="py-2.5 pr-4 text-muted whitespace-nowrap">
                      {formatDateTime(session.first_ts)} - {formatDateTime(session.last_ts)}
                    </td>
                    <td className="py-2.5 pr-4 text-muted">{session.records}</td>
                    <td className="py-2.5 pr-4 font-semibold text-text">{session.total_tokens.toLocaleString('en-US')}</td>
                    <td className="py-2.5 pr-4 text-muted">{formatCurrency(session.cost_usd)}</td>
                    <td className="py-2.5 pr-4 text-muted min-w-[240px] break-all">{session.source_paths.join('\n')}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </div>
  )
}

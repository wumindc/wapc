import { useState, useMemo } from 'react'
import type { DesktopSnapshot } from '../types'
import { UnitInfoIcon } from '../components/ui/UnitInfoIcon'
import { PageHeader } from '../components/layout/PageHeader'

type SortField = 'name' | 'records' | 'total' | 'input' | 'output' | 'cost'
type SortOrder = 'asc' | 'desc'

function formatNumber(n: number): string {
  return n.toLocaleString('en-US')
}

function formatCompact(n: number): string {
  if (n >= 1e9) return `${(n / 1e9).toFixed(1)}B`
  if (n >= 1e6) return `${(n / 1e6).toFixed(1)}M`
  if (n >= 1e3) return `${(n / 1e3).toFixed(1)}K`
  return String(n)
}

function formatCurrency(n: number): string {
  return `$${n.toFixed(2)}`
}

function SortIndicator({
  field,
  sortField,
  sortOrder,
}: {
  field: SortField
  sortField: SortField
  sortOrder: SortOrder
}) {
  if (sortField !== field) return <span className="opacity-0 group-hover:opacity-30 ml-1">↓</span>
  return <span className="ml-1 text-brand-blue">{sortOrder === 'desc' ? '↓' : '↑'}</span>
}

export function TokensPage({ 
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
  const [viewMode, setViewMode] = useState<'daily' | 'project'>('daily')
  const [sortField, setSortField] = useState<SortField>('name')
  const [sortOrder, setSortOrder] = useState<SortOrder>('desc')

  const toggleSort = (field: SortField) => {
    if (sortField === field) {
      setSortOrder(sortOrder === 'asc' ? 'desc' : 'asc')
    } else {
      setSortField(field)
      setSortOrder('desc') // Default new sort field to desc
    }
  }

  const data = viewMode === 'daily' ? snapshot.daily_summaries : snapshot.projects

  const sortedData = useMemo(() => {
    if (!data) return []
    return [...data].sort((a, b) => {
      let aVal: number | string = 0
      let bVal: number | string = 0

      switch (sortField) {
        case 'name':
          aVal = a.name
          bVal = b.name
          break
        case 'records':
          aVal = a.records
          bVal = b.records
          break
        case 'total':
          aVal = a.usage.input + a.usage.output + a.usage.cache_read + a.usage.cache_write + a.usage.reasoning + a.usage.tool
          bVal = b.usage.input + b.usage.output + b.usage.cache_read + b.usage.cache_write + b.usage.reasoning + b.usage.tool
          break
        case 'input':
          aVal = a.usage.input
          bVal = b.usage.input
          break
        case 'output':
          aVal = a.usage.output
          bVal = b.usage.output
          break
        case 'cost':
          aVal = a.cost_usd
          bVal = b.cost_usd
          break
      }

      if (aVal < bVal) return sortOrder === 'asc' ? -1 : 1
      if (aVal > bVal) return sortOrder === 'asc' ? 1 : -1
      return 0
    })
  }, [data, sortField, sortOrder])

  return (
    <div className="card p-4 sm:p-6 min-h-[500px] flex flex-col fade-in">
      <PageHeader 
        title="Token 使用明细"
        subtitle="按周期或项目统计 API 开销"
        showToolSelector={true}
        selectedTool={selectedTool}
        setSelectedTool={setSelectedTool}
        setIsSidebarOpen={setIsSidebarOpen}
      />
      
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4 mb-6">
        <div></div> {/* 占位以保持原有的 space-between 布局 */}
        
        {/* Toggle View */}
        <div className="flex bg-surface-soft p-1 rounded-lg border border-border">
          <button
            onClick={() => setViewMode('daily')}
            className={`px-4 py-1.5 rounded-md text-sm transition-colors ${
              viewMode === 'daily' ? 'bg-surface shadow-sm text-brand-blue font-medium' : 'text-muted hover:text-text'
            }`}
          >
            按每天查看
          </button>
          <button
            onClick={() => setViewMode('project')}
            className={`px-4 py-1.5 rounded-md text-sm transition-colors ${
              viewMode === 'project' ? 'bg-surface shadow-sm text-brand-blue font-medium' : 'text-muted hover:text-text'
            }`}
          >
            按使用目录查看
          </button>
        </div>
      </div>

      <div className="overflow-x-auto flex-1">
        <table className="w-full text-left border-collapse whitespace-nowrap">
          <thead>
            <tr className="border-b border-border">
              <th 
                className="py-3 px-4 font-semibold text-muted text-sm cursor-pointer group hover:text-text transition-colors"
                onClick={() => toggleSort('name')}
              >
                {viewMode === 'daily' ? '日期' : '项目目录'}
                <SortIndicator field="name" sortField={sortField} sortOrder={sortOrder} />
              </th>
              <th 
                className="py-3 px-4 font-semibold text-muted text-sm cursor-pointer group hover:text-text transition-colors"
                onClick={() => toggleSort('records')}
              >
                会话数
                <SortIndicator field="records" sortField={sortField} sortOrder={sortOrder} />
              </th>
              <th 
                className="py-3 px-4 font-semibold text-muted text-sm cursor-pointer group hover:text-text transition-colors"
                onClick={() => toggleSort('total')}
              >
                <div className="flex items-center">
                  总 Token <UnitInfoIcon /> <SortIndicator field="total" sortField={sortField} sortOrder={sortOrder} />
                </div>
              </th>
              <th 
                className="py-3 px-4 font-semibold text-muted text-sm cursor-pointer group hover:text-text transition-colors"
                onClick={() => toggleSort('input')}
              >
                <div className="flex items-center">
                  输入 <UnitInfoIcon /> <SortIndicator field="input" sortField={sortField} sortOrder={sortOrder} />
                </div>
              </th>
              <th 
                className="py-3 px-4 font-semibold text-muted text-sm cursor-pointer group hover:text-text transition-colors"
                onClick={() => toggleSort('output')}
              >
                <div className="flex items-center">
                  输出 <UnitInfoIcon /> <SortIndicator field="output" sortField={sortField} sortOrder={sortOrder} />
                </div>
              </th>
              <th 
                className="py-3 px-4 font-semibold text-muted text-sm cursor-pointer group hover:text-text transition-colors"
                onClick={() => toggleSort('cost')}
              >
                预估费用 (USD)
                <SortIndicator field="cost" sortField={sortField} sortOrder={sortOrder} />
              </th>
            </tr>
          </thead>
          <tbody>
            {sortedData.map((row, idx) => {
              const total = row.usage.input + row.usage.output + row.usage.cache_read + row.usage.cache_write + row.usage.reasoning + row.usage.tool;
              return (
                <tr key={`${row.name}-${idx}`} className="border-b border-border-soft hover:bg-surface-hover transition-colors">
                  <td className="py-3 px-4 text-text font-medium max-w-[200px] sm:max-w-[300px] truncate" title={row.name}>
                    {row.name}
                  </td>
                  <td className="py-3 px-4 text-muted">
                    {formatNumber(row.records)}
                  </td>
                  <td className="py-3 px-4 text-brand-blue font-semibold" title={total.toLocaleString()}>
                    {formatCompact(total)}
                  </td>
                  <td className="py-3 px-4 text-muted" title={row.usage.input.toLocaleString()}>
                    {formatCompact(row.usage.input)}
                  </td>
                  <td className="py-3 px-4 text-muted" title={row.usage.output.toLocaleString()}>
                    {formatCompact(row.usage.output)}
                  </td>
                  <td className="py-3 px-4 text-muted">
                    {formatCurrency(row.cost_usd)}
                  </td>
                </tr>
              )
            })}
            {sortedData.length === 0 && (
              <tr>
                <td colSpan={6} className="py-8 text-center text-muted">
                  暂无数据
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
    </div>
  )
}

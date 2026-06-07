import { ArrowUpCircle, ArrowDownCircle } from 'lucide-react'
import type { DesktopSnapshot } from '../types'
import {
  totalTokensToday,
  totalRecordsToday,
  estimatedCostToday,
} from '../types'
import { computeDelta } from '../hooks/use-chart-data'
import { TrendChart } from '../components/charts/TrendChart'
import { PageHeader } from '../components/layout/PageHeader'

function formatNumber(n: number): string {
  if (n >= 1e6) return `${(n / 1e6).toFixed(1)}M`
  if (n >= 1e3) return `${(n / 1e3).toFixed(1)}K`
  return n.toLocaleString('en-US')
}

function formatCurrency(n: number): string {
  return `$${n.toFixed(2)}`
}

export function OverviewPage({ 
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
  const todayTokens = totalTokensToday(snapshot)
  const yesterdayTokens = snapshot.yesterday.reduce((s, r) => s + (r.usage.input + r.usage.output), 0)
  const tokenDelta = computeDelta(todayTokens, yesterdayTokens)

  const todayRecords = totalRecordsToday(snapshot)
  const yesterdayRecords = snapshot.yesterday.reduce((s, r) => s + r.records, 0)
  const recordsDelta = computeDelta(todayRecords, yesterdayRecords)

  const todayCost = estimatedCostToday(snapshot)
  const yesterdayCost = snapshot.yesterday.reduce((s, r) => s + r.cost_usd, 0)
  const costDelta = computeDelta(todayCost, yesterdayCost)

  const kpis = [
    { 
      label: '今日 Tokens', 
      value: formatNumber(todayTokens), 
      change: tokenDelta ? `${tokenDelta > 0 ? '+' : ''}${tokenDelta.toFixed(1)}%` : '-', 
      color: 'text-cyan-400',
      isUp: tokenDelta ? tokenDelta > 0 : null
    },
    { 
      label: '活跃会话', 
      value: formatNumber(todayRecords), 
      change: recordsDelta ? `${recordsDelta > 0 ? '+' : ''}${recordsDelta.toFixed(1)}%` : '-', 
      color: 'text-purple-400',
      isUp: recordsDelta ? recordsDelta > 0 : null
    },
    { 
      label: '预估费用', 
      value: formatCurrency(todayCost), 
      change: costDelta ? `${costDelta > 0 ? '+' : ''}${costDelta.toFixed(1)}%` : '-', 
      color: 'text-emerald-400',
      isUp: costDelta ? costDelta > 0 : null
    },
    { 
      label: '已索引事件', 
      value: formatNumber(snapshot.scan_records), 
      change: '-', 
      color: 'text-blue-400',
      isUp: null
    },
  ]

  return (
    <div className="p-6 space-y-6 fade-in h-full overflow-y-auto custom-scrollbar">
      <PageHeader 
        title="概览 (Overview)"
        subtitle="全局 Token 与费用快照"
        showToolSelector={true}
        selectedTool={selectedTool}
        setSelectedTool={setSelectedTool}
        setIsSidebarOpen={setIsSidebarOpen}
      />

      {/* KPI Cards */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        {kpis.map((kpi, i) => (
          <div key={i} className="bg-surface border border-border rounded-xl p-5 shadow-sm hover:border-brand-blue/30 transition-colors">
            <div className="text-muted text-sm mb-2">{kpi.label}</div>
            <div className="flex items-end justify-between">
              <div className={`text-3xl font-bold ${kpi.color}`}>{kpi.value}</div>
              {kpi.change !== '-' && (
                <div className={`flex items-center text-xs px-2 py-1 rounded ${kpi.isUp ? 'text-emerald-400 bg-emerald-900/20' : 'text-rose-400 bg-rose-900/20'}`}>
                  {kpi.isUp ? <ArrowUpCircle size={12} className="mr-1" /> : <ArrowDownCircle size={12} className="mr-1" />}
                  {kpi.change}
                </div>
              )}
            </div>
          </div>
        ))}
      </div>

      {/* Chart */}
      <TrendChart snapshot={snapshot} />
    </div>
  )
}

import { useMemo } from 'react'
import {
  AreaChart,
  Area,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip as RechartsTooltip,
  ResponsiveContainer,
  PieChart,
  Pie,
  Cell,
  Legend,
} from 'recharts'
import type { DesktopSnapshot } from '../types'
import { getToolMeta, tokenTotal } from '../types'
import { PageHeader } from '../components/layout/PageHeader'

type UsageChartPoint = {
  name: string
} & Record<string, string | number>

type PieLegendEntry = {
  payload?: {
    displayName?: string
  }
}

const formatCompact = (num: number) => {
  if (num >= 1e9) return (num / 1e9).toFixed(1) + 'B'
  if (num >= 1e6) return (num / 1e6).toFixed(1) + 'M'
  if (num >= 1e3) return (num / 1e3).toFixed(1) + 'K'
  return num.toString()
}

function toNumericValue(value: unknown): number {
  if (typeof value === 'number') return value
  const parsed = Number(value)
  return Number.isFinite(parsed) ? parsed : 0
}

function toToolName(value: unknown): string {
  return String(value)
}

export function AnalyticsPage({ 
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
  // Process daily data for stacked area chart
  const { chartData, toolsPresent } = useMemo(() => {
    const dataMap = new Map<string, Record<string, number>>()
    const toolsSet = new Set<string>()

    // Ensure all trend_days are in the map, even if empty
    snapshot.trend_days.forEach(day => {
      dataMap.set(day, { name: 0 }) // dummy 'name' property will be overridden by actual string name, wait, name is day
    })

    snapshot.daily.forEach(d => {
      toolsSet.add(d.tool)
      if (!dataMap.has(d.day)) {
        dataMap.set(d.day, { name: 0 })
      }
      const record = dataMap.get(d.day)!
      record[d.tool] = d.total_tokens
    })

    const finalData = Array.from(dataMap.entries())
      .map(([day, toolCounts]) => {
        const obj: UsageChartPoint = { name: day.slice(5) } // "MM-DD"
        toolsSet.forEach(t => {
          obj[t] = toolCounts[t] || 0
        })
        return obj
      })
      .sort((a, b) => a.name.localeCompare(b.name))

    return { chartData: finalData, toolsPresent: Array.from(toolsSet) }
  }, [snapshot])

  // Process data for pie chart
  const pieData = useMemo(() => {
    return snapshot.tools.map(t => {
      const total = t.usage.input + t.usage.output + t.usage.cache_read + t.usage.cache_write + t.usage.reasoning + t.usage.tool
      return {
        name: t.name,
        displayName: getToolMeta(t.name).displayName,
        value: total,
        color: getToolMeta(t.name).color,
      }
    })
  }, [snapshot.tools])

  // Process insights
  const insights = useMemo(() => {
    const topTool = snapshot.tools[0]
    const totalTokens = snapshot.tools.reduce((s, t) => s + tokenTotal(t.usage), 0)
    
    const messages = []
    if (totalTokens > 0 && topTool) {
       const topTotal = tokenTotal(topTool.usage)
       const pct = ((topTotal / totalTokens) * 100).toFixed(1)
       messages.push(`您的主力 AI 工具是 ${getToolMeta(topTool.name).displayName}，消耗量占总体的 ${pct}%。`)
       
       if (Number(pct) > 80) {
         messages.push(`您的工具使用较为集中，高度依赖 ${getToolMeta(topTool.name).displayName}。`)
       } else {
         messages.push(`您的工具使用比较均衡，有多款 AI 编程助手协同工作。`)
       }
    } else {
       messages.push('暂无足够的数据生成汇总建议。')
    }
    
    if (snapshot.trend_days.length > 0) {
       messages.push(`当前数据基于最近 ${snapshot.trend_days.length} 天的活跃记录进行统计聚合。`)
    }

    return messages
  }, [snapshot])

  return (
    <div className="flex flex-col gap-6 fade-in p-6 pt-4 h-full overflow-y-auto custom-scrollbar">
      <PageHeader 
        title="数据分析"
        subtitle="可视化的宏观趋势与结构化见解"
        showToolSelector={true}
        selectedTool={selectedTool}
        setSelectedTool={setSelectedTool}
        setIsSidebarOpen={setIsSidebarOpen}
      />
      {/* Daily Fluctuation Chart */}
      <div className="card p-5 min-w-0">
        <h2 className="text-lg font-bold text-heading m-0 mb-6">每日用量波动趋势 (Tokens)</h2>
        <div className="w-full h-[320px]">
          <ResponsiveContainer width="100%" height="100%">
            <AreaChart data={chartData} margin={{ top: 10, right: 10, left: 0, bottom: 0 }}>
              <defs>
                {toolsPresent.map(tool => (
                  <linearGradient key={tool} id={`color${tool}`} x1="0" y1="0" x2="0" y2="1">
                    <stop offset="5%" stopColor={getToolMeta(tool).color} stopOpacity={0.3} />
                    <stop offset="95%" stopColor={getToolMeta(tool).color} stopOpacity={0} />
                  </linearGradient>
                ))}
              </defs>
              <CartesianGrid strokeDasharray="3 3" vertical={false} stroke="var(--color-border-soft-value)" />
              <XAxis 
                dataKey="name" 
                axisLine={false} 
                tickLine={false} 
                tick={{ fill: 'var(--color-muted-value)', fontSize: 12 }} 
                dy={10} 
              />
              <YAxis 
                tickFormatter={formatCompact} 
                axisLine={false} 
                tickLine={false} 
                tick={{ fill: 'var(--color-muted-value)', fontSize: 12 }}
                width={50}
              />
              <RechartsTooltip 
                contentStyle={{ 
                  borderRadius: '8px', 
                  border: '1px solid var(--color-border-value)',
                  background: 'var(--color-surface-value)',
                  boxShadow: '0 4px 12px 0 rgba(0,0,0,0.05)',
                  color: 'var(--color-text-value)'
                }}
                formatter={(value: unknown, name: unknown) => [
                  toNumericValue(value).toLocaleString(),
                  getToolMeta(toToolName(name)).displayName,
                ]}
              />
              <Legend 
                verticalAlign="top" 
                height={36} 
                formatter={(value: unknown) => (
                  <span className="text-text font-medium">{getToolMeta(toToolName(value)).displayName}</span>
                )}
              />
              {toolsPresent.map(tool => (
                <Area 
                  key={tool} 
                  type="monotone" 
                  dataKey={tool} 
                  stackId="1" 
                  stroke={getToolMeta(tool).color} 
                  fill={`url(#color${tool})`} 
                />
              ))}
            </AreaChart>
          </ResponsiveContainer>
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Tool Distribution */}
        <div className="card p-5 flex flex-col min-w-0">
          <h2 className="text-lg font-bold text-heading m-0 mb-6">历史工具构成比</h2>
          <div className="flex-1 flex items-center justify-center min-h-[260px]">
            {pieData.length > 0 ? (
              <ResponsiveContainer width="100%" height={260}>
                <PieChart>
                  <Pie
                    data={pieData}
                    cx="50%"
                    cy="50%"
                    innerRadius={60}
                    outerRadius={90}
                    paddingAngle={2}
                    dataKey="value"
                    stroke="none"
                  >
                    {pieData.map((entry, index) => (
                      <Cell key={`cell-${index}`} fill={entry.color} />
                    ))}
                  </Pie>
                  <RechartsTooltip 
                    formatter={(value: unknown) => [`${toNumericValue(value).toLocaleString()} Tokens`, '用量']}
                    contentStyle={{ 
                      borderRadius: '8px', 
                      border: '1px solid var(--color-border-value)',
                      background: 'var(--color-surface-value)'
                    }}
                  />
                  <Legend 
                    layout="vertical" 
                    verticalAlign="middle" 
                    align="right"
                    formatter={(_value: unknown, entry: unknown) => {
                      const legendEntry = entry as PieLegendEntry
                      return <span className="text-text font-medium">{legendEntry.payload?.displayName ?? '未知工具'}</span>
                    }}
                  />
                </PieChart>
              </ResponsiveContainer>
            ) : (
              <div className="text-muted text-sm">暂无数据</div>
            )}
          </div>
        </div>

        {/* Analytics Insights */}
        <div className="card p-5 flex flex-col min-w-0">
           <h2 className="text-lg font-bold text-heading m-0 mb-6">智能汇总建议</h2>
           <div className="flex-1 rounded-lg bg-surface-soft p-5 border border-border-soft flex flex-col justify-center gap-4">
              {insights.map((msg, i) => (
                <div key={i} className="flex items-start gap-3">
                  <span className="text-brand-blue mt-0.5 text-lg">✦</span>
                  <span className="text-text text-[14px] leading-relaxed">{msg}</span>
                </div>
              ))}
              <div className="mt-2 text-muted text-[13px] border-t border-border-soft pt-4 leading-relaxed">
                 <strong className="font-semibold text-heading">优化建议：</strong>对于高频使用的工具，可重点检查其“Cache Read（缓存命中）”占比是否符合预期。高命中率有助于大幅降低 API 调用费用及提升响应速度。
              </div>
           </div>
        </div>
      </div>
    </div>
  )
}

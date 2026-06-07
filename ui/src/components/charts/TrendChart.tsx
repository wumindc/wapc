/**
 * TrendChart — multi-tool line chart using recharts.
 * @author codex
 */

import { useState } from 'react'
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
} from 'recharts'
import { ChevronDown } from 'lucide-react'
import type { DesktopSnapshot } from '../../types'
import { useChartData } from '../../hooks/use-chart-data'

interface TrendChartProps {
  snapshot: DesktopSnapshot
}

function formatYAxis(value: number): string {
  if (value >= 1_000_000_000) return `${(value / 1_000_000_000).toFixed(1)}B`
  if (value >= 1_000_000) return `${(value / 1_000_000).toFixed(0)}M`
  if (value >= 1_000) return `${(value / 1_000).toFixed(0)}K`
  return String(value)
}

function formatTooltipValue(value: number): string {
  if (value >= 1_000_000_000) return `${(value / 1_000_000_000).toFixed(2)}B`
  if (value >= 1_000_000) return `${(value / 1_000_000).toFixed(1)}M`
  if (value >= 1_000) return `${(value / 1_000).toFixed(1)}K`
  return String(value)
}

function toNumericTooltipValue(value: unknown): number {
  if (typeof value === 'number') return value
  const parsed = Number(value)
  return Number.isFinite(parsed) ? parsed : 0
}

export function TrendChart({ snapshot }: TrendChartProps) {
  const [range, setRange] = useState<'7d' | '30d'>('7d')
  const { points, tools } = useChartData(snapshot, range)

  return (
    <div className="card p-4 sm:p-[18px]">
      {/* Header */}
      <div className="flex items-start justify-between mb-4">
        <div>
          <div className="text-[15px] font-bold text-heading mb-2.5">
            Token 使用趋势（近 {range === '7d' ? '7' : '30'} 天）
          </div>
          <div className="flex gap-5 flex-wrap">
            {tools.map(t => (
              <div key={t.name} className="flex items-center gap-1.5">
                <span
                  className="w-2.5 h-2.5 rounded-full inline-block"
                  style={{ background: t.color }}
                />
                <span className="text-xs text-muted font-medium">
                  {t.displayName}
                </span>
              </div>
            ))}
          </div>
        </div>

        {/* Range selector */}
        <button
          onClick={() => setRange(r => (r === '7d' ? '30d' : '7d'))}
          className="inline-flex items-center gap-1.5 py-1.5 px-3 border border-border rounded-md bg-surface text-[13px] text-text cursor-pointer transition-colors whitespace-nowrap shrink-0 hover:border-brand-blue/30 shadow-sm"
        >
          <span>近 {range === '7d' ? '7' : '30'} 天</span>
          <ChevronDown size={14} className="text-muted" />
        </button>
      </div>

      {/* Chart */}
      <div className="w-full h-[200px]">
        <ResponsiveContainer width="100%" height="100%">
          <LineChart data={points} margin={{ top: 8, right: 8, left: 0, bottom: 0 }}>
            <CartesianGrid strokeDasharray="3 3" stroke="var(--color-border-soft-value)" vertical={true} horizontal={true} />
            <XAxis
              dataKey="dayShort"
              tick={{ fontSize: 11, fill: 'var(--color-muted-value)' }}
              axisLine={false}
              tickLine={false}
            />
            <YAxis
              tickFormatter={formatYAxis}
              tick={{ fontSize: 11, fill: 'var(--color-muted-value)' }}
              axisLine={false}
              tickLine={false}
              width={42}
            />
            <Tooltip
              formatter={(value: unknown, name: unknown) => [
                formatTooltipValue(toNumericTooltipValue(value)),
                tools.find(t => t.name === String(name))?.displayName ?? String(name),
              ]}
              labelStyle={{ color: 'var(--color-text-value)', fontWeight: 600, fontSize: 12 }}
              contentStyle={{
                border: '1px solid var(--color-border-value)',
                borderRadius: '8px',
                boxShadow: '0 4px 12px rgba(0,0,0,0.1)',
                fontSize: '12px',
                background: 'var(--color-surface-value)'
              }}
            />
            {tools.map(t => (
              <Line
                key={t.name}
                type="monotone"
                dataKey={t.name}
                stroke={t.color}
                strokeWidth={2.5}
                dot={{ r: 4, fill: 'var(--color-surface-value)', stroke: t.color, strokeWidth: 2 }}
                activeDot={{ r: 5, fill: t.color }}
                animationDuration={800}
              />
            ))}
          </LineChart>
        </ResponsiveContainer>
      </div>
    </div>
  )
}

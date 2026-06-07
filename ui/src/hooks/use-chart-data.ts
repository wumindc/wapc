/**
 * Chart data processing hook.
 * @author Claude Sonnet 4.6 (Thinking)
 */

import { useMemo } from 'react'
import type { DesktopSnapshot, DailyToolSummary } from '../types'
import { getToolMeta } from '../types'

export interface TrendPoint {
  day: string
  dayShort: string
  [tool: string]: number | string
}

export function useChartData(snapshot: DesktopSnapshot, range: '7d' | '30d' = '7d') {
  return useMemo(() => {
    const days = snapshot.trend_days.slice(range === '7d' ? -7 : -30)
    const toolNames = Array.from(new Set(snapshot.daily.map(d => d.tool))).slice(0, 4)

    // Build lookup: day → tool → tokens
    const lookup: Record<string, Record<string, number>> = {}
    snapshot.daily.forEach((d: DailyToolSummary) => {
      if (!lookup[d.day]) lookup[d.day] = {}
      lookup[d.day][d.tool] = d.total_tokens
    })

    const points: TrendPoint[] = days.map(day => {
      const point: TrendPoint = {
        day,
        dayShort: day.slice(5), // MM-DD
      }
      toolNames.forEach(tool => {
        point[tool] = lookup[day]?.[tool] ?? 0
      })
      return point
    })

    const tools = toolNames.map((name, i) => ({
      name,
      displayName: getToolMeta(name).displayName,
      color: getToolMeta(name).color || ['#1F6FEB', '#12A0A6', '#7E57EB', '#F1761F'][i % 4],
    }))

    return { points, tools }
  }, [snapshot.trend_days, snapshot.daily, range])
}

// Compute sparkline values for a KPI (total daily tokens across all tools)
export function useSparkData(snapshot: DesktopSnapshot): number[] {
  return useMemo(() => {
    const lookup: Record<string, number> = {}
    snapshot.daily.forEach(d => {
      lookup[d.day] = (lookup[d.day] ?? 0) + d.total_tokens
    })
    return snapshot.trend_days.map(day => lookup[day] ?? 0)
  }, [snapshot.daily, snapshot.trend_days])
}

// Delta percentage compared to yesterday
export function computeDelta(todayVal: number, yesterdayVal: number): number | null {
  if (yesterdayVal === 0) return null
  return ((todayVal - yesterdayVal) / yesterdayVal) * 100
}

/**
 * KpiCard — key performance indicator card with sparkline and delta.
 * @author Claude Sonnet 4.6 (Thinking)
 */

import React from 'react'
import { TrendingUp, TrendingDown, Minus } from 'lucide-react'
import { SparkLine } from './SparkLine'

interface KpiCardProps {
  label: string
  value: string
  delta?: number | null   // percent, e.g. +4.06 or -2.1
  accent: string          // hex color
  icon: React.ReactNode
  sparkData?: number[]
}

export function KpiCard({ label, value, delta, accent, icon, sparkData = [] }: KpiCardProps) {
  const hasDelta = delta !== null && delta !== undefined
  const isPositive = hasDelta && delta > 0
  const isNegative = hasDelta && delta < 0

  return (
    <div className="card flex flex-col p-4 sm:p-[18px] min-h-[130px] transition-shadow duration-200 hover:shadow-[0_4px_12px_rgba(15,26,46,0.1),0_2px_4px_-1px_rgba(15,26,46,0.06)] dark:hover:shadow-[0_4px_12px_rgba(0,0,0,0.5)] group">
      {/* Top row: icon + label + value */}
      <div className="flex items-start gap-3 mb-3">
        {/* Icon badge */}
        <div
          className="w-11 h-11 rounded-full flex items-center justify-center shrink-0 transition-transform group-hover:scale-105"
          style={{ background: `${accent}18`, color: accent }}
        >
          {icon}
        </div>
        <div>
          <div className="text-[13px] font-semibold text-muted leading-tight mb-1">
            {label}
          </div>
          <div className="text-[26px] font-bold text-heading leading-none tracking-tight">
            {value}
          </div>
        </div>
      </div>

      {/* Delta row */}
      {hasDelta && (
        <div
          className={`flex items-center gap-1 text-[12px] font-semibold mb-1.5 ${isPositive ? 'text-brand-green' : isNegative ? 'text-brand-red' : 'text-muted'}`}
        >
          {isPositive && <TrendingUp size={13} />}
          {isNegative && <TrendingDown size={13} />}
          {!isPositive && !isNegative && <Minus size={13} />}
          <span>
            较昨日 {isPositive ? '+' : ''}{delta!.toFixed(2)}% {isPositive ? '↑' : isNegative ? '↓' : ''}
          </span>
        </div>
      )}

      {/* Sparkline */}
      {sparkData.length > 0 && (
        <div className="mt-auto pt-1">
          <SparkLine data={sparkData} color={accent} height={26} />
        </div>
      )}
    </div>
  )
}

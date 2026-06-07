/**
 * SparkLine — animated SVG mini trend line.
 * @author Claude Sonnet 4.6 (Thinking)
 */

import { useMemo, useRef, useEffect } from 'react'

interface SparkLineProps {
  data: number[]
  color: string
  height?: number
}

export function SparkLine({ data, color, height = 28 }: SparkLineProps) {
  const pathRef = useRef<SVGPathElement>(null)

  const path = useMemo(() => {
    if (!data.length) return ''
    const max = Math.max(...data, 1)
    const w = 100
    const h = height
    const pts = data.map((v, i) => {
      const x = (i / (data.length - 1)) * w
      const y = h - (v / max) * (h * 0.8) - h * 0.05
      return `${x},${y}`
    })
    return `M ${pts.join(' L ')}`
  }, [data, height])

  // Stroke animation on mount
  useEffect(() => {
    const el = pathRef.current
    if (!el) return
    const len = el.getTotalLength()
    el.style.strokeDasharray = String(len)
    el.style.strokeDashoffset = String(len)
    el.getBoundingClientRect()
    el.style.transition = 'stroke-dashoffset 1s ease'
    el.style.strokeDashoffset = '0'
  }, [path])

  if (!data.length) return null

  return (
    <svg
      viewBox={`0 0 100 ${height}`}
      preserveAspectRatio="none"
      width="100%"
      height={height}
      style={{ display: 'block' }}
    >
      <path
        ref={pathRef}
        d={path}
        fill="none"
        stroke={color}
        strokeWidth="1.8"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  )
}

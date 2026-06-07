/**
 * ProgressBar — animated fill bar for tool usage ratio.
 * @author Claude Sonnet 4.6 (Thinking)
 */

import { useEffect, useRef } from 'react'

interface ProgressBarProps {
  ratio: number   // 0~1
  color: string
  height?: number
}

export function ProgressBar({ ratio, color, height = 6 }: ProgressBarProps) {
  const fillRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    const el = fillRef.current
    if (!el) return
    el.style.width = '0%'
    el.getBoundingClientRect()
    el.style.transition = 'width 700ms cubic-bezier(0.4, 0, 0.2, 1)'
    el.style.width = `${Math.round(ratio * 10000) / 100}%`
  }, [ratio])

  return (
    <div
      style={{
        width: '100%',
        height,
        borderRadius: height,
        background: '#E8EEF7',
        overflow: 'hidden',
      }}
    >
      <div
        ref={fillRef}
        style={{
          height: '100%',
          borderRadius: height,
          background: color,
        }}
      />
    </div>
  )
}

/**
 * StatusChip — colored badge used in the page header.
 * @author Claude Sonnet 4.6 (Thinking)
 */

import { Lock } from 'lucide-react'

interface StatusChipProps {
  label: string
  color: string
  mode?: 'dot' | 'lock'
  onClick?: () => void
}

export function StatusChip({ label, color, mode = 'dot', onClick }: StatusChipProps) {
  return (
    <button
      onClick={onClick}
      className={`inline-flex items-center gap-2 px-3.5 h-9 rounded-lg text-[13px] font-semibold whitespace-nowrap transition-colors duration-150 ${onClick ? 'cursor-pointer' : 'cursor-default'}`}
      style={{
        border: `1px solid ${color}40`,
        background: `${color}15`,
        color: color,
      }}
      onMouseEnter={e => {
        if (!onClick) return
        ;(e.currentTarget as HTMLElement).style.background = `${color}25`
        ;(e.currentTarget as HTMLElement).style.borderColor = `${color}60`
      }}
      onMouseLeave={e => {
        ;(e.currentTarget as HTMLElement).style.background = `${color}15`
        ;(e.currentTarget as HTMLElement).style.borderColor = `${color}40`
      }}
    >
      {mode === 'dot' ? (
        <span
          className="w-2 h-2 rounded-full shrink-0"
          style={{ background: color }}
        />
      ) : (
        <Lock size={13} />
      )}
      {label}
    </button>
  )
}

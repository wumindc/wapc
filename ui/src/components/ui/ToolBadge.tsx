/**
 * ToolBadge — colored rounded square with tool initial letter.
 * @author Claude Sonnet 4.6 (Thinking)
 */

import { getToolMeta } from '../../types'

interface ToolBadgeProps {
  tool: string
  size?: number
}

export function ToolBadge({ tool, size = 26 }: ToolBadgeProps) {
  const meta = getToolMeta(tool)
  return (
    <span
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        justifyContent: 'center',
        width: size,
        height: size,
        borderRadius: 6,
        background: meta.bgColor,
        color: meta.color,
        fontWeight: 700,
        fontSize: size * 0.48,
        lineHeight: 1,
        flexShrink: 0,
        fontFamily: 'ui-sans-serif, system-ui, sans-serif',
        border: `1px solid ${meta.color}22`,
      }}
    >
      {meta.initial}
    </span>
  )
}

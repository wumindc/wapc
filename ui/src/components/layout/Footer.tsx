/**
 * Footer — bottom status bar showing DB path and last refresh.
 * @author Claude Sonnet 4.6 (Thinking)
 */

import type { DesktopSnapshot } from '../../types'

interface FooterProps {
  snapshot: DesktopSnapshot
  lastRefresh: Date | null
}

function truncateMiddle(str: string, max: number): string {
  if (str.length <= max) return str
  const half = Math.floor((max - 1) / 2)
  return `${str.slice(0, half)}…${str.slice(-half)}`
}

export function Footer({ snapshot, lastRefresh }: FooterProps) {
  const timeStr = lastRefresh
    ? lastRefresh.toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit', second: '2-digit' })
    : '—'

  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 16,
        paddingTop: 16,
        borderTop: '1px solid #E7ECF4',
        fontSize: 12,
        color: '#5C6A7F',
      }}
    >
      <span>数据库：{truncateMiddle(snapshot.db_path, 48)}</span>
      <div style={{ display: 'flex', alignItems: 'center', gap: 5 }}>
        <span
          style={{
            width: 7,
            height: 7,
            borderRadius: '50%',
            background: snapshot.db_exists ? '#26975B' : '#CC463A',
            display: 'inline-block',
          }}
        />
        <span>{snapshot.db_exists ? '数据库正常' : '数据库未创建'}</span>
      </div>
      <div style={{ marginLeft: 'auto' }}>
        最后刷新：{timeStr}
      </div>
    </div>
  )
}

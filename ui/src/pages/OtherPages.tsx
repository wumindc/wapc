import { CheckCircle } from 'lucide-react'
import type { DesktopSnapshot } from '../types'
import { useAutoScan } from '../hooks/auto-scan'
import { PageHeader } from '../components/layout/PageHeader'

// ── AboutPage (Merged with Help & Privacy info) ──────────────────────────────
export function AboutPage({ 
  snapshot,
  setIsSidebarOpen 
}: { 
  snapshot: DesktopSnapshot;
  setIsSidebarOpen?: (o: boolean) => void;
}) {
  const { enabled: autoScanEnabled, intervalMinutes } = useAutoScan()
  const audit = snapshot.privacy_audit

  return (
    <div className="flex flex-col h-full fade-in">
      <div className="px-6 pt-6 pb-2 border-b border-border bg-page shrink-0">
        <PageHeader 
          title="系统说明与隐私审计"
          subtitle="应用版本、底层逻辑与数据隐私边界"
          setIsSidebarOpen={setIsSidebarOpen}
        />
      </div>
      <div className="flex-1 overflow-y-auto p-4 sm:p-6 custom-scrollbar bg-page">
        <div className="max-w-5xl mx-auto grid grid-cols-1 md:grid-cols-2 gap-4">
          {/* App Info */}
          <div className="card p-4 sm:p-[18px]">
            <div className="text-xs text-muted mb-4">Workstation AI Programming Cost observer</div>
            <InfoRow label="定位" value="本机 AI 编程工具 Token 观测器" />
            <InfoRow label="采集方式" value="旁路读取工具本地 usage/session 文件" />
            <InfoRow label="安全原则" value="不代理、不注入、不上传、不保存正文" />
            <InfoRow label="发布版本" value={`v${snapshot.version}`} />
          </div>

          {/* System Status */}
          <div className="card p-4 sm:p-[18px]">
            <div className="text-[15px] font-bold text-heading mb-1">运行信息</div>
            <div className="text-xs text-muted mb-4">当前本机安装与数据库状态</div>
            <InfoRow label="版本" value={snapshot.version} />
            <InfoRow label="数据库" value={snapshot.db_path} />
            <InfoRow label="已索引事件" value={snapshot.scan_records.toLocaleString('en-US')} />
            <InfoRow label="自动扫描" value={autoScanEnabled ? `已开启（每 ${intervalMinutes} 分钟）` : '已关闭'} />
          </div>

          {/* Privacy & Security */}
          <div className="card p-4 sm:p-[18px]">
            <div className="text-[15px] font-bold text-heading mb-1">隐私边界</div>
            <div className="text-xs text-muted mb-4">审计生成时间：{formatDateTime(audit.generated_at)}</div>
            {[
              ['本机存储', audit.local_only, audit.local_only ? '不上传' : '需复核'],
              ['对话正文', true, '不入库'],
              ['工具改造', true, '无侵入'],
              ['数据库', snapshot.db_exists, audit.db_path],
            ].map(([label, ok, detail]) => (
              <div key={String(label)} className="flex items-center gap-2 mb-2.5 text-[13px]">
                <CheckCircle size={15} className={ok ? 'text-brand-green shrink-0' : 'text-brand-red shrink-0'} />
                <span className="text-muted flex-1">{label}</span>
                <span className={`font-semibold text-xs ${ok ? 'text-brand-green' : 'text-brand-red'}`}>{String(detail)}</span>
              </div>
            ))}
            <div className="h-px bg-border-soft my-4" />
            <InfoRow label="导出边界" value={audit.export_boundary} />
            <InfoRow label="不存字段" value={audit.forbidden_fields.join('、')} />
          </div>

          {/* Audit sources */}
          <div className="card p-4 sm:p-[18px]">
            <div className="text-[15px] font-bold text-heading mb-1">读取来源审计</div>
            <div className="text-xs text-muted mb-4">后端声明的本机只读来源</div>
            {audit.read_sources.map(source => (
              <div key={`${source.name}:${source.path}`} className="flex items-center gap-2 mb-2.5 text-[13px]">
                <CheckCircle size={15} className="text-brand-green shrink-0" />
                <span className="text-heading flex-1">{source.name}</span>
                <span className="text-muted text-[11.5px]" title={source.purpose}>{source.path}</span>
              </div>
            ))}
          </div>

          <div className="card p-4 sm:p-[18px] md:col-span-2">
            <div className="text-[15px] font-bold text-heading mb-1">落库字段审计</div>
            <div className="text-xs text-muted mb-4">SQLite 表与字段类别</div>
            <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
              {audit.stored_tables.map(table => (
                <div key={table.name} className="border border-border-soft rounded-lg p-3">
                  <div className="text-[13px] font-semibold text-heading mb-1">{table.name}</div>
                  <div className="text-[12px] text-muted leading-relaxed">{table.fields.join('、')}</div>
                </div>
              ))}
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}

// ── Shared sub-components ─────────────────────────────────────────────────────
function InfoRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-start justify-between gap-3 mb-2.5 text-[13px]">
      <span className="text-heading shrink-0">{label}</span>
      <span className="text-muted text-xs text-right break-all">{value}</span>
    </div>
  )
}

function formatDateTime(value: string): string {
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return '未知'
  return date.toLocaleString('zh-CN', { month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit' })
}

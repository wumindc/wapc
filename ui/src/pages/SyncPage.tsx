import { useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { save, open } from '@tauri-apps/plugin-dialog'
import { DownloadCloud, UploadCloud, AlertCircle, CheckCircle2 } from 'lucide-react'
import { PageHeader } from '../components/layout/PageHeader'
import type { BackupRequest, BackupResult } from '../types'

export function SyncPage({
  setIsSidebarOpen
}: {
  initialDeepLinkUrl?: string | null;
  setIsSidebarOpen?: (o: boolean) => void;
}) {
  const [busy, setBusy] = useState(false)
  const [message, setMessage] = useState<{ text: string, type: 'error' | 'success' | 'info' } | null>(null)

  const handleExportBackup = async () => {
    try {
      const selected = await save({
        filters: [{ name: 'WAPC Backup', extensions: ['bak'] }],
        title: '导出数据备份',
        defaultPath: 'wapc-backup.bak'
      })
      
      if (typeof selected !== 'string') return
      
      setBusy(true)
      setMessage({ text: '正在导出数据...', type: 'info' })
      
      const request: BackupRequest = { path: selected }
      const result = await invoke<BackupResult>('export_backup', { request })
      
      if (result.success) {
        setMessage({ text: `✅ 数据备份已成功保存至: ${result.path}`, type: 'success' })
      }
    } catch (err) {
      setMessage({ text: `❌ 导出失败：${err}`, type: 'error' })
    } finally {
      setBusy(false)
    }
  }

  const handleImportBackup = async () => {
    try {
      const selected = await open({
        filters: [{ name: 'WAPC Backup', extensions: ['bak'] }],
        title: '选择数据备份文件',
        multiple: false,
      })
      
      if (typeof selected !== 'string') return
      
      if (!window.confirm('警告：导入备份将覆盖当前的所有数据，并且会导致应用刷新。是否继续？')) {
        return
      }

      setBusy(true)
      setMessage({ text: '正在导入数据...', type: 'info' })
      
      const request: BackupRequest = { path: selected }
      const result = await invoke<BackupResult>('import_backup', { request })
      
      if (result.success) {
        setMessage({ text: `✅ 数据导入成功！应用即将刷新...`, type: 'success' })
        setTimeout(() => {
          window.location.reload()
        }, 1500)
      }
    } catch (err) {
      setMessage({ text: `❌ 导入失败：${err}`, type: 'error' })
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className="flex flex-col h-full fade-in">
      <div className="px-6 pt-6 pb-2 border-b border-border bg-page shrink-0">
        <PageHeader 
          title="数据同步"
          subtitle="导出或导入全局数据备份，方便设备间数据迁移"
          setIsSidebarOpen={setIsSidebarOpen}
        />
      </div>

      <div className="flex-1 p-6 overflow-y-auto custom-scrollbar bg-page">
        <div className="max-w-4xl mx-auto space-y-6">
          
          {message && (
            <div className={`p-4 rounded-xl text-[14px] flex items-center gap-3 border ${
              message.type === 'error' ? 'bg-red-900/10 border-red-900/30 text-red-500' : 
              message.type === 'success' ? 'bg-brand-green/10 border-brand-green/30 text-brand-green' : 
              'bg-brand-blue/10 border-brand-blue/30 text-brand-blue'
            }`}>
              {message.type === 'error' ? <AlertCircle size={18} /> : <CheckCircle2 size={18} />}
              <span>{message.text}</span>
            </div>
          )}

          <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
            {/* EXPORT SECTION */}
            <div className="card p-8 shadow-sm border-border-soft flex flex-col h-full hover:border-brand-blue/30 transition-colors">
              <div className="flex flex-col items-center text-center mb-8">
                <div className="w-16 h-16 bg-brand-blue/10 text-brand-blue rounded-full flex items-center justify-center mb-4">
                  <DownloadCloud size={32} />
                </div>
                <h3 className="text-[18px] font-bold text-heading mb-2">导出所有数据</h3>
                <p className="text-[13px] text-muted max-w-[250px]">
                  将当前设备中的所有配置、日志、工具信息完整导出为单个备份文件，用于迁移。
                </p>
              </div>
              
              <div className="mt-auto pt-6">
                <button
                  onClick={handleExportBackup}
                  disabled={busy}
                  className="w-full h-12 inline-flex items-center justify-center gap-2 rounded-xl bg-brand-blue text-white text-[15px] font-bold hover:bg-brand-blue/90 disabled:opacity-50 disabled:cursor-not-allowed shadow-sm transition-all"
                >
                  <DownloadCloud size={18} />
                  {busy ? '正在导出...' : '选择位置并导出'}
                </button>
              </div>
            </div>

            {/* IMPORT SECTION */}
            <div className="card p-8 shadow-sm border-border-soft flex flex-col h-full hover:border-brand-purple/30 transition-colors">
              <div className="flex flex-col items-center text-center mb-8">
                <div className="w-16 h-16 bg-brand-purple/10 text-brand-purple rounded-full flex items-center justify-center mb-4">
                  <UploadCloud size={32} />
                </div>
                <h3 className="text-[18px] font-bold text-heading mb-2">导入设备数据</h3>
                <p className="text-[13px] text-muted max-w-[250px]">
                  选择之前导出的 .bak 文件进行恢复。注意：导入操作将会覆盖当前设备上的所有数据！
                </p>
              </div>
              
              <div className="mt-auto pt-6">
                <button
                  onClick={handleImportBackup}
                  disabled={busy}
                  className="w-full h-12 inline-flex items-center justify-center gap-2 rounded-xl bg-surface border-2 border-brand-purple text-brand-purple text-[15px] font-bold hover:bg-brand-purple hover:text-white disabled:opacity-50 disabled:cursor-not-allowed shadow-sm transition-all"
                >
                  <UploadCloud size={18} />
                  {busy ? '正在导入...' : '选择备份并恢复'}
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}

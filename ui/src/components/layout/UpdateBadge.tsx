import { DownloadCloud, Loader2 } from 'lucide-react'
import { useAppUpdater } from '../../hooks/useAppUpdater'

export function UpdateBadge() {
  const { available, isInstalling, progress, installUpdate } = useAppUpdater()

  if (!available) return null

  return (
    <button
      onClick={() => void installUpdate()}
      disabled={isInstalling}
      className={`
        relative flex items-center justify-center p-2 rounded-xl transition-all duration-300
        ${isInstalling ? 'bg-surface border border-border cursor-not-allowed' : 'bg-brand-blue/10 text-brand-blue hover:bg-brand-blue/20 hover:shadow-sm'}
      `}
      title={isInstalling ? '正在下载更新...' : '发现新版本，点击更新'}
    >
      {isInstalling ? (
        <div className="relative flex items-center justify-center">
          <Loader2 size={18} className="animate-spin text-brand-blue" />
          {progress && progress.percent > 0 && (
            <span className="absolute text-[8px] font-bold text-brand-blue mt-[1px]">
              {progress.percent}%
            </span>
          )}
        </div>
      ) : (
        <>
          <DownloadCloud size={18} />
          <span className="absolute top-1.5 right-1.5 w-2 h-2 bg-red-500 rounded-full animate-pulse ring-2 ring-page" />
        </>
      )}
    </button>
  )
}

import { Box } from 'lucide-react'
import { PageHeader } from '../components/layout/PageHeader'

export function PluginsPage({ 
  setIsSidebarOpen
}: { 
  setIsSidebarOpen?: (o: boolean) => void;
}) {
  return (
    <div className="flex flex-col h-full fade-in">
      <div className="px-6 pt-6 pb-2 border-b border-border bg-page shrink-0">
        <PageHeader 
          title="Plugin 包管理"
          subtitle="应用插件及功能扩展"
          setIsSidebarOpen={setIsSidebarOpen}
        />
      </div>
      <div className="flex-1 p-6 flex flex-col items-center justify-center text-muted bg-page">
        <Box size={48} className="mb-4 opacity-20" />
        <p className="text-[15px] font-medium">Plugin 包管理功能开发中</p>
      </div>
    </div>
  )
}

import { useState, useRef, useEffect } from 'react'
import { Menu, Monitor, ChevronDown, CheckCircle } from 'lucide-react'

interface PageHeaderProps {
  title: string
  subtitle?: string
  showToolSelector?: boolean
  selectedTool?: string
  setSelectedTool?: (tool: string) => void
  setIsSidebarOpen?: (isOpen: boolean) => void
}

export function PageHeader({ 
  title, 
  subtitle, 
  showToolSelector = false, 
  selectedTool = 'all', 
  setSelectedTool, 
  setIsSidebarOpen 
}: PageHeaderProps) {
  const [isDropdownOpen, setIsDropdownOpen] = useState(false)
  const dropdownRef = useRef<HTMLDivElement>(null)

  const platforms = [
    { id: 'all', name: '全部工具平台' },
    { id: 'claude', name: 'Claude Code' },
    { id: 'codex', name: 'Codex' },
    { id: 'gemini', name: 'Gemini CLI' },
    { id: 'opencode', name: 'OpenCode' },
  ]

  const currentPlatform = platforms.find(p => p.id === selectedTool) || platforms[0]

  useEffect(() => {
    function handleClickOutside(event: MouseEvent) {
      if (dropdownRef.current && !dropdownRef.current.contains(event.target as Node)) {
        setIsDropdownOpen(false)
      }
    }
    document.addEventListener('mousedown', handleClickOutside)
    return () => document.removeEventListener('mousedown', handleClickOutside)
  }, [])

  return (
    <div className="flex flex-col md:flex-row md:items-center justify-between mb-6 gap-4">
      <div className="flex items-center">
        {/* Mobile menu toggle */}
        {setIsSidebarOpen && (
          <button 
            onClick={() => setIsSidebarOpen(true)} 
            className="md:hidden mr-3 p-2 -ml-2 text-muted hover:text-text hover:bg-surface-hover rounded-md transition-colors"
          >
            <Menu size={20} />
          </button>
        )}
        <div>
          <h2 className="text-2xl font-bold text-heading mb-1">{title}</h2>
          {subtitle && <p className="text-muted text-sm">{subtitle}</p>}
        </div>
      </div>

      {showToolSelector && setSelectedTool && (
        <div className="relative z-20" ref={dropdownRef}>
          <button 
            onClick={() => setIsDropdownOpen(!isDropdownOpen)}
            className="flex items-center space-x-2 bg-surface border border-border hover:border-brand-blue/50 px-4 py-2 rounded-lg text-sm font-medium text-text transition-colors shadow-sm w-full md:w-auto"
          >
            <Monitor size={16} className="text-brand-blue shrink-0" />
            <span className="flex-1 text-left">{currentPlatform.name}</span>
            <ChevronDown size={14} className="text-muted shrink-0" />
          </button>

          {isDropdownOpen && (
            <div className="absolute right-0 mt-2 w-full md:w-48 bg-surface border border-border rounded-lg shadow-card-hover overflow-hidden">
              <div className="py-1">
                {platforms.map(platform => (
                  <button
                    key={platform.id}
                    onClick={() => {
                      setSelectedTool(platform.id)
                      setIsDropdownOpen(false)
                    }}
                    className={`w-full text-left px-4 py-2 text-sm flex items-center justify-between transition-colors ${
                      selectedTool === platform.id 
                        ? 'bg-brand-blue/10 text-brand-blue' 
                        : 'text-text hover:bg-surface-hover'
                    }`}
                  >
                    {platform.name}
                    {selectedTool === platform.id && <CheckCircle size={14} />}
                  </button>
                ))}
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  )
}

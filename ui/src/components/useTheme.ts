/**
 * Theme hook for components that read or update UI theme state.
 * @author codex
 */

import { useContext } from 'react'
import { ThemeProviderContext } from './themeContext'

export const useTheme = () => {
  const context = useContext(ThemeProviderContext)

  if (context === undefined) {
    throw new Error('useTheme must be used within a ThemeProvider')
  }

  return context
}

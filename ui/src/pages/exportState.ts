/**
 * Export path helpers for metadata report downloads.
 * @author codex
 */

const viewSlugs: Record<string, string> = {
  projects: 'projects',
  tools: 'tools',
  daily: 'daily',
  redacted: 'redacted',
}

const formatExtensions: Record<string, string> = {
  markdown: 'md',
  csv: 'csv',
  json: 'json',
}

export function normalizeExportFormat(view: string, format: string): string {
  if (view === 'redacted' && format === 'csv') {
    return 'json'
  }
  return format
}

export function buildDefaultExportFilename(view: string, format: string, date = new Date()): string {
  const normalizedFormat = normalizeExportFormat(view, format)
  const sanitizedView = view.replace(/[^a-z0-9_-]+/gi, '-').replace(/^-|-$/g, '')
  const knownSlug = viewSlugs[view] ?? sanitizedView
  const slug = knownSlug || 'report'
  const extension = formatExtensions[normalizedFormat] ?? normalizedFormat

  return `wapc-${slug}-${formatDate(date)}.${extension}`
}

export function buildExportPath(directory: string, view: string, format: string, date = new Date()): string {
  const trimmed = directory.trim()
  const separator = trimmed.includes('\\') && !trimmed.includes('/') ? '\\' : '/'
  const normalizedDirectory = trimmed.endsWith('/') || trimmed.endsWith('\\')
    ? trimmed.slice(0, -1)
    : trimmed

  return `${normalizedDirectory}${separator}${buildDefaultExportFilename(view, format, date)}`
}

export function suggestExportPath(directory: string, view: string, format: string, date = new Date()): string {
  if (!directory.trim()) {
    return ''
  }
  return buildExportPath(directory, view, format, date)
}

function formatDate(date: Date): string {
  const year = date.getFullYear()
  const month = `${date.getMonth() + 1}`.padStart(2, '0')
  const day = `${date.getDate()}`.padStart(2, '0')
  return `${year}-${month}-${day}`
}

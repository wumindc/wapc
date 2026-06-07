/**
 * Export state contract tests.
 * @author codex
 */

import assert from 'node:assert/strict'
import test from 'node:test'
import {
  buildDefaultExportFilename,
  buildExportPath,
  normalizeExportFormat,
  suggestExportPath,
} from '../src/pages/exportState.ts'

const date = new Date('2026-06-07T08:09:10.000Z')

test('builds default export filenames with view name and local date', () => {
  assert.equal(buildDefaultExportFilename('projects', 'markdown', date), 'wapc-projects-2026-06-07.md')
  assert.equal(buildDefaultExportFilename('tools', 'csv', date), 'wapc-tools-2026-06-07.csv')
  assert.equal(buildDefaultExportFilename('daily', 'json', date), 'wapc-daily-2026-06-07.json')
  assert.equal(buildDefaultExportFilename('redacted', 'json', date), 'wapc-redacted-2026-06-07.json')
})

test('builds export paths inside the selected directory without losing separators', () => {
  assert.equal(
    buildExportPath('/Users/example/Documents', 'projects', 'markdown', date),
    '/Users/example/Documents/wapc-projects-2026-06-07.md',
  )
  assert.equal(
    buildExportPath('/Users/example/Documents/', 'tools', 'csv', date),
    '/Users/example/Documents/wapc-tools-2026-06-07.csv',
  )
  assert.equal(
    buildExportPath(String.raw`C:\Users\Example\Documents`, 'daily', 'json', date),
    String.raw`C:\Users\Example\Documents\wapc-daily-2026-06-07.json`,
  )
})

test('suggests a new path only when a directory has been selected', () => {
  assert.equal(suggestExportPath('', 'projects', 'markdown', date), '')
  assert.equal(
    suggestExportPath('/Users/example/Exports', 'redacted', 'markdown', date),
    '/Users/example/Exports/wapc-redacted-2026-06-07.md',
  )
})

test('normalizes unsupported view and format combinations', () => {
  assert.equal(normalizeExportFormat('redacted', 'csv'), 'json')
  assert.equal(normalizeExportFormat('projects', 'csv'), 'csv')
})

/**
 * Tool path state contract tests.
 * @author codex
 */

import assert from 'node:assert/strict'
import test from 'node:test'
import { buildToolPathVerificationSummary } from '../src/pages/toolPathState.ts'
import type { ToolPathVerificationRecord } from '../src/types/index.ts'

const baseRecord: ToolPathVerificationRecord = {
  tool: 'codex',
  platform: 'linux',
  scope: 'user',
  kind: 'mcp_config',
  path: '~/.codex/config.toml',
  candidate_verified: false,
  exists: false,
  is_file: false,
  is_dir: false,
  read_only: true,
  write_supported: false,
}

test('summarizes unverified non-macOS candidates as read-only unsupported writes', () => {
  const summary = buildToolPathVerificationSummary([
    baseRecord,
    {
      ...baseRecord,
      tool: 'gemini',
      platform: 'windows',
      path: String.raw`~\.gemini\settings.json`,
    },
  ])

  assert.equal(summary.total, 2)
  assert.equal(summary.verified, 0)
  assert.equal(summary.unverified, 2)
  assert.equal(summary.writeSupported, 0)
  assert.equal(summary.writeUnsupported, 2)
  assert.deepEqual(summary.labels, ['2 个待核验候选路径', '2 个写入 unsupported'])
})

test('summarizes verified macOS candidates without marking unsupported when writes exist', () => {
  const summary = buildToolPathVerificationSummary([
    {
      ...baseRecord,
      platform: 'macos',
      candidate_verified: true,
      exists: true,
      is_file: true,
      write_supported: true,
    },
  ])

  assert.equal(summary.verified, 1)
  assert.equal(summary.unverified, 0)
  assert.equal(summary.writeSupported, 1)
  assert.equal(summary.writeUnsupported, 0)
  assert.deepEqual(summary.labels, ['1 个已核验路径', '1 个可写路径'])
})

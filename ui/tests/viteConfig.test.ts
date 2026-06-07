/**
 * Vite build configuration contract tests.
 * @author codex
 */

import assert from 'node:assert/strict'
import test from 'node:test'
import config from '../vite.config.ts'

test('uses relative asset base for Tauri bundled app loading', () => {
  const value = typeof config === 'function' ? config({ command: 'build', mode: 'production' }) : config

  assert.equal(value.base, './')
})

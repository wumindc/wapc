/**
 * Deep link entry contract tests.
 * @author codex
 */

import assert from 'node:assert/strict'
import test from 'node:test'
import { attachWapcDeepLinkHandler, isWapcImportDeepLink } from '../src/hooks/deep-link.ts'

test('filters deep-link URLs to wapc import links only', () => {
  assert.equal(isWapcImportDeepLink('wapc://import?source=a&resource=b'), true)
  assert.equal(isWapcImportDeepLink('wapc://settings'), false)
  assert.equal(isWapcImportDeepLink('https://example.test/import'), false)
})

test('dispatches startup and runtime wapc import URLs without dispatching unrelated URLs', async () => {
  const received: string[] = []
  let runtimeHandler: ((urls: string[]) => void) | null = null
  const unlisten = await attachWapcDeepLinkHandler(
    {
      getCurrent: async () => [
        'https://example.test/import',
        'wapc://import?source=start&resource=payload',
      ],
      onOpenUrl: async handler => {
        runtimeHandler = handler
        return () => {
          runtimeHandler = null
        }
      },
    },
    url => received.push(url),
  )

  runtimeHandler?.([
    'wapc://settings',
    'wapc://import?source=runtime&resource=payload',
  ])
  assert.deepEqual(received, [
    'wapc://import?source=start&resource=payload',
    'wapc://import?source=runtime&resource=payload',
  ])

  unlisten()
  assert.equal(runtimeHandler, null)
})

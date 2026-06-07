/**
 * Resource Center state contract tests.
 * @author codex
 */

import assert from 'node:assert/strict'
import test from 'node:test'
import {
  buildDeepLinkImportPlanRequest,
  buildDeepLinkPreviewSummary,
  buildSyncPresetFromSelection,
  buildSyncPresetSummary,
  buildSyncOperationSummary,
  buildSyncApplyResultSummary,
  formatApplySyncNotice,
  buildResourceBackupSummary,
  buildDriftResolutionActions,
  buildResourceManagementActions,
  formatApplyChangeNotice,
  getManualEnvApplyDisabledReason,
  buildResourceGuideSummary,
  buildSyncTargetOptions,
  buildTemplatePreviewResource,
  buildResourceCenterSummary,
  buildResourceKindGroups,
  canPlanResourceSync,
  classifyResourceCenterState,
  getResourceManagementCapability,
  selectedToolsFromSyncPreset,
  selectedSyncTargetsRequireCrossScope,
  syncOperationMatchesResource,
  syncPlanHasApplicableTargets,
  type ResourceCenterState,
} from '../src/pages/resourceCenterState.ts'
import type { CanonicalResource, DeepLinkImportPreview, ResourceBackup, ResourceGuide, ResourceParseFailure, ResourceTemplate, SyncOperation, SyncPreset } from '../src/types/index.ts'

const baseResource: CanonicalResource = {
  id: 'mcp:user:claude:github',
  kind: 'mcp',
  name: 'github',
  scope: 'user',
  origin_tool: 'claude',
  origin_path: '/Users/example/.claude.json',
  origin_locator: 'mcpServers.github',
  enabled_in: ['claude'],
  confidence: 0.95,
  redacted: true,
  payload_json: '{"command":"gh","args":["api"]}',
  provided_by_plugin: null,
  last_seen: '2026-06-06T08:00:00Z',
}

const parseFailure: ResourceParseFailure = {
  path: '/Users/example/.mcp.json',
  tool: 'claude',
  kind: 'mcp',
  reason: 'invalid json',
  seen_at: '2026-06-06T08:00:00Z',
}

function classify(args: {
  resources?: CanonicalResource[]
  failures?: ResourceParseFailure[]
  scanning?: boolean
  error?: string | null
}): ResourceCenterState {
  return classifyResourceCenterState({
    resources: args.resources ?? [],
    failures: args.failures ?? [],
    scanning: args.scanning ?? false,
    error: args.error ?? null,
  })
}

test('classifies loading, empty, error, and normal states', () => {
  assert.equal(classify({ scanning: true }).name, 'loading')
  assert.equal(classify({}).name, 'empty')
  assert.equal(classify({ failures: [parseFailure] }).name, 'error')
  assert.equal(classify({ error: 'permission denied' }).name, 'error')
  assert.equal(classify({ resources: [baseResource] }).name, 'normal')
})

test('keeps valid resources visible when parse failures are partial', () => {
  const state = classify({ resources: [baseResource], failures: [parseFailure] })

  assert.equal(state.name, 'normal')
  assert.equal(state.severity, 'warning')
  assert.match(state.description, /1 个文件解析失败/)
})

test('builds real-data summary counts for state banners', () => {
  const summary = buildResourceCenterSummary([baseResource], [parseFailure])

  assert.deepEqual(summary.kindCounts, [['mcp', 1]])
  assert.deepEqual(summary.scopeCounts, [['user', 1]])
  assert.equal(summary.redacted, 1)
  assert.equal(summary.parseFailures, 1)
})

test('builds stable canonical kind groups with real counts', () => {
  const instructionResource: CanonicalResource = {
    ...baseResource,
    id: 'instruction:user:codex:agents',
    kind: 'instruction',
    name: 'AGENTS.md',
    redacted: false,
  }

  const groups = buildResourceKindGroups([baseResource, instructionResource])

  assert.deepEqual(
    groups.map(group => [group.kind, group.label, group.count]),
    [
      ['all', '全部资源', 2],
      ['mcp', 'MCP', 1],
      ['skill', 'Skills', 0],
      ['plugin', 'Plugins', 0],
      ['instruction', '指令文件', 1],
      ['subagent', 'Subagents', 0],
    ],
  )
})

test('enables disable action only for user json mcp resources without plugin ownership', () => {
  const capability = getResourceManagementCapability(baseResource)

  assert.equal(capability.enabled, true)
  assert.equal(capability.action, 'disable_mcp')
  assert.deepEqual(capability.request, {
    tool: 'claude',
    kind: 'mcp',
    op: 'disable',
    resource_id: 'mcp:user:claude:github',
    target_path: '/Users/example/.claude.json',
    resource_name: 'github',
  })
})

test('keeps unsupported resource management entries visibly read-only', () => {
  assert.equal(
    getResourceManagementCapability({ ...baseResource, scope: 'enterprise' }).reason,
    '企业或托管范围资源保持只读',
  )
  assert.equal(
    getResourceManagementCapability({ ...baseResource, provided_by_plugin: 'github-tools' }).reason,
    '插件提供的资源由插件管理，当前保持只读',
  )
  assert.equal(
    getResourceManagementCapability({ ...baseResource, kind: 'skill' }).reason,
    '当前切片仅开放 JSON MCP 禁用',
  )
})

test('keeps read-only scanned mcp dialects out of disable actions', () => {
  const opencodeCapability = getResourceManagementCapability({
    ...baseResource,
    origin_tool: 'opencode',
    origin_path: '/Users/example/.config/opencode/opencode.json',
    origin_locator: 'mcp.docs',
  })

  assert.equal(opencodeCapability.enabled, false)
  assert.equal(opencodeCapability.action, null)
  assert.equal(opencodeCapability.request, null)
  assert.equal(opencodeCapability.reason, '当前仅支持 Claude/Cursor JSON MCP 禁用')

  const vscodeCapability = getResourceManagementCapability({
    ...baseResource,
    origin_tool: 'vscode',
    origin_path: '/Users/example/repo/.vscode/mcp.json',
    origin_locator: 'servers.docs',
  })

  assert.equal(vscodeCapability.enabled, false)
  assert.equal(vscodeCapability.action, null)
  assert.equal(vscodeCapability.request, null)
  assert.equal(vscodeCapability.reason, '当前仅支持 Claude/Cursor JSON MCP 禁用')
})

test('builds explicit resource management action list with unsupported operations visible', () => {
  const actions = buildResourceManagementActions(baseResource)

  assert.deepEqual(actions.map(action => [action.action, action.label, action.enabled]), [
    ['disable_mcp', '禁用 MCP', true],
    ['enable_mcp', '启用 MCP', false],
    ['edit_mcp', '编辑 MCP', false],
    ['delete_mcp', '删除 MCP', false],
  ])
  assert.equal(actions[0].request?.op, 'disable')
  assert(actions.slice(1).every(action => action.request === null))
  assert(actions.slice(1).every(action => action.reason.includes('暂未开放')))

  const enterpriseActions = buildResourceManagementActions({ ...baseResource, scope: 'enterprise' })
  assert(enterpriseActions.every(action => !action.enabled))
  assert(enterpriseActions.every(action => action.request === null))
  assert(enterpriseActions.every(action => action.reason === '企业或托管范围资源保持只读'))
})

test('builds resource guide summary without exposing secrets or body content', () => {
  const guide: ResourceGuide = {
    id: 'guide:claude:mcp',
    tool: 'claude',
    kind: 'mcp',
    title: 'Claude Code MCP 使用说明',
    summary: '用于说明安全管理边界。',
    sections: [
      {
        title: '配置要点',
        body: 'MCP 配置只展示结构和脱敏字段，写入必须走 Sync Engine。',
      },
      {
        title: '安全提醒',
        body: '不要在 WAPC 中持久化密钥正文。',
      },
    ],
    risks: ['备份可能包含目标配置中原有密钥'],
    unsupported_actions: ['enterprise 范围资源不允许写入'],
    updated_at: '2026-06-06T00:00:00Z',
  }

  const summary = buildResourceGuideSummary(guide)

  assert.equal(summary.title, 'Claude Code MCP 使用说明')
  assert.equal(summary.sectionCount, 2)
  assert.equal(summary.hasRiskWarnings, true)
  assert.equal(summary.hasUnsupportedActions, true)
  assert(!summary.searchText.includes('secret-token'))
})

test('builds real user json mcp sync targets from home path and excludes source tool', () => {
  const options = buildSyncTargetOptions({ ...baseResource, origin_tool: 'claude' }, '/Users/example')

  assert.deepEqual(options.map(option => [option.id, option.tool, option.target_path, option.format]), [
    ['codex', 'codex', '/Users/example/.codex/config.toml', 'toml'],
    ['gemini', 'gemini', '/Users/example/.gemini/settings.json', 'json'],
    ['cursor', 'cursor', '/Users/example/.cursor/mcp.json', 'json'],
  ])
})

test('builds a template preview resource for target selection without persisted origin tool', () => {
  const template: ResourceTemplate = {
    id: 'builtin:context7-mcp',
    name: 'Context7 MCP',
    kind: 'mcp',
    scope: 'user',
    description: 'Current docs MCP',
    source: 'https://context7.com/docs/resources/all-clients',
    content_fingerprint: '0123456789abcdef',
    required_env_keys: ['CONTEXT7_API_KEY'],
    payload_json: '{"command":"npx","args":["-y","@upstash/context7-mcp"]}',
    updated_at: '2026-06-06T00:00:00Z',
  }

  const resource = buildTemplatePreviewResource(template)
  const options = buildSyncTargetOptions(resource, '/Users/example')

  assert.equal(resource.id, 'template:builtin:context7-mcp:0123456789abcdef')
  assert.equal(resource.origin_tool, 'template-library')
  assert.equal(resource.redacted, true)
  assert.deepEqual(options.map(option => option.tool), ['codex', 'claude', 'gemini', 'cursor'])
})

test('builds complete deep link preview summary from backend preview payload', () => {
  const preview: DeepLinkImportPreview = {
    schema: 'wapc.deep_link_import_preview.v1',
    source: 'https://example.test/templates/docs-mcp',
    content_fingerprint: '0123456789abcdef',
    risks: ['source is not https; review origin before syncing'],
    resource: {
      ...baseResource,
      id: 'deep-link:mcp:docs:0123456789abcdef',
      name: 'docs',
      origin_tool: 'deep-link',
      origin_path: 'https://example.test/templates/docs-mcp',
      origin_locator: 'wapc://import',
      enabled_in: [],
      payload_json: '{"transport":"http","url":"https://example.test/mcp","env_keys":["DOCS_TOKEN"]}',
    },
  }

  const summary = buildDeepLinkPreviewSummary(preview)

  assert.equal(summary.resourceLabel, 'mcp · docs')
  assert.equal(summary.scope, 'user')
  assert.equal(summary.source, 'https://example.test/templates/docs-mcp')
  assert.equal(summary.contentFingerprint, '0123456789abcdef')
  assert.equal(summary.payloadJson, preview.resource.payload_json)
  assert.equal(summary.risks.length, 1)
  assert.match(summary.boundary, /预览/)
  assert.match(summary.boundary, /Sync Engine/)
})

test('builds deep link import plan request from selected real sync targets', () => {
  const preview: DeepLinkImportPreview = {
    schema: 'wapc.deep_link_import_preview.v1',
    source: 'https://example.test/templates/docs-mcp',
    content_fingerprint: '0123456789abcdef',
    risks: [],
    resource: {
      ...baseResource,
      id: 'deep-link:mcp:docs:0123456789abcdef',
      name: 'docs',
      origin_tool: 'deep-link',
      origin_path: 'https://example.test/templates/docs-mcp',
      origin_locator: 'wapc://import',
      enabled_in: [],
      payload_json: '{"transport":"http","url":"https://example.test/mcp","env_keys":["DOCS_TOKEN"]}',
    },
  }
  const targets = buildSyncTargetOptions(preview.resource, '/Users/example', '/Users/example/repo')

  const request = buildDeepLinkImportPlanRequest({
    url: '  wapc://import?source=x&resource=y  ',
    targets,
    selectedTargetIds: ['gemini', 'project:claude'],
    allowCrossScope: true,
    envStrategy: 'manual',
  })

  assert.deepEqual(request, {
    url: 'wapc://import?source=x&resource=y',
    targets: [
      {
        tool: 'gemini',
        scope: 'user',
        project_path: null,
        target_path: '/Users/example/.gemini/settings.json',
        format: 'json',
      },
      {
        tool: 'claude',
        scope: 'project',
        project_path: '/Users/example/repo',
        target_path: '/Users/example/repo/.mcp.json',
        format: 'json',
      },
    ],
    allow_cross_scope: true,
    env_strategy: 'manual',
  })
})

test('builds explicit project sync targets only when a project path is provided', () => {
  const withoutProjectPath = buildSyncTargetOptions({ ...baseResource, origin_tool: 'claude' }, '/Users/example')
  assert.equal(withoutProjectPath.some(option => option.scope === 'project'), false)

  const options = buildSyncTargetOptions(
    { ...baseResource, origin_tool: 'claude' },
    '/Users/example',
    '/Users/example/repo',
  )

  assert.deepEqual(
    options
      .filter(option => option.scope === 'project')
      .map(option => [option.id, option.tool, option.label, option.project_path, option.target_path, option.format]),
    [
      ['project:claude', 'claude', 'Claude Project', '/Users/example/repo', '/Users/example/repo/.mcp.json', 'json'],
      ['project:cursor', 'cursor', 'Cursor Project', '/Users/example/repo', '/Users/example/repo/.cursor/mcp.json', 'json'],
    ],
  )
})

test('enables sync planning for user and project mcp resources while keeping enterprise read-only', () => {
  assert.equal(canPlanResourceSync(baseResource).enabled, true)
  assert.equal(canPlanResourceSync({ ...baseResource, scope: 'project' }).enabled, true)
  assert.equal(canPlanResourceSync({ ...baseResource, scope: 'enterprise' }).enabled, false)
  assert.equal(canPlanResourceSync({ ...baseResource, scope: 'managed' }).enabled, false)
  assert.equal(canPlanResourceSync({ ...baseResource, kind: 'skill' }).enabled, false)
})

test('builds user targets for project mcp sources and requires explicit cross-scope authorization', () => {
  const projectResource = { ...baseResource, scope: 'project', origin_tool: 'cursor' }
  const options = buildSyncTargetOptions(projectResource, '/Users/example')

  assert.deepEqual(options.map(option => [option.tool, option.scope, option.format]), [
    ['codex', 'user', 'toml'],
    ['claude', 'user', 'json'],
    ['gemini', 'user', 'json'],
  ])
  assert.equal(selectedSyncTargetsRequireCrossScope(projectResource, options, ['codex']), true)
})

test('detects when selected sync targets require explicit cross-scope authorization', () => {
  const userTargetOptions = buildSyncTargetOptions({ ...baseResource, origin_tool: 'claude' }, '/Users/example')
  assert.equal(selectedSyncTargetsRequireCrossScope(baseResource, userTargetOptions, ['gemini']), false)

  const projectResource = { ...baseResource, scope: 'project' }
  const mixedTargets = [
    ...userTargetOptions,
    {
      ...userTargetOptions[0],
      id: 'project:cursor',
      tool: 'cursor',
      label: 'Project Cursor',
      scope: 'project',
      project_path: '/Users/example/project',
      target_path: '/Users/example/project/.cursor/mcp.json',
    },
  ]

  assert.equal(selectedSyncTargetsRequireCrossScope(projectResource, mixedTargets, ['gemini']), true)
  assert.equal(selectedSyncTargetsRequireCrossScope(projectResource, mixedTargets, ['project:cursor']), false)
})

test('detects applicable sync plans from backend target statuses', () => {
  assert.equal(syncPlanHasApplicableTargets({ source_resource_id: 'mcp:user:codex:github', created_at: 'now', targets: [] }), false)
  assert.equal(
    syncPlanHasApplicableTargets({
      source_resource_id: 'mcp:user:codex:github',
      created_at: 'now',
      targets: [
        {
          target: {
            tool: 'gemini',
            scope: 'user',
            project_path: null,
            target_path: '/Users/example/.gemini/settings.json',
            format: 'json',
          },
          status: 'unsupported',
          reason: 'missing file',
          required_env_keys: [],
          plan: null,
        },
      ],
    }),
    false,
  )
  assert.equal(
    syncPlanHasApplicableTargets({
      source_resource_id: 'mcp:user:codex:github',
      created_at: 'now',
      targets: [
        {
          target: {
            tool: 'gemini',
            scope: 'user',
            project_path: null,
            target_path: '/Users/example/.gemini/settings.json',
            format: 'json',
          },
          status: 'planned',
          reason: null,
          required_env_keys: [],
          plan: {
            plan_id: 'plan:1',
            tool: 'gemini',
            kind: 'mcp',
            op: 'sync',
            resource_id: 'mcp:user:codex:github',
            resource_name: 'github',
            target_path: '/Users/example/.gemini/settings.json',
            before_fingerprint: 'before',
            after_fingerprint: 'after',
            diff: '+github',
            preview_before: '{}',
            preview_after: '{"mcpServers":{"github":{}}}',
            requires_backup: true,
            risks: [],
            created_at: 'now',
          },
        },
      ],
    }),
    true,
  )
})

test('summarizes sync apply result targets with rollback eligibility', () => {
  const summary = buildSyncApplyResultSummary({
    sync_id: 'sync:multi',
    changes: [
      {
        plan_id: 'plan:codex',
        target_path: '/Users/example/.codex/config.toml',
        status: 'committed',
        change_id: 'chg:codex',
        backup_path: '/Users/example/.wapc/backups/codex/config.toml',
        reason: null,
      },
      {
        plan_id: 'plan:gemini',
        target_path: '/Users/example/.gemini/settings.json',
        status: 'noop',
        change_id: 'plan:gemini',
        backup_path: null,
        reason: null,
      },
      {
        plan_id: 'plan:cursor',
        target_path: '/Users/example/.cursor/mcp.json',
        status: 'failed',
        change_id: 'chg:cursor-failed',
        backup_path: null,
        reason: 'drift detected',
      },
    ],
  })

  assert.equal(summary.syncId, 'sync:multi')
  assert.equal(summary.committedCount, 1)
  assert.equal(summary.noopCount, 1)
  assert.equal(summary.failedCount, 1)
  assert.deepEqual(summary.targets.map(target => [target.planId, target.rollbackable, target.reason]), [
    ['plan:codex', true, null],
    ['plan:gemini', false, '目标已是同步后的状态，未产生新变更'],
    ['plan:cursor', false, 'drift detected'],
  ])
  assert.equal(formatApplySyncNotice(summary), '同步完成 sync:multi，成功 1 个目标，已是最新 1 个目标，失败 1 个目标')
})

test('summarizes resource backups with source change and original path', () => {
  const backup: ResourceBackup = {
    backup_path: '/Users/example/.wapc/backups/claude/20260607/.claude.json',
    tool: 'claude',
    original_path: '/Users/example/.claude.json',
    change_id: 'chg:disable-github',
    created_at: '2026-06-07T09:10:11Z',
  }

  const summary = buildResourceBackupSummary(backup)

  assert.equal(summary.backupPath, backup.backup_path)
  assert.equal(summary.tool, 'claude')
  assert.equal(summary.originalPath, '/Users/example/.claude.json')
  assert.equal(summary.sourceChangeId, 'chg:disable-github')
  assert.equal(summary.hasSourceChange, true)
  assert.match(summary.sourceLabel, /来源变更 chg:disable-github/)
})

test('builds drift resolution actions with real rescan and explicit overwrite', () => {
  const driftActions = buildDriftResolutionActions(true, false)

  assert.equal(driftActions.showRescan, true)
  assert.equal(driftActions.rescanDisabled, false)
  assert.equal(driftActions.rescanLabel, '以工具现状为准重新识别')
  assert.equal(driftActions.applyLabel, '确认覆盖当前状态')

  const normalActions = buildDriftResolutionActions(false, false)

  assert.equal(normalActions.showRescan, false)
  assert.equal(normalActions.applyLabel, '确认写入')
})

test('formats apply change notices without claiming no-op writes were committed', () => {
  assert.equal(
    formatApplyChangeNotice({
      change_id: 'chg:disable-github',
      backup_path: '/Users/example/.wapc/backups/claude/.claude.json',
      status: 'committed',
    }),
    '已提交变更 chg:disable-github',
  )
  assert.equal(
    formatApplyChangeNotice({
      change_id: 'plan:disable-github',
      backup_path: null,
      status: 'noop',
    }),
    '写入计划已应用，当前文件已是目标状态，未产生新变更',
  )
})

test('summarizes persisted sync operations from real target metadata without secrets', () => {
  const operation: SyncOperation = {
    sync_id: 'sync:abc123',
    source_resource_id: 'mcp:user:codex:github',
    targets_json: JSON.stringify([
      {
        plan_id: 'plan:gemini',
        tool: 'gemini',
        kind: 'mcp',
        op: 'sync',
        target_path: '/Users/example/.gemini/settings.json',
      },
      {
        plan_id: 'plan:cursor',
        tool: 'cursor',
        kind: 'mcp',
        op: 'sync',
        target_path: '/Users/example/.cursor/mcp.json',
      },
    ]),
    allow_cross_scope: false,
    env_strategy: 'manual',
    created_at: '2026-06-06T08:00:00Z',
  }

  const summary = buildSyncOperationSummary(operation)

  assert.equal(summary.syncId, 'sync:abc123')
  assert.equal(summary.targetCount, 2)
  assert.deepEqual(summary.targetTools, ['cursor', 'gemini'])
  assert.deepEqual(summary.targetPaths, ['/Users/example/.gemini/settings.json', '/Users/example/.cursor/mcp.json'])
  assert.equal(summary.envStrategy, 'manual')
  assert.equal(summary.parseError, null)
  assert.equal(JSON.stringify(summary).includes('secret'), false)
})

test('keeps malformed sync operation target metadata explicit', () => {
  const summary = buildSyncOperationSummary({
    sync_id: 'sync:bad',
    source_resource_id: null,
    targets_json: '{bad-json',
    allow_cross_scope: false,
    env_strategy: 'none',
    created_at: '2026-06-06T08:00:00Z',
  })

  assert.equal(summary.targetCount, 0)
  assert.equal(summary.parseError, '目标元数据解析失败')
})

test('matches sync operations to the selected resource by source id or target path', () => {
  const bySource: SyncOperation = {
    sync_id: 'sync:source',
    source_resource_id: baseResource.id,
    targets_json: '[]',
    allow_cross_scope: false,
    env_strategy: 'none',
    created_at: '2026-06-06T08:00:00Z',
  }
  const byTarget: SyncOperation = {
    sync_id: 'sync:target',
    source_resource_id: null,
    targets_json: JSON.stringify([{ tool: 'codex', target_path: baseResource.origin_path }]),
    allow_cross_scope: false,
    env_strategy: 'reuse',
    created_at: '2026-06-06T08:00:00Z',
  }

  assert.equal(syncOperationMatchesResource(bySource, baseResource), true)
  assert.equal(syncOperationMatchesResource(byTarget, baseResource), true)
  assert.equal(
    syncOperationMatchesResource({ ...byTarget, targets_json: JSON.stringify([{ target_path: '/tmp/other.json' }]) }, baseResource),
    false,
  )
})

test('builds sync preset payload from current resource and selected available targets without env values', () => {
  const options = buildSyncTargetOptions({ ...baseResource, origin_tool: 'claude' }, '/Users/example')
  const preset = buildSyncPresetFromSelection({
    resource: baseResource,
    targets: options,
    selectedTools: ['gemini'],
    name: 'GitHub MCP targets',
    now: '2026-06-06T08:00:00Z',
  })

  assert.equal(preset.id, 'preset:github-mcp-targets:2026-06-06t08-00-00z')
  assert.equal(preset.name, 'GitHub MCP targets')
  assert.deepEqual(JSON.parse(preset.resources_json), [baseResource.id])
  assert.deepEqual(JSON.parse(preset.targets_json), [
    {
      tool: 'gemini',
      scope: 'user',
      project_path: null,
      target_path: '/Users/example/.gemini/settings.json',
      format: 'json',
    },
  ])
  assert.equal(JSON.stringify(preset).includes('env_values'), false)
})

test('requires non-empty manual env values before sync apply', () => {
  const result = {
    source_resource_id: 'template:builtin:context7-mcp:fingerprint',
    created_at: '2026-06-07T00:00:00Z',
    targets: [
      {
        target: {
          tool: 'claude',
          scope: 'user',
          project_path: null,
          target_path: '/Users/example/.claude.json',
          format: 'json',
        },
        status: 'planned',
        reason: null,
        required_env_keys: ['CONTEXT7_API_KEY', 'DOCS_TOKEN'],
        plan: null,
      },
    ],
  }

  assert.equal(
    getManualEnvApplyDisabledReason(result, 'manual', {
      CONTEXT7_API_KEY: '   ',
      DOCS_TOKEN: 'docs-token',
    }),
    '请先填写手动 env：CONTEXT7_API_KEY',
  )
  assert.equal(
    getManualEnvApplyDisabledReason(result, 'manual', {
      CONTEXT7_API_KEY: 'context-token',
      DOCS_TOKEN: 'docs-token',
    }),
    null,
  )
  assert.equal(getManualEnvApplyDisabledReason(result, 'reuse', {}), null)
})

test('summarizes sync presets and keeps malformed preset metadata explicit', () => {
  const preset: SyncPreset = {
    id: 'preset:github',
    name: 'GitHub MCP targets',
    resources_json: JSON.stringify([baseResource.id]),
    targets_json: JSON.stringify([{ tool: 'gemini', target_path: '/Users/example/.gemini/settings.json' }]),
    updated_at: '2026-06-06T08:00:00Z',
  }

  const summary = buildSyncPresetSummary(preset)
  assert.equal(summary.resourceCount, 1)
  assert.deepEqual(summary.targetTools, ['gemini'])
  assert.equal(summary.parseError, null)

  const malformed = buildSyncPresetSummary({ ...preset, targets_json: '{bad-json' })
  assert.equal(malformed.targetCount, 0)
  assert.equal(malformed.parseError, '预设元数据解析失败')
})

test('selects only currently available targets when applying a sync preset', () => {
  const options = buildSyncTargetOptions({ ...baseResource, origin_tool: 'claude' }, '/Users/example')
  const preset: SyncPreset = {
    id: 'preset:github',
    name: 'GitHub MCP targets',
    resources_json: JSON.stringify([baseResource.id]),
    targets_json: JSON.stringify([
      { tool: 'gemini', target_path: '/Users/example/.gemini/settings.json' },
      { tool: 'codex', target_path: '/Users/example/.codex/config.toml' },
    ]),
    updated_at: '2026-06-06T08:00:00Z',
  }

  assert.deepEqual(selectedToolsFromSyncPreset(preset, options), ['codex', 'gemini'])
})

test('applies sync presets by exact target path when user and project targets share a tool', () => {
  const options = buildSyncTargetOptions(
    { ...baseResource, origin_tool: 'claude' },
    '/Users/example',
    '/Users/example/repo',
  )
  const preset: SyncPreset = {
    id: 'preset:cursor-user',
    name: 'Cursor user target',
    resources_json: JSON.stringify([baseResource.id]),
    targets_json: JSON.stringify([
      { tool: 'cursor', target_path: '/Users/example/.cursor/mcp.json' },
    ]),
    updated_at: '2026-06-06T08:00:00Z',
  }

  assert.deepEqual(selectedToolsFromSyncPreset(preset, options), ['cursor'])
})

/**
 * PricingPage — local pricing rules and historical cost recomputation.
 * @author codex
 */

import { useCallback, useEffect, useState } from 'react'
import type { ReactNode } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { Save, Trash2, Calculator } from 'lucide-react'
import type { CostRecomputeResult, PricingRule } from '../types'
import { PageHeader } from '../components/layout/PageHeader'

type FormState = {
  model_match: string
  match_kind: 'exact' | 'prefix'
  price_input: string
  price_output: string
  price_cache_read: string
  price_cache_write: string
  price_reasoning: string
  price_tool: string
}

const EMPTY_FORM: FormState = {
  model_match: '',
  match_kind: 'exact',
  price_input: '',
  price_output: '',
  price_cache_read: '',
  price_cache_write: '',
  price_reasoning: '',
  price_tool: '',
}

export function PricingPage({ 
  setIsSidebarOpen
}: { 
  setIsSidebarOpen?: (o: boolean) => void;
}) {
  const [rules, setRules] = useState<PricingRule[]>([])
  const [form, setForm] = useState<FormState>(EMPTY_FORM)
  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)
  const [message, setMessage] = useState<string | null>(null)

  const loadRules = useCallback(async () => {
    setLoading(true)
    try {
      setRules(await invoke<PricingRule[]>('list_pricing_rules'))
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    void Promise.resolve()
      .then(loadRules)
      .catch(err => setMessage(`读取失败：${err}`))
  }, [loadRules])

  const saveRule = async () => {
    if (!form.model_match.trim()) {
      setMessage('模型匹配不能为空')
      return
    }
    setSaving(true)
    try {
      await invoke<PricingRule>('upsert_pricing_rule', { rule: toPricingRule(form) })
      setForm(EMPTY_FORM)
      setMessage('价格规则已保存')
      await loadRules()
    } catch (err) {
      setMessage(`保存失败：${err}`)
    } finally {
      setSaving(false)
    }
  }

  const deleteRule = async (id: number | null) => {
    if (id == null) return
    await invoke('delete_pricing_rule', { id })
    setMessage('价格规则已删除')
    await loadRules()
  }

  const recompute = async () => {
    setSaving(true)
    try {
      const result = await invoke<CostRecomputeResult>('recompute_costs')
      setMessage(`已重算 ${result.updated} 条记录：精确 ${result.exact_matches}，前缀 ${result.prefix_matches}，未知 ${result.no_matches}`)
    } catch (err) {
      setMessage(`重算失败：${err}`)
    } finally {
      setSaving(false)
    }
  }

  return (
    <div className="flex flex-col h-full fade-in">
      <div className="px-6 pt-6 pb-2 border-b border-border bg-page shrink-0">
        <PageHeader 
          title="模型价格配置"
          subtitle="本地化管理大模型 API 的成本规则及预估"
          setIsSidebarOpen={setIsSidebarOpen}
        />
      </div>
      
      <div className="flex-1 overflow-y-auto p-4 sm:p-6 custom-scrollbar bg-page">
        <div className="grid grid-cols-1 xl:grid-cols-[1fr_380px] gap-4 max-w-7xl mx-auto">
          <div className="card p-4 sm:p-[18px] min-w-0">
        <div className="flex items-center justify-between gap-3 mb-4">
          <div className="text-[15px] font-bold text-heading">本地价格规则</div>
          <button
            onClick={recompute}
            disabled={saving}
            className="h-9 inline-flex items-center gap-2 px-3 rounded-lg border border-brand-blue/30 bg-brand-blue/5 text-brand-blue text-[13px] font-semibold disabled:opacity-60"
          >
            <Calculator size={15} />重算历史费用
          </button>
        </div>
        {loading ? (
          <div className="text-sm text-muted">加载中...</div>
        ) : rules.length === 0 ? (
          <div className="text-sm text-muted">暂无本地价格规则。无匹配模型会保持价格未知。</div>
        ) : (
          <div className="overflow-x-auto">
            <table className="w-full border-collapse text-[13px]">
              <thead>
                <tr>
                  <TableHead>匹配</TableHead>
                  <TableHead>方式</TableHead>
                  <TableHead>输入</TableHead>
                  <TableHead>输出</TableHead>
                  <TableHead>缓存读</TableHead>
                  <TableHead>缓存写</TableHead>
                  <TableHead>推理</TableHead>
                  <TableHead>工具</TableHead>
                  <TableHead>操作</TableHead>
                </tr>
              </thead>
              <tbody>
                {rules.map(rule => (
                  <tr key={rule.id ?? rule.model_match} className="border-t border-border-soft hover:bg-surface-hover">
                    <td className="py-2.5 pr-4 font-semibold text-text whitespace-nowrap">{rule.model_match}</td>
                    <td className="py-2.5 pr-4 text-muted">{rule.match_kind === 'exact' ? '精确' : '前缀'}</td>
                    <PriceCell value={rule.price_input} />
                    <PriceCell value={rule.price_output} />
                    <PriceCell value={rule.price_cache_read} />
                    <PriceCell value={rule.price_cache_write} />
                    <PriceCell value={rule.price_reasoning} />
                    <PriceCell value={rule.price_tool} />
                    <td className="py-2.5 pr-4">
                      <button
                        onClick={() => deleteRule(rule.id).catch(err => setMessage(`删除失败：${err}`))}
                        className="h-8 w-8 inline-flex items-center justify-center rounded-lg border border-border text-muted hover:text-red-600 hover:border-red-200"
                        title="删除"
                      >
                        <Trash2 size={14} />
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
        {message && <div className="mt-4 text-[13px] text-muted">{message}</div>}
      </div>

      <div className="card p-4 sm:p-[18px]">
        <div className="text-[15px] font-bold text-heading mb-4">新增规则</div>
        <Field label="模型匹配">
          <input value={form.model_match} onChange={event => setForm({ ...form, model_match: event.target.value })} className={inputClass} placeholder="claude-opus 或 claude-" />
        </Field>
        <Field label="匹配方式">
          <select value={form.match_kind} onChange={event => setForm({ ...form, match_kind: event.target.value as FormState['match_kind'] })} className={inputClass}>
            <option value="exact">精确</option>
            <option value="prefix">前缀</option>
          </select>
        </Field>
        <div className="grid grid-cols-2 gap-3">
          <Field label="输入">
            <PriceInput value={form.price_input} onChange={value => setForm({ ...form, price_input: value })} />
          </Field>
          <Field label="输出">
            <PriceInput value={form.price_output} onChange={value => setForm({ ...form, price_output: value })} />
          </Field>
          <Field label="缓存读">
            <PriceInput value={form.price_cache_read} onChange={value => setForm({ ...form, price_cache_read: value })} />
          </Field>
          <Field label="缓存写">
            <PriceInput value={form.price_cache_write} onChange={value => setForm({ ...form, price_cache_write: value })} />
          </Field>
          <Field label="推理">
            <PriceInput value={form.price_reasoning} onChange={value => setForm({ ...form, price_reasoning: value })} />
          </Field>
          <Field label="工具">
            <PriceInput value={form.price_tool} onChange={value => setForm({ ...form, price_tool: value })} />
          </Field>
        </div>
        <button
          onClick={saveRule}
          disabled={saving}
          className="mt-2 w-full h-10 inline-flex items-center justify-center gap-2 rounded-lg border border-brand-blue/30 bg-brand-blue/10 text-brand-blue text-[13px] font-semibold disabled:opacity-60"
        >
          <Save size={15} />保存规则
        </button>
      </div>
        </div>
      </div>
    </div>
  )
}

function toPricingRule(form: FormState): PricingRule {
  return {
    id: null,
    model_match: form.model_match.trim(),
    match_kind: form.match_kind,
    provider: null,
    currency: 'USD',
    price_input: parsePrice(form.price_input),
    price_output: parsePrice(form.price_output),
    price_cache_read: parsePrice(form.price_cache_read),
    price_cache_write: parsePrice(form.price_cache_write),
    price_reasoning: parsePrice(form.price_reasoning),
    price_tool: parsePrice(form.price_tool),
    source: 'user',
    updated_at: new Date().toISOString(),
  }
}

function parsePrice(value: string): number | null {
  if (value.trim() === '') return null
  const parsed = Number(value)
  return Number.isFinite(parsed) ? parsed : null
}

function PriceInput({ value, onChange }: { value: string; onChange: (value: string) => void }) {
  return <input value={value} onChange={event => onChange(event.target.value)} className={inputClass} inputMode="decimal" placeholder="USD / 1M" />
}

function PriceCell({ value }: { value: number | null }) {
  return <td className="py-2.5 pr-4 text-muted whitespace-nowrap">{value == null ? '-' : `$${value.toFixed(4)}`}</td>
}

function TableHead({ children }: { children: ReactNode }) {
  return <th className="text-left pb-2.5 text-muted font-semibold text-[12px] pr-4 whitespace-nowrap">{children}</th>
}

function Field({ label, children }: { label: string; children: ReactNode }) {
  return (
    <label className="block mb-3">
      <span className="block text-[12px] font-semibold text-muted mb-1.5">{label}</span>
      {children}
    </label>
  )
}

const inputClass = 'w-full h-9 rounded-lg border border-border bg-surface px-3 text-[13px] text-text outline-none focus:border-brand-blue/50'

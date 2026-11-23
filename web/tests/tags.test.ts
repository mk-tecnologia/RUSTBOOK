import assert from 'node:assert/strict'
import { test, after } from 'node:test'
import { filterTags, selectTag, tagPopupPlacement } from '../src/utils/tags.ts'
import { createServer } from 'vite'
import { createSSRApp } from 'vue'
import { renderToString } from 'vue/server-renderer'

const tags = [
  { name: 'Produção São Paulo', color: 1 },
  { name: 'Produção Curitiba', color: 2 },
  { name: 'Financeiro', color: 3 },
]
test('searching tags ignores accents/case, matches all terms and does not hide current selections', () => {
  assert.deepEqual(filterTags(tags, 'SAO producao').map(tag => tag.name), ['Produção São Paulo'])
  assert.equal(filterTags(tags, ' produção ').length, 2)
  assert.equal(filterTags(tags, 'missing').length, 0)
  assert.equal(filterTags(tags, '').length, 3)
  const selection = ['Financeiro']
  filterTags(tags, 'producao')
  assert.deepEqual(selection, ['Financeiro'])
})
test('multi-select adds/removes independently while single selection replaces or clears', () => {
  const existing = ['Financeiro', 'Produção Curitiba']
  assert.deepEqual(selectTag(existing, 'Financeiro', true), ['Produção Curitiba'])
  assert.deepEqual(selectTag(existing, 'Produção São Paulo', true), [...existing, 'Produção São Paulo'])
  assert.deepEqual(selectTag(existing, 'Financeiro', false), ['Financeiro'])
  assert.deepEqual(selectTag(existing, '', false), [])
  assert.deepEqual(existing, ['Financeiro', 'Produção Curitiba'])
})
const server = await createServer({ server: { middlewareMode: true, hmr: false, ws: false }, appType: 'custom', optimizeDeps: { noDiscovery: true, include: [] } })
after(() => server.close())
const { default: Picker } = await server.ssrLoadModule('/src/components/tags/TagSelect.vue')
test('hundreds of tags render a compact closed selector and preserve a multi-selection summary', async () => {
  const many = Array.from({ length: 500 }, (_, index) => ({ name: `Customer ${index}`, color: index }))
  const html = await renderToString(createSSRApp(Picker, { id: 'fixture-tags', label: 'Device tags', tags: many, modelValue: ['Customer 1', 'Customer 400'], multiple: true }))
  assert.ok(html.includes('Customer 1 +1'))
  assert.ok(html.includes('title="Customer 1, Customer 400"'))
  assert.ok(!html.includes('Customer 499'))
  assert.ok(!html.includes('role="listbox"'))
  assert.match(html, /aria-expanded="false"/)
})


test('tag popup fits the viewport and opens above when a dialog has little space below', () => {
  const above = tagPopupPlacement({ left: 100, top: 600, bottom: 640, width: 400 }, { left: 0, top: 0, width: 1000, height: 700 }, 310)
  assert.ok(above.top < 600)
  assert.ok(above.top + Math.min(310, above.maxHeight) <= 600)
  const mobile = tagPopupPlacement({ left: 30, top: 260, bottom: 300, width: 400 }, { left: 0, top: 80, width: 320, height: 280 }, 310)
  assert.ok(mobile.left >= 8 && mobile.left + mobile.width <= 312)
  assert.ok(mobile.top >= 88 && mobile.top + Math.min(310, mobile.maxHeight) <= 352)
})

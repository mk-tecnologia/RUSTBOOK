import assert from 'node:assert/strict'
import { after, test } from 'node:test'
import { createServer } from 'vite'
import { createSSRApp } from 'vue'
import { renderToString } from 'vue/server-renderer'
import { createPinia, disposePinia } from 'pinia'

// Compile the actual Vue/TS modules without opening a browser or a network port.
const server = await createServer({ server: { middlewareMode: true, hmr: false, ws: false }, appType: 'custom', optimizeDeps: { noDiscovery: true, include: [] } })
after(() => server.close())
const { useAddressBookStore } = await server.ssrLoadModule('/src/stores/addressBook.ts') as typeof import('../src/stores/addressBook.ts')
const { default: View } = await server.ssrLoadModule('/src/views/AddressBookView.vue')
const { default: Editor } = await server.ssrLoadModule('/src/components/peers/AddPeerDialog.vue')

const groups = [
  { id: 1, name: 'BR SUL', note: 'Customer site', peer_count: 2 },
  { id: 2, name: 'CCS', note: '', peer_count: 0 },
]
const tags = [{ name: 'Production', color: 0xff0071ff }]

test('device folders show full counts and tag filters hide empty folders', async () => {
  const pinia = createPinia()
  try {
    const store = useAddressBookStore(pinia)
    store.$patch({ abGuid: 'fixture', overviewReady: true, totalPeers: 3, ungroupedCount: 1, groups, tags })
    const app = createSSRApp(View); app.use(pinia)
    const html = await renderToString(app)
    for (const text of ['BR SUL', 'CCS', 'Ungrouped', 'Production', 'New group', '3 devices']) assert.ok(html.includes(text), text)
    store.query = 'Office'
    const filteredApp = createSSRApp(View); filteredApp.use(pinia)
    const filtered = await renderToString(filteredApp)
    assert.ok(filtered.includes('BR SUL'))
    assert.ok(!filtered.includes('>CCS<'))
  } finally { disposePinia(pinia) }
})

test('editing a peer retains its ID, folder and selected tags in the form', async () => {
  const peer = { id: '123', alias: 'Office PC', hostname: '', username: '', platform: 'Windows', tags: ['Production'], note: '', group_id: 1 }
  const app = createSSRApp(Editor, { peer, groups, tags, createTag: async () => {}, busy: false })
  const html = await renderToString(app)
  assert.ok(html.includes('Edit device'))
  assert.match(html, /<input(?=[^>]*value="Production")(?=[^>]*checked)[^>]*>/)
  assert.match(html, /value="1"[^>]*selected/)
  assert.match(html, /id="peer-id"[^>]*readonly/)
  assert.ok(html.includes('Office PC'))
})

test('store sends folder and tag associations on create/edit and protects duplicate creates', async (t) => {
  const pinia = createPinia()
  const mutations: { path: string; method: string; body: Record<string, unknown> }[] = []
  const originalFetch = globalThis.fetch
  const storageDescriptor = Object.getOwnPropertyDescriptor(globalThis, 'localStorage')
  Object.defineProperty(globalThis, 'localStorage', { configurable: true, value: { getItem: () => null } })
  let rejectCreate = false
  globalThis.fetch = async (input, init) => {
    const path = String(input)
    const method = init?.method || 'GET'
    if (method !== 'GET') {
      mutations.push({ path, method, body: JSON.parse(String(init?.body || '{}')) })
      if (rejectCreate) return new Response(JSON.stringify({ error: 'Duplicate ID' }), { status: 409 })
      return new Response('')
    }
    if (path.includes('/personal')) return Response.json({ data: { guid: 'fixture' } })
    if (path.includes('/peer-groups/')) return Response.json({ data: groups, total: 3, ungrouped_count: 1 })
    if (path.includes('/tags/')) return Response.json({ data: tags, total: tags.length })
    throw new Error(`Unexpected test request: ${path}`)
  }
  t.after(() => {
    globalThis.fetch = originalFetch
    if (storageDescriptor) Object.defineProperty(globalThis, 'localStorage', storageDescriptor)
    else Reflect.deleteProperty(globalThis, 'localStorage')
    disposePinia(pinia)
  })
  const store = useAddressBookStore(pinia)
  await store.init()
  const peer = { id: '123', alias: 'Office PC', hostname: '', username: '', platform: 'Windows', note: '', tags: ['Production'], group_id: 1 }
  await store.addPeer(peer)
  assert.equal(mutations[0]?.body.fail_if_exists, true)
  assert.equal(mutations[0]?.body.group_id, 1)
  assert.deepEqual(mutations[0]?.body.tags, ['Production'])
  await store.updatePeer({ ...peer, group_id: null, tags: [] })
  assert.equal(mutations[1]?.method, 'PUT')
  assert.equal(mutations[1]?.body.group_id, null)
  assert.deepEqual(mutations[1]?.body.tags, [])
  const revision = store.refreshKey
  rejectCreate = true
  await assert.rejects(store.addPeer(peer), /Duplicate ID/)
  assert.equal(store.refreshKey, revision)
})

test('moving a peer with an unknown platform does not silently assign Windows', async () => {
  const peer = { id: '456', alias: 'Unknown device', hostname: '', username: '', platform: '', tags: [], note: '', group_id: null }
  const app = createSSRApp(Editor, { peer, groups, tags, createTag: async () => {}, busy: false })
  const html = await renderToString(app)
  assert.match(html, /<option value="" selected>Unknown platform<\/option>/)
  assert.doesNotMatch(html, /<option value="Windows" selected>/)
})

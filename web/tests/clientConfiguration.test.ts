import assert from 'node:assert/strict'
import { test, after } from 'node:test'
import { clientConfigurationText } from '../src/utils/clientConfiguration.ts'
import { createSSRApp } from 'vue'
import { renderToString } from 'vue/server-renderer'
import { createServer } from 'vite'
const configuration = { id_server: 'id.example.com:21116', relay_server: 'relay.example.com:21117', api_url: 'https://api.example.com', public_key: 'AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=' }
test('client copy includes precisely the four client fields and never internal configuration', () => {
  const text = clientConfigurationText({ ...configuration, presence_server: 'internal-secret-name:21115', revision: 'not-a-client-value', private_key: 'must-not-export' } as typeof configuration)
  assert.equal(text.split('\n').length, 4)
  for (const value of Object.values(configuration)) assert.ok(text.includes(value))
  assert.ok(!text.includes('internal-secret-name'))
  assert.ok(!text.includes('must-not-export'))
  assert.ok(!text.includes('not-a-client-value'))
})
const server = await createServer({ server: { middlewareMode: true, hmr: false, ws: false }, appType: 'custom', optimizeDeps: { noDiscovery: true, include: [] } })
after(() => server.close())
const { default: Panel } = await server.ssrLoadModule('/src/components/server/ClientConfigurationPanel.vue')
test('client setup renders copyable values and disables full copy when configuration is incomplete', async () => {
  const html = await renderToString(createSSRApp(Panel, { configuration }))
  for (const label of ['Servidor de ID', 'Servidor Relay', 'Servidor da API', 'Chave / Key pública']) assert.ok(html.includes(label))
  assert.match(html, /id="client-public_key"[^>]*readonly/)
  const incomplete = await renderToString(createSSRApp(Panel, { configuration: { ...configuration, relay_server: '' } }))
  assert.match(incomplete, /<button[^>]*disabled[^>]*>Copiar tudo/)
  assert.ok(incomplete.includes('ainda não configurados'))
})

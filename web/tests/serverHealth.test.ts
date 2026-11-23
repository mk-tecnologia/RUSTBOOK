import assert from 'node:assert/strict'
import { test } from 'node:test'
import { serverHealthAction, serverHealthItems, serverHealthSummary } from '../src/utils/serverHealth.ts'
import type { ServerHealth } from '../src/api/serverSettings.ts'
function fixture(): ServerHealth {
  return { report: { settings: { id_server: 'id.example.com:21116', relay_server: 'relay.example.com:21117', api_url: 'https://api.example.com', presence_server: 'hbbs:21115', public_key: 'existing-key', allow_private_relay: false, revision: 'one' }, warnings: [], agent: { connected: true, running: true, can_apply: true, public_key: 'existing-key', relay: 'relay.example.com:21117', configured_relay: 'relay.example.com:21117' }, runtime: { port: 21114, token_expiry_hours: 168, cors_origins: [], anonymous_device_events: false } }, checks: { id_server: 'tcp_reachable', relay_server: 'tcp_reachable', api_url: 'tcp_reachable', presence_server: 'protocol_replied' } }
}
test('dashboard separates TCP, protocol and confirmed persisted relay states', () => {
  const data = fixture(), items = serverHealthItems(data)
  assert.equal(items.find(i => i.key === 'id_server')?.status, 'TCP acessível')
  assert.equal(items.find(i => i.key === 'presence_server')?.status, 'Protocolo respondeu')
  assert.equal(items.find(i => i.key === 'relay_applied')?.status, 'Ativo e persistido')
  assert.equal(serverHealthSummary(items, []).state, 'ok')
  data.report.warnings = ['A API usa HTTP.']
  assert.equal(serverHealthSummary(items, data.report.warnings).state, 'warning')
})
test('dashboard identifies missing configuration and unreachable services', () => {
  const data = fixture()
  data.report.settings.id_server = ''
  data.report.settings.public_key = ''
  data.checks.relay_server = 'unreachable'
  const items = serverHealthItems(data), summary = serverHealthSummary(items, [])
  assert.equal(items.find(i => i.key === 'id_server')?.state, 'missing')
  assert.equal(items.find(i => i.key === 'key')?.state, 'missing')
  assert.equal(items.find(i => i.key === 'relay_server')?.state, 'error')
  assert.equal(summary.errors, 1)
  assert.equal(summary.missing, 2)
  assert.equal(summary.state, 'error')
})
test('dashboard does not claim a saved key or relay is confirmed without a running agent', () => {
  const data = fixture()
  data.report.agent = { connected: false }
  delete data.checks.api_url
  const items = serverHealthItems(data)
  for (const key of ['agent', 'key', 'relay_applied', 'api_url']) assert.equal(items.find(i => i.key === key)?.state, 'warning')
  assert.equal(serverHealthSummary(items, []).state, 'warning')
  data.report.agent = { ...fixture().report.agent, running: false }
  assert.equal(serverHealthItems(data).find(i => i.key === 'relay_applied')?.state, 'warning')
  assert.equal(serverHealthItems(data).find(i => i.key === 'agent')?.state, 'error')
})
test('dashboard detects a mismatched key and a runtime-only relay correction', () => {
  const data = fixture()
  data.report.agent.public_key = 'different-key'
  data.report.agent.configured_relay = 'old.example.com:21117'
  const items = serverHealthItems(data)
  assert.equal(items.find(i => i.key === 'key')?.state, 'error')
  assert.equal(items.find(i => i.key === 'relay_applied')?.status, 'Aplicação pendente')
  assert.equal(serverHealthSummary(items, []).state, 'error')
})
test('dashboard actions point to the configuration needed for each pending item', () => {
  const data = fixture()
  assert.equal(serverHealthAction('id_server', data).to, '/server-settings#server-id')
  assert.equal(serverHealthAction('presence_server', data).to, '/server-settings#server-presence')
  assert.equal(serverHealthAction('api_url', data).to, '/server-settings#server-api')
  assert.equal(serverHealthAction('relay_applied', data).to, '/server-settings#server-apply-relay')
  data.report.agent = { connected: false }
  assert.equal(serverHealthAction('key', data).to, '/server-settings#server-agent')
  assert.equal(serverHealthAction('relay_applied', data).to, '/server-settings#server-agent')
  data.report.settings.public_key = ''
  assert.equal(serverHealthAction('key', data).to, '/server-settings#server-key')
  data.report.settings.relay_server = ''
  assert.equal(serverHealthAction('relay_applied', data).to, '/server-settings#server-relay')
})

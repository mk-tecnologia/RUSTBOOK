import assert from 'node:assert/strict'
import { test } from 'node:test'
import { rustdeskLink } from '../src/utils/connection.ts'

test('native connection link supports numeric and custom IDs without credentials', () => {
  assert.equal(rustdeskLink('295214626'), 'rustdesk://connect/295214626')
  assert.equal(rustdeskLink(' 295 214 626 '), 'rustdesk://connect/295214626')
  assert.equal(rustdeskLink('Office-PC_1'), 'rustdesk://connect/Office-PC_1')
})
test('invalid IDs cannot inject a protocol, query, path or command option', () => {
  for (const id of ['', ' ', 'javascript:alert(1)', '123?password=secret', '123/456', '123#token', '--password', '123@other-server', 'A'.repeat(129)]) {
    assert.equal(rustdeskLink(id), null, id)
  }
})


test('explicit hbbs links preserve the public key without account credentials', () => {
  const public_key = '//////////////////////////////////////////8='
  const link = rustdeskLink('295214626', { server: 'rustdesk.example.com:21116', public_key })!
  const parsed = new URL(link)
  assert.equal(parsed.pathname, '/295214626@rustdesk.example.com:21116')
  assert.equal(parsed.searchParams.get('key'), public_key)
  assert.deepEqual([...parsed.searchParams.keys()], ['key'])
  for (const server of ['user@host:21116', 'host:21116/path', 'host:21116?password=x', 'https://host:21116']) {
    assert.equal(rustdeskLink('123', { server, public_key }), null)
  }
  assert.equal(rustdeskLink('123', { server: 'host:21116', public_key: '' }), null)
})

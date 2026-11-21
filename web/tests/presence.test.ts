import assert from 'node:assert/strict'
import { test } from 'node:test'
import { presenceSummary } from '../src/utils/presence.ts'

test('only a live negative hbbs response is displayed as offline', () => {
  assert.equal(presenceSummary({ online: false, presence_source: 'hbbs' }).label, 'Offline')
  assert.equal(presenceSummary({ online: true, presence_source: 'hbbs' }).label, 'Online')
  assert.equal(presenceSummary({ online: false }).label, 'Unknown')
  assert.equal(presenceSummary({ online: false, presence_source: 'stale' }).label, 'Unknown')
})
test('API activity and query failure do not pretend to confirm hbbs presence', () => {
  assert.equal(presenceSummary({ online: true, presence_source: 'api_report' }).label, 'Recent activity')
  assert.equal(presenceSummary({ presence_source: 'probe_failed' }).tone, 'unknown')
  assert.match(presenceSummary({ presence_source: 'probe_failed' }).description, /does not mean.*offline/)
})
test('invalid or future API timestamps are never presented as a valid last report', () => {
  for (const last_seen of ['not-a-date', '2999-01-01 12:00:00']) {
    const presence = presenceSummary({ presence_source: 'stale', last_seen })
    assert.equal(presence.label, 'Unknown')
    assert.ok(!presence.description.includes('Last API report:'))
  }
})

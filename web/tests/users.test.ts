import assert from 'node:assert/strict'
import { after, test } from 'node:test'
import { createServer } from 'vite'
import { createSSRApp } from 'vue'
import { renderToString } from 'vue/server-renderer'
import { userDraft, userPayload } from '../src/utils/userEditor.ts'

const user = { id: 2, username: 'operator', name: 'Operator Name', email: 'operator@example.com',
  is_admin: false, status: 1, created_at: '' }

test('creating a user requires matching password confirmation and never sends confirmation', () => {
  const draft = { ...userDraft(), username: 'operator', password: 'a-secure-password', confirmation: '' }
  assert.throws(() => userPayload(draft), /do not match/)
  draft.confirmation = 'different-password'
  assert.throws(() => userPayload(draft), /do not match/)
  draft.confirmation = draft.password
  assert.deepEqual(userPayload(draft), { username: 'operator', name: '', email: '', is_admin: false, password: draft.password })
  draft.password = draft.confirmation = 'short'
  assert.throws(() => userPayload(draft), /12/)
})

test('editing a profile preserves password and sessions unless a change is requested', () => {
  const draft = userDraft(user)
  assert.equal(draft.password, '')
  draft.name = 'Updated Operator'
  const profile = userPayload(draft, user)
  assert.deepEqual(profile, { name: 'Updated Operator', email: user.email, is_admin: false })
  draft.status = 0
  assert.equal(userPayload(draft, user).status, 0)
  draft.password = 'replacement-password'
  assert.throws(() => userPayload(draft, user), /do not match/)
  draft.confirmation = draft.password
  assert.equal(userPayload(draft, user).password, draft.password)
})

const server = await createServer({ server: { middlewareMode: true, hmr: false, ws: false }, appType: 'custom', optimizeDeps: { noDiscovery: true, include: [] } })
after(() => server.close())
const { default: Editor } = await server.ssrLoadModule('/src/components/users/UserDialog.vue')
test('create and edit dialogs render confirmation, existing profile and optional password correctly', async () => {
  const create = await renderToString(createSSRApp(Editor, { busy: false, error: '' }))
  assert.match(create, /id="user-confirmation"[^>]*required/)
  assert.match(create, /id="user-password"[^>]*required/)
  const edit = await renderToString(createSSRApp(Editor, { user, busy: false, error: '' }))
  assert.ok(edit.includes('Edit user'))
  assert.ok(edit.includes('Updated Operator') === false)
  assert.ok(edit.includes('Operator Name'))
  assert.match(edit, /id="user-username"[^>]*readonly/)
  assert.doesNotMatch(edit, /id="user-password"[^>]*required/)
  assert.match(edit, /id="user-confirmation"/)
  assert.ok(edit.includes('Leave both password fields empty'))
})

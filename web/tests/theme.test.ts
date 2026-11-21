import assert from 'node:assert/strict'
import { test, type TestContext } from 'node:test'
import { createPinia, disposePinia } from 'pinia'
import { useUiStore } from '../src/stores/ui.ts'

function setup(t: TestContext, options: { stored?: string; systemDark?: boolean; blockStorage?: boolean } = {}) {
  const values = new Map<string, string>()
  if (options.stored !== undefined) values.set('theme', options.stored)
  const classes = new Set<string>()
  const root = {
    classList: {
      toggle(name: string, enabled: boolean) { if (enabled) classes.add(name); else classes.delete(name) },
      contains(name: string) { return classes.has(name) },
    },
    style: { colorScheme: '' },
  }
  const systemListeners = new Set<(event: { matches: boolean }) => void>()
  const storageListeners = new Set<(event: { key: string | null; newValue: string | null }) => void>()
  const media = {
    matches: options.systemDark ?? false,
    addEventListener(_type: string, listener: (event: { matches: boolean }) => void) { systemListeners.add(listener) },
    removeEventListener(_type: string, listener: (event: { matches: boolean }) => void) { systemListeners.delete(listener) },
  }
  const globals = {
    document: { documentElement: root },
    localStorage: {
      getItem(key: string) { if (options.blockStorage) throw new Error('Blocked'); return values.get(key) ?? null },
      setItem(key: string, value: string) { if (options.blockStorage) throw new Error('Blocked'); values.set(key, value) },
    },
    window: {
      matchMedia() { return media },
      addEventListener(_type: string, listener: (event: { key: string | null; newValue: string | null }) => void) { storageListeners.add(listener) },
      removeEventListener(_type: string, listener: (event: { key: string | null; newValue: string | null }) => void) { storageListeners.delete(listener) },
    },
  }
  const previous = new Map<string, PropertyDescriptor | undefined>()
  for (const [key, value] of Object.entries(globals)) {
    previous.set(key, Object.getOwnPropertyDescriptor(globalThis, key))
    Object.defineProperty(globalThis, key, { configurable: true, value })
  }
  const pinia = createPinia()
  const ui = useUiStore(pinia)
  t.after(() => {
    disposePinia(pinia)
    for (const [key, descriptor] of previous) {
      if (descriptor) Object.defineProperty(globalThis, key, descriptor)
      else Reflect.deleteProperty(globalThis, key)
    }
  })
  return {
    ui, root, values, systemListeners, storageListeners,
    system(dark: boolean) {
      media.matches = dark
      for (const listener of systemListeners) listener({ matches: dark })
    },
    storage(key: string | null, value: string | null) {
      for (const listener of storageListeners) listener({ key, newValue: value })
    },
  }
}

test('default automatic mode keeps its preference and follows live system changes', (t) => {
  const f = setup(t, { systemDark: true })
  assert.equal(f.ui.theme, 'system')
  assert.equal(f.values.get('theme'), 'system')
  assert.equal(f.ui.dark, true)
  assert.equal(f.root.classList.contains('dark'), true)
  f.system(false)
  assert.equal(f.ui.dark, false)
  assert.equal(f.root.classList.contains('dark'), false)
  assert.equal(f.root.style.colorScheme, 'light')
  assert.equal(f.values.get('theme'), 'system')
  f.system(true)
  assert.equal(f.root.classList.contains('dark'), true)
  assert.equal(f.root.style.colorScheme, 'dark')
})

test('manual light overrides a dark system and survives system changes', (t) => {
  const f = setup(t, { systemDark: true })
  f.ui.setTheme('light')
  f.system(false); f.system(true)
  assert.equal(f.ui.theme, 'light')
  assert.equal(f.ui.dark, false)
  assert.equal(f.root.classList.contains('dark'), false)
  assert.equal(f.values.get('theme'), 'light')
})

test('manual dark overrides a light system and switching back restores automatic behavior', (t) => {
  const f = setup(t)
  f.ui.setTheme('dark')
  assert.equal(f.root.classList.contains('dark'), true)
  assert.equal(f.values.get('theme'), 'dark')
  f.system(true); f.system(false)
  assert.equal(f.ui.dark, true)
  f.ui.setTheme('system')
  assert.equal(f.ui.dark, false)
  f.system(true)
  assert.equal(f.ui.dark, true)
  assert.equal(f.values.get('theme'), 'system')
})

test('existing light preference is restored even when the system is dark', (t) => {
  const f = setup(t, { stored: 'light', systemDark: true })
  assert.equal(f.ui.theme, 'light')
  assert.equal(f.ui.dark, false)
})

test('existing dark preference is restored even when the system is light', (t) => {
  const f = setup(t, { stored: 'dark' })
  assert.equal(f.ui.theme, 'dark')
  assert.equal(f.ui.dark, true)
})

test('unknown saved values fall back to the system preference', (t) => {
  const f = setup(t, { stored: 'automatic', systemDark: true })
  assert.equal(f.ui.theme, 'system')
  f.system(false)
  assert.equal(f.ui.dark, false)
})

test('blocked storage does not prevent selecting and applying a theme', (t) => {
  const f = setup(t, { blockStorage: true })
  assert.equal(f.ui.theme, 'system')
  f.ui.setTheme('dark')
  assert.equal(f.ui.dark, true)
  assert.equal(f.root.style.colorScheme, 'dark')
})

test('other tabs synchronize preferences, unrelated keys are ignored and clearing resets to system', (t) => {
  const f = setup(t)
  f.storage('theme', 'dark')
  assert.equal(f.ui.theme, 'dark')
  assert.equal(f.root.classList.contains('dark'), true)
  f.storage('token', null)
  assert.equal(f.ui.theme, 'dark')
  f.storage(null, null)
  assert.equal(f.ui.theme, 'system')
  assert.equal(f.ui.dark, false)
})

test('disposing the store removes system and cross-tab listeners', (t) => {
  const f = setup(t)
  assert.equal(f.systemListeners.size, 1)
  assert.equal(f.storageListeners.size, 1)
  f.ui.$dispose()
  assert.equal(f.systemListeners.size, 0)
  assert.equal(f.storageListeners.size, 0)
})

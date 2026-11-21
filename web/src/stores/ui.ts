import { defineStore } from 'pinia'
import { computed, onScopeDispose, ref, watch } from 'vue'

export type ThemeMode = 'system' | 'light' | 'dark'

function normalizeTheme(value: string | null): ThemeMode {
  return value === 'light' || value === 'dark' ? value : 'system'
}

function savedTheme(): ThemeMode {
  try { return normalizeTheme(localStorage.getItem('theme')) }
  catch { return 'system' }
}

export const useUiStore = defineStore('ui', () => {
  const theme = ref<ThemeMode>(savedTheme())
  const media = window.matchMedia('(prefers-color-scheme: dark)')
  const systemDark = ref(media.matches)
  const dark = computed(() => theme.value === 'dark' || (theme.value === 'system' && systemDark.value))

  function updateSystem(event: MediaQueryListEvent) { systemDark.value = event.matches }
  function syncPreference(event: StorageEvent) {
    if (event.key === 'theme' || event.key === null) theme.value = normalizeTheme(event.newValue)
  }
  media.addEventListener('change', updateSystem)
  window.addEventListener('storage', syncPreference)
  onScopeDispose(() => {
    media.removeEventListener('change', updateSystem)
    window.removeEventListener('storage', syncPreference)
  })

  watch(theme, (value) => {
    // Keep the preference, not its current resolved color. Storage can be blocked.
    try { localStorage.setItem('theme', value) } catch { /* In-memory preference still works. */ }
  }, { immediate: true, flush: 'sync' })
  watch(dark, (value) => {
    document.documentElement.classList.toggle('dark', value)
    document.documentElement.style.colorScheme = value ? 'dark' : 'light'
  }, { immediate: true, flush: 'sync' })

  function setTheme(value: ThemeMode) { theme.value = normalizeTheme(value) }
  return { theme, dark, setTheme }
})

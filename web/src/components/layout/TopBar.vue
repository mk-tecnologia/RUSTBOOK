<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useAuthStore } from '../../stores/auth'
import { useUiStore, type ThemeMode } from '../../stores/ui'
import { getServerVersion } from '../../api/system'
import { useRouter } from 'vue-router'

const auth = useAuthStore()
const ui = useUiStore()
const router = useRouter()
const serverVersion = ref<string | null>(null)
const versionPending = ref(true)

onMounted(async () => {
  try { serverVersion.value = await getServerVersion() }
  catch { serverVersion.value = null }
  finally { versionPending.value = false }
})

function changeTheme(event: Event) {
  ui.setTheme((event.target as HTMLSelectElement).value as ThemeMode)
}

async function handleLogout() {
  await auth.logout()
  router.push('/login')
}
</script>

<template>
  <header class="min-h-14 shrink-0 flex flex-wrap items-center gap-2 border-b border-gray-200 dark:border-rd-border bg-white dark:bg-rd-card px-4 py-2 md:px-6">
    <slot name="navigation" />
    <div class="min-w-0 flex-1 text-sm text-gray-500 dark:text-rd-text-secondary">
      <slot name="title" />
    </div>
    <div class="ml-auto flex max-w-full flex-wrap items-center justify-end gap-2">
      <span
        class="rounded-md border border-gray-200 dark:border-rd-border px-2 py-1 text-xs font-mono text-gray-600 dark:text-rd-text-secondary"
        :title="serverVersion ? 'Running server version' : 'Running server version could not be confirmed'"
        aria-live="polite"
      >{{ serverVersion ? `v${serverVersion}` : versionPending ? 'Version…' : 'Version unavailable' }}</span>

      <div class="flex items-center gap-1 text-gray-600 dark:text-rd-text-secondary">
        <svg v-if="ui.theme === 'system'" aria-hidden="true" class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4h16v12H4zM8 20h8M12 16v4" />
        </svg>
        <svg v-else-if="ui.theme === 'light'" aria-hidden="true" class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 3v1m0 16v1m9-9h-1M4 12H3m15.364 6.364l-.707.707M6.343 6.343l-.707-.707m12.728 0l-.707.707M6.343 17.657l-.707.707M16 12a4 4 0 11-8 0 4 4 0 018 0z" />
        </svg>
        <svg v-else aria-hidden="true" class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20.354 15.354A9 9 0 018.646 3.646 9.003 9.003 0 0012 21a9.003 9.003 0 008.354-5.646z" />
        </svg>
        <label for="theme-mode" class="sr-only">Theme preference</label>
        <select
          id="theme-mode"
          :value="ui.theme"
          @change="changeTheme"
          title="Theme preference"
          class="rounded-lg border border-gray-300 dark:border-rd-border bg-white dark:bg-rd-card px-2 py-1.5 text-sm text-gray-700 dark:text-rd-text"
        >
          <option value="system">System</option>
          <option value="light">Light</option>
          <option value="dark">Dark</option>
        </select>
      </div>

      <button
        @click="handleLogout"
        class="px-3 py-1.5 text-sm rounded-lg text-gray-600 dark:text-rd-text-secondary hover:bg-gray-100 dark:hover:bg-rd-card-hover transition-colors"
      >Logout</button>
    </div>
  </header>
</template>

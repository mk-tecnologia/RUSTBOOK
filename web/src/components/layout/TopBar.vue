<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { useAuthStore } from '../../stores/auth'
import ThemeSwitcher from './ThemeSwitcher.vue'
import { getServerVersion, type ServerVersion } from '../../api/system'
import { useRouter } from 'vue-router'

const auth = useAuthStore()
const router = useRouter()
const serverVersion = ref<ServerVersion | null>(null)
const versionPending = ref(true)
const versionLabel = computed(() => {
  if (!serverVersion.value) return versionPending.value ? 'Version…' : 'Version unavailable'
  const { version, commit } = serverVersion.value
  return `v${version}${commit ? ` · ${commit.slice(0, 7)}` : ''}`
})
const versionTitle = computed(() => serverVersion.value
  ? `Running server: v${serverVersion.value.version}${serverVersion.value.commit ? ` (${serverVersion.value.commit})` : ''}`
  : 'Running server version could not be confirmed')
let refreshTimer: ReturnType<typeof setInterval> | undefined
let requestPending = false

async function refreshVersion() {
  if (requestPending) return
  requestPending = true
  try { serverVersion.value = await getServerVersion() }
  catch { serverVersion.value = null }
  finally { versionPending.value = false; requestPending = false }
}

onMounted(() => {
  void refreshVersion()
  refreshTimer = setInterval(refreshVersion, 60_000)
  window.addEventListener('focus', refreshVersion)
})
onUnmounted(() => {
  clearInterval(refreshTimer)
  window.removeEventListener('focus', refreshVersion)
})

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
        :title="versionTitle"
        aria-live="polite"
      >{{ versionLabel }}</span>

      <ThemeSwitcher />

      <button
        @click="handleLogout"
        class="px-3 py-1.5 text-sm rounded-lg text-gray-600 dark:text-rd-text-secondary hover:bg-gray-100 dark:hover:bg-rd-card-hover transition-colors"
      >Logout</button>
    </div>
  </header>
</template>

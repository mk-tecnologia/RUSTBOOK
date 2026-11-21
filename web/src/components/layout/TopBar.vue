<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useAuthStore } from '../../stores/auth'
import ThemeSwitcher from './ThemeSwitcher.vue'
import { getServerVersion } from '../../api/system'
import { useRouter } from 'vue-router'

const auth = useAuthStore()
const router = useRouter()
const serverVersion = ref<string | null>(null)
const versionPending = ref(true)

onMounted(async () => {
  try { serverVersion.value = await getServerVersion() }
  catch { serverVersion.value = null }
  finally { versionPending.value = false }
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
        :title="serverVersion ? 'Running server version' : 'Running server version could not be confirmed'"
        aria-live="polite"
      >{{ serverVersion ? `v${serverVersion}` : versionPending ? 'Version…' : 'Version unavailable' }}</span>

      <ThemeSwitcher />

      <button
        @click="handleLogout"
        class="px-3 py-1.5 text-sm rounded-lg text-gray-600 dark:text-rd-text-secondary hover:bg-gray-100 dark:hover:bg-rd-card-hover transition-colors"
      >Logout</button>
    </div>
  </header>
</template>

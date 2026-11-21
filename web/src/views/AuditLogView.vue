<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { api } from '../api/client'
import PaginationControls from '../components/ui/PaginationControls.vue'
import ErrorNotice from '../components/ui/ErrorNotice.vue'

interface AuditEntry {
  id: number
  action: string
  rustdesk_id: string
  peer_id: string
  ip: string
  note: string
  created_at: string
}

const entries = ref<AuditEntry[]>([])

const page = ref(1)
const total = ref(0)
const loading = ref(false)
const error = ref('')
async function load(nextPage = page.value) {
  loading.value = true; error.value = ''
  try {
    const params = new URLSearchParams({ current: String(nextPage), pageSize: '50' })
    const res = await api<{ data: AuditEntry[]; total: number }>(`/api/audit-log?${params}`)
    entries.value = res.data; total.value = res.total; page.value = nextPage
  } catch (e) { error.value = e instanceof Error ? e.message : 'Unable to load audit log' }
  finally { loading.value = false }
}
onMounted(() => load())

function actionBadgeClass(action: string) {
  switch (action) {
    case 'login': return 'bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400'
    case 'connect': return 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400'
    case 'file_transfer': return 'bg-yellow-100 text-yellow-700 dark:bg-yellow-900/30 dark:text-yellow-400'
    default: return 'bg-gray-100 text-gray-600 dark:bg-rd-border dark:text-rd-text-secondary'
  }
}
</script>

<template>
  <div>
    <h1 class="text-lg font-semibold text-gray-900 dark:text-rd-text mb-6">Audit Log</h1>

    <ErrorNotice :message="error" retry @retry="load()" />
    <p v-if="loading" role="status" class="text-sm py-4">Loading audit log…</p>
    <div v-else-if="!error" class="bg-white dark:bg-rd-card border border-gray-200 dark:border-rd-border rounded-xl overflow-x-auto">
      <table class="w-full">
        <thead>
          <tr class="border-b border-gray-200 dark:border-rd-border">
            <th class="text-left text-xs font-medium text-gray-500 dark:text-rd-text-secondary uppercase tracking-wide px-6 py-3">Time</th>
            <th class="text-left text-xs font-medium text-gray-500 dark:text-rd-text-secondary uppercase tracking-wide px-6 py-3">Action</th>
            <th class="text-left text-xs font-medium text-gray-500 dark:text-rd-text-secondary uppercase tracking-wide px-6 py-3">Device ID</th>
            <th class="text-left text-xs font-medium text-gray-500 dark:text-rd-text-secondary uppercase tracking-wide px-6 py-3">Peer ID</th>
            <th class="text-left text-xs font-medium text-gray-500 dark:text-rd-text-secondary uppercase tracking-wide px-6 py-3">IP</th>
            <th class="text-left text-xs font-medium text-gray-500 dark:text-rd-text-secondary uppercase tracking-wide px-6 py-3">Note</th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="entry in entries"
            :key="entry.id"
            class="border-b border-gray-100 dark:border-rd-border/50 last:border-0"
          >
            <td class="px-6 py-3 text-xs text-gray-500 dark:text-rd-text-secondary whitespace-nowrap">{{ entry.created_at }}</td>
            <td class="px-6 py-3">
              <span class="text-xs px-2 py-0.5 rounded" :class="actionBadgeClass(entry.action)">{{ entry.action }}</span>
            </td>
            <td class="px-6 py-3 text-sm text-gray-900 dark:text-rd-text font-mono">{{ entry.rustdesk_id || '-' }}</td>
            <td class="px-6 py-3 text-sm text-gray-600 dark:text-rd-text-secondary font-mono">{{ entry.peer_id || '-' }}</td>
            <td class="px-6 py-3 text-sm text-gray-600 dark:text-rd-text-secondary font-mono">{{ entry.ip || '-' }}</td>
            <td class="px-6 py-3 text-sm text-gray-600 dark:text-rd-text-secondary">{{ entry.note || '-' }}</td>
          </tr>
        </tbody>
      </table>
      <div v-if="!entries.length" class="text-center py-10 text-sm text-gray-400 dark:text-rd-text-secondary">
        No audit entries yet. Entries will appear as devices connect.
      </div>
    </div>
    <PaginationControls :page="page" :page-size="50" :total="total" :loading="loading" @change="load" />
  </div>
</template>

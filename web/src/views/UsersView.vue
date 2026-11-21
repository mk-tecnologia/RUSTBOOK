<script setup lang="ts">
import { ref, onMounted } from 'vue'
import UserDialog from '../components/users/UserDialog.vue'
import type { userPayload } from '../utils/userEditor'
import ErrorNotice from '../components/ui/ErrorNotice.vue'
import { useAsyncTask } from '../composables/useAsyncTask'
import * as abApi from '../api/addressBook'

const users = ref<abApi.UserItem[]>([])
const showEditor = ref(false)
const editing = ref<abApi.UserItem>()
const editorError = ref('')
const { pending, error, run } = useAsyncTask()

async function fetchUsers() {
  const res = await abApi.getUsers()
  users.value = res.data
}
async function loadUsers() { await run(fetchUsers) }

function openEditor(user?: abApi.UserItem) {
  editing.value = user
  editorError.value = ''
  showEditor.value = true
}
async function saveUser(payload: ReturnType<typeof userPayload>) {
  await run(async () => {
    editorError.value = ''
    try {
      if (editing.value) await abApi.updateUser(editing.value.id, payload)
      else await abApi.createUser(payload as Parameters<typeof abApi.createUser>[0])
    } catch (e) {
      editorError.value = e instanceof Error ? e.message : 'Unable to save user'
      return
    }
    showEditor.value = false
    editing.value = undefined
    await fetchUsers()
  })
}

async function handleDelete(id: number, username: string) {
  if (confirm(`Delete user "${username}"?`)) {
    await run(async () => { await abApi.deleteUser(id); await fetchUsers() })
  }
}

async function toggleStatus(user: abApi.UserItem) {
  await run(async () => { await abApi.updateUser(user.id, { status: user.status === 1 ? 0 : 1 }); await fetchUsers() })
}

onMounted(loadUsers)
</script>

<template>
  <div>
    <div class="flex items-center justify-between mb-6">
      <h1 class="text-lg font-semibold text-gray-900 dark:text-rd-text">Users</h1>
      <button :disabled="pending"
        @click="openEditor()"
        class="px-4 py-2 text-sm rounded-lg bg-rd-primary hover:bg-rd-primary-hover text-white font-medium transition-colors flex items-center gap-2"
      >
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
        </svg>
        Add User
      </button>
    </div>

    <ErrorNotice :message="error" retry @retry="loadUsers" />
    <p v-if="pending" role="status" class="my-3 text-sm text-gray-500">Working…</p>

    <UserDialog v-if="showEditor" :user="editing" :busy="pending" :error="editorError" @close="showEditor = false; editing = undefined" @save="saveUser" />

    <!-- Users table -->
    <div class="bg-white dark:bg-rd-card border border-gray-200 dark:border-rd-border rounded-xl overflow-x-auto">
      <table class="w-full">
        <thead>
          <tr class="border-b border-gray-200 dark:border-rd-border">
            <th class="text-left text-xs font-medium text-gray-500 dark:text-rd-text-secondary uppercase tracking-wide px-6 py-3">Username</th>
            <th class="text-left text-xs font-medium text-gray-500 dark:text-rd-text-secondary uppercase tracking-wide px-6 py-3">Name</th>
            <th class="text-left text-xs font-medium text-gray-500 dark:text-rd-text-secondary uppercase tracking-wide px-6 py-3">Email</th>
            <th class="text-left text-xs font-medium text-gray-500 dark:text-rd-text-secondary uppercase tracking-wide px-6 py-3">Role</th>
            <th class="text-left text-xs font-medium text-gray-500 dark:text-rd-text-secondary uppercase tracking-wide px-6 py-3">Status</th>
            <th class="text-right text-xs font-medium text-gray-500 dark:text-rd-text-secondary uppercase tracking-wide px-6 py-3">Actions</th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="user in users"
            :key="user.id"
            class="border-b border-gray-100 dark:border-rd-border/50 last:border-0 hover:bg-gray-50 dark:hover:bg-rd-card-hover"
          >
            <td class="px-6 py-3 text-sm text-gray-900 dark:text-rd-text font-medium">{{ user.username }}</td>
            <td class="px-6 py-3 text-sm text-gray-600 dark:text-rd-text-secondary">{{ user.name || '-' }}</td>
            <td class="px-6 py-3 text-sm text-gray-600 dark:text-rd-text-secondary">{{ user.email || '-' }}</td>
            <td class="px-6 py-3">
              <span class="text-xs px-2 py-0.5 rounded" :class="user.is_admin ? 'bg-purple-100 text-purple-700 dark:bg-purple-900/30 dark:text-purple-400' : 'bg-gray-100 text-gray-600 dark:bg-rd-border dark:text-rd-text-secondary'">
                {{ user.is_admin ? 'Admin' : 'User' }}
              </span>
            </td>
            <td class="px-6 py-3">
              <span class="text-xs px-2 py-0.5 rounded" :class="user.status === 1 ? 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400' : 'bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400'">
                {{ user.status === 1 ? 'Active' : 'Disabled' }}
              </span>
            </td>
            <td class="px-6 py-3">
              <div class="flex justify-end gap-1 whitespace-nowrap">
                <button :disabled="pending" type="button" :aria-label="`Edit user ${user.username}`" title="Edit user" @click="openEditor(user)" class="user-action text-blue-600 hover:bg-blue-50 dark:text-blue-400 dark:hover:bg-blue-500/10"><svg aria-hidden="true" class="size-4" fill="none" stroke="currentColor" stroke-width="1.6" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="m16 4 4 4M4 20l4-1L20 7a2.8 2.8 0 0 0-4-4L4 15l-1 6Z" /></svg></button>
                <button :disabled="pending" type="button" :aria-label="`${user.status === 1 ? 'Disable' : 'Enable'} user ${user.username}`" :title="user.status === 1 ? 'Disable user' : 'Enable user'" @click="toggleStatus(user)" class="user-action" :class="user.status === 1 ? 'text-amber-600 hover:bg-amber-50 dark:text-amber-400 dark:hover:bg-amber-500/10' : 'text-emerald-600 hover:bg-emerald-50 dark:text-emerald-400 dark:hover:bg-emerald-500/10'"><svg aria-hidden="true" class="size-4" fill="none" stroke="currentColor" stroke-width="1.6" viewBox="0 0 24 24"><path stroke-linecap="round" d="M12 3v9m-5-7a8 8 0 1 0 10 0" /></svg></button>
                <button :disabled="pending" type="button" :aria-label="`Delete user ${user.username}`" title="Delete user" @click="handleDelete(user.id, user.username)" class="user-action text-red-600 hover:bg-red-50 dark:text-red-400 dark:hover:bg-red-500/10"><svg aria-hidden="true" class="size-4" fill="none" stroke="currentColor" stroke-width="1.6" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M4 7h16M9 7V4h6v3m-9 0 1 13h10l1-13M10 11v5m4-5v5" /></svg></button>
              </div>
            </td>
          </tr>
        </tbody>
      </table>
      <div v-if="!pending && !error && !users.length" class="text-center py-10 text-sm text-gray-400 dark:text-rd-text-secondary">
        No users found
      </div>
    </div>
  </div>
</template>

<style scoped>
@reference "../style.css";
.user-action {
  @apply inline-flex size-9 shrink-0 items-center justify-center rounded-lg transition-colors focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-current disabled:cursor-not-allowed disabled:opacity-40 motion-reduce:transition-none;
}
</style>

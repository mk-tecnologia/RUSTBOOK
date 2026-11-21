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

async function toggleAdmin(user: abApi.UserItem) {
  await run(async () => { await abApi.updateUser(user.id, { is_admin: !user.is_admin }); await fetchUsers() })
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
              <button :disabled="pending" @click="toggleAdmin(user)" class="text-xs px-2 py-0.5 rounded" :class="user.is_admin ? 'bg-purple-100 text-purple-700 dark:bg-purple-900/30 dark:text-purple-400' : 'bg-gray-100 text-gray-600 dark:bg-rd-border dark:text-rd-text-secondary'">
                {{ user.is_admin ? 'Admin' : 'User' }}
              </button>
            </td>
            <td class="px-6 py-3">
              <button :disabled="pending" @click="toggleStatus(user)" class="text-xs px-2 py-0.5 rounded" :class="user.status === 1 ? 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400' : 'bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400'">
                {{ user.status === 1 ? 'Active' : 'Disabled' }}
              </button>
            </td>
            <td class="px-6 py-3 text-right whitespace-nowrap">
              <button :disabled="pending" type="button" :aria-label="`Edit user ${user.username}`" @click="openEditor(user)" class="mr-3 text-sm font-medium text-rd-primary hover:text-rd-primary-hover">Edit</button>
              <button :disabled="pending"
                @click="handleDelete(user.id, user.username)"
                class="text-sm text-gray-400 hover:text-rd-danger transition-colors"
                title="Delete user"
              >
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                </svg>
              </button>
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

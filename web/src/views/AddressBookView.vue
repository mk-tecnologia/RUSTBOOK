<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, watch } from 'vue'
import { useAddressBookStore } from '../stores/addressBook'
import PeerGrid from '../components/peers/PeerGrid.vue'
import TagPanel from '../components/tags/TagPanel.vue'
import AddPeerDialog from '../components/peers/AddPeerDialog.vue'
import PaginationControls from '../components/ui/PaginationControls.vue'
import ErrorNotice from '../components/ui/ErrorNotice.vue'

const store = useAddressBookStore()
const showAddPeer = ref(false)
const search = ref(store.query)
const saving = ref(false)
const actionError = ref('')
let timer: ReturnType<typeof setTimeout> | undefined
onMounted(store.init)
onBeforeUnmount(() => clearTimeout(timer))
watch(search, () => {
  clearTimeout(timer)
  timer = setTimeout(() => store.setFilters(search.value, store.selectedTag), 250)
})
async function selectTag(tag: string | null) {
  clearTimeout(timer)
  await store.setFilters(search.value, tag)
}
async function action(work: () => Promise<void>) {
  if (saving.value) return
  actionError.value = ''; saving.value = true
  try { await work() } catch (e) { actionError.value = e instanceof Error ? e.message : 'Operation failed' }
  finally { saving.value = false }
}
async function handleAddPeer(peer: { id: string; alias: string; platform: string; tags: string[] }) {
  await action(async () => { await store.addPeer(peer); showAddPeer.value = false })
}
async function handleDeletePeer(id: string) {
  if (confirm(`Remove peer ${id}?`)) await action(() => store.removePeer([id]))
}
async function handleAddTag(name: string, color: number) { await action(() => store.addTag(name, color)) }
async function handleDeleteTag(name: string) {
  if (confirm(`Delete tag "${name}"?`)) await action(() => store.removeTag([name]))
}
</script>

<template>
  <div class="flex flex-col lg:flex-row gap-6">
    <!-- Tag sidebar -->
    <div class="w-full lg:w-48 shrink-0">
      <div class="bg-white dark:bg-rd-card border border-gray-200 dark:border-rd-border rounded-xl p-4">
        <TagPanel
          :tags="store.tags"
          :selected-tag="store.selectedTag"
          @select="selectTag"
          @add="handleAddTag"
          @delete="handleDeleteTag"
        />
      </div>
    </div>

    <!-- Main content -->
    <div class="flex-1 min-w-0">
      <!-- Toolbar -->
      <div class="flex flex-wrap items-center justify-between gap-3 mb-4">
        <div class="flex flex-wrap items-center gap-3">
          <h1 class="text-lg font-semibold text-gray-900 dark:text-rd-text">Address Book</h1>
          <span class="text-sm text-gray-500 dark:text-rd-text-secondary">{{ store.totalPeers }} peers</span>
        </div>
        <div class="flex flex-wrap items-center gap-3">
          <!-- Search -->
          <div class="relative">
            <svg class="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
            </svg>
            <input
              v-model="search"
              aria-label="Search peers"
              maxlength="128"
              type="search"
              placeholder="Search peers..."
              class="pl-9 pr-3 py-2 text-sm rounded-lg border border-gray-300 dark:border-rd-border bg-white dark:bg-rd-card text-gray-900 dark:text-rd-text focus:outline-none focus:ring-2 focus:ring-rd-primary w-56"
            />
          </div>
          <!-- Add button -->
          <button
            @click="showAddPeer = true"
            class="px-4 py-2 text-sm rounded-lg bg-rd-primary hover:bg-rd-primary-hover text-white font-medium transition-colors flex items-center gap-2"
          >
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
            </svg>
            Add Peer
          </button>
        </div>
      </div>

      <ErrorNotice :message="actionError" />
      <ErrorNotice :message="store.error" retry @retry="store.init" />
      <!-- Peer grid -->
      <div v-if="store.loading" class="flex items-center justify-center py-20">
        <div class="w-8 h-8 border-2 border-rd-primary border-t-transparent rounded-full animate-spin" />
      </div>
      <PeerGrid v-else-if="!store.error" :peers="store.peers" @delete="handleDeletePeer" />
      <PaginationControls :page="store.page" :page-size="store.pageSize" :total="store.totalPeers" :loading="store.loading" @change="(page) => { store.page = page; store.loadPeers() }" />
    </div>
  </div>

  <!-- Add peer dialog -->
  <AddPeerDialog v-if="showAddPeer" :busy="saving" :server-error="actionError" @close="showAddPeer = false" @add="handleAddPeer" />
</template>
